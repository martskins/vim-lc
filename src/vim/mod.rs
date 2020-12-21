mod types;

use crate::{config, language_client::Context};
use crate::{
    language_client::LanguageClient,
    rpc::{self, RPCClient},
};
use anyhow::Result;
use jsonrpc_core::Params;
use lsp_types::{CodeAction, CodeActionOrCommand};
use serde::de::DeserializeOwned;
pub use types::*;

impl<C, S> LanguageClient<C, S>
where
    C: RPCClient,
    S: RPCClient,
{
    // handles messages sent from vim to the language client
    pub async fn handle_vim_message(&self, message: rpc::Message) -> Result<()> {
        // handle start separetely as not to try to create a context
        match message {
            rpc::Message::MethodCall(msg) if msg.method.as_str() == "start" => {
                let params: BufInfo = serde_json::from_value(msg.params.into())?;
                self.start_server(&params.language_id).await?;
                return Ok(());
            }
            _ => {}
        }

        let ctx = Context::new(&message, self);
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                "initialize" => {
                    crate::lsp::initialize(&ctx).await?;
                    crate::lsp::initialized(&ctx)?;
                }
                "shutdown" => {
                    crate::lsp::shutdown(&ctx)?;
                }
                "exit" => {
                    crate::lsp::exit(&ctx)?;
                }
                "completionItem/resolve" => {
                    let params: CompletionItemWithContext =
                        serde_json::from_value(msg.params.into())?;
                    resolve_completion(&ctx, params).await?;
                }
                "textDocument/completion" => {
                    completion(&ctx, msg.params).await?;
                }
                "textDocument/codeLens" => {
                    code_lens(&ctx, msg.params).await?;
                }
                "textDocument/codeAction" => {
                    code_action(&ctx, msg.params).await?;
                }
                "textDocument/definition" => {
                    definition(&ctx, msg.params).await?;
                }
                "textDocument/hover" => {
                    hover(&ctx, msg.params).await?;
                }
                "textDocument/references" => {
                    references(&ctx, msg.params).await?;
                }
                "textDocument/rename" => {
                    rename(&ctx, msg.params).await?;
                }
                "textDocument/implementation" => {
                    implementation(&ctx, msg.params).await?;
                }
                "textDocument/formatting" => {
                    formatting(&ctx, msg.params).await?;
                }
                "vlc/codeLensAction" => {
                    code_lens_action(&ctx, msg.params).await?;
                }
                "vlc/resolveCodeLensAction" => {
                    resolve_code_lens_action(&ctx, msg.params).await?;
                }
                "vlc/resolveCodeAction" => {
                    resolve_code_action(&ctx, msg.params).await?;
                }
                "vlc/diagnosticDetail" => {
                    diagnostic_detail(&ctx, msg.params)?;
                }
                _ => log::debug!("unhandled vim method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "textDocument/didSave" => {
                    did_save(&ctx, msg.params).await?;
                }
                "textDocument/didOpen" => {
                    did_open(&ctx, msg.params).await?;
                }
                "textDocument/didClose" => {
                    did_close(&ctx, msg.params).await?;
                }
                "textDocument/didChange" => {
                    did_change(&ctx, msg.params).await?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }
}

pub fn getbufvar<T: DeserializeOwned, C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    var: &str,
) -> Result<T> {
    let val: T = ctx
        .vim
        .call("getbufvar", serde_json::json!([ctx.bufnr, var]))?;
    Ok(val)
}

pub fn diagnostic_detail<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let pos: CursorPosition = serde_json::from_value(params.into())?;
    // let filename = ctx.filename.replace(&ctx.root_path, "");
    let filename = ctx.filename.clone();
    let diagnostics = ctx
        .state
        .read()
        .diagnostics
        .get(&filename)
        .cloned()
        .map(|d| {
            d.into_iter()
                .filter(|d| {
                    pos.position.line - 1 >= d.range.start.line
                        && pos.position.column >= d.range.start.character
                        && pos.position.line - 1 <= d.range.end.line
                        && pos.position.column <= d.range.end.character
                })
                .collect::<Vec<lsp_types::Diagnostic>>()
        })
        .unwrap_or_default();

    if diagnostics.is_empty() {
        return Ok(());
    }

    let lines = diagnostics
        .first()
        .unwrap()
        .message
        .split("\n")
        .map(String::from)
        .collect();

    // todo: extract to function
    let filetype = "text".into();
    match ctx.config.hover.strategy {
        config::DisplayMode::Preview => {
            ctx.vim.notify(
                "vlc#show_preview",
                serde_json::json!([PreviewContent { filetype, lines }]),
            )?;
        }
        config::DisplayMode::FloatingWindow => {
            ctx.vim.notify(
                "vlc#show_float_win",
                serde_json::json!([PreviewContent { filetype, lines }]),
            )?;
        }
    }
    Ok(())
}

