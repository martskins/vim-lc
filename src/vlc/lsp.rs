use super::VLC;
use crate::language_client::LANGUAGE_CLIENT;
use crate::vim::*;
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
