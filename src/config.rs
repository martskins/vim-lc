use failure::Fallible;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub log: Log,
    pub diagnostics: Diagnostics,
    pub locations: Locations,
    pub servers: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Log {
    pub output: String,
    pub level: String,
}

impl Default for Log {
    fn default() -> Self {
        Log {
            output: "/tmp/vlc.log".into(),
            // TODO: this should be error as a default
            level: "error".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Locations {
    pub auto_open: bool,
}

impl Default for Locations {
    fn default() -> Self {
        Locations { auto_open: true }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Diagnostics {
    pub auto_open: bool,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Diagnostics { auto_open: true }
    }
}

impl Config {
    pub async fn parse(path: &str) -> Fallible<Config> {
        if path.is_empty() {
            return Ok(Config::default());
        }

        let file = tokio::fs::File::open(path).await;
        if let Err(err) = file {
            eprintln!("Could not open config file: {}", err);
            return Ok(Config::default());
        }

        let mut config = String::new();
        file.unwrap().read_to_string(&mut config).await?;
        let config = toml::from_str(config.as_str())?;
        Ok(config)
    }
}