pub fn apply_text_edits<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    filename: &str,
    edits: &[lsp_types::TextEdit],
) -> Result<()> {
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
        .notify("vlc#apply_edit", serde_json::json!([changes]))?;
    Ok(())
}

pub fn apply_workspace_edit<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    edits: &lsp_types::WorkspaceEdit,
) -> Result<()> {
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
                        .filter_map(|e| match e {
                            lsp_types::OneOf::Left(e) => Some(BufChanges {
                                start: e.range.start.into(),
                                end: e.range.end.into(),
                                lines: vec![e.new_text],
                            }),
                            // annotated text edits are not supported yet
                            lsp_types::OneOf::Right(_) => None,
                        })
                        .collect(),
                }
            })
            .collect(),
        lsp_types::DocumentChanges::Operations(_) => vec![],
    };

    ctx.vim
        .notify("vlc#apply_edits", serde_json::json!([changes]))?;
    Ok(())
}

pub fn show_diagnostics<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    file: &str,
    diagnostics: Vec<Diagnostic>,
) -> Result<()> {
    let quickfix_list: Vec<QuickfixItem> =
        diagnostics.clone().into_iter().map(|l| l.into()).collect();
    set_quickfix(ctx, quickfix_list)?;

    let signs: Vec<Sign> = diagnostics.into_iter().map(|l| l.into()).collect();
    set_signs(ctx, file, signs)?;

    Ok(())
}

pub fn show_hover<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: lsp_types::Hover,
) -> Result<()> {
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
                "vlc#show_preview",
                serde_json::json!([PreviewContent { filetype, lines }]),
            )?;
        }
        config::DisplayMode::FloatingWindow => {
            ctx.vim.notify(
                "vlc#show_float_win",
                serde_json::json!([PreviewContent { filetype, lines }]),
            )?;
        }
    }
    Ok(())
}

pub fn setloclist<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    items: Vec<LocationItem>,
) -> Result<()> {
    ctx.vim
        .notify("vlc#show_locations", serde_json::json!([items, ""]))?;

    Ok(())
}

pub fn selection<C: RPCClient, S: RPCClient, I: ListItem>(
    ctx: &Context<C, S>,
    items: Vec<I>,
) -> Result<()> {
    let text: Vec<String> = items.into_iter().map(|i| i.text()).collect();
    let sink = I::sink();
    ctx.vim
        .notify("vlc#selection", serde_json::json!([text, sink]))?;

    Ok(())
}

pub async fn show_locations<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: Vec<Location>,
) -> Result<()> {
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
) -> Result<()> {
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
) -> Result<Vec<serde_json::Value>> {
    let res: Vec<serde_json::Value> = ctx.vim.call("execute", cmd)?;
    Ok(res)
}

pub fn set_signs<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    filename: &str,
    list: Vec<Sign>,
) -> Result<()> {
    ctx.vim
        .notify("vlc#set_signs", serde_json::json!([filename, list]))?;
    Ok(())
}

pub fn set_quickfix<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    list: Vec<QuickfixItem>,
) -> Result<()> {
    ctx.vim
        .notify("vlc#set_quickfix", serde_json::json!([list]))?;
    Ok(())
}

pub fn log_message<C: RPCClient, S: RPCClient>(
    _ctx: &Context<C, S>,
    params: lsp_types::LogMessageParams,
) -> Result<()> {
    log::debug!("{}", params.message);
    Ok(())
}

pub fn show_message<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    message: Message,
) -> Result<()> {
    ctx.vim
        .notify("vlc#show_message", serde_json::json!([message]))?;
    Ok(())
}

