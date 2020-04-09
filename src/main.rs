mod config;
mod language_client;
mod rpc;
mod state;
mod vim;
mod vlc;

use config::Config;
use failure::Fallible;
use futures::executor::block_on;
use language_client::LanguageClient;
use lazy_static::lazy_static;
use std::str::FromStr;

lazy_static! {
    pub static ref VIM: vlc::VLC<rpc::Client> = vlc::VLC::default();
    pub static ref LANGUAGE_CLIENT: LanguageClient<rpc::Client> = LanguageClient::default();
    pub static ref CONFIG: Config = block_on(Config::parse(
        shellexpand::tilde("~/.vlc/config.toml").as_ref()
    ))
    .unwrap();
}

#[tokio::main]
async fn main() -> Fallible<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::from_str(&CONFIG.log.level).unwrap())
        .chain(fern::log_file(&CONFIG.log.output).unwrap())
        .apply()
        .unwrap();

    VIM.run().await?;
    Ok(())
}
