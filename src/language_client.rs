use crate::config::Config;
use crate::rpc;
use crate::rpc::RPCClient;
use crate::state::State;
use crate::vim;
use failure::Fallible;
use lsp_types::*;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct LanguageClient<T> {
    pub clients: Arc<RwLock<HashMap<String, T>>>,
    pub process_ids: Arc<RwLock<HashMap<String, u64>>>,
    pub state: Arc<RwLock<State>>,
    pub root_path: String,
    pub config: Config,
    pub vim: T,
}

impl<T> Clone for LanguageClient<T>
where
    T: Clone,
{
    fn clone(&self) -> LanguageClient<T> {
        Self {
            clients: self.clients.clone(),
            process_ids: self.process_ids.clone(),
            state: self.state.clone(),
            root_path: self.root_path.clone(),
            config: self.config.clone(),
            vim: self.vim.clone(),
        }
    }
}

impl<T> Default for LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let process_ids = Arc::new(RwLock::new(HashMap::new()));
        let state = Arc::new(RwLock::new(State::default()));
        let vim = T::new(
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

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
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

        let cmd: Child = Command::new(binpath.unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("could not run command");

        let process_id = cmd.id() as u64;
        let client = T::new(
            rpc::ServerID::LanguageServer,
            BufReader::new(cmd.stdout.unwrap()),
            cmd.stdin.unwrap(),
        );

        self.process_ids
            .write()
            .await
            .insert(language_id.into(), process_id);

        self.clients
            .write()
            .await
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
        let state = self.state.read().await;
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

    pub async fn get_process_id(&self, language_id: &str) -> Fallible<u64> {
        let client = self.process_ids.read().await;
        let client = client.get(language_id).cloned();
        if client.is_none() {
            failure::bail!("server not running for language {}", language_id);
        }

        Ok(client.unwrap())
    }

    pub async fn get_client(&self, language_id: &str) -> Fallible<T> {
        let client = self.clients.read().await;
        let client = client.get(language_id).cloned();
        if client.is_none() {
            failure::bail!("server not running for language {}", language_id);
        }

        Ok(client.unwrap())
    }

    pub async fn get_line(&self, filename: &str, line_number: u64) -> Fallible<String> {
        let state = self.state.read().await;
        let text_document = state.text_documents.get(filename).cloned();
        drop(state);

        let idx = line_number as usize - 1;
        match text_document {
            Some((_, lines)) => Ok(lines[idx].clone()),
            None => {
                use tokio::io::AsyncReadExt;
                let mut file = tokio::fs::File::open(filename).await?;
                let mut text = String::new();
                file.read_to_string(&mut text).await?;
                let lines: Vec<&str> = text.split('\n').collect();
                let line = lines[idx].to_owned();
                Ok(line)
            }
        }
    }

    pub async fn resolve_code_lens_action(
        &self,
        input: vim::ResolveCodeActionParams,
    ) -> Fallible<()> {
        let state = self.state.read().await;
        let code_lens = state.code_lens.get(&input.position.text_document).cloned();
        drop(state);

        let code_lens: Vec<lsp_types::Command> = code_lens
            .unwrap_or_default()
            .into_iter()
            .filter(|x| {
                let parts: Vec<&str> = input.selection.split(": ").collect();
                x.command.is_some()
                    && x.range.start.line + 1 == input.position.position.line
                    && x.command.as_ref().unwrap().title == parts[1]
                    && x.command.as_ref().unwrap().command == parts[0]
            })
            .map(|x| x.command.unwrap())
            .collect();

        if code_lens.is_empty() {
            return Ok(());
        }

        // this should always have at most one item
        let code_lens = code_lens.first().cloned().unwrap();
        if let Err(err) = self
            .run_command(&input.position.language_id, code_lens)
            .await
        {
            log::error!("{}", err);
        }

        Ok(())
    }

    pub async fn resolve_code_action(&self, input: vim::ResolveCodeActionParams) -> Fallible<()> {
        let state = self.state.read().await;
        let code_actions = state.code_actions.clone();
        drop(state);

        for ca in code_actions {
            match ca {
                CodeActionOrCommand::CodeAction(action) if action.title == input.selection => {
                    let action: CodeAction = action;
                    if action.command.is_none() {
                        log::error!("action has no command: {:?}", action);
                        return Ok(());
                    }

                    self.run_command(&input.position.language_id, action.command.unwrap())
                        .await?;
                }
                CodeActionOrCommand::Command(command) if command.title == input.selection => {
                    self.run_command(&input.position.language_id, command)
                        .await?;
                }
                _ => {}
            }
        }

        let mut state = self.state.write().await;
        state.code_actions = vec![];

        Ok(())
    }

    pub async fn store_code_lens(&self, filename: &str, code_lens: Vec<CodeLens>) {
        let mut state = self.state.write().await;
        state.code_lens.insert(filename.into(), code_lens);
    }

    pub async fn resolve_code_lens(
        &self,
        language_id: &str,
        input: Vec<CodeLens>,
    ) -> Fallible<Vec<CodeLens>> {
        if !self.config.features.code_lens_resolve {
            return Ok(input);
        }

        let res: Vec<_> = input
            .into_iter()
            .map(|cl| async {
                let lc = self.clone();
                let language_id = language_id.to_string();
                if cl.data.is_none() {
                    return cl;
                }

                lc.code_lens_resolve(&language_id, &cl).await.unwrap_or(cl)
            })
            .collect();

        let res = futures::future::join_all(res).await;
        Ok(res)
    }
}