pub async fn get_line<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    filename: &str,
    line_number: u32,
) -> Result<String> {
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
    params: Params,
) -> Result<()> {
    let params: ResolveCodeActionParams = serde_json::from_value(params.into())?;
    let state = ctx.state.read();
    let code_lens = state.code_lens.get(&params.position.text_document).cloned();
    drop(state);

    if code_lens.is_none() {
        return Ok(());
    }

    let code_lens = code_lens.as_ref().unwrap().get(params.selection);
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
    params: Params,
) -> Result<()> {
    let params: ResolveCodeActionParams = serde_json::from_value(params.into())?;
    let state = ctx.state.read();
    let code_actions = state.code_actions.clone();
    drop(state);

    let action = code_actions.get(params.selection);
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

pub async fn rename<C: RPCClient, S: RPCClient>(ctx: &Context<C, S>, params: Params) -> Result<()> {
    let params: RenameParams = serde_json::from_value(params.into()).unwrap();
    let response = crate::lsp::text_document::rename(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    apply_workspace_edit(ctx, &response.unwrap())?;
    Ok(())
}

pub async fn did_open<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let req: TextDocumentContent = serde_json::from_value(params.clone().into())?;
    crate::lsp::text_document::did_open(ctx, req).await?;
    code_lens(ctx, params).await?;
    Ok(())
}

pub async fn did_save<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let req: TextDocumentContent = serde_json::from_value(params.clone().into())?;
    crate::lsp::text_document::did_save(ctx, req).await?;
    code_lens(ctx, params).await?;
    Ok(())
}

pub async fn did_close<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: TextDocumentContent = serde_json::from_value(params.into())?;
    crate::lsp::text_document::did_close(ctx, params).await?;
    Ok(())
}

pub async fn did_change<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: TextDocumentContent = serde_json::from_value(params.into())?;
    crate::lsp::text_document::did_change(ctx, params.clone()).await?;
    Ok(())
}

pub async fn implementation<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: CursorPosition = serde_json::from_value(params.into())?;
    let response = crate::lsp::text_document::implementation(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    match response.unwrap() {
        lsp_types::GotoDefinitionResponse::Scalar(l) => {
            crate::vim::jump_to_location(ctx, l.into())?
        }
        lsp_types::GotoDefinitionResponse::Array(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
        lsp_types::GotoDefinitionResponse::Link(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
    }

    Ok(())
}

pub async fn hover<C: RPCClient, S: RPCClient>(ctx: &Context<C, S>, params: Params) -> Result<()> {
    let params: CursorPosition = serde_json::from_value(params.into())?;
    let response = crate::lsp::text_document::hover(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    show_hover(ctx, response.unwrap())?;
    Ok(())
}

pub async fn references<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: CursorPosition = serde_json::from_value(params.into())?;
    let response = crate::lsp::text_document::references(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    let response = response.unwrap();
    match response.len() {
        1 => {
            crate::vim::jump_to_location(ctx, response.first().cloned().unwrap().into())?;
        }
        _ => {
            let locations = response.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?;
        }
    }

    Ok(())
}

pub async fn formatting<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: BufInfo = serde_json::from_value(params.into())?;
    let edits = crate::lsp::text_document::formatting(ctx, &params.filename).await?;
    crate::vim::apply_text_edits(ctx, &params.filename, &edits)?;

    Ok(())
}

pub async fn code_action<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    if !ctx.features()?.code_actions {
        return Ok(());
    }

    let params: SelectionRange = serde_json::from_value(params.into())?;
    let response: Vec<lsp_types::CodeActionOrCommand> =
        crate::lsp::text_document::code_action(ctx, params).await?;
    if response.is_empty() {
        return Ok(());
    }

    let actions: Vec<Action> = response
        .into_iter()
        .map(|a| match a {
            lsp_types::CodeActionOrCommand::Command(command) => Action {
                text: command.title,
                command: command.command,
            },
            lsp_types::CodeActionOrCommand::CodeAction(action) => Action {
                text: action.title,
                command: action.command.unwrap_or_default().command,
            },
        })
        .collect();

    selection(ctx, actions)?;
    Ok(())
}

pub async fn code_lens_action<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: CursorPosition = serde_json::from_value(params.into())?;
    let code_lens = code_lens_for_position(ctx, params)?;
    if code_lens.is_empty() {
        return Ok(());
    }

    crate::vim::selection(ctx, code_lens)?;
    Ok(())
}

