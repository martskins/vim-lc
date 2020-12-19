pub use super::types::*;
use crate::rpc::RPCClient;
use crate::{config, lsp::Context};
use failure::Fallible;
use lsp_types::{CodeAction, CodeActionOrCommand};
use serde::de::DeserializeOwned;

pub fn getbufvar<T: DeserializeOwned, C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    var: &str,
) -> Fallible<T> {
    let val: T = ctx
        .vim
        .call("getbufvar", serde_json::json!([ctx.bufnr, var]))?;
    Ok(val)
}

pub fn apply_text_edits<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    filename: &str,
    edits: &[lsp_types::TextEdit],
) -> Fallible<()> {
    // let changes: &lsp_types::DocumentChanges = &edits.document_changes.as_ref().unwrap();
    let changes: Vec<BufChanges> = edits
        .into_iter()
        .map(|e| BufChanges {
            start: e.range.start.into(),
            end: e.range.end.into(),
            lines: vec![e.new_text.clone()],
        })
        .collect();

    let changes = DocumentChanges {
        text_document: filename.to_string(),
        changes,
    };

    ctx.vim
        .notify("vim#apply_edit", serde_json::json!([changes]))?;
    Ok(())
}

pub fn apply_workspace_edit<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    edits: &lsp_types::WorkspaceEdit,
) -> Fallible<()> {
    let changes: &lsp_types::DocumentChanges = &edits.document_changes.as_ref().unwrap();
    let changes: Vec<DocumentChanges> = match changes {
        lsp_types::DocumentChanges::Edits(edits) => edits
            .into_iter()
            .map(|tde| {
                let tde: lsp_types::TextDocumentEdit = tde.clone();
                let text_document = tde
                    .text_document
                    .uri
                    .to_string()
                    .replace(ctx.root_path.as_str(), "");

                DocumentChanges {
                    text_document,
                    changes: tde
                        .edits
                        .into_iter()
                        .map(|e| BufChanges {
                            start: e.range.start.into(),
                            end: e.range.end.into(),
                            lines: vec![e.new_text],
                        })
                        .collect(),
                }
            })
            .collect(),
        lsp_types::DocumentChanges::Operations(_) => vec![],
    };

    ctx.vim
        .notify("vim#apply_edits", serde_json::json!([changes]))?;
    Ok(())
}

pub fn show_diagnostics<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    mut diagnostics: Vec<Diagnostic>,
) -> Fallible<()> {
    diagnostics.iter_mut().for_each(|d| {
        d.text_document = d.text_document.replace(ctx.root_path.as_str(), "");
    });

    let quickfix_list: Vec<QuickfixItem> =
        diagnostics.clone().into_iter().map(|l| l.into()).collect();
    set_quickfix(ctx, quickfix_list)?;

    if ctx.config.diagnostics.show_signs {
        let signs: Vec<Sign> = diagnostics.into_iter().map(|l| l.into()).collect();
        set_signs(ctx, signs)?;
    }

    Ok(())
}

pub fn show_hover<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: lsp_types::Hover,
) -> Fallible<()> {
    let filetype = match input.contents {
        lsp_types::HoverContents::Scalar(ref c) => match &c {
            lsp_types::MarkedString::String(_) => String::new(),
            lsp_types::MarkedString::LanguageString(s) => s.language.clone(),
        },
        lsp_types::HoverContents::Array(ref c) => {
            if c.is_empty() {
                String::new()
            } else {
                match c[0].clone() {
                    lsp_types::MarkedString::String(_) => String::new(),
                    lsp_types::MarkedString::LanguageString(s) => s.language,
                }
            }
        }
        lsp_types::HoverContents::Markup(ref c) => match &c.kind {
            lsp_types::MarkupKind::Markdown => "markdown".into(),
            lsp_types::MarkupKind::PlainText => String::new(),
        },
    };

    let lines = match input.contents {
        lsp_types::HoverContents::Scalar(ref c) => match c.clone() {
            lsp_types::MarkedString::String(s) => s.split('\n').map(|s| s.to_owned()).collect(),
            lsp_types::MarkedString::LanguageString(s) => {
                s.value.split('\n').map(|s| s.to_owned()).collect()
            }
        },
        lsp_types::HoverContents::Array(ref c) => {
            if c.is_empty() {
                vec![]
            } else {
                match c[0].clone() {
                    lsp_types::MarkedString::String(s) => {
                        s.split('\n').map(|s| s.to_owned()).collect()
                    }
                    lsp_types::MarkedString::LanguageString(s) => {
                        s.value.split('\n').map(|s| s.to_owned()).collect()
                    }
                }
            }
        }
        lsp_types::HoverContents::Markup(c) => c.value.split('\n').map(|s| s.to_owned()).collect(),
    };

    match ctx.config.hover.strategy {
        config::DisplayMode::Preview => {
            ctx.vim.notify(
                "vim#show_preview",
                serde_json::json!([PreviewContent { filetype, lines }]),
            )?;
        }
        config::DisplayMode::FloatingWindow => {
            ctx.vim.notify(
                "vim#show_float_win",
                serde_json::json!([PreviewContent { filetype, lines }]),
            )?;
        }
    }
    Ok(())
}

