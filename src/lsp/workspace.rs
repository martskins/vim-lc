use crate::language_client::Context;
use crate::rpc::RPCClient;
use anyhow::Result;
use lsp_types::{request, WorkDoneProgressParams};
use lsp_types::{request::Request, ExecuteCommandParams};

pub fn execute_command<C, S>(ctx: &Context<C, S>, command: &lsp_types::Command) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    let _: serde_json::Value = ctx.server.as_ref().unwrap().call(
        request::ExecuteCommand::METHOD,
        ExecuteCommandParams {
            command: command.command.clone(),
            arguments: command.arguments.clone().unwrap_or_default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        },
    )?;

    Ok(())
}

pub fn apply_edit<C, S>(
    ctx: &Context<C, S>,
    params: &lsp_types::ApplyWorkspaceEditParams,
) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    crate::vim::apply_workspace_edit(ctx, &params.edit)?;
    Ok(())
}
