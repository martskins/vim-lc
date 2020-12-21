pub mod rust_analyzer;

use crate::language_client::Context;
use crate::rpc::RPCClient;
use anyhow::Result;
use lsp_types::*;

pub fn run_command<C, S>(ctx: &Context<C, S>, cmd: &Command) -> Result<()>
where
    C: RPCClient,
    S: RPCClient,
{
    match cmd.command.as_str() {
        "rust-analyzer.applySourceChange" => {
            rust_analyzer::apply_source_change(ctx, &cmd.arguments)?
        }
        "rust-analyzer.showReferences" => rust_analyzer::show_references(ctx, &cmd.arguments)?,
        "rust-analyzer.run" | "rust-analyzer.runSingle" => rust_analyzer::run(ctx, &cmd.arguments)?,
        _ => crate::lsp::workspace::execute_command(ctx, cmd)?,
    }

    Ok(())
}
