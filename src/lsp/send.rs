use crate::language_client::LanguageClient;
use crate::rpc;
use crate::rpc::RPCClient;
use crate::vim;
use failure::Fallible;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::*;

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
{
    pub async fn text_document_code_action(
        &self,
        language_id: &str,
        input: vim::SelectionRange,
    ) -> Fallible<Vec<CodeActionOrCommand>> {
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
        let params = CodeLensParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document.clone()).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let client = self.get_client(language_id).await?;
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

    pub async fn initialize(&self, language_id: &str) -> Fallible<()> {
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
        let client = self.get_client(language_id).await?;
        client.call(request::Shutdown::METHOD, ())?;
        Ok(())
    }

    pub async fn exit(&self, language_id: &str) -> Fallible<()> {
        let client = self.get_client(language_id).await?;
        client.notify(notification::Exit::METHOD, ())?;
        Ok(())
    }

    pub async fn initialized(&self, language_id: &str) -> Fallible<()> {
        let client = self.get_client(language_id).await?;
        client.notify(notification::Initialized::METHOD, InitializedParams {})?;
        Ok(())
    }

    pub async fn text_document_implementation(
        &self,
        language_id: &str,
        input: vim::CursorPosition,
    ) -> Fallible<Option<request::GotoImplementationResponse>> {
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
        let input = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
        };

        let client = self.get_client(language_id).await?;
        client.notify(notification::DidSaveTextDocument::METHOD, input)
    }

    pub async fn text_document_did_close(
        &self,
        language_id: &str,
        input: vim::TextDocumentContent,
    ) -> Fallible<()> {
        let mut state = self.state.write().await;
        state.text_documents.remove(&input.text_document);

        let input = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(input.text_document).unwrap(),
            },
        };

        let client = self.get_client(language_id).await?;
        client.notify(notification::DidCloseTextDocument::METHOD, input)
    }

    pub async fn text_document_did_change(
        &self,
        language_id: &str,
        input: vim::TextDocumentContent,
    ) -> Fallible<()> {
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

        let client = self.get_client(language_id).await?;
        client.notify(notification::DidChangeTextDocument::METHOD, input)
    }

    pub async fn text_document_rename(
        &self,
        language_id: &str,
        input: vim::RenameParams,
    ) -> Fallible<Option<WorkspaceEdit>> {
        let params = RenameParams {
            text_document_position: input.position.into(),
            new_name: input.new_name,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let client = self.get_client(language_id).await?;
        let response: Option<WorkspaceEdit> = client.call(request::Rename::METHOD, params)?;
        Ok(response)
    }

    pub async fn text_document_did_open(
        &self,
        language_id: &str,
        input: vim::TextDocumentContent,
    ) -> Fallible<()> {
        let state = self.state.clone();
        let mut state = state.write().await;
        let mut version = state.text_documents.get(&input.text_document).cloned();

        if version.is_none() {
            let v = (0, input.text.split('\n').map(|l| l.to_owned()).collect());
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

        let client = self.get_client(language_id).await?;
        client.notify(notification::DidOpenTextDocument::METHOD, input)
    }

    pub async fn code_lens_resolve(
        &self,
        language_id: &str,
        code_lens: &CodeLens,
    ) -> Fallible<CodeLens> {
        let client = self.get_client(language_id).await?;
        let result: CodeLens = client.call(request::CodeLensResolve::METHOD, code_lens)?;
        Ok(result)
    }

    pub async fn text_document_completion(
        &self,
        language_id: &str,
        input: vim::CursorPosition,
    ) -> Fallible<Option<CompletionResponse>> {
        let input = CompletionParams {
            text_document_position: input.into(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: Default::default(),
        };

        let client = self.get_client(language_id).await?;
        let message = client.call(request::Completion::METHOD, input)?;

        Ok(message)
    }

    pub async fn workspace_execute_command(
        &self,
        language_id: &str,
        command: lsp_types::Command,
    ) -> Fallible<()> {
        let client = self.get_client(language_id).await?;
        let _: serde_json::Value = client.call(
            request::ExecuteCommand::METHOD,
            ExecuteCommandParams {
                command: command.command,
                arguments: command.arguments.unwrap_or_default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        )?;

        Ok(())
    }

    pub async fn text_document_hover(
        &self,
        language_id: &str,
        input: vim::CursorPosition,
    ) -> Fallible<Option<Hover>> {
        let input: TextDocumentPositionParams = input.into();
        let client = self.get_client(language_id).await?;
        let response: Option<Hover> = client.call(request::HoverRequest::METHOD, input)?;

        Ok(response)
    }
}
