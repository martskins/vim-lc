mod extensions;

use crate::rpc;
use crate::rpc::RPCClient;
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
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct LanguageClient<T> {
    clients: Arc<RwLock<HashMap<String, T>>>,
    state: Arc<RwLock<State>>,
}

impl<T> Clone for LanguageClient<T>
where
    T: Clone,
{
    fn clone(&self) -> LanguageClient<T> {
        Self {
            clients: self.clients.clone(),
            state: self.state.clone(),
        }
    }
}

impl LanguageClient<rpc::Client> {
    // runs the binary specified in the config file for the given language_id
    pub async fn start_server(&self, language_id: &str) -> Fallible<()> {
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

        self.spawn_reader(language_id.into(), client).await?;

        Ok(())
    }
}

impl<T> Default for LanguageClient<T> {
    fn default() -> Self {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        let state = Arc::new(RwLock::new(State::default()));
        Self { clients, state }
    }
}

#[allow(deprecated)]
impl<T> LanguageClient<T>
where
    T: RPCClient + Clone + Send + Sync + 'static,
{
    async fn spawn_reader(&self, language_id: String, client: T) -> Fallible<()> {
        self.clients
            .write()
            .await
            .insert(language_id, client.clone());

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

    async fn get_client(&self, language_id: &str) -> Fallible<T> {
        log::debug!("started LanguageClient::get_client");
        let client = self.clients.read().await;
        let client = client.get(language_id).cloned();
        if client.is_none() {
            failure::bail!("server not running for language {}", language_id);
        }

        Ok(client.unwrap())
    }

    pub async fn get_line(&self, file: &str, line_number: u64) -> Fallible<String> {
        log::debug!("started LanguageClient::get_line");
        let state = self.state.read().await;
        let text_document = state.text_documents.get(file).cloned();
        drop(state);

        let idx = line_number as usize - 1;
        match text_document {
            Some((_, lines)) => Ok(lines[idx].clone()),
            None => {
                use tokio::io::AsyncReadExt;
                let mut file = tokio::fs::File::open(file).await?;
                let mut text = String::new();
                file.read_to_string(&mut text).await?;
                let lines: Vec<&str> = text.split('\n').collect();
                let line = lines[idx].to_owned();
                Ok(line)
            }
        }
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

        let code_lens = code_lens
            .unwrap()
            .iter()
            .filter(|x| x.range.start.line + 1 == position.position.line)
            .cloned()
            .collect();

        Ok(code_lens)
    }

    // handles messages sent from vim to the language client
    async fn handle_message(&self, message: rpc::Message) -> Fallible<()> {
        log::debug!("started LanguageClient::handle_message");
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                _ => log::debug!("unhandled method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "window/logMessage" => {
                    let params: lsp_types::LogMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    VIM.log_message(params)?;
                }
                "textDocument/publishDiagnostics" => {
                    let params: lsp_types::PublishDiagnosticsParams =
                        serde_json::from_value(msg.params.into())?;
                    self.text_document_publish_diagnostics(params)?;
                }
                "$/progress" => {
                    let params: lsp_types::ProgressParams =
                        serde_json::from_value(msg.params.into())?;
                    self.progress(params)?;
                }
                "window/showMessage" => {
                    let params: lsp_types::ShowMessageParams =
                        serde_json::from_value(msg.params.into())?;
                    self.window_show_message(params)?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }

    pub async fn text_document_hover(
        &self,
        language_id: &str,
        input: vim::CursorPosition,
    ) -> Fallible<Option<Hover>> {
        log::debug!("started LanguageClient::text_document_hover");
        let input: TextDocumentPositionParams = input.into();
        let client = self.get_client(language_id).await?;
        let response: Option<Hover> = client.call(request::HoverRequest::METHOD, input)?;

        Ok(response)
    }

    pub fn text_document_publish_diagnostics(
        &self,
        input: PublishDiagnosticsParams,
    ) -> Fallible<()> {
        log::debug!("started LanguageClient::text_document_publish_diagnostics");
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

        VIM.show_diagnostics(diagnostics)?;
        Ok(())
    }

    pub fn window_show_message(&self, input: ShowMessageParams) -> Fallible<()> {
        log::debug!("started LanguageClient::window_show_message");
        let message = input.message;
        VIM.show_message(vim::Message {
            message,
            level: vim::LogLevel::Info,
        })?;

        Ok(())
    }

    pub fn progress(&self, params: lsp_types::ProgressParams) -> Fallible<()> {
        log::debug!("started LanguageClient::progress");
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
            level: vim::LogLevel::Info,
        };

        VIM.show_message(message)?;
        Ok(())
    }

    pub async fn initialize(&self, language_id: &str) -> Fallible<()> {
        log::debug!("started LanguageClient::initialize");
        let client = self.get_client(language_id).await?;
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

        let res: InitializeResult = client.call(request::Initialize::METHOD, message)?;

        let mut state = self.state.write().await;
        state
            .server_capabilities
            .insert(language_id.into(), res.capabilities);

        Ok(())
    }

    pub async fn shutdown(&self, language_id: &str) -> Fallible<()> {
        log::debug!("started LanguageClient::shutdown");
        let client = self.get_client(language_id).await?;
        client.call(request::Shutdown::METHOD, ())?;
        Ok(())
    }

    pub async fn exit(&self, language_id: &str) -> Fallible<()> {
        log::debug!("started LanguageClient::exit");
        let client = self.get_client(language_id).await?;
        client.notify(notification::Exit::METHOD, ())?;
        Ok(())
    }

    pub async fn initialized(&self, language_id: &str) -> Fallible<()> {
        log::debug!("started LanguageClient::initialized");
        let client = self.get_client(language_id).await?;
        client.notify(notification::Initialized::METHOD, InitializedParams {})?;
        Ok(())
    }

    pub async fn text_document_implementation(
        &self,
        language_id: &str,
        input: vim::CursorPosition,
    ) -> Fallible<Option<request::GotoImplementationResponse>> {
        log::debug!("started LanguageClient::text_document_implementation");
        let input: TextDocumentPositionParams = input.into();
        let client = self.get_client(language_id).await?;
        let message: Option<request::GotoImplementationResponse> =
            client.call(request::GotoImplementation::METHOD, input)?;
        Ok(message)
    }

    pub async fn text_document_references(
        &self,
        language_id: &str,
        input: vim::CursorPosition,
    ) -> Fallible<Option<Vec<lsp_types::Location>>> {
        log::debug!("started LanguageClient::text_document_references");
        let input: ReferenceParams = input.into();
        let client = self.get_client(language_id).await?;
        let message: Option<Vec<lsp_types::Location>> =
            client.call(request::References::METHOD, input)?;
        Ok(message)
    }

    pub async fn text_document_definition(
        &self,
        language_id: &str,
        params: vim::CursorPosition,
    ) -> Fallible<Option<request::GotoDefinitionResponse>> {
        log::debug!("started LanguageClient::text_document_definition");
        let input: TextDocumentPositionParams = params.into();
        let client = self.get_client(language_id).await?;
        let message: Option<request::GotoDefinitionResponse> =
            client.call(request::GotoDefinition::METHOD, input)?;
        Ok(message)
    }

    pub async fn text_document_did_save(
        &self,
        language_id: &str,
        input: vim::TextDocumentContent,
    ) -> Fallible<()> {
        log::debug!("started LanguageClient::text_document_did_save");
        let input = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
        };

        let client = LANGUAGE_CLIENT.get_client(language_id).await?;
        client.notify(notification::DidSaveTextDocument::METHOD, input)
    }

    pub async fn text_document_did_close(
        &self,
        language_id: &str,
        input: vim::TextDocumentContent,
    ) -> Fallible<()> {
        log::debug!("started LanguageClient::text_document_did_close");
        let mut state = self.state.write().await;
        state.text_documents.remove(&input.text_document);

        let input = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
        };

        let client = LANGUAGE_CLIENT.get_client(language_id).await?;
        client.notify(notification::DidCloseTextDocument::METHOD, input)
    }

    pub async fn text_document_did_change(
        &self,
        language_id: &str,
        input: vim::TextDocumentContent,
    ) -> Fallible<()> {
        log::debug!("started LanguageClient::text_document_did_change");
        let state = self.state.clone();
        let state = state.read().await;
        let (version, _) = state
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

        let client = LANGUAGE_CLIENT.get_client(language_id).await?;
        client.notify(notification::DidChangeTextDocument::METHOD, input)
    }

    pub async fn text_document_rename(
        &self,
        language_id: &str,
        input: vim::RenameParams,
    ) -> Fallible<Option<WorkspaceEdit>> {
        log::debug!("started LanguageClient::text_document_rename");
        let params = RenameParams {
            text_document_position: input.position.into(),
            new_name: input.new_name,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let client = LANGUAGE_CLIENT.get_client(language_id).await?;
        let response: Option<WorkspaceEdit> = client.call(request::Rename::METHOD, params)?;
        Ok(response)
    }

    pub async fn text_document_did_open(
        &self,
        language_id: &str,
        input: vim::TextDocumentContent,
    ) -> Fallible<()> {
        log::debug!("started LanguageClient::text_did_open");
        let state = self.state.clone();
        let mut state = state.write().await;
        let mut version = state.text_documents.get(&input.text_document).cloned();

        if version.is_none() {
            let v = (0, input.text.split("\n").map(|l| l.to_owned()).collect());
            state
                .text_documents
                .insert(input.text_document.clone(), v.clone());
            version = Some(v);
        }

        let (version, _) = version.unwrap();
        let input = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::from_file_path(input.text_document).unwrap(),
                language_id: input.language_id,
                version: version as i64,
                text: input.text,
            },
        };

        let client = LANGUAGE_CLIENT.get_client(language_id).await?;
        client.notify(notification::DidOpenTextDocument::METHOD, input)
    }

    pub async fn resolve_code_lens_action(
        &self,
        input: vim::ResolveCodeActionParams,
    ) -> Fallible<()> {
        log::debug!("started LanguageClient::resolve_code_lens_action");
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
            .map(|x| x.command.clone().unwrap())
            .collect();

        if code_lens.is_empty() {
            return Ok(());
        }

        // this should always have at most one item
        let code_lens = code_lens.first().cloned().unwrap();
        if let Err(err) = self.run_command(code_lens).await {
            log::error!("{}", err);
        }

        Ok(())
    }

    pub async fn resolve_code_action(&self, input: vim::ResolveCodeActionParams) -> Fallible<()> {
        log::debug!("started LanguageClient::resolve_code_action");
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

                    self.run_command(action.command.unwrap()).await?;
                }
                CodeActionOrCommand::Command(command) if command.title == input.selection => {
                    log::error!("COMMAND: {:?}", command.title);
                }
                _ => {}
            }
        }

        let mut state = self.state.write().await;
        state.code_actions = vec![];

        Ok(())
    }

    pub async fn text_document_code_action(
        &self,
        language_id: &str,
        input: vim::SelectionRange,
    ) -> Fallible<Vec<CodeActionOrCommand>> {
        log::debug!("started LanguageClient::text_document_code_action");
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            range: Range {
                start: input.range.start.into(),
                end: input.range.end.into(),
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: None,
            },
        };

        let client = self.get_client(language_id).await?;
        let res: Option<CodeActionResponse> =
            client.call(request::CodeActionRequest::METHOD, params)?;

        if res.is_none() {
            return Ok(vec![]);
        }

        let actions = res.unwrap();
        let mut state = self.state.write().await;
        state.code_actions = actions.clone();

        Ok(actions)
    }

    pub async fn text_document_code_lens(
        &self,
        language_id: &str,
        input: vim::TextDocumentIdentifier,
    ) -> Fallible<Vec<CodeLens>> {
        log::debug!("started LanguageClient::text_document_code_lens");
        let params = CodeLensParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document.clone()).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let client = LANGUAGE_CLIENT.get_client(language_id).await?;
        let response: Option<Vec<CodeLens>> =
            client.call(request::CodeLensRequest::METHOD, params)?;
        let response = response.unwrap_or_default();
        if response.is_empty() {
            return Ok(vec![]);
        }

        let state = self.state.read().await;
        let capabilities = state.server_capabilities.get(language_id).cloned();
        drop(state);

        if capabilities.is_none() {
            log::debug!("skipping codeLens/resolve, capabilities is None");
            self.store_code_lens(&input.text_document, response.clone())
                .await;
            return Ok(response);
        }
        let capabilities = capabilities.unwrap();

        if capabilities.code_lens_provider.is_none() {
            log::debug!("skipping codeLens/resolve, server is not codeLens provider");
            self.store_code_lens(&input.text_document, response.clone())
                .await;
            return Ok(response);
        }
        let code_lens_provider = capabilities.code_lens_provider.clone().unwrap();

        if !code_lens_provider.resolve_provider.unwrap_or_default() {
            log::debug!("skipping codeLens/resolve, server is not codeLens resolve provider");
            self.store_code_lens(&input.text_document, response.clone())
                .await;
            return Ok(response);
        }

        let response = self.resolve_code_lens(language_id, response).await?;
        self.store_code_lens(&input.text_document, response.clone())
            .await;

        Ok(response)
    }

    async fn store_code_lens(&self, filename: &str, code_lens: Vec<CodeLens>) {
        log::error!("CL: {:?}", code_lens);
        let mut state = self.state.write().await;
        state.code_lens.insert(filename.into(), code_lens);
    }

    pub async fn resolve_code_lens(
        &self,
        language_id: &str,
        input: Vec<CodeLens>,
    ) -> Fallible<Vec<CodeLens>> {
        if !CONFIG.features.code_lens_resolve {
            return Ok(input);
        }

        log::debug!("started LanguageClient::resolve_code_lens");
        let res: Vec<_> = input
            .into_iter()
            .map(|cl| {
                let lc = self.clone();
                let language_id = language_id.to_string();
                if cl.data.is_none() {
                    return cl;
                }

                futures::executor::block_on(lc.code_lens_resolve(&language_id, &cl)).unwrap_or(cl)
            })
            .collect();

        Ok(res)
    }

    pub async fn code_lens_resolve(
        &self,
        language_id: &str,
        code_lens: &CodeLens,
    ) -> Fallible<CodeLens> {
        log::debug!("started LanguageClient::code_lens_resolve");
        let client = self.get_client(language_id).await?;
        let result: CodeLens = client.call(request::CodeLensResolve::METHOD, code_lens)?;
        Ok(result)
    }

    pub async fn text_document_completion(
        &self,
        language_id: &str,
        input: vim::CursorPosition,
    ) -> Fallible<Option<CompletionResponse>> {
        log::debug!("started LanguageClient::text_document_completion");
        let input = CompletionParams {
            text_document_position: input.into(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: Default::default(),
        };

        let client = LANGUAGE_CLIENT.get_client(language_id).await?;
        let message = client.call(request::Completion::METHOD, input)?;

        Ok(message)
    }
}
