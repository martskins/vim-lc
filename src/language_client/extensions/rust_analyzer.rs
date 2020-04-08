use crate::rpc::RPCClient;
use crate::LanguageClient;
use crate::VIM;
use failure::Fallible;
use lsp_types::*;
use serde::*;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct RustAnalyzerSourceChanges {
    #[serde(rename = "cursorPosition")]
    cursor_position: Option<TextDocumentPositionParams>,
    #[serde(rename = "workspaceEdit")]
    workspace_edit: WorkspaceEdit,
}

impl<T> LanguageClient<T>
where
    T: RPCClient + Clone + Send + Sync + 'static,
{
    pub(super) fn rust_analyzer_apply_source_change(
        &self,
        arguments: Option<Vec<Value>>,
    ) -> Fallible<()> {
        if arguments.is_none() {
            return Ok(());
        }

        for argument in arguments.unwrap() {
            let params: RustAnalyzerSourceChanges = serde_json::from_value(argument)?;
            VIM.apply_edits(params.workspace_edit)?;
        }

        Ok(())
    }
}
