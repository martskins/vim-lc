pub use super::types::*;
use crate::language_client::LanguageClient;
use crate::rpc::RPCClient;
use failure::Fallible;

// #[async_trait]
// pub trait VimReceiver {
//     async fn code_action(&self, params: SelectionRange) -> Fallible<()>;
//     async fn code_lens(&self, params: TextDocumentIdentifier) -> Fallible<()>;
//     async fn code_lens_action(&self, position: CursorPosition) -> Fallible<()>;
//     async fn completion(&self, id: &jsonrpc_core::Id, pos: CursorPosition) -> Fallible<()>;
//     async fn definition(&self, params: CursorPosition) -> Fallible<()>;
//     async fn did_change(&self, params: TextDocumentContent) -> Fallible<()>;
//     async fn did_close(&self, params: TextDocumentContent) -> Fallible<()>;
//     async fn did_open(&self, params: TextDocumentContent) -> Fallible<()>;
//     async fn did_save(&self, params: TextDocumentContent) -> Fallible<()>;
//     async fn hover(&self, params: CursorPosition) -> Fallible<()>;
//     async fn implementation(&self, params: CursorPosition) -> Fallible<()>;
//     async fn references(&self, params: CursorPosition) -> Fallible<()>;
//     async fn rename(&self, params: RenameParams) -> Fallible<()>;
// }

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
{
    pub async fn rename(&self, params: RenameParams) -> Fallible<()> {
        let language_id = params.position.language_id.clone();
        let response = self.text_document_rename(&language_id, params).await?;
        if response.is_none() {
            return Ok(());
        }

        self.apply_edits(response.unwrap())?;
        Ok(())
    }

    pub async fn did_open(&self, params: TextDocumentContent) -> Fallible<()> {
        if !self.config.features.did_open {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        self.text_document_did_open(&language_id, params.clone())
            .await?;
        Ok(())
    }

    pub async fn did_save(&self, params: TextDocumentContent) -> Fallible<()> {
        if !self.config.features.did_save {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        self.text_document_did_save(&language_id, params.clone())
            .await?;
        self.code_lens(params.into()).await?;
        Ok(())
    }

    pub async fn did_close(&self, params: TextDocumentContent) -> Fallible<()> {
        if !self.config.features.did_close {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        self.text_document_did_close(&language_id, params).await?;
        Ok(())
    }

    pub async fn did_change(&self, params: TextDocumentContent) -> Fallible<()> {
        if !self.config.features.did_change {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        self.text_document_did_change(&language_id, params.clone())
            .await?;
        Ok(())
    }

    pub async fn implementation(&self, params: CursorPosition) -> Fallible<()> {
        if !self.config.features.implementation {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        let response = self
            .text_document_implementation(&language_id, params)
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = self.clone();
        match response.unwrap() {
            lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
                vim.jump_to_location(l.into())?
            }
            lsp_types::request::GotoDefinitionResponse::Array(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations).await?
            }
            lsp_types::request::GotoDefinitionResponse::Link(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations).await?
            }
        }

        Ok(())
    }

    pub async fn hover(&self, params: CursorPosition) -> Fallible<()> {
        if !self.config.features.hover {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        let response = self.text_document_hover(&language_id, params).await?;
        if response.is_none() {
            return Ok(());
        }

        self.show_hover(response.unwrap())?;
        Ok(())
    }

    pub async fn references(&self, params: CursorPosition) -> Fallible<()> {
        if !self.config.features.references {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        let response = self.text_document_references(&language_id, params).await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = self.clone();
        let response = response.unwrap();
        match response.len() {
            1 => {
                vim.jump_to_location(response.first().cloned().unwrap().into())?;
            }
            _ => {
                let locations = response.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations).await?;
            }
        }

        Ok(())
    }

    pub async fn code_action(&self, params: SelectionRange) -> Fallible<()> {
        if !self.config.features.code_action {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        let response: Vec<lsp_types::CodeActionOrCommand> =
            self.text_document_code_action(&language_id, params).await?;
        if response.is_empty() {
            return Ok(());
        }

        let actions: Vec<Action> = response
            .into_iter()
            .map(|a| match a {
                lsp_types::CodeActionOrCommand::Command(command) => Action {
                    text: command.title,
                    command: command.command,
                },
                lsp_types::CodeActionOrCommand::CodeAction(action) => Action {
                    text: action.title,
                    command: action.command.unwrap_or_default().command,
                },
            })
            .collect();

        self.show_in_fzf(actions)?;
        Ok(())
    }

    pub async fn code_lens_action(&self, position: CursorPosition) -> Fallible<()> {
        if !self.config.features.code_action {
            return Ok(());
        }

        let code_lens = self.code_lens_for_position(position).await?;
        self.show_in_fzf(code_lens)?;
        Ok(())
    }

    pub async fn code_lens(&self, params: TextDocumentIdentifier) -> Fallible<()> {
        if !self.config.features.code_lens {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        let response: Vec<lsp_types::CodeLens> =
            self.text_document_code_lens(&language_id, params).await?;
        if response.is_empty() {
            return Ok(());
        }

        let mut virtual_texts = vec![];
        response.into_iter().for_each(|cl| {
            if cl.command.is_none() {
                return;
            }

            let text = cl.command.unwrap().title;
            let line = cl.range.start.line;

            match virtual_texts
                .iter()
                .position(|v: &VirtualText| v.line == line)
            {
                Some(idx) => virtual_texts[idx]
                    .text
                    .push_str(format!(" | {}", text).as_str()),
                None => virtual_texts.push(VirtualText {
                    line,
                    text,
                    hl_group: HLGroup::Comment,
                }),
            }
        });

        if virtual_texts.is_empty() {
            return Ok(());
        }

        self.vim.notify("setVirtualTexts", virtual_texts)?;
        Ok(())
    }

    pub async fn completion(
        &self,
        message_id: &jsonrpc_core::Id,
        params: CursorPosition,
    ) -> Fallible<()> {
        if !self.config.completion.enabled {
            self.vim.reply_success(&message_id, serde_json::json!([]))?;
            return Ok(());
        }

        let language_id = params.language_id.clone();
        let response = self.text_document_completion(&language_id, params).await?;
        if response.is_none() {
            return Ok(());
        }

        fn menu_from_documentation(d: Option<lsp_types::Documentation>) -> Option<String> {
            match d {
                Some(lsp_types::Documentation::String(s)) => Some(
                    s.split('\n')
                        .collect::<Vec<&str>>()
                        .first()
                        .cloned()
                        .unwrap()
                        .to_owned(),
                ),
                Some(lsp_types::Documentation::MarkupContent(mc)) => Some(mc.value),
                _ => None,
            }
        }

        let list = match response.unwrap() {
            lsp_types::CompletionResponse::Array(vec) => vec
                .into_iter()
                .map(|i| CompletionItem {
                    word: i.label,
                    kind: completion_item_kind(i.kind),
                    menu: menu_from_documentation(i.documentation),
                    ..Default::default()
                })
                .collect(),
            lsp_types::CompletionResponse::List(list) => list
                .items
                .into_iter()
                .map(|i| CompletionItem {
                    word: i.label,
                    kind: completion_item_kind(i.kind),
                    menu: menu_from_documentation(i.documentation),
                    ..Default::default()
                })
                .collect(),
        };

        let list = CompletionList { words: list };
        self.vim
            .reply_success(&message_id, serde_json::to_value(&list)?)?;

        Ok(())
    }

    pub async fn definition(&self, params: CursorPosition) -> Fallible<()> {
        if !self.config.features.definition {
            return Ok(());
        }

        let language_id = params.language_id.clone();
        let response = self.text_document_definition(&language_id, params).await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = self.clone();
        match response.unwrap() {
            lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
                vim.jump_to_location(l.into())?
            }
            lsp_types::request::GotoDefinitionResponse::Array(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations).await?
            }
            lsp_types::request::GotoDefinitionResponse::Link(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations).await?
            }
        }

        Ok(())
    }
}
