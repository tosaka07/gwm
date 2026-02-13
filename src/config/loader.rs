use crate::git::RepoInfo;
use crate::theme::ThemeColorsConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Unresolved template variable(s) in naming template: {0}")]
    UnresolvedTemplateVariable(String),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WorktreeConfig {
    /// Base directory for worktrees
    pub basedir: Option<String>,
    /// Automatically create base directory if it doesn't exist
    pub auto_mkdir: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NamingConfig {
    /// Directory naming template (supports {branch} variable)
    pub template: Option<String>,
    /// Characters to sanitize in names (e.g., "/" -> "-")
    pub sanitize_chars: Option<HashMap<String, String>>,
}

impl NamingConfig {
    /// Get the default sanitize_chars map
    fn default_sanitize_chars() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("/".to_string(), "-".to_string());
        map
    }

    /// Sanitize a name using the configured character replacements
    pub fn sanitize(&self, name: &str) -> String {
        let chars_map = self
            .sanitize_chars
            .clone()
            .unwrap_or_else(Self::default_sanitize_chars);

        let mut result = name.to_string();
        for (from, to) in chars_map {
            result = result.replace(&from, &to);
        }
        result
    }

    /// Generate worktree directory name from branch name using template
    /// Supports variables: {branch}, {host}, {owner}, {repository}
    /// Returns an error if template variables remain unreplaced
    pub fn generate_worktree_name(
        &self,
        branch_name: &str,
        repo_info: Option<&RepoInfo>,
    ) -> Result<String, ConfigError> {
        let sanitized_branch = self.sanitize(branch_name);

        if let Some(template) = &self.template {
            let mut result = template.replace("{branch}", &sanitized_branch);

            if let Some(info) = repo_info {
                result = result
                    .replace("{host}", &info.host)
                    .replace("{owner}", &info.owner)
                    .replace("{repository}", &info.repository);
            }

            let unreplaced = Self::find_unreplaced_variables(&result);
            if !unreplaced.is_empty() {
                return Err(ConfigError::UnresolvedTemplateVariable(
                    unreplaced.join(", "),
                ));
            }

            Ok(result)
        } else {
            Ok(sanitized_branch)
        }
    }

    /// Find unreplaced template variables in a string
    fn find_unreplaced_variables(s: &str) -> Vec<&'static str> {
        let known_variables = ["{host}", "{owner}", "{repository}"];
        known_variables
            .iter()
            .filter(|var| s.contains(**var))
            .copied()
            .collect()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UiConfig {
    /// Show icons in output (requires NerdFont)
    pub icons: Option<bool>,
    /// Display ~ instead of full home path
    pub tilde_home: Option<bool>,
    /// Theme name: "default" (256-color/True Color) or "classic" (8-bit 16-color)
    pub theme: Option<String>,
    /// Custom color overrides
    pub colors: Option<ThemeColorsConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RepositorySettings {
    /// Repository path (used as key for merging)
    pub repository: String,
    /// Files to copy from main worktree to new worktree
    pub copy_files: Option<Vec<String>>,
    /// Commands to run after creating worktree
    pub setup_commands: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// Worktree configuration
    #[serde(default)]
    pub worktree: WorktreeConfig,
    /// Naming configuration
    #[serde(default)]
    pub naming: NamingConfig,
    /// UI configuration
    #[serde(default)]
    pub ui: UiConfig,
    /// Per-repository settings
    #[serde(default)]
    pub repository_settings: Vec<RepositorySettings>,
    /// Top-level copy_files (applies to all repositories when no specific repository_settings match)
    #[serde(default)]
    pub copy_files: Option<Vec<String>>,
    /// Top-level setup_commands (applies to all repositories when no specific repository_settings match)
    #[serde(default)]
    pub setup_commands: Option<Vec<String>>,
}

impl Config {
    /// Merge two configs, with `other` taking precedence
    pub fn merge(self, other: Config) -> Config {
        // Merge repository_settings by repository path
        let mut repo_settings_map: HashMap<String, RepositorySettings> = HashMap::new();

        // Add global settings first
        for settings in self.repository_settings {
            repo_settings_map.insert(settings.repository.clone(), settings);
        }

        // Local settings override global
        for settings in other.repository_settings {
            repo_settings_map.insert(settings.repository.clone(), settings);
        }

        let merged_repo_settings: Vec<RepositorySettings> =
            repo_settings_map.into_values().collect();

        Config {
            worktree: WorktreeConfig {
                basedir: other.worktree.basedir.or(self.worktree.basedir),
                auto_mkdir: other.worktree.auto_mkdir.or(self.worktree.auto_mkdir),
            },
            naming: NamingConfig {
                template: other.naming.template.or(self.naming.template),
                sanitize_chars: other.naming.sanitize_chars.or(self.naming.sanitize_chars),
            },
            ui: UiConfig {
                icons: other.ui.icons.or(self.ui.icons),
                tilde_home: other.ui.tilde_home.or(self.ui.tilde_home),
                theme: other.ui.theme.or(self.ui.theme),
                colors: other.ui.colors.or(self.ui.colors),
            },
            repository_settings: merged_repo_settings,
            copy_files: other.copy_files.or(self.copy_files),
            setup_commands: other.setup_commands.or(self.setup_commands),
        }
    }

    /// Get the worktree base directory, defaulting to "~/worktrees"
    pub fn worktree_basedir(&self) -> String {
        self.worktree
            .basedir
            .clone()
            .unwrap_or_else(|| "~/worktrees".to_string())
    }

    /// Expand ~ to home directory
    pub fn expand_path(&self, path: &str) -> String {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return path.replacen("~", &home.to_string_lossy(), 1);
            }
        }
        path.to_string()
    }

    /// Compress home directory to ~ (reverse of expand_path)
    pub fn compress_path(&self, path: &str) -> String {
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy();
            if path.starts_with(home_str.as_ref()) {
                return path.replacen(home_str.as_ref(), "~", 1);
            }
        }
        path.to_string()
    }

    /// Format path for display (compress to ~ if tilde_home is enabled)
    pub fn format_path_for_display(&self, path: &str) -> String {
        if self.tilde_home() {
            self.compress_path(path)
        } else {
            path.to_string()
        }
    }

    /// Get expanded worktree base directory
    #[allow(dead_code)]
    pub fn worktree_basedir_expanded(&self) -> String {
        self.expand_path(&self.worktree_basedir())
    }

    /// Get expanded worktree base directory with repo root for relative paths
    /// - Absolute paths and ~ paths are expanded normally
    /// - Relative paths (starting with . or not starting with /) are resolved from repo_root
    pub fn worktree_basedir_expanded_with_repo_root(&self, repo_root: &Path) -> String {
        let basedir = self.worktree_basedir();

        // Handle ~ expansion first
        if basedir.starts_with("~/") {
            return self.expand_path(&basedir);
        }

        // Absolute path - return as-is
        if basedir.starts_with('/') {
            return basedir;
        }

        // Relative path - resolve from repo_root
        let resolved = repo_root.join(&basedir);
        resolved.to_string_lossy().to_string()
    }

    /// Check if auto_mkdir is enabled (default: true)
    pub fn auto_mkdir(&self) -> bool {
        self.worktree.auto_mkdir.unwrap_or(true)
    }

    /// Generate worktree directory name from branch name
    pub fn generate_worktree_name(
        &self,
        branch_name: &str,
        repo_info: Option<&RepoInfo>,
    ) -> Result<String, ConfigError> {
        self.naming.generate_worktree_name(branch_name, repo_info)
    }

    /// Check if icons are enabled (default: true)
    pub fn icons_enabled(&self) -> bool {
        self.ui.icons.unwrap_or(true)
    }

    /// Check if tilde_home is enabled (default: true)
    pub fn tilde_home(&self) -> bool {
        self.ui.tilde_home.unwrap_or(true)
    }

    /// Get the theme name (default: "default")
    pub fn theme_name(&self) -> &str {
        self.ui.theme.as_deref().unwrap_or("default")
    }

    /// Get the custom theme colors config
    pub fn theme_colors(&self) -> Option<&ThemeColorsConfig> {
        self.ui.colors.as_ref()
    }

    /// Get repository settings for a specific repository path
    pub fn get_repository_settings(&self, repo_path: &str) -> Option<&RepositorySettings> {
        self.repository_settings
            .iter()
            .find(|s| repo_path.ends_with(&s.repository) || s.repository.ends_with(repo_path))
    }

    /// Get effective settings for a repository path, considering top-level defaults
    /// Priority: repository_settings (if matched) > top-level copy_files/setup_commands
    pub fn get_effective_settings(&self, repo_path: &str) -> RepositorySettings {
        // Check if there's a specific repository_settings match
        if let Some(repo_settings) = self.get_repository_settings(repo_path) {
            // Use repository_settings as-is (it overrides top-level settings)
            return repo_settings.clone();
        }

        // Fall back to top-level settings
        RepositorySettings {
            repository: repo_path.to_string(),
            copy_files: self.copy_files.clone(),
            setup_commands: self.setup_commands.clone(),
        }
    }
}

