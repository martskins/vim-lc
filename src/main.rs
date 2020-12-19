mod config;
mod language_client;
mod lsp;
mod rpc;
mod state;
mod vim;

use anyhow::Result;
use config::Config;
use language_client::LanguageClient;
use rpc::RPCClient;
use std::str::FromStr;
use tokio::io::BufReader;

#[tokio::main]
async fn main() -> Result<()> {
    let vim = crate::rpc::Client::new(
        rpc::ServerID::VIM,
        BufReader::new(tokio::io::stdin()),
        tokio::io::stdout(),
    );
    let config = Config::parse(&vim)?;

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

    let lc: LanguageClient<rpc::Client, rpc::Client> =
        language_client::LanguageClient::new(vim, config);

    deadlock_detection();
    Ok(lc.run().await)
}

fn deadlock_detection() {
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
}
