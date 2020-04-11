use crate::rpc::RPCClient;
use crate::vim;
use crate::LanguageClient;
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

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
// This type is defined in rust-analyzer. Should this ever cause any issues we could consider
// importing the crate directly and using it from there.
struct Runnable {
    pub range: Range,
    pub label: String,
    pub bin: String,
    pub args: Vec<String>,
    pub extra_args: Vec<String>,
    pub cwd: Option<String>,
}

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
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
            self.apply_edits(params.workspace_edit)?;
        }

        Ok(())
    }

    pub(super) async fn rust_analyzer_show_references(
        &self,
        arguments: Option<Vec<Value>>,
    ) -> Fallible<()> {
        let locations = arguments
            .unwrap_or_default()
            .get(2)
            .cloned()
            .unwrap_or_else(|| Value::Array(vec![]));
        let locations: Vec<Location> = serde_json::from_value(locations)?;
        let locations = locations.into_iter().map(|l| l.into()).collect();

        self.show_locations(locations).await?;
        Ok(())
    }

    pub(super) fn rust_analyzer_run(&self, arguments: Option<Vec<Value>>) -> Fallible<()> {
        // TODO: check for terminal support
        if arguments.is_none() && arguments.as_ref().unwrap().is_empty() {
            return Ok(());
        }

        let args = arguments.unwrap().first().cloned().unwrap();
        let args: Runnable = serde_json::from_value(args)?;
        let cmd = format!("term {} {}", args.bin, args.args.join(" "));
        let command = cmd.replace('"', "");
        self.execute(vec![vim::ExecuteParams {
            action: "execute".into(),
            command,
        }])?;

        Ok(())
    }
}
