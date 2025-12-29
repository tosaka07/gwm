use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Result;

use super::{default_bindings, Config, KeyBinding};

const CONFIG_DIR_NAME: &str = "gwm";
const CONFIG_FILE_NAME: &str = "config.toml";
const LOCAL_CONFIG_DIR: &str = ".gwm";

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from global and local config files
    pub fn load() -> Result<Config> {
        let global_config = Self::load_global()?;
        let local_config = Self::load_local()?;

        Ok(Self::merge(global_config, local_config))
    }

    /// Get global config file path (~/.config/gwm/config.toml or $XDG_CONFIG_HOME/gwm/config.toml)
    pub fn global_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME))
    }

    /// Get local config file path (.gwm/config.toml in repository root)
    pub fn local_config_path() -> Option<PathBuf> {
        Self::find_repo_root().map(|p| p.join(LOCAL_CONFIG_DIR).join(CONFIG_FILE_NAME))
    }

    /// Load global configuration
    fn load_global() -> Result<Option<Config>> {
        let Some(path) = Self::global_config_path() else {
            return Ok(None);
        };

        Self::load_from_file(&path)
    }

    /// Load local configuration from current directory
    fn load_local() -> Result<Option<Config>> {
        let Some(path) = Self::local_config_path() else {
            return Ok(None);
        };

        Self::load_from_file(&path)
    }

    /// Load configuration from a file
    fn load_from_file(path: &Path) -> Result<Option<Config>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(Some(config))
    }

    /// Merge global and local configurations
    /// Local config overrides/extends global config
    fn merge(global: Option<Config>, local: Option<Config>) -> Config {
        let mut config = Config::default();

        // Start with default bindings
        let default_bindings = default_bindings();

        // Apply global config
        if let Some(global) = global {
            config.worktree = global.worktree;
            config.hooks = global.hooks;
            config.bindings = Self::merge_bindings(&default_bindings, &global.bindings);
        } else {
            config.bindings = default_bindings.clone();
        }

        // Apply local config (overrides global)
        if let Some(local) = local {
            // Merge worktree config
            if local.worktree.base_dir.is_some() {
                config.worktree.base_dir = local.worktree.base_dir;
            }
            if !local.worktree.copy_files.is_empty() {
                config.worktree.copy_files = local.worktree.copy_files;
            }

            // Merge bindings
            config.bindings = Self::merge_bindings(&config.bindings, &local.bindings);

            // Merge hooks (append)
            config.hooks.extend(local.hooks);
        }

        config
    }

    /// Merge key bindings (user bindings override defaults for same key+mods combination)
    fn merge_bindings(base: &[KeyBinding], overrides: &[KeyBinding]) -> Vec<KeyBinding> {
        let mut result: Vec<KeyBinding> = base.to_vec();

        for override_binding in overrides {
            // Find and replace existing binding with same key+mods, or add new one
            let existing_idx = result.iter().position(|b| {
                b.key == override_binding.key
                    && b.mods == override_binding.mods
                    && b.mode == override_binding.mode
            });

            if let Some(idx) = existing_idx {
                // Check if this is a "None" action (remove binding)
                if override_binding.action.as_deref() == Some("None") {
                    result.remove(idx);
                } else {
                    result[idx] = override_binding.clone();
                }
            } else if override_binding.action.as_deref() != Some("None") {
                result.push(override_binding.clone());
            }
        }

        result
    }

    /// Find repository root by looking for .git directory
    fn find_repo_root() -> Option<PathBuf> {
        let current = std::env::current_dir().ok()?;
        let mut path = current.as_path();

        loop {
            if path.join(".git").exists() {
                return Some(path.to_path_buf());
            }

            path = path.parent()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_bindings_override() {
        let base = vec![
            KeyBinding::new("j").with_action("MoveDown"),
            KeyBinding::new("k").with_action("MoveUp"),
        ];

        let overrides = vec![KeyBinding::new("j").with_action("CustomAction")];

        let result = ConfigLoader::merge_bindings(&base, &overrides);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].action.as_deref(), Some("CustomAction"));
        assert_eq!(result[1].action.as_deref(), Some("MoveUp"));
    }

    #[test]
    fn test_merge_bindings_remove() {
        let base = vec![
            KeyBinding::new("j").with_action("MoveDown"),
            KeyBinding::new("k").with_action("MoveUp"),
        ];

        let overrides = vec![KeyBinding::new("j").with_action("None")];

        let result = ConfigLoader::merge_bindings(&base, &overrides);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "k");
    }

    #[test]
    fn test_merge_bindings_add() {
        let base = vec![KeyBinding::new("j").with_action("MoveDown")];

        let overrides = vec![KeyBinding::new("x").with_action("Custom")];

        let result = ConfigLoader::merge_bindings(&base, &overrides);

        assert_eq!(result.len(), 2);
    }
}
