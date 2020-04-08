mod rust_analyzer;

use crate::rpc::RPCClient;
use crate::LanguageClient;
use failure::Fallible;
use lsp_types::*;

impl<T> LanguageClient<T>
where
    T: RPCClient + Clone + Send + Sync + 'static,
{
    pub(super) async fn run_command(&self, cmd: Command) -> Fallible<()> {
        match cmd.command.as_str() {
            "rust-analyzer.applySourceChange" => {
                self.rust_analyzer_apply_source_change(cmd.arguments)?
            }
            _ => {}
        }

        Ok(())
    }
}