/// Get XDG config directory (respects $XDG_CONFIG_HOME, defaults to ~/.config)
fn get_xdg_config_dir() -> Option<std::path::PathBuf> {
    // First check XDG_CONFIG_HOME environment variable
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.is_empty() {
            return Some(std::path::PathBuf::from(xdg_config_home));
        }
    }

    // Fall back to ~/.config
    dirs::home_dir().map(|home| home.join(".config"))
}

/// Get global config paths in priority order
/// 1. ~/.gwm.toml (simple, traditional UNIX style)
/// 2. $XDG_CONFIG_HOME/gwm/config.toml or ~/.config/gwm/config.toml
fn get_global_config_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // 1. ~/.gwm.toml (highest priority)
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".gwm.toml"));
    }

    // 2. XDG config directory
    if let Some(xdg_config) = get_xdg_config_dir() {
        paths.push(xdg_config.join("gwm").join("config.toml"));
    }

    paths
}

/// Load global config from ~/.gwm.toml or $XDG_CONFIG_HOME/gwm/config.toml
fn load_global_config() -> Result<Option<Config>, ConfigError> {
    for path in get_global_config_paths() {
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(Some(config));
        }
    }

    Ok(None)
}

/// Load local config from .gwm.toml in the current directory or parent directories
fn load_local_config(start_path: &Path) -> Result<Option<Config>, ConfigError> {
    let mut current = start_path.to_path_buf();

    loop {
        // Check for .gwm.toml (new format)
        let config_path = current.join(".gwm.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(Some(config));
        }

        // Also check .gwm/config.toml (old format, for backwards compatibility)
        let old_config_path = current.join(".gwm").join("config.toml");
        if old_config_path.exists() {
            let content = std::fs::read_to_string(&old_config_path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(Some(config));
        }

        if !current.pop() {
            break;
        }
    }

    Ok(None)
}

/// Load config from environment variables
fn load_env_config() -> Config {
    Config {
        worktree: WorktreeConfig {
            basedir: std::env::var("GWM_WORKTREE_BASEDIR").ok(),
            auto_mkdir: std::env::var("GWM_WORKTREE_AUTO_MKDIR")
                .ok()
                .and_then(|v| parse_bool(&v)),
        },
        naming: NamingConfig::default(),
        ui: UiConfig {
            icons: std::env::var("GWM_UI_ICONS")
                .ok()
                .and_then(|v| parse_bool(&v)),
            tilde_home: std::env::var("GWM_UI_TILDE_HOME")
                .ok()
                .and_then(|v| parse_bool(&v)),
            theme: std::env::var("GWM_UI_THEME").ok(),
            colors: None, // Colors can only be set via config file
        },
        repository_settings: Vec::new(),
        copy_files: None,     // copy_files can only be set via config file
        setup_commands: None, // setup_commands can only be set via config file
    }
}

/// Parse boolean from string (supports "true", "false", "1", "0")
fn parse_bool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

/// Load config from a specific file path
fn load_config_from_path(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

/// Load and merge configs (global + local + env)
/// Priority: env > custom/local > global
/// If custom_path is provided, it replaces both global and local config
pub fn load_config(custom_path: Option<&Path>) -> Result<Config, ConfigError> {
    let env = load_env_config();

    if let Some(path) = custom_path {
        let custom = load_config_from_path(path)?;
        return Ok(custom.merge(env));
    }

    let current_dir = std::env::current_dir()?;

    let global = load_global_config()?.unwrap_or_default();
    let local = load_local_config(&current_dir)?.unwrap_or_default();

    Ok(global.merge(local).merge(env))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert!(config.worktree.basedir.is_none());
        assert!(config.worktree.auto_mkdir.is_none());
        assert!(config.ui.icons.is_none());
        assert!(config.ui.tilde_home.is_none());
        assert!(config.repository_settings.is_empty());
    }

    #[test]
    fn test_worktree_basedir_default() {
        let config = Config::default();

        assert_eq!(config.worktree_basedir(), "~/worktrees");
    }

    #[test]
    fn test_worktree_basedir_custom() {
        let config = Config {
            worktree: WorktreeConfig {
                basedir: Some("/custom/path".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(config.worktree_basedir(), "/custom/path");
    }

    #[test]
    fn test_parse_new_format() {
        let toml_content = r#"
            [worktree]
            basedir = "~/my-worktrees"
            auto_mkdir = true

            [ui]
            icons = true
            tilde_home = false

            [[repository_settings]]
            repository = "~/src/myproject"
            copy_files = [".env.example"]
            setup_commands = ["npm install"]
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.worktree.basedir, Some("~/my-worktrees".to_string()));
        assert_eq!(config.worktree.auto_mkdir, Some(true));
        assert_eq!(config.ui.icons, Some(true));
        assert_eq!(config.ui.tilde_home, Some(false));
        assert_eq!(config.repository_settings.len(), 1);
        assert_eq!(config.repository_settings[0].repository, "~/src/myproject");
    }

    #[test]
    fn test_config_merge_repository_settings() {
        let global = Config {
            repository_settings: vec![
                RepositorySettings {
                    repository: "project-a".to_string(),
                    setup_commands: Some(vec!["npm install".to_string()]),
                    copy_files: None,
                },
                RepositorySettings {
                    repository: "project-b".to_string(),
                    setup_commands: Some(vec!["go mod download".to_string()]),
                    copy_files: None,
                },
            ],
            ..Default::default()
        };

        let local = Config {
            repository_settings: vec![
                RepositorySettings {
                    repository: "project-a".to_string(),
                    setup_commands: Some(vec!["yarn install".to_string()]),
                    copy_files: None,
                },
                RepositorySettings {
                    repository: "project-c".to_string(),
                    setup_commands: Some(vec!["make setup".to_string()]),
                    copy_files: None,
                },
            ],
            ..Default::default()
        };

        let merged = global.merge(local);

        assert_eq!(merged.repository_settings.len(), 3);

        // project-a should be overridden by local
        let project_a = merged
            .repository_settings
            .iter()
            .find(|s| s.repository == "project-a")
            .unwrap();
        assert_eq!(
            project_a.setup_commands,
            Some(vec!["yarn install".to_string()])
        );
    }

    #[test]
    fn test_default_values() {
        let config = Config::default();

        assert!(config.auto_mkdir());
        assert!(config.icons_enabled());
        assert!(config.tilde_home());
    }

    #[test]
    fn test_get_repository_settings_exact_match() {
        let config = Config {
            repository_settings: vec![RepositorySettings {
                repository: "my-project".to_string(),
                setup_commands: Some(vec!["npm install".to_string()]),
                copy_files: None,
            }],
            ..Default::default()
        };

        let settings = config.get_repository_settings("my-project");
        assert!(settings.is_some());
        assert_eq!(settings.unwrap().repository, "my-project");
    }

    #[test]
    fn test_get_repository_settings_ends_with_match() {
        let config = Config {
            repository_settings: vec![RepositorySettings {
                repository: "my-project".to_string(),
                setup_commands: Some(vec!["npm install".to_string()]),
                copy_files: None,
            }],
            ..Default::default()
        };

        // repo_path ends with repository
        let settings = config.get_repository_settings("/home/user/src/my-project");
        assert!(settings.is_some());
        assert_eq!(settings.unwrap().repository, "my-project");
    }

    #[test]
    fn test_get_repository_settings_repository_ends_with_repo_path() {
        let config = Config {
            repository_settings: vec![RepositorySettings {
                repository: "~/src/my-project".to_string(),
                setup_commands: Some(vec!["npm install".to_string()]),
                copy_files: None,
            }],
            ..Default::default()
        };

        // repository ends with repo_path
        let settings = config.get_repository_settings("my-project");
        assert!(settings.is_some());
        assert_eq!(settings.unwrap().repository, "~/src/my-project");
    }

    #[test]
    fn test_get_repository_settings_no_match() {
        let config = Config {
            repository_settings: vec![RepositorySettings {
                repository: "other-project".to_string(),
                setup_commands: Some(vec!["npm install".to_string()]),
                copy_files: None,
            }],
            ..Default::default()
        };

        let settings = config.get_repository_settings("/home/user/src/my-project");
        assert!(settings.is_none());
    }

    #[test]
    fn test_get_repository_settings_empty() {
        let config = Config::default();

        let settings = config.get_repository_settings("/home/user/src/my-project");
        assert!(settings.is_none());
    }

    #[test]
    fn test_expand_path_with_tilde() {
        let config = Config::default();

        let expanded = config.expand_path("~/worktrees");

        // Should start with home directory, not ~
        assert!(!expanded.starts_with("~"));
        assert!(expanded.ends_with("/worktrees"));
    }

    #[test]
    fn test_expand_path_without_tilde() {
        let config = Config::default();

        let expanded = config.expand_path("/absolute/path");

        assert_eq!(expanded, "/absolute/path");
    }

    #[test]
    fn test_expand_path_relative() {
        let config = Config::default();

        let expanded = config.expand_path("relative/path");

        assert_eq!(expanded, "relative/path");
    }

    #[test]
    fn test_worktree_basedir_expanded() {
        let config = Config::default();

        let expanded = config.worktree_basedir_expanded();

        // Default is ~/worktrees, should be expanded
        assert!(!expanded.starts_with("~"));
        assert!(expanded.ends_with("/worktrees"));
    }

    #[test]
    fn test_worktree_basedir_expanded_with_repo_root_tilde() {
        let config = Config::default(); // basedir = ~/worktrees
        let repo_root = std::path::Path::new("/some/repo");

        let expanded = config.worktree_basedir_expanded_with_repo_root(repo_root);

        // ~ should be expanded to home, not relative to repo_root
        assert!(!expanded.starts_with("~"));
        assert!(expanded.ends_with("/worktrees"));
        assert!(!expanded.starts_with("/some/repo"));
    }

    #[test]
    fn test_worktree_basedir_expanded_with_repo_root_absolute() {
        let config = Config {
            worktree: WorktreeConfig {
                basedir: Some("/absolute/path".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let repo_root = std::path::Path::new("/some/repo");

        let expanded = config.worktree_basedir_expanded_with_repo_root(repo_root);

        // Absolute path should remain unchanged
        assert_eq!(expanded, "/absolute/path");
    }

    #[test]
    fn test_worktree_basedir_expanded_with_repo_root_relative() {
        let config = Config {
            worktree: WorktreeConfig {
                basedir: Some(".git/wt".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let repo_root = std::path::Path::new("/some/repo");

        let expanded = config.worktree_basedir_expanded_with_repo_root(repo_root);

        // Relative path should be resolved from repo_root
        assert_eq!(expanded, "/some/repo/.git/wt");
    }

    #[test]
    fn test_worktree_basedir_expanded_with_repo_root_parent_relative() {
        let config = Config {
            worktree: WorktreeConfig {
                basedir: Some("../worktrees".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let repo_root = std::path::Path::new("/some/repo");

        let expanded = config.worktree_basedir_expanded_with_repo_root(repo_root);

        // Parent relative path should be resolved from repo_root
        assert_eq!(expanded, "/some/repo/../worktrees");
    }

    #[test]
    fn test_compress_path_with_home() {
        let config = Config::default();
        let home = dirs::home_dir().unwrap();
        let full_path = format!("{}/projects/test", home.to_string_lossy());

        let compressed = config.compress_path(&full_path);

        assert!(compressed.starts_with("~"));
        assert_eq!(compressed, "~/projects/test");
    }

    #[test]
    fn test_compress_path_without_home() {
        let config = Config::default();

        let compressed = config.compress_path("/var/log/test");

        assert_eq!(compressed, "/var/log/test");
    }

    #[test]
    fn test_format_path_for_display_tilde_home_enabled() {
        let config = Config::default(); // tilde_home defaults to true
        let home = dirs::home_dir().unwrap();
        let full_path = format!("{}/projects/test", home.to_string_lossy());

        let formatted = config.format_path_for_display(&full_path);

        assert_eq!(formatted, "~/projects/test");
    }

    #[test]
    fn test_format_path_for_display_tilde_home_disabled() {
        let config = Config {
            ui: UiConfig {
                tilde_home: Some(false),
                ..Default::default()
            },
            ..Default::default()
        };
        let home = dirs::home_dir().unwrap();
        let full_path = format!("{}/projects/test", home.to_string_lossy());

        let formatted = config.format_path_for_display(&full_path);

        // Should NOT compress when tilde_home is false
        assert_eq!(formatted, full_path);
    }

    #[test]
    fn test_parse_bool_true_values() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("yes"), Some(true));
        assert_eq!(parse_bool("YES"), Some(true));
    }

    #[test]
    fn test_parse_bool_false_values() {
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("no"), Some(false));
        assert_eq!(parse_bool("NO"), Some(false));
    }

    #[test]
    fn test_parse_bool_invalid_values() {
        assert_eq!(parse_bool(""), None);
        assert_eq!(parse_bool("invalid"), None);
        assert_eq!(parse_bool("2"), None);
    }

    #[test]
    #[serial]
    fn test_load_env_config_basedir() {
        // Save original value
        let original = std::env::var("GWM_WORKTREE_BASEDIR").ok();

        std::env::set_var("GWM_WORKTREE_BASEDIR", "/custom/path");
        let config = load_env_config();
        assert_eq!(config.worktree.basedir, Some("/custom/path".to_string()));

        // Restore original value
        match original {
            Some(v) => std::env::set_var("GWM_WORKTREE_BASEDIR", v),
            None => std::env::remove_var("GWM_WORKTREE_BASEDIR"),
        }
    }

    #[test]
    #[serial]
    fn test_load_env_config_booleans() {
        // Save original values
        let orig_icons = std::env::var("GWM_UI_ICONS").ok();
        let orig_tilde = std::env::var("GWM_UI_TILDE_HOME").ok();

        std::env::set_var("GWM_UI_ICONS", "false");
        std::env::set_var("GWM_UI_TILDE_HOME", "0");

        let config = load_env_config();
        assert_eq!(config.ui.icons, Some(false));
        assert_eq!(config.ui.tilde_home, Some(false));

        // Restore original values
        match orig_icons {
            Some(v) => std::env::set_var("GWM_UI_ICONS", v),
            None => std::env::remove_var("GWM_UI_ICONS"),
        }
        match orig_tilde {
            Some(v) => std::env::set_var("GWM_UI_TILDE_HOME", v),
            None => std::env::remove_var("GWM_UI_TILDE_HOME"),
        }
    }

    #[test]
    #[serial]
    fn test_env_overrides_local() {
        // Save original value
        let original = std::env::var("GWM_WORKTREE_BASEDIR").ok();

        let local = Config {
            worktree: WorktreeConfig {
                basedir: Some("/local/path".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        std::env::set_var("GWM_WORKTREE_BASEDIR", "/env/path");
        let env = load_env_config();
        let merged = local.merge(env);

        assert_eq!(merged.worktree.basedir, Some("/env/path".to_string()));

        // Restore original value
        match original {
            Some(v) => std::env::set_var("GWM_WORKTREE_BASEDIR", v),
            None => std::env::remove_var("GWM_WORKTREE_BASEDIR"),
        }
    }

    #[test]
    #[serial]
    fn test_env_overrides_custom_config_path() {
        use std::fs;

        // Save original value
        let original = std::env::var("GWM_WORKTREE_BASEDIR").ok();

        // Create temp config file
        let temp_dir = std::env::temp_dir().join("gwm_test_env_override_custom");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("custom.toml");
        let config_content = r#"
            [worktree]
            basedir = "/from-custom-file"
        "#;
        fs::write(&config_path, config_content).unwrap();

        // Set env var
        std::env::set_var("GWM_WORKTREE_BASEDIR", "/from-env");

        // Load config with custom path
        let config = load_config(Some(&config_path)).unwrap();

        // Env should override custom config
        assert_eq!(config.worktree.basedir, Some("/from-env".to_string()));

        // Restore original value
        match original {
            Some(v) => std::env::set_var("GWM_WORKTREE_BASEDIR", v),
            None => std::env::remove_var("GWM_WORKTREE_BASEDIR"),
        }

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // ========== NamingConfig Tests ==========

    #[test]
    fn test_naming_sanitize_default_chars() {
        let naming = NamingConfig::default();

        let result = naming.sanitize("feature/login");

        assert_eq!(result, "feature-login");
    }

    #[test]
    fn test_naming_sanitize_multiple_slashes() {
        let naming = NamingConfig::default();

        let result = naming.sanitize("feature/user/auth");

        assert_eq!(result, "feature-user-auth");
    }

    #[test]
    fn test_naming_sanitize_no_special_chars() {
        let naming = NamingConfig::default();

        let result = naming.sanitize("simple-branch");

        assert_eq!(result, "simple-branch");
    }

    #[test]
    fn test_naming_sanitize_custom_chars() {
        let mut custom_chars = HashMap::new();
        custom_chars.insert("/".to_string(), "_".to_string());
        custom_chars.insert(":".to_string(), "-".to_string());

        let naming = NamingConfig {
            template: None,
            sanitize_chars: Some(custom_chars),
        };

        let result = naming.sanitize("feature/bug:123");

        assert_eq!(result, "feature_bug-123");
    }

    #[test]
    fn test_naming_generate_worktree_name_without_template() {
        let naming = NamingConfig::default();

        let result = naming
            .generate_worktree_name("feature/login", None)
            .unwrap();

        assert_eq!(result, "feature-login");
    }

    #[test]
    fn test_naming_generate_worktree_name_with_template() {
        let naming = NamingConfig {
            template: Some("wt-{branch}".to_string()),
            sanitize_chars: None,
        };

        let result = naming
            .generate_worktree_name("feature/login", None)
            .unwrap();

        assert_eq!(result, "wt-feature-login");
    }

    #[test]
    fn test_naming_generate_worktree_name_with_suffix_template() {
        let naming = NamingConfig {
            template: Some("{branch}-dev".to_string()),
            sanitize_chars: None,
        };

        let result = naming.generate_worktree_name("main", None).unwrap();

        assert_eq!(result, "main-dev");
    }

    #[test]
    fn test_config_generate_worktree_name() {
        let config = Config {
            naming: NamingConfig {
                template: Some("worktree-{branch}".to_string()),
                sanitize_chars: None,
            },
            ..Default::default()
        };

        let result = config.generate_worktree_name("feature/test", None).unwrap();

        assert_eq!(result, "worktree-feature-test");
    }

    #[test]
    fn test_naming_generate_worktree_name_with_repo_info() {
        use crate::git::RepoInfo;

        let naming = NamingConfig {
            template: Some("{host}/{owner}/{repository}/{branch}".to_string()),
            sanitize_chars: None,
        };
        let repo_info = RepoInfo {
            host: "github.com".to_string(),
            owner: "user".to_string(),
            repository: "myrepo".to_string(),
        };

        let result = naming
            .generate_worktree_name("feature/login", Some(&repo_info))
            .unwrap();

        assert_eq!(result, "github.com/user/myrepo/feature-login");
    }

    #[test]
    fn test_naming_generate_worktree_name_partial_repo_info() {
        use crate::git::RepoInfo;

        let naming = NamingConfig {
            template: Some("{owner}-{repository}-{branch}".to_string()),
            sanitize_chars: None,
        };
        let repo_info = RepoInfo {
            host: "github.com".to_string(),
            owner: "myorg".to_string(),
            repository: "project".to_string(),
        };

        let result = naming
            .generate_worktree_name("main", Some(&repo_info))
            .unwrap();

        assert_eq!(result, "myorg-project-main");
    }

    #[test]
    fn test_naming_generate_worktree_name_unreplaced_variables_error() {
        let naming = NamingConfig {
            template: Some("{host}/{owner}/{repository}/{branch}".to_string()),
            sanitize_chars: None,
        };

        let result = naming.generate_worktree_name("feature/login", None);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("{host}"));
        assert!(err_msg.contains("{owner}"));
        assert!(err_msg.contains("{repository}"));
    }

    #[test]
    fn test_naming_generate_worktree_name_partial_unreplaced_variables() {
        let naming = NamingConfig {
            template: Some("{owner}-{branch}".to_string()),
            sanitize_chars: None,
        };

        let result = naming.generate_worktree_name("main", None);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("{owner}"));
        assert!(!err_msg.contains("{host}"));
    }

    #[test]
    fn test_naming_generate_worktree_name_branch_only_template_no_error() {
        let naming = NamingConfig {
            template: Some("wt-{branch}".to_string()),
            sanitize_chars: None,
        };

        let result = naming.generate_worktree_name("feature/login", None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "wt-feature-login");
    }

    // ========== Global Config Path Tests ==========

    #[test]
    #[serial]
    fn test_get_xdg_config_dir_from_env() {
        // Save original value
        let original = std::env::var("XDG_CONFIG_HOME").ok();

        std::env::set_var("XDG_CONFIG_HOME", "/custom/config");
        let result = get_xdg_config_dir();

        assert_eq!(result, Some(std::path::PathBuf::from("/custom/config")));

        // Restore original value
        match original {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    #[serial]
    fn test_get_xdg_config_dir_empty_env() {
        // Save original value
        let original = std::env::var("XDG_CONFIG_HOME").ok();

        std::env::set_var("XDG_CONFIG_HOME", "");
        let result = get_xdg_config_dir();

        // Should fall back to ~/.config
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, Some(home.join(".config")));

        // Restore original value
        match original {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    #[serial]
    fn test_get_xdg_config_dir_no_env() {
        // Save original value
        let original = std::env::var("XDG_CONFIG_HOME").ok();

        std::env::remove_var("XDG_CONFIG_HOME");
        let result = get_xdg_config_dir();

        // Should fall back to ~/.config
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, Some(home.join(".config")));

        // Restore original value
        if let Some(v) = original {
            std::env::set_var("XDG_CONFIG_HOME", v);
        }
    }

    #[test]
    #[serial]
    fn test_get_global_config_paths_order() {
        // Save original value
        let original = std::env::var("XDG_CONFIG_HOME").ok();

        std::env::remove_var("XDG_CONFIG_HOME");
        let paths = get_global_config_paths();

        let home = dirs::home_dir().unwrap();

        // Should have 2 paths in correct order
        assert_eq!(paths.len(), 2);
        // 1. ~/.gwm.toml (highest priority)
        assert_eq!(paths[0], home.join(".gwm.toml"));
        // 2. ~/.config/gwm/config.toml
        assert_eq!(
            paths[1],
            home.join(".config").join("gwm").join("config.toml")
        );

        // Restore original value
        if let Some(v) = original {
            std::env::set_var("XDG_CONFIG_HOME", v);
        }
    }

    #[test]
    #[serial]
    fn test_get_global_config_paths_with_xdg_env() {
        // Save original value
        let original = std::env::var("XDG_CONFIG_HOME").ok();

        std::env::set_var("XDG_CONFIG_HOME", "/custom/xdg");
        let paths = get_global_config_paths();

        let home = dirs::home_dir().unwrap();

        // Should have 2 paths
        assert_eq!(paths.len(), 2);
        // 1. ~/.gwm.toml (highest priority)
        assert_eq!(paths[0], home.join(".gwm.toml"));
        // 2. $XDG_CONFIG_HOME/gwm/config.toml
        assert_eq!(
            paths[1],
            std::path::PathBuf::from("/custom/xdg/gwm/config.toml")
        );

        // Restore original value
        match original {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn test_load_global_config_home_gwm_toml() {
        use std::fs;

        // Create temporary home directory
        let temp_dir = std::env::temp_dir().join("gwm_test_global_home");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create ~/.gwm.toml
        let config_content = r#"
            [worktree]
            basedir = "~/from-home-gwm"
        "#;
        fs::write(temp_dir.join(".gwm.toml"), config_content).unwrap();

        // Also create XDG config (should be ignored due to priority)
        let xdg_dir = temp_dir.join(".config").join("gwm");
        fs::create_dir_all(&xdg_dir).unwrap();
        let xdg_content = r#"
            [worktree]
            basedir = "~/from-xdg"
        "#;
        fs::write(xdg_dir.join("config.toml"), xdg_content).unwrap();

        // Test: ~/.gwm.toml should be loaded (has higher priority)
        let gwm_toml_path = temp_dir.join(".gwm.toml");
        assert!(gwm_toml_path.exists());

        let content = fs::read_to_string(&gwm_toml_path).unwrap();
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(config.worktree.basedir, Some("~/from-home-gwm".to_string()));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_global_config_xdg_fallback() {
        use std::fs;

        // Create temporary XDG directory
        let temp_dir = std::env::temp_dir().join("gwm_test_global_xdg");
        let _ = fs::remove_dir_all(&temp_dir);
        let xdg_dir = temp_dir.join("gwm");
        fs::create_dir_all(&xdg_dir).unwrap();

        // Create config.toml in XDG directory
        let config_content = r#"
            [worktree]
            basedir = "~/from-xdg-fallback"
        "#;
        fs::write(xdg_dir.join("config.toml"), config_content).unwrap();

        // Test: XDG config should be loadable
        let xdg_config_path = xdg_dir.join("config.toml");
        assert!(xdg_config_path.exists());

        let content = fs::read_to_string(&xdg_config_path).unwrap();
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(
            config.worktree.basedir,
            Some("~/from-xdg-fallback".to_string())
        );

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[serial]
    fn test_load_config_from_custom_path() {
        use std::fs;

        // Save and clear env vars that could affect the test
        let orig_basedir = std::env::var("GWM_WORKTREE_BASEDIR").ok();
        std::env::remove_var("GWM_WORKTREE_BASEDIR");

        let temp_dir = std::env::temp_dir().join("gwm_test_custom_config");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("custom.toml");
        let config_content = r#"
            [worktree]
            basedir = "~/custom-worktrees"
            auto_mkdir = false

            [ui]
            icons = false
            tilde_home = true
        "#;
        fs::write(&config_path, config_content).unwrap();

        let config = load_config(Some(&config_path)).unwrap();

        assert_eq!(
            config.worktree.basedir,
            Some("~/custom-worktrees".to_string())
        );
        assert_eq!(config.worktree.auto_mkdir, Some(false));
        assert_eq!(config.ui.icons, Some(false));
        assert_eq!(config.ui.tilde_home, Some(true));

        // Restore env var
        match orig_basedir {
            Some(v) => std::env::set_var("GWM_WORKTREE_BASEDIR", v),
            None => {}
        }

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_config_from_custom_path_not_found() {
        let result = load_config(Some(Path::new("/nonexistent/path/config.toml")));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_without_custom_path() {
        // When no custom path is provided, should not error (uses default loading)
        let result = load_config(None);
        // This should succeed (returns default if no config found)
        assert!(result.is_ok());
    }

    #[test]
    fn test_theme_name_default() {
        let config = Config::default();
        assert_eq!(config.theme_name(), "default");
    }

    #[test]
    fn test_theme_name_custom() {
        let config = Config {
            ui: UiConfig {
                theme: Some("classic".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(config.theme_name(), "classic");
    }

    #[test]
    fn test_theme_colors_none_by_default() {
        let config = Config::default();
        assert!(config.theme_colors().is_none());
    }

    #[test]
    fn test_config_merge_theme() {
        let base = Config {
            ui: UiConfig {
                theme: Some("classic".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let override_config = Config {
            ui: UiConfig {
                theme: Some("default".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let merged = base.merge(override_config);
        assert_eq!(merged.ui.theme, Some("default".to_string()));
    }

    #[test]
    #[serial]
    fn test_load_env_config_theme() {
        std::env::set_var("GWM_UI_THEME", "classic");

        let config = load_env_config();
        assert_eq!(config.ui.theme, Some("classic".to_string()));

        std::env::remove_var("GWM_UI_THEME");
    }

    // ========== Top-level copy_files and setup_commands Tests ==========

    #[test]
    fn test_config_with_top_level_copy_files() {
        let toml_content = r#"
            copy_files = [".env", ".claude"]
            setup_commands = ["npm install"]
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(
            config.copy_files,
            Some(vec![".env".to_string(), ".claude".to_string()])
        );
        assert_eq!(config.setup_commands, Some(vec!["npm install".to_string()]));
    }

    #[test]
    fn test_config_merge_top_level_settings() {
        let global = Config {
            copy_files: Some(vec![".env".to_string()]),
            setup_commands: Some(vec!["make setup".to_string()]),
            ..Default::default()
        };

        let local = Config {
            copy_files: Some(vec![".env.local".to_string()]),
            ..Default::default()
        };

        let merged = global.merge(local);

        // Local copy_files overrides global
        assert_eq!(merged.copy_files, Some(vec![".env.local".to_string()]));
        // Global setup_commands is preserved since local doesn't have it
        assert_eq!(merged.setup_commands, Some(vec!["make setup".to_string()]));
    }

    #[test]
    fn test_config_merge_local_empty_array_disables_global() {
        let global = Config {
            copy_files: Some(vec![".env".to_string(), ".claude".to_string()]),
            setup_commands: Some(vec!["npm install".to_string()]),
            ..Default::default()
        };

        // Local explicitly sets empty array to disable global
        let local = Config {
            copy_files: Some(vec![]),
            ..Default::default()
        };

        let merged = global.merge(local);

        // Local empty array overrides global (disables copy_files)
        assert_eq!(merged.copy_files, Some(vec![]));
        // Global setup_commands is preserved since local doesn't specify it
        assert_eq!(merged.setup_commands, Some(vec!["npm install".to_string()]));
    }

    #[test]
    fn test_get_effective_settings_with_empty_copy_files() {
        let config = Config {
            copy_files: Some(vec![]),
            setup_commands: Some(vec!["npm install".to_string()]),
            ..Default::default()
        };

        let settings = config.get_effective_settings("/home/user/my-project");

        // Empty array is valid (explicitly disabled)
        assert_eq!(settings.copy_files, Some(vec![]));
        assert_eq!(
            settings.setup_commands,
            Some(vec!["npm install".to_string()])
        );
    }

    #[test]
    fn test_get_effective_settings_with_no_match_uses_top_level() {
        let config = Config {
            copy_files: Some(vec![".env".to_string(), ".claude".to_string()]),
            setup_commands: Some(vec!["npm install".to_string()]),
            repository_settings: vec![RepositorySettings {
                repository: "other-project".to_string(),
                copy_files: Some(vec!["other.txt".to_string()]),
                setup_commands: None,
            }],
            ..Default::default()
        };

        // repo_path doesn't match any repository_settings, should fall back to top-level
        let settings = config.get_effective_settings("/home/user/my-project");

        assert_eq!(
            settings.copy_files,
            Some(vec![".env".to_string(), ".claude".to_string()])
        );
        assert_eq!(
            settings.setup_commands,
            Some(vec!["npm install".to_string()])
        );
    }

    #[test]
    fn test_get_effective_settings_with_match_uses_repository_settings() {
        let config = Config {
            copy_files: Some(vec![".env".to_string()]),
            setup_commands: Some(vec!["npm install".to_string()]),
            repository_settings: vec![RepositorySettings {
                repository: "my-project".to_string(),
                copy_files: Some(vec![".env.local".to_string()]),
                setup_commands: Some(vec!["yarn install".to_string()]),
            }],
            ..Default::default()
        };

        // repo_path matches repository_settings
        let settings = config.get_effective_settings("/home/user/my-project");

        // Should use repository_settings, not top-level
        assert_eq!(settings.copy_files, Some(vec![".env.local".to_string()]));
        assert_eq!(
            settings.setup_commands,
            Some(vec!["yarn install".to_string()])
        );
    }

    #[test]
    fn test_get_effective_settings_empty_top_level() {
        let config = Config::default();

        let settings = config.get_effective_settings("/home/user/my-project");

        assert!(settings.copy_files.is_none());
        assert!(settings.setup_commands.is_none());
    }

    #[test]
    fn test_mixed_top_level_and_repository_settings() {
        let toml_content = r#"
            copy_files = [".env", ".claude"]

            [worktree]
            basedir = "~/worktrees"

            [[repository_settings]]
            repository = "special-project"
            copy_files = [".env", ".env.local", "secrets.json"]
            setup_commands = ["make setup"]
        "#;

        let config: Config = toml::from_str(toml_content).unwrap();

        // Top-level settings
        assert_eq!(
            config.copy_files,
            Some(vec![".env".to_string(), ".claude".to_string()])
        );

        // For non-matching repo, use top-level
        let settings1 = config.get_effective_settings("/home/user/normal-project");
        assert_eq!(
            settings1.copy_files,
            Some(vec![".env".to_string(), ".claude".to_string()])
        );

        // For matching repo, use repository_settings
        let settings2 = config.get_effective_settings("/home/user/special-project");
        assert_eq!(
            settings2.copy_files,
            Some(vec![
                ".env".to_string(),
                ".env.local".to_string(),
                "secrets.json".to_string()
            ])
        );
        assert_eq!(
            settings2.setup_commands,
            Some(vec!["make setup".to_string()])
        );
    }
}
