use crate::config::HooksConfig;
use crate::git::Worktree;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HookError {
    #[error("Hook execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct HookRunner {
    config: HooksConfig,
}

impl HookRunner {
    pub fn new(config: HooksConfig) -> Self {
        Self { config }
    }

    /// Run post_create hook
    pub fn run_post_create(&self, worktree: &Worktree) -> Result<(), HookError> {
        if let Some(cmd) = &self.config.post_create {
            self.run_hook(cmd, worktree)?;
        }
        Ok(())
    }

    /// Run pre_delete hook
    pub fn run_pre_delete(&self, worktree: &Worktree) -> Result<(), HookError> {
        if let Some(cmd) = &self.config.pre_delete {
            self.run_hook(cmd, worktree)?;
        }
        Ok(())
    }

    /// Run post_delete hook
    pub fn run_post_delete(&self, worktree: &Worktree) -> Result<(), HookError> {
        if let Some(cmd) = &self.config.post_delete {
            self.run_hook(cmd, worktree)?;
        }
        Ok(())
    }

    /// Execute a hook command with worktree context
    fn run_hook(&self, cmd: &str, worktree: &Worktree) -> Result<(), HookError> {
        let expanded_cmd = self.expand_variables(cmd, worktree);

        let status = Command::new("sh")
            .arg("-c")
            .arg(&expanded_cmd)
            .current_dir(&worktree.path)
            .status()?;

        if !status.success() {
            return Err(HookError::ExecutionFailed(format!(
                "Command '{}' exited with status: {}",
                expanded_cmd,
                status.code().unwrap_or(-1)
            )));
        }

        Ok(())
    }

    /// Expand variables in hook command
    fn expand_variables(&self, cmd: &str, worktree: &Worktree) -> String {
        cmd.replace("$WORKTREE_NAME", &worktree.name)
            .replace("$WORKTREE_PATH", &worktree.path.to_string_lossy())
            .replace(
                "$WORKTREE_BRANCH",
                worktree.branch.as_deref().unwrap_or(""),
            )
    }
}
