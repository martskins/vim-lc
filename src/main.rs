mod language_client;
mod rpc;
mod vlc;

use failure::Fallible;

#[tokio::main]
async fn main() -> Fallible<()> {
    vlc::run().await?;

    Ok(())
}
