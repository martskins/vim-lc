mod lsp;
mod vim;

use crate::config::Config;
use crate::language_client::LANGUAGE_CLIENT;
use crate::rpc;
use crate::rpc::Client;
use crate::vim::*;
use failure::Fallible;
use futures::executor::block_on;
use lazy_static::lazy_static;
use std::str::FromStr;
use tokio::io::{BufReader, Stdin, Stdout};

lazy_static! {
    pub static ref VIM: VLC = VLC::new();
}

#[derive(Debug, Clone)]
pub struct VLC {
    client: Client<BufReader<Stdin>, Stdout>,
}

impl VLC {
    pub fn new() -> VLC {
        let config = block_on(Config::parse("/home/martin/Desktop/config.toml")).unwrap();
        fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{}][{}] {}",
                    record.target(),
                    record.level(),
                    message
                ))
            })
            .level(log::LevelFilter::from_str(&config.log.level).unwrap())
            .chain(fern::log_file(&config.log.output).unwrap())
            .apply()
            .unwrap();

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
        log::error!("asdasd");
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
                "textDocument/definition" => {
                    let params: TextDocumentPosition = serde_json::from_value(msg.params.into())?;
                    self.definition(params).await?;
                }
                "textDocument/references" => {
                    let params: TextDocumentPosition = serde_json::from_value(msg.params.into())?;
                    self.references(params).await?;
                }
                _ => log::debug!("unhandled method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(msg) => log::debug!("asdasd"),
        }

        Ok(())
    }
}
