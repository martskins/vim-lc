mod config;
mod language_client;
mod rpc;
mod vim;
mod vlc;

use failure::Fallible;

#[tokio::main]
async fn main() -> Fallible<()> {
    vlc::VIM.run().await?;
    Ok(())
}
