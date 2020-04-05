use crate::rpc::{self, Client};
use crate::vlc::VIM;
use crossbeam::Receiver;
use crossbeam::Sender;
use failure::Fallible;
use lazy_static::lazy_static;
use lsp_types::*;
use std::collections::HashMap;
use std::io::BufReader;
use std::process::{ChildStdin, ChildStdout};
use std::sync::Arc;

lazy_static! {
    pub static ref LANGUAGE_CLIENT: LanguageClient = LanguageClient::new();
}

#[derive(Debug)]
pub struct LanguageClient {
    senders: HashMap<String, Sender<rpc::Message>>,
    tx: Sender<rpc::Message>,
    rx: Receiver<rpc::Message>,
}

impl LanguageClient {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam::bounded(1);
        let mut senders = HashMap::new();
        Self { senders, tx, rx }
    }

    fn spawn(&self) {
        std::thread::spawn(|| {});
    }

    fn get_sender(&self, language_id: &str) -> Sender<rpc::Message> {
        unimplemented!();
    }

    pub fn text_document_definition(&self) {
        unimplemented!();
        // let server = self.server.get(language_id);
        // server.call("textDocument/definition");
    }

    /// handles messages sent from vim to the language client
    async fn handle_message(&self, message: rpc::Message) -> Fallible<()> {
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                _ => log::debug!("unhandled method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "window/logMessage" => {
                    let params: lsp_types::LogMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    VIM.log_message(params).await?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(msg) => log::debug!("asdasd"),
        }

        Ok(())
    }
}
