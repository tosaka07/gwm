use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HooksConfig {
    pub post_create: Option<String>,
    pub pre_delete: Option<String>,
    pub post_delete: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// Base directory for worktrees (relative to repository root)
    pub worktree_base: Option<String>,
    /// Default branch for new worktrees
    pub default_branch: Option<String>,
    /// Hooks configuration
    #[serde(default)]
    pub hooks: HooksConfig,
}

impl Config {
    /// Merge two configs, with `other` taking precedence
    pub fn merge(self, other: Config) -> Config {
        Config {
            worktree_base: other.worktree_base.or(self.worktree_base),
            default_branch: other.default_branch.or(self.default_branch),
            hooks: HooksConfig {
                post_create: other.hooks.post_create.or(self.hooks.post_create),
                pre_delete: other.hooks.pre_delete.or(self.hooks.pre_delete),
                post_delete: other.hooks.post_delete.or(self.hooks.post_delete),
            },
        }
    }

    /// Get the worktree base directory, defaulting to ".worktrees"
    pub fn worktree_base(&self) -> &str {
        self.worktree_base.as_deref().unwrap_or(".worktrees")
    }
}

/// Load global config from ~/.config/gwm/config.toml
fn load_global_config() -> Result<Option<Config>, ConfigError> {
    let config_dir = dirs::config_dir().map(|p| p.join("gwm").join("config.toml"));

    if let Some(path) = config_dir {
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(Some(config));
        }
    }

    Ok(None)
}

/// Load local config from .gwm/config.toml in the current directory or parent directories
fn load_local_config(start_path: &Path) -> Result<Option<Config>, ConfigError> {
    let mut current = start_path.to_path_buf();

    loop {
        let config_path = current.join(".gwm").join("config.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(Some(config));
        }

        if !current.pop() {
            break;
        }
    }

    Ok(None)
}

/// Load and merge configs (global + local)
pub fn load_config() -> Result<Config, ConfigError> {
    let current_dir = std::env::current_dir()?;

    let global = load_global_config()?.unwrap_or_default();
    let local = load_local_config(&current_dir)?.unwrap_or_default();

    Ok(global.merge(local))
}
