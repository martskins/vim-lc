use crate::config::Config;
use crate::rpc;
use crate::rpc::RPCClient;
use crate::state::State;
use crate::vim;
use failure::Fallible;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::{Child, Command};

#[derive(Debug)]
pub struct LanguageClient<C, S> {
    pub clients: Arc<RwLock<HashMap<String, S>>>,
    pub process_ids: Arc<RwLock<HashMap<String, u64>>>,
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
            process_ids: Arc::clone(&self.process_ids),
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
        let process_ids = Arc::new(RwLock::new(HashMap::new()));
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
            process_ids,
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
    pub fn new(config: Config) -> Self {
        let mut lc = LanguageClient::default();
        lc.config = config;
        lc
    }

    // runs the binary specified in the config file for the given language_id
    pub async fn start_server(&self, language_id: &str) -> Fallible<()> {
        let binpath = self.config.servers.get(language_id);
        if binpath.is_none() {
            return Ok(());
        }

        let stderr: Stdio = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/Users/martin/Desktop/lserr.log")
            .unwrap()
            .into();

        let cmd: Child = Command::new(binpath.unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .expect("could not run command");

        // let process_id = cmd.id() as u64;
        let client = S::new(
            rpc::ServerID::LanguageServer,
            BufReader::new(cmd.stdout.unwrap()),
            cmd.stdin.unwrap(),
        );

        // self.process_ids
        //     .write()
        //     .await
        //     .insert(language_id.into(), process_id);

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
    pub async fn run(&self) -> Fallible<()> {
        let rx = self.vim.get_reader();
        for msg in rx.iter() {
            let lc = self.clone();
            if let Err(err) = lc.handle_vim_message(msg).await {
                log::error!("error: {:?}", err);
            }
        }

        Ok(())
    }

    pub async fn code_lens_for_position(
        &self,
        position: vim::CursorPosition,
    ) -> Fallible<Vec<lsp_types::CodeLens>> {
        let state = self.state.read();
        let code_lens = state.code_lens.get(&position.text_document);
        if code_lens.is_none() {
            return Ok(vec![]);
        }

        let code_lens: Vec<lsp_types::CodeLens> = code_lens
            .unwrap()
            .iter()
            .filter(|x| x.range.start.line + 1 == position.position.line)
            .cloned()
            .collect();

        if code_lens.is_empty() {
            return Ok(vec![]);
        }

        let code_lens = code_lens
            .into_iter()
            .filter(|x| x.command.is_some())
            .collect();
        Ok(code_lens)
    }

    // pub async fn get_process_id(&self, language_id: &str) -> Fallible<u64> {
    //     let client = self.process_ids.read().await;
    //     let client = client.get(language_id).cloned();
    //     if client.is_none() {
    //         failure::bail!("server not running for language {}", language_id);
    //     }

    //     Ok(client.unwrap())
    // }

    // pub async fn get_client(&self, language_id: &str) -> Fallible<S> {
    //     let client = self.clients.read().await;
    //     let client = client.get(language_id).cloned();
    //     if client.is_none() {
    //         failure::bail!("server not running for language {}", language_id);
    //     }

    //     Ok(client.unwrap())
    // }

    // pub async fn resolve_code_lens(
    //     &self,
    //     language_id: &str,
    //     input: Vec<CodeLens>,
    // ) -> Fallible<Vec<CodeLens>> {
    //     if !self.config.features.code_lens_resolve {
    //         return Ok(input);
    //     }

    //     let res: Vec<_> = input
    //         .into_iter()
    //         .map(|cl| async {
    //             let lc = self.clone();
    //             let language_id = language_id.to_string();
    //             if cl.data.is_none() {
    //                 return cl;
    //             }

    //             lc.code_lens_resolve(&language_id, &cl).await.unwrap_or(cl)
    //         })
    //         .collect();

    //     let res = futures::future::join_all(res).await;
    //     Ok(res)
    // }
}
