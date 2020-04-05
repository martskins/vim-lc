mod lsp;
mod vim;

use crate::rpc;
use crate::rpc::Client;
use crate::vim::*;
use crate::LANGUAGE_CLIENT;
use crate::VIM;
use failure::Fallible;
use tokio::io::{BufReader, Stdin, Stdout};

#[derive(Debug, Clone)]
pub struct VLC {
    client: Client<BufReader<Stdin>, Stdout>,
}

impl VLC {
    pub fn new() -> VLC {
        let client = Client::new(
            rpc::ServerID::VIM,
            BufReader::new(tokio::io::stdin()),
            tokio::io::stdout(),
        );

        Self { client }
    }

    pub async fn run(&self) -> Fallible<()> {
        let vlc = self.clone();
        vlc.loop_read().await?;
        Ok(())
    }

    async fn loop_read(&self) -> Fallible<()> {
        let mut vlc = self.clone();
        loop {
            let message = vlc.client.read().await?;
            if let Err(err) = vlc.handle_message(message).await {
                log::error!("{}", err);
            }
        }
    }

    /// handles messages sent from vim to the language client
    async fn handle_message(&self, message: rpc::Message) -> Fallible<()> {
        let message_id = message.id();
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                "start" => {
                    let params: BaseParams = serde_json::from_value(msg.params.into())?;
                    LANGUAGE_CLIENT
                        .clone()
                        .start_server(&params.language_id)
                        .await?;
                }
                "initialize" => {
                    let params: BaseParams = serde_json::from_value(msg.params.into())?;
                    self.initialize(params).await?;
                }
                "textDocument/completion" => {
                    let params: TextDocumentPosition = serde_json::from_value(msg.params.into())?;
                    self.completion(&message_id, params).await?;
                }
                "textDocument/definition" => {
                    let params: TextDocumentPosition = serde_json::from_value(msg.params.into())?;
                    self.definition(params).await?;
                }
                "textDocument/hover" => {
                    let params: TextDocumentPosition = serde_json::from_value(msg.params.into())?;
                    self.hover(params).await?;
                }
                "textDocument/references" => {
                    let params: TextDocumentPosition = serde_json::from_value(msg.params.into())?;
                    self.references(params).await?;
                }
                "textDocument/implementation" => {
                    let params: TextDocumentPosition = serde_json::from_value(msg.params.into())?;
                    self.implementation(params).await?;
                }
                _ => log::debug!("unhandled method call {}", msg.method),
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
            rpc::Message::Output(msg) => {
                self.clone().client.resolve(&message_id, msg).await?;
            }
        }

        Ok(())
    }
}
