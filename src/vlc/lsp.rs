use super::VLC;
use crate::config::Config;
use crate::language_client::LANGUAGE_CLIENT;
use crate::rpc;
use crate::rpc::Client;
use crate::vim::*;
use failure::Fallible;
use futures::executor::block_on;
use lazy_static::lazy_static;
use std::str::FromStr;

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
        LANGUAGE_CLIENT
            .clone()
            .text_document_definition(&language_id, params.into())
            .await?;
        Ok(())
    }
}
