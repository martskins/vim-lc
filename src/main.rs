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

    let lc: LanguageClient<rpc::Client, rpc::Client> = language_client::LanguageClient::new(config);

    // Create a background thread which checks for deadlocks every 10s

    use parking_lot::deadlock;
    use std::thread;
    use std::time::Duration;
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let deadlocks = deadlock::check_deadlock();
        if deadlocks.is_empty() {
            continue;
        }

        log::error!("{} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            log::error!("Deadlock #{}", i);
            for t in threads {
                log::error!("Thread Id {:#?}", t.thread_id());
                log::error!("{:#?}", t.backtrace());
            }
        }
    });

    lc.run().await
}
