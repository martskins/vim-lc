use crate::config::Config;
use crate::rpc;
use crate::state::State;
use crate::vim;
use crate::vlc::VIM;
use failure::Fallible;
use lazy_static::lazy_static;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::*;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

lazy_static! {
    pub static ref LANGUAGE_CLIENT: LanguageClient = LanguageClient::new();
}

type Client = rpc::Client<BufReader<ChildStdout>, ChildStdin>;

#[derive(Debug, Clone)]
pub struct LanguageClient {
    clients: Arc<Mutex<HashMap<String, Client>>>,
    state: Arc<Mutex<State>>,
    config: Config,
}

impl LanguageClient {
    pub fn new() -> Self {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let config =
            futures::executor::block_on(Config::parse("/home/martin/Desktop/config.toml")).unwrap();
        let state = Arc::new(Mutex::new(State::default()));

        Self {
            clients,
            state,
            config,
        }
    }

    fn spawn_reader(&self, language_id: String, mut client: Client) -> Fallible<()> {
        self.clients
            .try_lock()?
            .insert(language_id.clone().into(), client.clone());

        let lc = self.clone();
        tokio::spawn(async move {
            let language_id = language_id.clone();
            loop {
                let message = client.read().await.unwrap();
                if let Err(err) = lc.handle_message(language_id.as_str(), message).await {
                    log::error!("{}", err);
                }
            }
        });

        Ok(())
    }

    pub async fn start_server(&mut self, language_id: &str) -> Fallible<()> {
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

        // let process_id = cmd.id() as u64;
        let client = rpc::Client::new(
            rpc::ServerID::LanguageServer,
            BufReader::new(cmd.stdout.unwrap()),
            cmd.stdin.unwrap(),
        );

        self.spawn_reader(language_id.into(), client.clone())?;

        log::error!("DONE");
        Ok(())
    }

    fn get_client(&self, language_id: &str) -> Fallible<Client> {
        let client = self.clients.try_lock()?.get(language_id).cloned();
        if client.is_none() {
            failure::bail!("server not running for language {}", language_id);
        }

        Ok(client.unwrap())
    }

    /// handles messages sent from vim to the language client
    async fn handle_message(&self, language_id: &str, message: rpc::Message) -> Fallible<()> {
        let message_id = message.id();
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
                "$/progress" => {
                    let params: lsp_types::ProgressParams =
                        serde_json::from_value(msg.params.into())?;
                    self.progress(params).await?;
                }
                "window/showMessage" => {
                    let params: lsp_types::ShowMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    self.window_show_message(params).await?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(o) => {
                let mut client = self.get_client(language_id)?;
                client.resolve(&message_id, o.clone()).await?;
            }
        }

        Ok(())
    }

    pub async fn window_show_message(&self, input: ShowMessageParams) -> Fallible<()> {
        let message = input.message;
        VIM.clone()
            .show_message(vim::Message { message, level: 3 })
            .await?;

        Ok(())
    }

    pub async fn progress(&self, params: lsp_types::ProgressParams) -> Fallible<()> {
        let message = match params.value {
            ProgressParamsValue::WorkDone(wd) => match wd {
                WorkDoneProgress::Begin(r) => {
                    Some(format!("{} {}", r.title, r.message.unwrap_or_default()))
                }
                WorkDoneProgress::Report(r) => r.message,
                WorkDoneProgress::End(r) => r.message,
            },
        };

        if message.is_none() {
            return Ok(());
        }

        let message = vim::Message {
            message: message.unwrap(),
            level: 3,
        };

        VIM.show_message(message).await?;
        Ok(())
    }

    pub async fn initialize(&self, language_id: &str) -> Fallible<()> {
        let mut client = self.get_client(language_id)?;
        let message = InitializeParams {
            // TODO: set the process id
            process_id: Some(1234),
            root_path: None,
            root_uri: Some(Url::from_directory_path(std::env::current_dir()?).unwrap()),
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceOption::Verbose),
            workspace_folders: None,
            client_info: Some(ClientInfo {
                name: "vim-lc".into(),
                version: Some("1.0".into()),
            }),
        };

        let _: InitializeResult = client
            .call_and_wait(request::Initialize::METHOD, message)
            .await?;
        Ok(())
    }

    pub async fn initialized(&self, language_id: &str) -> Fallible<()> {
        let mut client = self.get_client(language_id)?;
        client
            .notify(notification::Initialized::METHOD, InitializedParams {})
            .await?;
        Ok(())
    }

    pub async fn text_document_implementation(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentPosition,
    ) -> Fallible<Option<request::GotoImplementationResponse>> {
        let input: TextDocumentPositionParams = input.into();
        let mut client = self.get_client(language_id)?;
        let message: Option<request::GotoImplementationResponse> = client
            .call_and_wait(request::GotoImplementation::METHOD, input)
            .await?;
        Ok(message)
    }

    pub async fn text_document_references(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentPosition,
    ) -> Fallible<Option<Vec<lsp_types::Location>>> {
        let input: TextDocumentPositionParams = input.into();
        let mut client = self.get_client(language_id)?;
        let message: Option<Vec<lsp_types::Location>> = client
            .call_and_wait(request::References::METHOD, input)
            .await?;
        Ok(message)
    }

    pub async fn text_document_definition(
        &self,
        language_id: &str,
        params: TextDocumentPositionParams,
    ) -> Fallible<Option<request::GotoDefinitionResponse>> {
        let input: TextDocumentPositionParams = params.into();
        let mut client = self.get_client(language_id)?;
        let message: Option<request::GotoDefinitionResponse> = client
            .call_and_wait(request::GotoDefinition::METHOD, input)
            .await?;
        Ok(message)
    }

    pub async fn text_document_did_save(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentContent,
    ) -> Fallible<()> {
        let input = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
        };

        let mut client = LANGUAGE_CLIENT.get_client(language_id)?;
        client
            .notify(notification::DidSaveTextDocument::METHOD, input)
            .await
    }

    pub async fn text_document_did_close(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentContent,
    ) -> Fallible<()> {
        let state = self.state.clone();
        let mut state = state.try_lock()?;
        state.text_documents.remove(&input.text_document);

        let input = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
        };

        let mut client = LANGUAGE_CLIENT.get_client(language_id)?;
        client
            .notify(notification::DidCloseTextDocument::METHOD, input)
            .await
    }

    pub async fn text_document_did_change(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentContent,
    ) -> Fallible<()> {
        let state = self.state.clone();
        let state = state.try_lock()?;
        let version = state
            .text_documents
            .get(&input.text_document)
            .cloned()
            .unwrap_or_default();

        // TODO: not sure if version should actually be an u64
        let input = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
                version: Some(version as i64),
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: input.text,
            }],
        };

        let mut client = LANGUAGE_CLIENT.get_client(language_id)?;
        client
            .notify(notification::DidChangeTextDocument::METHOD, input)
            .await
    }

    pub async fn text_document_did_open(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentContent,
    ) -> Fallible<()> {
        let state = self.state.clone();
        let mut state = state.try_lock()?;
        let mut version = state.text_documents.get(&input.text_document).cloned();

        if version.is_none() {
            version = Some(0);
            state.text_documents.insert(input.text_document.clone(), 0);
        }

        let input = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::from_file_path(input.text_document).unwrap(),
                language_id: input.language_id,
                version: version.unwrap() as i64,
                text: input.text,
            },
        };

        let mut client = LANGUAGE_CLIENT.get_client(language_id)?;
        client
            .notify(notification::DidOpenTextDocument::METHOD, input)
            .await
    }
}
