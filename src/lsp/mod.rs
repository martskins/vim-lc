mod extensions;
mod receive;
mod send;

use crate::language_client::LanguageClient;
use crate::rpc;
use crate::rpc::RPCClient;
use failure::Fallible;

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
{
    // handles messages sent from vim to the language client
    pub async fn handle_message(&self, message: rpc::Message) -> Fallible<()> {
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                _ => log::debug!("unhandled method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "window/logMessage" => {
                    let params: lsp_types::LogMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    self.log_message(params)?;
                }
                "textDocument/publishDiagnostics" => {
                    let params: lsp_types::PublishDiagnosticsParams =
                        serde_json::from_value(msg.params.into())?;
                    self.text_document_publish_diagnostics(params)?;
                }
                "$/progress" => {
                    let params: lsp_types::ProgressParams =
                        serde_json::from_value(msg.params.into())?;
                    self.progress(params)?;
                }
                "window/showMessage" => {
                    let params: lsp_types::ShowMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    self.window_show_message(params)?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }
}
