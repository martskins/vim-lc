use crate::language_client::Context;
use crate::rpc::RPCClient;
use anyhow::Result;

pub async fn register_ncm2_source<C: RPCClient, S: RPCClient>(ctx: &Context<C, S>) -> Result<()> {
    let state = ctx.state.read();
    let caps = state.server_capabilities.get(&ctx.language_id).cloned();
    drop(state);

    if caps.is_none() {
        return Ok(());
    }

    let opts = caps.unwrap().completion_provider;
    if opts.is_none() {
        return Ok(());
    }

    let opts = opts.unwrap();
    let complete_pattern: Vec<_> = opts.trigger_characters.unwrap_or_default();
    let params = serde_json::json!({
        "complete_pattern": complete_pattern,
        "language_id": ctx.language_id,
    });

    ctx.vim
        .notify("vlc#register_ncm2", serde_json::json!([params]))?;
    Ok(())
}
