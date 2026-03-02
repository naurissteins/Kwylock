use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_ui_command")]
    pub ui_command: Vec<String>,
    #[serde(default)]
    pub auth: AuthConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            ui_command: default_ui_command(),
            auth: AuthConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_max_failures_before_lockout")]
    pub max_failures_before_lockout: u32,
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,
    #[serde(default = "default_lockout_seconds")]
    pub lockout_seconds: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            max_failures_before_lockout: default_max_failures_before_lockout(),
            initial_backoff_ms: default_initial_backoff_ms(),
            max_backoff_ms: default_max_backoff_ms(),
            lockout_seconds: default_lockout_seconds(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    daemon: DaemonConfig,
}

pub fn load_daemon_config() -> Result<DaemonConfig> {
    let config_path = resolve_config_path();
    if !config_path.exists() {
        return Ok(DaemonConfig::default());
    }

    let config_data = fs::read_to_string(&config_path)
        .with_context(|| format!("failed reading config file {}", config_path.display()))?;
    let parsed: FileConfig = toml::from_str(&config_data)
        .with_context(|| format!("failed parsing config file {}", config_path.display()))?;
    Ok(parsed.daemon)
}

fn resolve_config_path() -> PathBuf {
    if let Some(path) = env::var_os("KWYLOCK_CONFIG") {
        return PathBuf::from(path);
    }

    let base_config = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base_config.join("kwylock").join("config.toml")
}

fn default_ui_command() -> Vec<String> {
    vec!["kwylock-ui".to_string()]
}

fn default_max_failures_before_lockout() -> u32 {
    5
}

fn default_initial_backoff_ms() -> u64 {
    1_000
}

fn default_max_backoff_ms() -> u64 {
    30_000
}

fn default_lockout_seconds() -> u64 {
    60
}
