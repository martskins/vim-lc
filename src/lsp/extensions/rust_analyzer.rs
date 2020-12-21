use crate::language_client::Context;
use crate::rpc::RPCClient;
use crate::vim;
use anyhow::Result;
use lsp_types::*;
use serde::*;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct RustAnalyzerSourceChanges {
    #[serde(rename = "cursorPosition")]
    cursor_position: Option<TextDocumentPositionParams>,
    #[serde(rename = "workspaceEdit")]
    workspace_edit: WorkspaceEdit,
}

// Runnable wraps the two possible shapes of a runnable action from rust-analyzer. Old-ish versions
// of it will use BinRunnable, whereas the newer ones use CargoRunnable.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
enum Runnable {
    Bin(BinRunnable),
    Generic(GenericRunnable),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BinRunnable {
    pub label: String,
    pub bin: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct GenericRunnable {
    pub label: String,
    pub kind: GenericRunnableKind,
    pub location: Option<lsp_types::LocationLink>,
    pub args: GenericRunnableArgs,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct GenericRunnableArgs {
    pub workspace_root: Option<PathBuf>,
    pub cargo_args: Vec<String>,
    pub executable_args: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum GenericRunnableKind {
    Cargo,
}

pub(super) fn apply_source_change<C, S>(
    ctx: &Context<C, S>,
    arguments: &Option<Vec<Value>>,
) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    if arguments.is_none() {
        return Ok(());
    }

    for argument in arguments.as_ref().unwrap() {
        let params = RustAnalyzerSourceChanges::deserialize(argument)?;
        crate::vim::apply_workspace_edit(ctx, &params.workspace_edit)?;
    }

    Ok(())
}

pub(super) fn show_references<C, S>(
    ctx: &Context<C, S>,
    arguments: &Option<Vec<Value>>,
) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    let locations = arguments
        .clone()
        .unwrap_or_default()
        .get(2)
        .cloned()
        .unwrap_or_else(|| Value::Array(vec![]));
    let locations: Vec<Location> = serde_json::from_value(locations)?;
    let locations = locations.into_iter().map(|l| l.into()).collect();

    crate::vim::show_locations(ctx, locations)?;
    Ok(())
}

pub(super) fn run<C, S>(ctx: &Context<C, S>, arguments: &Option<Vec<Value>>) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    let has_term: u8 = ctx.vim.call("eval", vec!["exists(':terminal')"])?;
    if has_term == 0 {
        todo!();
    }

    if let Some(ref args) = arguments {
        if let Some(args) = args.first() {
            let runnable = Runnable::deserialize(args)?;
            let cmd = match runnable {
                Runnable::Bin(runnable) => {
                    format!("term {} {}", runnable.bin, runnable.args.join(" "))
                }
                Runnable::Generic(runnable) => format!(
                    "term cargo {} -- {}",
                    runnable.args.cargo_args.join(" "),
                    runnable.args.executable_args.join(" "),
                ),
            };

            crate::vim::execute(
                ctx,
                vec![vim::ExecuteParams {
                    action: "execute".into(),
                    command: cmd.replace('"', ""),
                }],
            )?;
        }
    }

    Ok(())
}