pub fn setloclist<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    items: Vec<LocationItem>,
) -> Fallible<()> {
    ctx.vim
        .notify("vim#show_locations", serde_json::json!([items, ""]))?;

    Ok(())
}

pub fn selection<C: RPCClient, S: RPCClient, I: ListItem>(
    ctx: &Context<C, S>,
    items: Vec<I>,
) -> Fallible<()> {
    let text: Vec<String> = items.into_iter().map(|i| i.text()).collect();
    let sink = I::sink();
    ctx.vim
        .notify("vim#selection", serde_json::json!([text, sink]))?;

    Ok(())
}

pub async fn show_locations<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: Vec<Location>,
) -> Fallible<()> {
    if input.is_empty() {
        return Ok(());
    }

    if input.len() == 1 {
        return jump_to_location(ctx, input.first().cloned().unwrap());
    }

    let locations: Vec<_> = input
        .into_iter()
        .map(|l| async move {
            let filename = l.filename.replace(ctx.root_path.as_str(), "");
            let text = crate::vim::get_line(ctx, &filename, l.position.line)
                .await
                .unwrap_or_default();
            LocationItem {
                filename,
                lnum: l.position.line,
                col: l.position.column,
                text,
            }
        })
        .collect();

    let locations = futures::future::join_all(locations).await;
    setloclist(ctx, locations)?;
    Ok(())
}

pub fn jump_to_location<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: Location,
) -> Fallible<()> {
    execute(
        ctx,
        vec![
            ExecuteParams {
                action: "execute".into(),
                command: format!(
                    "execute 'edit' '{}'",
                    input.filename.replace(ctx.root_path.as_str(), "")
                ),
            },
            ExecuteParams {
                action: "call".into(),
                command: format!("cursor({}, {})", input.position.line, input.position.column),
            },
        ],
    )?;
    Ok(())
}

// evaluates multiple commands and returns a vec of values.
pub fn execute<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    cmd: Vec<ExecuteParams>,
) -> Fallible<Vec<serde_json::Value>> {
    let res: Vec<serde_json::Value> = ctx.vim.call("execute", cmd)?;
    Ok(res)
}

pub fn set_signs<C: RPCClient, S: RPCClient>(ctx: &Context<C, S>, list: Vec<Sign>) -> Fallible<()> {
    ctx.vim.notify("vim#set_signs", serde_json::json!([list]))?;
    Ok(())
}

pub fn set_quickfix<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    list: Vec<QuickfixItem>,
) -> Fallible<()> {
    ctx.vim
        .notify("vim#set_quickfix", serde_json::json!([list]))?;
    Ok(())
}

pub fn log_message<C: RPCClient, S: RPCClient>(
    _ctx: &Context<C, S>,
    params: lsp_types::LogMessageParams,
) -> Fallible<()> {
    log::debug!("{}", params.message);
    Ok(())
}

pub fn show_message<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    message: Message,
) -> Fallible<()> {
    ctx.vim
        .notify("vim#show_message", serde_json::json!([message]))?;
    Ok(())
}

pub async fn get_line<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    filename: &str,
    line_number: u64,
) -> Fallible<String> {
    let state = ctx.state.read();
    let text_document = state.text_documents.get(filename).cloned();
    drop(state);

    let idx = line_number as usize - 1;
    match text_document {
        Some((_, lines)) => Ok(lines[idx].clone()),
        None => {
            use tokio::io::AsyncReadExt;
            let mut file = tokio::fs::File::open(filename).await?;
            let mut text = String::new();
            file.read_to_string(&mut text).await?;
            let lines: Vec<&str> = text.split('\n').collect();
            let line = lines[idx].to_owned();
            Ok(line)
        }
    }
}

pub async fn resolve_code_lens_action<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: ResolveCodeActionParams,
) -> Fallible<()> {
    let state = ctx.state.read();
    let code_lens = state.code_lens.get(&input.position.text_document).cloned();
    drop(state);

    if code_lens.is_none() {
        return Ok(());
    }

    let code_lens = code_lens.as_ref().unwrap().get(input.selection);
    match code_lens {
        None => {}
        Some(code_lens) => {
            let response = crate::lsp::code_lens::resolve(ctx, code_lens)?;
            if code_lens.command.is_none() {
                return Ok(());
            }

            log::error!("{:?}", code_lens);
            crate::lsp::extensions::run_command(ctx, response.command.as_ref().unwrap()).await?;
        }
    }

    Ok(())
}

pub async fn resolve_code_action<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: ResolveCodeActionParams,
) -> Fallible<()> {
    let state = ctx.state.read();
    let code_actions = state.code_actions.clone();
    drop(state);

    let action = code_actions.get(input.selection);
    match action {
        None => {}
        Some(action) => match action {
            CodeActionOrCommand::CodeAction(action) => {
                let action: &CodeAction = action;
                if action.command.is_none() {
                    log::error!("action has no command: {:?}", action);
                    return Ok(());
                }

                crate::lsp::extensions::run_command(ctx, &action.command.as_ref().unwrap()).await?;
            }
            CodeActionOrCommand::Command(command) => {
                crate::lsp::extensions::run_command(ctx, command).await?;
            }
        },
    }

    ctx.state.write().code_actions = vec![];

    Ok(())
}
