pub mod code_lens;
pub mod extensions;
pub mod text_document;
pub mod window;
pub mod workspace;

use crate::language_client::{Context, LanguageClient};
use crate::rpc;
use crate::rpc::RPCClient;
use anyhow::Result;
use lsp_types::{
    notification::{self, Notification},
    request::{self, Request},
    ClientCapabilities, ClientInfo, HoverCapability, InitializeParams, InitializeResult,
    InitializedParams, TextDocumentClientCapabilities, TraceOption, Url,
};

impl<C, S> LanguageClient<C, S>
where
    C: RPCClient,
    S: RPCClient,
{
    // handles messages sent from vim to the language client
    pub async fn handle_message(&self, message: rpc::Message) -> Result<()> {
        let ctx = Context::new(&message, self);
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                "workspace/applyEdit" => {
                    let params: lsp_types::ApplyWorkspaceEditParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::workspace::apply_edit(&ctx, &params)?;
                }
                _ => log::debug!("unhandled server method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "window/logMessage" => {
                    let params: lsp_types::LogMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::vim::log_message(&ctx, params)?;
                }
                "textDocument/publishDiagnostics" => {
                    let params: lsp_types::PublishDiagnosticsParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::text_document::publish_diagnostics(&ctx, params)?;
                }
                "$/progress" => {
                    let params: lsp_types::ProgressParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::window::progress(&ctx, params)?;
                }
                "window/showMessage" => {
                    let params: lsp_types::ShowMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::lsp::window::show_message(&ctx, params)?;
                }
                _ => log::debug!("unhandled server notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }
}

#[allow(deprecated)]
pub async fn initialize<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    let server_command = ctx.server()?;
    let message = InitializeParams {
        process_id: Some(ctx.state.read().process_id),
        root_path: None,
        root_uri: Some(Url::from_directory_path(std::env::current_dir()?).unwrap()),
        initialization_options: server_command.initialization_options.clone(),
        capabilities: ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                hover: Some(HoverCapability {
                    content_format: Some(ctx.config.hover.preferred_markup_kind.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        trace: Some(TraceOption::Verbose),
        workspace_folders: None,
        client_info: Some(ClientInfo {
            name: "vim-lc".into(),
            version: Some("1.0".into()),
        }),
    };

    let res: InitializeResult = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::Initialize::METHOD, message)?;

    ctx.state
        .write()
        .server_capabilities
        .insert(ctx.language_id.clone(), res.capabilities);

    Ok(())
}

pub async fn shutdown<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    ctx.server
        .as_ref()
        .unwrap()
        .call(request::Shutdown::METHOD, ())?;
    Ok(())
}

pub async fn exit<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::Exit::METHOD, ())?;
    Ok(())
}

pub async fn initialized<C, S>(ctx: &Context<C, S>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    ctx.server
        .as_ref()
        .unwrap()
        .notify(notification::Initialized::METHOD, InitializedParams {})?;
    Ok(())
}
