mod extensions;
mod receive;
mod send;
mod types;

use crate::config;
use crate::language_client::LanguageClient;
use crate::rpc::{self, RPCClient};
use failure::Fallible;
pub use types::*;

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
{
    // handles messages sent from vim to the language client
    pub async fn handle_vim_message(&self, message: rpc::Message) -> Fallible<()> {
        let message_id = message.id();
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                "start" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    self.start_server(&params.language_id).await?;
                }
                "initialize" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    self.initialize(&params.language_id).await?;
                    self.initialized(&params.language_id).await?;
                    if self.config.completion.enabled
                        && self.config.completion.strategy == config::CompletionStrategy::NCM2
                    {
                        self.register_ncm2_source(&params.language_id).await?;
                    }
                }
                "shutdown" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    self.shutdown(&params.language_id).await?;
                }
                "exit" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    self.exit(&params.language_id).await?;
                }
                "completionItem/resolve" => {
                    let params: CompletionItemWithContext =
                        serde_json::from_value(msg.params.into())?;
                    self.resolve_completion(&message_id, params).await?;
                }
                "textDocument/completion" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.completion(&message_id, params).await?;
                }
                "textDocument/codeLens" => {
                    let params: TextDocumentIdentifier = serde_json::from_value(msg.params.into())?;
                    self.code_lens(params).await?;
                }
                // not part of LSP
                "codeLensAction" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.code_lens_action(params).await?;
                }
                // not part of LSP
                "resolveCodeLensAction" => {
                    let params: ResolveCodeActionParams =
                        serde_json::from_value(msg.params.into())?;
                    self.resolve_code_lens_action(params).await?;
                }
                // not part of LSP
                "resolveCodeAction" => {
                    let params: ResolveCodeActionParams =
                        serde_json::from_value(msg.params.into())?;
                    self.resolve_code_action(params).await?;
                }
                "textDocument/codeAction" => {
                    let params: SelectionRange = serde_json::from_value(msg.params.into())?;
                    self.code_action(params).await?;
                }
                "textDocument/definition" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.definition(params).await?;
                }
                "textDocument/hover" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.hover(params).await?;
                }
                "textDocument/references" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.references(params).await?;
                }
                "textDocument/rename" => {
                    let params: RenameParams = serde_json::from_value(msg.params.into()).unwrap();
                    self.rename(params).await?;
                }
                "textDocument/implementation" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.implementation(params).await?;
                }
                _ => log::debug!("unhandled vim method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "textDocument/didSave" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_save(params).await?;
                }
                "textDocument/didOpen" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_open(params).await?;
                }
                "textDocument/didClose" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_close(params).await?;
                }
                "textDocument/didChange" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_change(params).await?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }
}
