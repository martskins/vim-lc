use crate::language_client::Context;
use crate::{rpc::RPCClient, vim};
use anyhow::Result;
use lsp_types::ShowMessageParams;

pub fn progress<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    params: lsp_types::ProgressParams,
) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    let message = match params.value {
        lsp_types::ProgressParamsValue::WorkDone(wd) => match wd {
            lsp_types::WorkDoneProgress::Begin(r) => {
                Some(format!("{} {}", r.title, r.message.unwrap_or_default()))
            }
            lsp_types::WorkDoneProgress::Report(r) => r.message,
            lsp_types::WorkDoneProgress::End(r) => r.message,
        },
    };

    if message.is_none() {
        return Ok(());
    }

    let message = vim::Message {
        message: message.unwrap(),
        level: vim::LogLevel::Info,
    };

    vim::show_message(ctx, message)?;
    Ok(())
}

pub fn show_message<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: ShowMessageParams,
) -> Result<()> {
    let message = input.message;
    vim::show_message(
        ctx,
        vim::Message {
            message,
            level: vim::LogLevel::Info,
        },
    )?;

    Ok(())
}
