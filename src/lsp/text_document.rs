use crate::language_client::Context;
use crate::{rpc::RPCClient, vim};
use anyhow::Result;
use lsp_types::{
    notification::{self, Notification},
    request::{self, Request},
    CodeActionContext, CodeActionOrCommand, CodeActionParams, CodeActionResponse, CodeLens,
    CodeLensParams, CompletionItem, CompletionParams, CompletionResponse, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, DocumentFormattingParams, FormattingOptions, GotoDefinitionResponse,
    Hover, PublishDiagnosticsParams, Range, ReferenceParams, RenameParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, TextEdit, Url, VersionedTextDocumentIdentifier,
    WorkDoneProgressParams, WorkspaceEdit,
};
use std::collections::HashMap;

pub async fn formatting<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    text_document: &str,
) -> Result<Vec<TextEdit>> {
    let tab_size = crate::vim::getbufvar(ctx, "&shiftwidth")?;
    let insert_spaces: bool = crate::vim::getbufvar::<u8, _, _>(ctx, "&expandtab")? == 1;
    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier {
            uri: Url::from_file_path(text_document).unwrap(),
        },
        work_done_progress_params: Default::default(),
        options: FormattingOptions {
            tab_size,
            insert_spaces,
            properties: HashMap::new(),
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        },
    };

    let res: Option<Vec<TextEdit>> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::Formatting::METHOD, params)?;

    if res.is_none() {
        return Ok(vec![]);
    }

    let res = res.unwrap();
    Ok(res)
}

pub async fn code_action<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::SelectionRange,
) -> Result<Vec<CodeActionOrCommand>> {
    let params = CodeActionParams {
        text_document: TextDocumentIdentifier {
            uri: Url::from_file_path(input.text_document).unwrap(),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        range: Range {
            start: input.range.start.into(),
            end: input.range.end.into(),
        },
        context: CodeActionContext {
            diagnostics: vec![],
            only: None,
        },
    };

    let res: Option<CodeActionResponse> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::CodeActionRequest::METHOD, params)?;

    if res.is_none() {
        return Ok(vec![]);
    }

    let actions = res.unwrap();
    ctx.state.write().code_actions = actions.clone();

    Ok(actions)
}

pub async fn code_lens<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::TextDocumentIdentifier,
) -> Result<Vec<CodeLens>> {
    let params = CodeLensParams {
        text_document: TextDocumentIdentifier {
            uri: Url::from_file_path(input.text_document.clone()).unwrap(),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let response: Option<Vec<CodeLens>> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::CodeLensRequest::METHOD, params)?;
    let response = response.unwrap_or_default();
    if response.is_empty() {
        return Ok(vec![]);
    }

    ctx.state
        .write()
        .code_lens
        .insert(input.text_document.into(), response.clone());
    Ok(response)
}

pub async fn implementation<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::CursorPosition,
) -> Result<Option<request::GotoImplementationResponse>> {
    let input: TextDocumentPositionParams = input.into();
    let message: Option<request::GotoImplementationResponse> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::GotoImplementation::METHOD, input)?;
    Ok(message)
}

pub async fn references<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::CursorPosition,
) -> Result<Option<Vec<lsp_types::Location>>> {
    let input: ReferenceParams = input.into();
    let message: Option<Vec<lsp_types::Location>> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::References::METHOD, input)?;
    Ok(message)
}

pub async fn definition<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: vim::CursorPosition,
) -> Result<Option<GotoDefinitionResponse>> {
    let input: TextDocumentPositionParams = params.into();
    let message: Option<GotoDefinitionResponse> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::GotoDefinition::METHOD, input)?;
    Ok(message)
}

pub async fn did_save<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::TextDocumentContent,
) -> Result<()> {
    let input = DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: Url::from_file_path(input.text_document).unwrap(),
        },
        text: None,
    };

    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::DidSaveTextDocument::METHOD, input)
}

pub async fn did_close<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::TextDocumentContent,
) -> Result<()> {
    let _ = ctx
        .state
        .write()
        .text_documents
        .remove(&input.text_document);

    let input = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: Url::from_file_path(input.text_document).unwrap(),
        },
    };

    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::DidCloseTextDocument::METHOD, input)
}

pub async fn did_change<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::TextDocumentContent,
) -> Result<()> {
    let (version, _) = ctx
        .state
        .read()
        .text_documents
        .get(&input.text_document)
        .cloned()
        .unwrap_or_default();
    // let (version, _) = state
    //     .text_documents
    //     .get(&input.text_document)
    //     .cloned()
    //     .unwrap_or_default();

    // TODO: not sure if version should actually be an u64
    let input = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Url::from_file_path(input.text_document).unwrap(),
            version: version as i32,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: input.text,
        }],
    };

    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::DidChangeTextDocument::METHOD, input)
}

pub async fn rename<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::RenameParams,
) -> Result<Option<WorkspaceEdit>> {
    let params = RenameParams {
        text_document_position: input.position.into(),
        new_name: input.new_name,
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let response: Option<WorkspaceEdit> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::Rename::METHOD, params)?;
    Ok(response)
}

pub async fn did_open<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::TextDocumentContent,
) -> Result<()> {
    let mut state = ctx.state.write();
    let mut version = state.text_documents.get(&input.text_document).cloned();

    if version.is_none() {
        let v = (0, input.text.split('\n').map(|l| l.to_owned()).collect());
        state
            .text_documents
            .insert(input.text_document.clone(), v.clone());
        version = Some(v);
    }
    drop(state);

    let (version, _) = version.unwrap();
    let input = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Url::from_file_path(input.text_document).unwrap(),
            language_id: input.language_id,
            version: version as i32,
            text: input.text,
        },
    };

    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::DidOpenTextDocument::METHOD, input)
}

pub async fn hover<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::CursorPosition,
) -> Result<Option<Hover>> {
    let input: TextDocumentPositionParams = input.into();
    let response: Option<Hover> = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::HoverRequest::METHOD, input)?;
    Ok(response)
}

pub async fn completion<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::CursorPosition,
) -> Result<Option<CompletionResponse>> {
    let input = CompletionParams {
        text_document_position: input.into(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: Default::default(),
    };

    let message = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::Completion::METHOD, input)?;

    Ok(message)
}

pub async fn completion_item_resolve<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: vim::CompletionItem,
) -> Result<CompletionItem> {
    let params: CompletionItem = input.into();
    let message: CompletionItem = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::ResolveCompletionItem::METHOD, params)?;

    Ok(message)
}

pub fn publish_diagnostics<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: PublishDiagnosticsParams,
) -> Result<()> {
    let uri = input.uri.to_string().replace("file://", "");
    ctx.state
        .write()
        .diagnostics
        .insert(uri.clone(), input.diagnostics.clone());

    let diagnostics = input
        .diagnostics
        .into_iter()
        .map(|d| vim::Diagnostic {
            text_document: uri.clone(),
            line: d.range.start.line + 1,
            col: d.range.start.character + 1,
            text: d.message,
            severity: d.severity.unwrap_or(DiagnosticSeverity::Warning),
        })
        .collect();

    crate::vim::show_diagnostics(ctx, &uri, diagnostics)?;
    Ok(())
}
