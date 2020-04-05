use super::VLC;
use crate::vim::*;
use crate::LANGUAGE_CLIENT;
use failure::Fallible;

impl VLC {
    pub async fn initialize(&self, params: BaseParams) -> Fallible<()> {
        LANGUAGE_CLIENT
            .clone()
            .initialize(&params.language_id)
            .await?;

        LANGUAGE_CLIENT
            .clone()
            .initialized(&params.language_id)
            .await?;

        Ok(())
    }

    pub async fn did_open(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .clone()
            .text_document_did_open(&language_id, params.into())
            .await?;
        Ok(())
    }

    pub async fn did_save(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .clone()
            .text_document_did_save(&language_id, params.into())
            .await?;
        Ok(())
    }

    pub async fn did_close(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .clone()
            .text_document_did_close(&language_id, params.into())
            .await?;
        Ok(())
    }

    pub async fn did_change(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .clone()
            .text_document_did_change(&language_id, params.into())
            .await?;
        Ok(())
    }

    pub async fn implementation(&self, params: TextDocumentPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .clone()
            .text_document_implementation(&language_id, params.into())
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = super::VIM.clone();
        match response.unwrap() {
            lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
                vim.jump_to_location(l.into()).await?
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

    pub async fn hover(&self, params: TextDocumentPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .clone()
            .text_document_hover(&language_id, params.into())
            .await?;
        if response.is_none() {
            return Ok(());
        }

        self.show_hover(response.unwrap()).await?;
        Ok(())
    }

    pub async fn references(&self, params: TextDocumentPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .clone()
            .text_document_references(&language_id, params.into())
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = super::VIM.clone();
        let response = response.unwrap();
        match response.len() {
            1 => {
                vim.jump_to_location(response.first().cloned().unwrap().into())
                    .await?;
            }
            _ => {
                let locations = response.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations).await?;
            }
        }

        Ok(())
    }

    pub async fn completion(
        &self,
        message_id: &jsonrpc_core::Id,
        params: TextDocumentPosition,
    ) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .clone()
            .text_document_completion(&language_id, params.into())
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let list = match response.unwrap() {
            lsp_types::CompletionResponse::Array(vec) => vec
                .into_iter()
                .map(|i| CompletionItem {
                    word: i.label,
                    kind: completion_item_kind(i.kind),
                    ..Default::default()
                })
                .collect(),
            lsp_types::CompletionResponse::List(list) => list
                .items
                .into_iter()
                .map(|i| CompletionItem {
                    word: i.label,
                    kind: completion_item_kind(i.kind),
                    ..Default::default()
                })
                .collect(),
        };

        let list = CompletionList { words: list };
        let mut client = self.clone().client;
        client
            .reply_success(&message_id, serde_json::to_value(&list)?)
            .await?;

        Ok(())
    }

    pub async fn definition(&self, params: TextDocumentPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .clone()
            .text_document_definition(&language_id, params.into())
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = super::VIM.clone();
        match response.unwrap() {
            lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
                vim.jump_to_location(l.into()).await?
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