pub fn code_lens_for_position<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    position: CursorPosition,
) -> Result<Vec<lsp_types::CodeLens>> {
    let state = ctx.state.read();
    let code_lens = state.code_lens.get(&position.text_document);
    if code_lens.is_none() {
        return Ok(vec![]);
    }

    let code_lens: Vec<lsp_types::CodeLens> = code_lens
        .unwrap()
        .iter()
        .filter(|x| x.range.start.line + 1 == position.position.line)
        .cloned()
        .collect();

    if code_lens.is_empty() {
        return Ok(vec![]);
    }

    let code_lens = code_lens
        .into_iter()
        .filter(|x| x.command.is_some())
        .collect();
    Ok(code_lens)
}

pub async fn code_lens<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: TextDocumentIdentifier = serde_json::from_value(params.into())?;
    let response: Vec<lsp_types::CodeLens> =
        crate::lsp::text_document::code_lens(ctx, params).await?;
    if response.is_empty() {
        return Ok(());
    }

    let mut virtual_texts = vec![];
    response.into_iter().for_each(|cl| {
        if cl.command.is_none() {
            return;
        }

        let text = cl.command.unwrap().title;
        let line = cl.range.start.line;

        match virtual_texts
            .iter()
            .position(|v: &VirtualText| v.line == line)
        {
            Some(idx) => virtual_texts[idx]
                .text
                .push_str(format!(" | {}", text).as_str()),
            None => virtual_texts.push(VirtualText {
                line,
                text,
                hl_group: HLGroup::Comment,
            }),
        }
    });

    if virtual_texts.is_empty() {
        return Ok(());
    }

    ctx.vim
        .notify("vlc#set_virtual_texts", serde_json::json!([virtual_texts]))?;
    Ok(())
}

pub async fn completion<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    if !ctx.features()?.completion {
        return Ok(());
    }

    let params: CursorPosition = serde_json::from_value(params.into())?;
    let response = crate::lsp::text_document::completion(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    let list = match response.unwrap() {
        lsp_types::CompletionResponse::Array(vec) => vec.into_iter().map(|i| i.into()).collect(),
        lsp_types::CompletionResponse::List(list) => {
            list.items.into_iter().map(|i| i.into()).collect()
        }
    };

    let list = CompletionList { words: list };
    ctx.vim
        .reply_success(&ctx.message_id, serde_json::to_value(&list)?)?;

    Ok(())
}

pub async fn resolve_completion<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: CompletionItemWithContext,
) -> Result<()> {
    if !ctx.features()?.completion {
        return Ok(());
    }

    let state = ctx.state.read();
    let caps = state.server_capabilities.get(&params.language_id).cloned();
    drop(state);

    if caps.is_none() {
        log::debug!("skipping completionItem/resolve, no server capabilities found");
        return Ok(());
    }

    let opts: Option<lsp_types::CompletionOptions> = caps.unwrap().completion_provider;
    if opts.is_none() {
        log::debug!("skipping completionItem/resolve, server is not completion provider");
        return Ok(());
    }

    if !opts.unwrap().resolve_provider.unwrap_or_default() {
        log::debug!("skipping completionItem/resolve, server is not resolve provider");
        return Ok(());
    }

    let ci: lsp_types::CompletionItem =
        crate::lsp::text_document::completion_item_resolve(ctx, params.completion_item).await?;
    let ci: CompletionItem = ci.into();
    ctx.vim
        .reply_success(&ctx.message_id, serde_json::to_value(&ci)?)?;
    Ok(())
}

pub async fn definition<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: Params,
) -> Result<()> {
    let params: CursorPosition = serde_json::from_value(params.into())?;
    let response = crate::lsp::text_document::definition(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    match response.unwrap() {
        lsp_types::GotoDefinitionResponse::Scalar(l) => {
            crate::vim::jump_to_location(ctx, l.into())?
        }
        lsp_types::GotoDefinitionResponse::Array(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
        lsp_types::GotoDefinitionResponse::Link(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
    }

    Ok(())
}
