use crate::language_client::LANGUAGE_CLIENT;
use crate::rpc;
use crate::rpc::Client;
use failure::Fallible;
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::io::{BufReader, Stdin, Stdout};
use tokio::sync::Mutex;

lazy_static! {
    pub static ref VIM: VLC = VLC::new();
}

#[derive(Debug, Clone)]
pub struct VLC {
    rpcclient: Arc<Mutex<Client<BufReader<Stdin>, Stdout>>>,
    // config
}

#[derive(Debug)]
pub enum VLCError {}

pub async fn run() -> Fallible<()> {
    VIM.loop_read().await?;

    Ok(())
}

impl VLC {
    pub fn new() -> VLC {
        let rpcclient = Arc::new(Mutex::new(Client::new(
            "VIM".into(),
            BufReader::new(tokio::io::stdin()),
            tokio::io::stdout(),
        )));

        let vlc = Self { rpcclient };
        {
            let vlc = vlc.clone();
            tokio::spawn(async move {
                vlc.loop_read().await.unwrap();
            });
        }

        vlc
    }

    async fn loop_read(&self) -> Fallible<()> {
        let vlc = self.clone();
        loop {
            let mut client = vlc.rpcclient.try_lock()?;
            let message = client.read().await?;
            if let Err(err) = vlc.handle_message(message).await {
                log::error!("{}", err);
            }
        }
    }

    /// handles messages sent from vim to the language client
    async fn handle_message(&self, message: rpc::Message) -> Fallible<()> {
        match message {
            rpc::Message::MethodCall(msg) => match msg.method {
                _ => log::debug!("unhandled method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method {
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(msg) => log::debug!("asdasd"),
        }

        Ok(())
    }

    fn jump_to_location() {
        unimplemented!();
    }

    pub async fn log_message(&self, params: lsp_types::LogMessageParams) -> Fallible<()> {
        log::debug!("{}", params.message);
        Ok(())
    }

    fn definition() {
        LANGUAGE_CLIENT.text_document_definition();
    }
}
