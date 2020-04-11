mod rust_analyzer;

use crate::rpc::RPCClient;
use crate::LanguageClient;
use failure::Fallible;
use lsp_types::*;

impl<T> LanguageClient<T>
where
    T: RPCClient + Send + Sync + Clone + 'static,
{
    pub async fn run_command(&self, language_id: &str, cmd: Command) -> Fallible<()> {
        match cmd.command.as_str() {
            "rust-analyzer.applySourceChange" => {
                self.rust_analyzer_apply_source_change(cmd.arguments)?
            }
            "rust-analyzer.showReferences" => {
                self.rust_analyzer_show_references(cmd.arguments).await?
            }
            "rust-analyzer.run" | "rust-analyzer.runSingle" => {
                self.rust_analyzer_run(cmd.arguments)?
            }
            _ => self.workspace_execute_command(language_id, cmd).await?,
        }

        Ok(())
    }
}
