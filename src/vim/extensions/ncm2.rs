use crate::language_client::LanguageClient;
use crate::rpc::RPCClient;
use failure::Fallible;

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
{
    pub async fn register_ncm2_source(&self, language_id: &str) -> Fallible<()> {
        let state = self.state.read().await;
        let caps = state.server_capabilities.get(language_id);
        if caps.is_none() {
            return Ok(());
        }

        let opts = caps.cloned().unwrap().completion_provider;
        if opts.is_none() {
            return Ok(());
        }

        let opts = opts.unwrap();
        let complete_pattern: Vec<_> = opts.trigger_characters.unwrap_or_default();
        let params = serde_json::json!({
            "complete_pattern": complete_pattern,
            "language_id": language_id,
        });

        self.vim.notify("registerNCM2Source", params)?;
        Ok(())
    }
}
