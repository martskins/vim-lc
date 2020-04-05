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
    pub static ref VIM: vlc::VLC = vlc::VLC::new();
    pub static ref LANGUAGE_CLIENT: LanguageClient = LanguageClient::new();
    pub static ref CONFIG: Config =
        block_on(Config::parse("/home/martin/Desktop/config.toml")).unwrap();
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
