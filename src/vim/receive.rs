pub use super::types::*;
use super::{apply_workspace_edit, selection, show_hover, show_locations};
use crate::lsp::Context;
use crate::rpc::RPCClient;
use failure::Fallible;

pub async fn rename<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: RenameParams,
) -> Fallible<()> {
    let response = crate::lsp::text_document::rename(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    apply_workspace_edit(ctx, &response.unwrap())?;
    Ok(())
}

pub async fn did_open<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: TextDocumentContent,
) -> Fallible<()> {
    if !ctx.config.features.did_open {
        return Ok(());
    }

    crate::lsp::text_document::did_open(ctx, params.clone()).await?;
    code_lens(ctx, params.into()).await?;
    Ok(())
}

pub async fn did_save<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: TextDocumentContent,
) -> Fallible<()> {
    if !ctx.config.features.did_save {
        return Ok(());
    }

    crate::lsp::text_document::did_save(ctx, params.clone()).await?;
    code_lens(ctx, params.into()).await?;
    Ok(())
}

pub async fn did_close<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: TextDocumentContent,
) -> Fallible<()> {
    if !ctx.config.features.did_close {
        return Ok(());
    }

    crate::lsp::text_document::did_close(ctx, params).await?;
    Ok(())
}

pub async fn did_change<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: TextDocumentContent,
) -> Fallible<()> {
    if !ctx.config.features.did_change {
        return Ok(());
    }

    crate::lsp::text_document::did_change(ctx, params.clone()).await?;
    Ok(())
}

pub async fn implementation<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: CursorPosition,
) -> Fallible<()> {
    if !ctx.config.features.implementation {
        return Ok(());
    }

    let response = crate::lsp::text_document::implementation(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    match response.unwrap() {
        lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
            crate::vim::jump_to_location(ctx, l.into())?
        }
        lsp_types::request::GotoDefinitionResponse::Array(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
        lsp_types::request::GotoDefinitionResponse::Link(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
    }

    Ok(())
}

pub async fn hover<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: CursorPosition,
) -> Fallible<()> {
    if !ctx.config.features.hover {
        return Ok(());
    }

    let response = crate::lsp::text_document::hover(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    show_hover(ctx, response.unwrap())?;
    Ok(())
}

pub async fn references<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: CursorPosition,
) -> Fallible<()> {
    if !ctx.config.features.references {
        return Ok(());
    }

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
    params: BufInfo,
) -> Fallible<()> {
    let edits = crate::lsp::text_document::formatting(ctx, &params.filename).await?;
    crate::vim::apply_text_edits(ctx, &params.filename, &edits)?;

    Ok(())
}

pub async fn code_action<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: SelectionRange,
) -> Fallible<()> {
    if !ctx.config.features.code_action {
        return Ok(());
    }

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
    position: CursorPosition,
) -> Fallible<()> {
    if !ctx.config.features.code_action {
        return Ok(());
    }

    let code_lens = code_lens_for_position(ctx, position)?;
    if code_lens.is_empty() {
        return Ok(());
    }

    crate::vim::send::selection(ctx, code_lens)?;
    Ok(())
}

pub fn code_lens_for_position<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    position: CursorPosition,
) -> Fallible<Vec<lsp_types::CodeLens>> {
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
    params: TextDocumentIdentifier,
) -> Fallible<()> {
    if !ctx.config.features.code_lens {
        return Ok(());
    }

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
        .notify("vim#set_virtual_texts", serde_json::json!([virtual_texts]))?;
    Ok(())
}

pub async fn completion<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: CursorPosition,
) -> Fallible<()> {
    if !ctx.config.completion.enabled {
        ctx.vim
            .reply_success(&ctx.message_id, serde_json::json!([]))?;
        return Ok(());
    }

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
) -> Fallible<()> {
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
    params: CursorPosition,
) -> Fallible<()> {
    if !ctx.config.features.definition {
        return Ok(());
    }

    let response = crate::lsp::text_document::definition(ctx, params).await?;
    if response.is_none() {
        return Ok(());
    }

    match response.unwrap() {
        lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
            crate::vim::jump_to_location(ctx, l.into())?
        }
        lsp_types::request::GotoDefinitionResponse::Array(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
        lsp_types::request::GotoDefinitionResponse::Link(ll) => {
            let locations = ll.into_iter().map(|l| l.into()).collect();
            crate::vim::show_locations(ctx, locations).await?
        }
    }

    Ok(())
}
