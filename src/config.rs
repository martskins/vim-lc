use failure::Fallible;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub log: Log,
    pub diagnostics: Diagnostics,
    pub hover: Hover,
    pub locations: Locations,
    pub servers: HashMap<String, String>,
    pub features: FeatureFlags,
}

#[derive(Debug, Clone, Deserialize)]
pub enum DisplayMode {
    #[serde(rename = "preview")]
    Preview,
    #[serde(rename = "floating_window")]
    FloatingWindow,
}

impl Default for DisplayMode {
    fn default() -> Self {
        DisplayMode::Preview
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Hover {
    pub display_mode: DisplayMode,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
// INFO: some of these make no sense to toggle, might revisit this later.
pub struct FeatureFlags {
    pub code_lens: bool,
    pub code_lens_resolve: bool,
    pub code_action: bool,
    pub completion: bool,
    pub references: bool,
    pub definition: bool,
    pub implementation: bool,
    pub hover: bool,
    pub diagnostics: bool,
    pub rename: bool,
    pub did_close: bool,
    pub did_open: bool,
    pub did_change: bool,
    pub did_save: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            code_lens: true,
            code_lens_resolve: true,
            code_action: true,
            completion: true,
            references: true,
            definition: true,
            implementation: true,
            hover: true,
            diagnostics: true,
            rename: true,
            did_close: true,
            did_open: true,
            did_change: true,
            did_save: true,
        }
    }
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
            output: shellexpand::tilde("~/.vlc/vlc.log").into(),
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
    pub show_signs: bool,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Diagnostics {
            auto_open: true,
            show_signs: false,
        }
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
