mod extensions;
mod receive;
mod send;
mod types;

use crate::language_client::LanguageClient;
use crate::rpc::{self, RPCClient};
use crate::{config, lsp::Context};
use failure::Fallible;
pub use receive::*;
pub use send::*;
pub use types::*;

impl<C, S> LanguageClient<C, S>
where
    C: RPCClient,
    S: RPCClient,
{
    // handles messages sent from vim to the language client
    pub async fn handle_vim_message(&self, message: rpc::Message) -> Fallible<()> {
        // handle start separetely as not to try to create a context
        match message {
            rpc::Message::MethodCall(msg) if msg.method.as_str() == "start" => {
                let params: BufInfo = serde_json::from_value(msg.params.into())?;
                self.start_server(&params.language_id).await?;
                return Ok(());
            }
            _ => {}
        }

        let ctx = Context::new(&message, self).await;
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                // "start" => {
                //     let params: BufInfo = serde_json::from_value(msg.params.into())?;
                //     self.start_server(&params.language_id).await?;
                // }
                "initialize" => {
                    crate::lsp::initialize(&ctx).await?;
                    crate::lsp::initialized(&ctx).await?;
                    if self.config.completion.enabled
                        && self.config.completion.strategy == config::CompletionStrategy::NCM2
                    {
                        crate::vim::extensions::ncm2::register_ncm2_source(&ctx).await?;
                    }
                }
                "shutdown" => {
                    crate::lsp::shutdown(&ctx).await?;
                }
                "exit" => {
                    crate::lsp::exit(&ctx).await?;
                }
                "completionItem/resolve" => {
                    let params: CompletionItemWithContext =
                        serde_json::from_value(msg.params.into())?;
                    crate::vim::resolve_completion(&ctx, params).await?;
                }
                "textDocument/completion" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    crate::vim::completion(&ctx, params).await?;
                }
                "textDocument/codeLens" => {
                    let params: TextDocumentIdentifier = serde_json::from_value(msg.params.into())?;
                    crate::vim::code_lens(&ctx, params).await?;
                }
                // not part of LSP
                "vlc/codeLensAction" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    crate::vim::code_lens_action(&ctx, params).await?;
                }
                // not part of LSP
                "vlc/resolveCodeLensAction" => {
                    let params: ResolveCodeActionParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::vim::resolve_code_lens_action(&ctx, params).await?;
                }
                // not part of LSP
                "vlc/resolveCodeAction" => {
                    let params: ResolveCodeActionParams =
                        serde_json::from_value(msg.params.into())?;
                    crate::vim::resolve_code_action(&ctx, params).await?;
                }
                "textDocument/codeAction" => {
                    let params: SelectionRange = serde_json::from_value(msg.params.into())?;
                    crate::vim::code_action(&ctx, params).await?;
                }
                "textDocument/definition" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    crate::vim::definition(&ctx, params).await?;
                }
                "textDocument/hover" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    crate::vim::hover(&ctx, params).await?;
                }
                "textDocument/references" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    crate::vim::references(&ctx, params).await?;
                }
                "textDocument/rename" => {
                    let params: RenameParams = serde_json::from_value(msg.params.into()).unwrap();
                    crate::vim::rename(&ctx, params).await?;
                }
                "textDocument/implementation" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    crate::vim::implementation(&ctx, params).await?;
                }
                "textDocument/formatting" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    crate::vim::formatting(&ctx, params).await?;
                }
                _ => log::debug!("unhandled vim method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "textDocument/didSave" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    crate::vim::did_save(&ctx, params).await?;
                }
                "textDocument/didOpen" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    crate::vim::did_open(&ctx, params).await?;
                }
                "textDocument/didClose" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    crate::vim::did_close(&ctx, params).await?;
                }
                "textDocument/didChange" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    crate::vim::did_change(&ctx, params).await?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }
}
