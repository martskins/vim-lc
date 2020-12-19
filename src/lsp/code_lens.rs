use crate::language_client::Context;
use crate::rpc::RPCClient;
use anyhow::Result;
use lsp_types::{
    request::{self, Request},
    CodeLens,
};

pub fn resolve<C: RPCClient, S: RPCClient>(
    ctx: &Context<C, S>,
    input: &CodeLens,
) -> Result<CodeLens> {
    if input.data.is_none() {
        return Ok(input.clone());
    }

    let state = ctx.state.read();
    let capabilities = state.server_capabilities.get(&ctx.language_id).cloned();
    drop(state);

    if capabilities.is_none()
        || capabilities.as_ref().unwrap().code_lens_provider.is_none()
        || !capabilities
            .as_ref()
            .unwrap()
            .code_lens_provider
            .as_ref()
            .unwrap()
            .resolve_provider
            .unwrap_or_default()
    {
        log::debug!("skipping codeLens/resolve, server is not code lens resolve provider");
        return Ok(input.clone());
    }

    let res: CodeLens = ctx
        .server
        .as_ref()
        .unwrap()
        .call(request::CodeLensResolve::METHOD, input)?;

    Ok(res)
}
