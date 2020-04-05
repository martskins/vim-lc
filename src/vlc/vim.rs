use super::VIM;
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
    fn jump_to_location() {
        unimplemented!();
    }

    pub async fn show_message(&self, message: Message) -> Fallible<()> {
        let mut client = VIM.clone().client;
        client.notify("showMessage", message).await?;
        Ok(())
    }

    pub async fn log_message(&self, params: lsp_types::LogMessageParams) -> Fallible<()> {
        log::debug!("{}", params.message);
        Ok(())
    }
}
