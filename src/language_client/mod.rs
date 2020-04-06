use crate::rpc;
use crate::state::State;
use crate::vim;
use crate::CONFIG;
use crate::LANGUAGE_CLIENT;
use crate::VIM;
use failure::Fallible;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::*;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

type Client<I, O> = rpc::Client<BufReader<I>, O>;

#[derive(Debug)]
pub struct LanguageClient<I, O> {
    clients: Arc<Mutex<HashMap<String, Client<I, O>>>>,
    state: Arc<Mutex<State>>,
}

impl<I, O> Clone for LanguageClient<I, O> {
    fn clone(&self) -> LanguageClient<I, O> {
        Self {
            clients: self.clients.clone(),
            state: self.state.clone(),
        }
    }
}

impl LanguageClient<ChildStdout, ChildStdin> {
    /// runs the binary specified in the config file for the given language_id
    pub async fn start_server(&mut self, language_id: &str) -> Fallible<()> {
        let binpath = CONFIG.servers.get(language_id);
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

        self.spawn_reader(language_id.into(), client)?;

        Ok(())
    }
}

#[allow(deprecated)]
impl<I, O> LanguageClient<I, O>
where
    I: AsyncRead + Unpin + Send + 'static,
    O: AsyncWrite + Unpin + Send + 'static,
{
    pub fn new() -> Self {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let state = Arc::new(Mutex::new(State::default()));
        Self { clients, state }
    }

    fn spawn_reader(&self, language_id: String, client: Client<I, O>) -> Fallible<()> {
        self.clients
            .try_lock()?
            .insert(language_id.clone(), client.clone());

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

    fn get_client(&self, language_id: &str) -> Fallible<Client<I, O>> {
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
                "textDocument/publishDiagnostics" => {
                    let params: lsp_types::PublishDiagnosticsParams =
                        serde_json::from_value(msg.params.into())?;
                    self.text_document_publish_diagnostics(params).await?;
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
                let client = self.get_client(language_id)?;
                client.resolve(&message_id, o.clone()).await?;
            }
        }

        Ok(())
    }

    pub async fn text_document_hover(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentPosition,
    ) -> Fallible<Option<Hover>> {
        let input: TextDocumentPositionParams = input.into();
        let client = self.get_client(language_id)?;
        let response: Option<Hover> = client.call(request::HoverRequest::METHOD, input).await?;

        Ok(response)
    }

    pub async fn text_document_publish_diagnostics(
        &self,
        input: PublishDiagnosticsParams,
    ) -> Fallible<()> {
        if input.diagnostics.is_empty() {
            return Ok(());
        }

        let uri = input.uri.to_string();
        let diagnostics = input
            .diagnostics
            .into_iter()
            .map(|d| vim::Diagnostic {
                text_document: uri.clone(),
                line: d.range.start.line + 1,
                col: d.range.start.character + 1,
                text: d.message,
                severity: d.severity.unwrap_or(DiagnosticSeverity::Warning),
            })
            .collect();

        VIM.show_diagnostics(diagnostics).await?;
        Ok(())
    }

    pub async fn window_show_message(&self, input: ShowMessageParams) -> Fallible<()> {
        let message = input.message;
        VIM.show_message(vim::Message { message, level: 3 }).await?;

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
        let client = self.get_client(language_id)?;
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

        let res: InitializeResult = client.call(request::Initialize::METHOD, message).await?;

        let mut state = self.state.try_lock()?;
        state
            .server_capabilities
            .insert(language_id.into(), res.capabilities);

        Ok(())
    }

    pub async fn shutdown(&self, language_id: &str) -> Fallible<()> {
        let client = self.get_client(language_id)?;
        client.call(request::Shutdown::METHOD, ()).await?;
        Ok(())
    }

    pub async fn exit(&self, language_id: &str) -> Fallible<()> {
        let client = self.get_client(language_id)?;
        client.notify(notification::Exit::METHOD, ()).await?;
        Ok(())
    }

    pub async fn initialized(&self, language_id: &str) -> Fallible<()> {
        let client = self.get_client(language_id)?;
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
        let client = self.get_client(language_id)?;
        let message: Option<request::GotoImplementationResponse> = client
            .call(request::GotoImplementation::METHOD, input)
            .await?;
        Ok(message)
    }

    pub async fn text_document_references(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentPosition,
    ) -> Fallible<Option<Vec<lsp_types::Location>>> {
        let input: TextDocumentPositionParams = input.into();
        let client = self.get_client(language_id)?;
        let message: Option<Vec<lsp_types::Location>> =
            client.call(request::References::METHOD, input).await?;
        Ok(message)
    }

    pub async fn text_document_definition(
        &self,
        language_id: &str,
        params: TextDocumentPositionParams,
    ) -> Fallible<Option<request::GotoDefinitionResponse>> {
        let input: TextDocumentPositionParams = params;
        let client = self.get_client(language_id)?;
        let message: Option<request::GotoDefinitionResponse> =
            client.call(request::GotoDefinition::METHOD, input).await?;
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

        let client = LANGUAGE_CLIENT.get_client(language_id)?;
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

        let client = LANGUAGE_CLIENT.get_client(language_id)?;
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

        let client = LANGUAGE_CLIENT.get_client(language_id)?;
        client
            .notify(notification::DidChangeTextDocument::METHOD, input)
            .await
    }

    pub async fn text_document_rename(
        &self,
        language_id: &str,
        input: super::vim::RenameParams,
    ) -> Fallible<Option<WorkspaceEdit>> {
        let params = RenameParams {
            text_document_position: input.text_document_position.into(),
            new_name: input.new_name,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let client = LANGUAGE_CLIENT.get_client(language_id)?;
        let response: Option<WorkspaceEdit> = client.call(request::Rename::METHOD, params).await?;
        Ok(response)
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

        let client = LANGUAGE_CLIENT.get_client(language_id)?;
        client
            .notify(notification::DidOpenTextDocument::METHOD, input)
            .await
    }

    pub async fn text_document_code_lens(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentIdentifier,
    ) -> Fallible<Vec<CodeLens>> {
        let input = CodeLensParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let client = LANGUAGE_CLIENT.get_client(language_id)?;
        let response: Option<Vec<CodeLens>> =
            client.call(request::CodeLensRequest::METHOD, input).await?;
        let response = response.unwrap_or_default();
        if response.is_empty() {
            return Ok(vec![]);
        }

        let state = self.state.try_lock()?;
        let capabilities = state.server_capabilities.get(language_id);
        if capabilities.is_none() {
            return Ok(response);
        }
        let capabilities = capabilities.unwrap();

        if capabilities.code_lens_provider.is_none() {
            return Ok(response);
        }
        let code_lens_provider = capabilities.code_lens_provider.clone().unwrap();

        if !code_lens_provider.resolve_provider.unwrap_or_default() {
            return Ok(response);
        }

        let response = self.resolve_code_lens(language_id, response).await?;
        Ok(response)
    }

    pub async fn resolve_code_lens(
        &self,
        language_id: &str,
        input: Vec<CodeLens>,
    ) -> Fallible<Vec<CodeLens>> {
        let tasks: Vec<_> = input
            .into_iter()
            .map(|cl| {
                let lc = self.clone();
                let language_id = language_id.to_string();
                tokio::task::spawn(async move {
                    if cl.data.is_none() {
                        return cl;
                    }

                    lc.code_lens_resolve(&language_id, &cl).await.unwrap_or(cl)
                })
            })
            .collect();

        let res = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter(|c| c.is_ok())
            .map(|c| c.unwrap())
            .collect();
        Ok(res)
    }

    pub async fn code_lens_resolve(
        &self,
        language_id: &str,
        code_lens: &CodeLens,
    ) -> Fallible<CodeLens> {
        let client = self.get_client(language_id)?;
        let result: CodeLens = client
            .call(request::CodeLensResolve::METHOD, code_lens)
            .await?;
        Ok(result)
    }

    pub async fn text_document_completion(
        &self,
        language_id: &str,
        input: super::vim::TextDocumentPosition,
    ) -> Fallible<Option<CompletionResponse>> {
        let input = CompletionParams {
            text_document_position: input.into(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: Default::default(),
        };

        let client = LANGUAGE_CLIENT.get_client(language_id)?;
        let message = client.call(request::Completion::METHOD, input).await?;

        Ok(message)
    }
}
