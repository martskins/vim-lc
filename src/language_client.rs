use crate::state::State;
use crate::{config::Config, rpc::Message};
use crate::{config::FeatureSet, rpc::RPCClient};
use crate::{config::ServerConfig, rpc};
use anyhow::Result;
use jsonrpc_core::Value;
use parking_lot::RwLock;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::{Child, Command};

pub struct Context<C, S>
where
    C: RPCClient,
    S: RPCClient,
{
    pub vim: C,
    pub server: Option<S>,
    pub language_id: String,
    pub bufnr: usize,
    pub filename: String,
    pub message_id: jsonrpc_core::Id,
    pub state: Arc<RwLock<State>>,
    pub config: Config,
    pub root_path: String,
}

impl<C: RPCClient, S: RPCClient> Context<C, S> {
    pub fn new(message: &Message, lc: &LanguageClient<C, S>) -> Self {
        let message_id = message.id();
        let language_id = match message {
            Message::MethodCall(msg) => Into::<Value>::into(msg.params.clone())
                .get("language_id")
                .cloned()
                .unwrap_or_default(),
            Message::Notification(msg) => Into::<Value>::into(msg.params.clone())
                .get("language_id")
                .cloned()
                .unwrap_or_default(),
            Message::Output(_) => Value::String("".into()),
        };

        let bufnr = match message {
            Message::MethodCall(msg) => Into::<Value>::into(msg.params.clone())
                .get("bufnr")
                .cloned()
                .unwrap_or_default(),
            Message::Notification(msg) => Into::<Value>::into(msg.params.clone())
                .get("bufnr")
                .cloned()
                .unwrap_or_default(),
            Message::Output(_) => serde_json::to_value(0).unwrap(),
        };

        let filename = match message {
            Message::MethodCall(msg) => Into::<Value>::into(msg.params.clone())
                .get("filename")
                .cloned()
                .unwrap_or_default(),
            Message::Notification(msg) => Into::<Value>::into(msg.params.clone())
                .get("filename")
                .cloned()
                .unwrap_or_default(),
            Message::Output(_) => Value::String("".into()),
        };

        let bufnr = <usize>::deserialize(bufnr).unwrap_or_default();
        let filename = <String>::deserialize(filename).unwrap_or_default();
        let language_id = <Option<String>>::deserialize(language_id)
            .unwrap_or_default()
            .unwrap_or_default();

        let root_path = lc
            .state
            .read()
            .roots
            .get(&language_id)
            .cloned()
            .unwrap_or_default();
        let server = lc.servers.read().get(&language_id).cloned();
        Self {
            vim: lc.vim.clone(),
            server,
            language_id,
            bufnr,
            filename,
            message_id,
            state: Arc::clone(&lc.state),
            config: lc.config.clone(),
            root_path,
        }
    }

    pub fn server(&self) -> Result<&ServerConfig> {
        let command = self.config.server(&self.language_id)?;
        Ok(command)
    }

    pub fn features(&self) -> Result<&FeatureSet> {
        let features = self.config.features(&self.language_id)?;
        Ok(&features)
    }
}

#[derive(Debug)]
pub struct LanguageClient<C, S> {
    pub servers: Arc<RwLock<HashMap<String, S>>>,
    pub state: Arc<RwLock<State>>,
    pub root_path: String,
    pub config: Config,
    pub vim: C,
}

impl<C, S> Clone for LanguageClient<C, S>
where
    C: Clone,
    S: Clone,
{
    fn clone(&self) -> LanguageClient<C, S> {
        Self {
            servers: Arc::clone(&self.servers),
            state: Arc::clone(&self.state),
            root_path: self.root_path.clone(),
            config: self.config.clone(),
            vim: self.vim.clone(),
        }
    }
}

impl<C, S> Default for LanguageClient<C, S>
where
    C: RPCClient,
    S: RPCClient,
{
    fn default() -> Self {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let state = Arc::new(RwLock::new(State::default()));
        let vim = C::new(
            rpc::ClientID::VIM,
            BufReader::new(tokio::io::stdin()),
            tokio::io::stdout(),
        );
        let root_path = std::env::current_dir().unwrap();
        let root_path = format!("file://{}/", root_path.to_str().unwrap());

        Self {
            servers: clients,
            state,
            root_path,
            config: Config::default(),
            vim,
        }
    }
}

impl<C, S> LanguageClient<C, S>
where
    C: RPCClient,
    S: RPCClient,
{
    pub fn new(vim: C, config: Config) -> Self {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let state = Arc::new(RwLock::new(State::default()));
        let root_path = std::env::current_dir().unwrap();
        let root_path = format!("file://{}/", root_path.to_str().unwrap());
        Self {
            servers: clients,
            state,
            root_path,
            config,
            vim,
        }
    }

    // runs the binary specified in the config file for the given language_id
    pub async fn start_server(&self, language_id: &str) -> Result<()> {
        let server_config = self.config.server(language_id)?;
        let stderr: Stdio = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/Users/martin/Desktop/lserr.log")?
            .into();

        let cmd: Child = Command::new(&server_config.command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .expect("could not run command");

        let client = S::new(
            rpc::ClientID::LanguageServer,
            BufReader::new(cmd.stdout.unwrap()),
            cmd.stdin.unwrap(),
        );

        self.servers
            .write()
            .insert(language_id.into(), client.clone());

        let rx = client.get_reader();
        let lc = self.clone();
        tokio::spawn(async move {
            for message in rx.iter() {
                if let Err(err) = lc.handle_message(message).await {
                    log::error!("{}", err);
                }
            }
        });

        Ok(())
    }

    /// spins up the readers for both vim and language server messages.
    pub async fn run(&self) -> () {
        let rx = self.vim.get_reader();
        for msg in rx.iter() {
            let lc = self.clone();
            if let Err(err) = lc.handle_vim_message(msg).await {
                log::error!("error: {:?}", err);
            }
        }
    }
}
