use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub worktree: WorktreeConfig,

    #[serde(default)]
    pub bindings: Vec<KeyBinding>,

    #[serde(default)]
    pub hooks: Vec<Hook>,
}

/// Worktree-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorktreeConfig {
    /// Base directory template for new worktrees
    /// Supports placeholders: {name} for branch name, {repo} for repository name
    /// Example: "../worktrees/{name}", "/tmp/wt/{repo}/{name}"
    #[serde(default)]
    pub base_dir: Option<String>,

    /// Files/directories to copy when creating a new worktree (glob patterns)
    #[serde(default)]
    pub copy_files: Vec<String>,
}

/// Key binding configuration (Alacritty-style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    /// Key to bind (e.g., "j", "Enter", "Esc", "F1")
    pub key: String,

    /// Modifier keys (e.g., "Control", "Shift", "Alt", "Control|Shift")
    #[serde(default)]
    pub mods: Option<String>,

    /// Mode restriction (e.g., "Normal", "Insert", "Search", "~Normal")
    #[serde(default)]
    pub mode: Option<String>,

    /// Built-in action to execute
    #[serde(default)]
    pub action: Option<String>,

    /// Shell command to execute
    #[serde(default)]
    pub command: Option<String>,

    /// Characters to send (for Insert mode)
    #[serde(default)]
    pub chars: Option<String>,
}

impl KeyBinding {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            mods: None,
            mode: None,
            action: None,
            command: None,
            chars: None,
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_mods(mut self, mods: impl Into<String>) -> Self {
        self.mods = Some(mods.into());
        self
    }

    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = Some(mode.into());
        self
    }
}

/// Hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// Event that triggers this hook
    pub event: HookEvent,

    /// Command to execute
    pub command: String,

    /// Working directory (defaults to target worktree)
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Hook event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreCreate,
    PostCreate,
    PreDelete,
    PostDelete,
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookEvent::PreCreate => write!(f, "pre_create"),
            HookEvent::PostCreate => write!(f, "post_create"),
            HookEvent::PreDelete => write!(f, "pre_delete"),
            HookEvent::PostDelete => write!(f, "post_delete"),
        }
    }
}
