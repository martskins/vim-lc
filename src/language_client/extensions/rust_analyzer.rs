use crate::rpc::RPCClient;
use crate::vim;
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

    pub(super) fn rust_analyzer_show_references(
        &self,
        arguments: Option<Vec<Value>>,
    ) -> Fallible<()> {
        let locations = arguments
            .unwrap_or_else(|| vec![])
            .get(2)
            .cloned()
            .unwrap_or_else(|| Value::Array(vec![]));
        let locations: Vec<Location> = serde_json::from_value(locations)?;
        let locations = locations.into_iter().map(|l| l.into()).collect();

        VIM.show_locations(locations)?;
        Ok(())
    }

    pub(super) fn rust_analyzer_run(&self, arguments: Option<Vec<Value>>) -> Fallible<()> {
        // TODO: check for terminal support
        if arguments.is_none() && arguments.as_ref().unwrap().is_empty() {
            return Ok(());
        }

        // TODO: clean up these unwraps
        let args = arguments.unwrap().first().cloned().unwrap();
        let args: std::collections::HashMap<String, Value> = serde_json::from_value(args)?;
        let bin = args.get("bin").unwrap();
        let args: Vec<String> = serde_json::from_value(args.get("args").cloned().unwrap())?;
        let cmd = format!("term {} {}", bin, args.join(" "));
        let command = cmd.replace('"', "");
        VIM.execute(vec![vim::ExecuteParams {
            action: "execute".into(),
            command,
        }])?;

        Ok(())
    }
}
