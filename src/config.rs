use crate::rpc::RPCClient;
use anyhow::Result;
use jsonrpc_core::Value;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub initialization_options: Option<Value>,
    #[serde(default)]
    pub features: FeatureSet,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeatureSet {
    pub code_lenses: bool,
    pub code_actions: bool,
    pub completion: bool,
    pub diagnostics: bool,
}

impl Default for FeatureSet {
    fn default() -> Self {
        Self {
            code_lenses: true,
            code_actions: true,
            completion: true,
            diagnostics: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub servers: HashMap<String, ServerConfig>,
    #[serde(default)]
    pub log: Log,
    #[serde(default)]
    pub hover: Hover,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Completion {
    pub enabled: bool,
    pub strategy: CompletionStrategy,
}

impl Default for Completion {
    fn default() -> Self {
        Completion {
            enabled: true,
            strategy: CompletionStrategy::default(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub enum CompletionStrategy {
    #[serde(rename = "omnifunc")]
    Omnifunc,
    #[serde(rename = "ncm2")]
    NCM2,
}

impl Default for CompletionStrategy {
    fn default() -> Self {
        CompletionStrategy::Omnifunc
    }
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
        DisplayMode::FloatingWindow
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Hover {
    pub strategy: DisplayMode,
    pub preferred_markup_kind: Vec<lsp_types::MarkupKind>,
}

impl Default for Hover {
    fn default() -> Self {
        Hover {
            strategy: DisplayMode::default(),
            preferred_markup_kind: vec![lsp_types::MarkupKind::PlainText],
        }
    }
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
    pub fn parse<C: RPCClient>(vim: &C) -> Result<Config> {
        let req = r#"{
            "servers": get(g:, 'vlc#servers', {}),
            "log": {
                "level": get(g:, 'vlc#log#level', 'error'),
                "output": get(g:, 'vlc#log#output', '/tmp/vlc.log'),
            },
        }"#;

        let config: Config = vim.call("eval", [req.replace("\n", "")])?;
        Ok(config)
    }

    pub fn server(&self, language_id: &str) -> Result<&ServerConfig> {
        let command = self.servers.get(language_id).ok_or(anyhow::anyhow!(
            "no server command found for filetype {}",
            language_id
        ))?;

        Ok(command)
    }

    pub fn features(&self, language_id: &str) -> Result<&FeatureSet> {
        let command = self.servers.get(language_id).ok_or(anyhow::anyhow!(
            "no server command found for filetype {}",
            language_id
        ))?;

        Ok(&command.features)
    }
}
