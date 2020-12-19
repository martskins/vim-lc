use crate::config::Config;
use crate::rpc;
use crate::rpc::RPCClient;
use crate::state::State;
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::{Child, Command};

#[derive(Debug)]
pub struct LanguageClient<C, S> {
    pub clients: Arc<RwLock<HashMap<String, S>>>,
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
            clients: Arc::clone(&self.clients),
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
            rpc::ServerID::VIM,
            BufReader::new(tokio::io::stdin()),
            tokio::io::stdout(),
        );
        let root_path = std::env::current_dir().unwrap();
        let root_path = format!("file://{}/", root_path.to_str().unwrap());

        Self {
            clients,
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
            clients,
            state,
            root_path,
            config,
            vim,
        }
    }

    // runs the binary specified in the config file for the given language_id
    pub async fn start_server(&self, language_id: &str) -> Result<()> {
        let server_config = self
            .config
            .servers
            .get(language_id)
            .ok_or(anyhow::anyhow!(
                "No configured server command for {}",
                language_id
            ))?
            .clone();
        let binpath = server_config.command;
        let stderr: Stdio = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/Users/martin/Desktop/lserr.log")?
            .into();

        let cmd: Child = Command::new(binpath)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .expect("could not run command");

        let client = S::new(
            rpc::ServerID::LanguageServer,
            BufReader::new(cmd.stdout.unwrap()),
            cmd.stdin.unwrap(),
        );

        self.clients
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
