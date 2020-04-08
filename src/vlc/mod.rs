mod vim;

use crate::rpc;
use crate::rpc::RPCClient;
use crate::vim::*;
use crate::LANGUAGE_CLIENT;
use failure::Fallible;
use tokio::io::{stdin, stdout, BufReader};

#[derive(Debug)]
pub struct VLC<T> {
    client: T,
    root_path: String,
}

impl Default for VLC<rpc::Client> {
    fn default() -> VLC<rpc::Client> {
        let client = rpc::Client::new(rpc::ServerID::VIM, BufReader::new(stdin()), stdout());
        let root_path = std::env::current_dir().unwrap();
        let root_path = format!("file://{}/", root_path.to_str().unwrap());
        Self { client, root_path }
    }
}

impl<T> Clone for VLC<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        VLC {
            client: self.client.clone(),
            root_path: self.root_path.clone(),
        }
    }
}

impl<T> VLC<T>
where
    T: RPCClient,
{
    pub async fn run(&self) -> Fallible<()> {
        loop {
            let message = self.client.read()?;
            if let Err(err) = self.handle_message(message).await {
                log::error!("{}", err);
            }
        }
    }

    async fn initialize(&self, params: BufInfo) -> Fallible<()> {
        LANGUAGE_CLIENT.initialize(&params.language_id).await?;
        LANGUAGE_CLIENT.initialized(&params.language_id).await?;
        Ok(())
    }

    async fn exit(&self, params: BufInfo) -> Fallible<()> {
        LANGUAGE_CLIENT.exit(&params.language_id).await?;
        Ok(())
    }

    async fn shutdown(&self, params: BufInfo) -> Fallible<()> {
        LANGUAGE_CLIENT.shutdown(&params.language_id).await?;
        Ok(())
    }

    async fn rename(&self, params: RenameParams) -> Fallible<()> {
        let language_id = params.position.language_id.clone();
        let response = LANGUAGE_CLIENT
            .text_document_rename(&language_id, params)
            .await?;
        if response.is_none() {
            return Ok(());
        }

        self.apply_edits(response.unwrap())?;
        Ok(())
    }

    async fn did_open(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .text_document_did_open(&language_id, params.clone())
            .await?;
        self.code_lens(params.into()).await?;
        Ok(())
    }

    async fn did_save(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .text_document_did_save(&language_id, params.clone())
            .await?;
        self.code_lens(params.into()).await?;
        Ok(())
    }

    async fn did_close(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .text_document_did_close(&language_id, params)
            .await?;
        Ok(())
    }

    async fn did_change(&self, params: TextDocumentContent) -> Fallible<()> {
        let language_id = params.language_id.clone();
        LANGUAGE_CLIENT
            .text_document_did_change(&language_id, params.clone())
            .await?;
        self.code_lens(params.into()).await?;
        Ok(())
    }

    async fn implementation(&self, params: CursorPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .text_document_implementation(&language_id, params)
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = super::VIM.clone();
        match response.unwrap() {
            lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
                vim.jump_to_location(l.into())?
            }
            lsp_types::request::GotoDefinitionResponse::Array(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations)?
            }
            lsp_types::request::GotoDefinitionResponse::Link(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations)?
            }
        }

        Ok(())
    }

    async fn hover(&self, params: CursorPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .text_document_hover(&language_id, params)
            .await?;
        if response.is_none() {
            return Ok(());
        }

        self.show_hover(response.unwrap())?;
        Ok(())
    }

    async fn references(&self, params: CursorPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .text_document_references(&language_id, params)
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = super::VIM.clone();
        let response = response.unwrap();
        match response.len() {
            1 => {
                vim.jump_to_location(response.first().cloned().unwrap().into())?;
            }
            _ => {
                let locations = response.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations)?;
            }
        }

        Ok(())
    }

    async fn code_action(&self, params: TextDocumentIdentifier) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response: Vec<lsp_types::CodeAction> = LANGUAGE_CLIENT
            .text_document_code_action(&language_id, params)
            .await?;
        if response.is_empty() {
            return Ok(());
        }

        Ok(())
    }

    async fn code_lens(&self, params: TextDocumentIdentifier) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response: Vec<lsp_types::CodeLens> = LANGUAGE_CLIENT
            .text_document_code_lens(&language_id, params)
            .await?;
        if response.is_empty() {
            return Ok(());
        }

        let virtual_texts: Vec<Option<VirtualText>> = response
            .into_iter()
            .map(|cl| {
                let text = cl.command?.title;
                let line = cl.range.start.line;

                Some(VirtualText {
                    line,
                    text,
                    hl_group: "Comment".into(),
                })
            })
            .filter(|i| !i.is_none())
            .collect();

        self.client.notify("setVirtualTexts", virtual_texts)?;
        Ok(())
    }

    async fn completion(
        &self,
        message_id: &jsonrpc_core::Id,
        params: CursorPosition,
    ) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .text_document_completion(&language_id, params)
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
        self.client
            .reply_success(&message_id, serde_json::to_value(&list)?)?;

        Ok(())
    }

    async fn definition(&self, params: CursorPosition) -> Fallible<()> {
        let language_id = params.language_id.clone();
        let response = LANGUAGE_CLIENT
            .text_document_definition(&language_id, params)
            .await?;
        if response.is_none() {
            return Ok(());
        }

        let vim = super::VIM.clone();
        match response.unwrap() {
            lsp_types::request::GotoDefinitionResponse::Scalar(l) => {
                vim.jump_to_location(l.into())?
            }
            lsp_types::request::GotoDefinitionResponse::Array(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations)?
            }
            lsp_types::request::GotoDefinitionResponse::Link(ll) => {
                let locations = ll.into_iter().map(|l| l.into()).collect();
                vim.show_locations(locations)?
            }
        }

        Ok(())
    }

    /// handles messages sent from vim to the language client
    async fn handle_message(&self, message: rpc::Message) -> Fallible<()> {
        let message_id = message.id();
        match message {
            rpc::Message::MethodCall(msg) => match msg.method.as_str() {
                "start" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    LANGUAGE_CLIENT
                        .clone()
                        .start_server(&params.language_id)
                        .await?;
                }
                "initialize" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    self.initialize(params).await?;
                }
                "shutdown" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    self.shutdown(params).await?;
                }
                "exit" => {
                    let params: BufInfo = serde_json::from_value(msg.params.into())?;
                    self.exit(params).await?;
                }
                "textDocument/completion" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.completion(&message_id, params).await?;
                }
                "textDocument/codeLens" => {
                    let params: TextDocumentIdentifier = serde_json::from_value(msg.params.into())?;
                    self.code_lens(params).await?;
                }
                "textDocument/codeAction" => {
                    let params: TextDocumentIdentifier = serde_json::from_value(msg.params.into())?;
                    self.code_action(params).await?;
                }
                "textDocument/definition" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.definition(params).await?;
                }
                "textDocument/hover" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.hover(params).await?;
                }
                "textDocument/references" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.references(params).await?;
                }
                "textDocument/rename" => {
                    let params: RenameParams = serde_json::from_value(msg.params.into()).unwrap();
                    self.rename(params).await?;
                }
                "textDocument/implementation" => {
                    let params: CursorPosition = serde_json::from_value(msg.params.into())?;
                    self.implementation(params).await?;
                }
                _ => log::debug!("unhandled method call {}", msg.method),
            },
            rpc::Message::Notification(msg) => match msg.method.as_str() {
                "textDocument/didSave" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_save(params).await?;
                }
                "textDocument/didOpen" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_open(params).await?;
                }
                "textDocument/didClose" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_close(params).await?;
                }
                "textDocument/didChange" => {
                    let params: TextDocumentContent = serde_json::from_value(msg.params.into())?;
                    self.did_change(params).await?;
                }
                _ => log::debug!("unhandled notification {}", msg.method),
            },
            rpc::Message::Output(_) => unreachable!(),
        }

        Ok(())
    }
}
