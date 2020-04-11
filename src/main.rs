mod config;
mod language_client;
mod lsp;
mod rpc;
mod state;
mod vim;

use config::Config;
use failure::Fallible;
use language_client::LanguageClient;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    #[structopt(short, long, default_value = "~/.vlc/config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Fallible<()> {
    let opts = Opts::from_args();
    let config = Config::parse(shellexpand::tilde(&opts.config).as_ref()).await?;
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::from_str(&config.log.level).unwrap())
        .chain(fern::log_file(&config.log.output).unwrap())
        .apply()
        .unwrap();

    let lc: LanguageClient<rpc::Client> = language_client::LanguageClient::new(config);
    lc.run().await
}
