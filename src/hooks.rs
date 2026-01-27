use crate::config::RepositorySettings;
use crate::git::Worktree;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HookError {
    #[error("Setup command failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("File copy failed: {0}")]
    CopyFailed(String),
}

pub struct SetupRunner {
    settings: Option<RepositorySettings>,
    main_worktree_path: Option<PathBuf>,
}

impl SetupRunner {
    pub fn new(settings: Option<RepositorySettings>) -> Self {
        Self {
            settings,
            main_worktree_path: None,
        }
    }

    /// Set the main worktree path for file copying
    pub fn with_main_worktree(mut self, path: PathBuf) -> Self {
        self.main_worktree_path = Some(path);
        self
    }

    /// Run setup tasks after creating a worktree (copy files, then run commands)
    pub fn run_setup(&self, worktree: &Worktree) -> Result<(), HookError> {
        let Some(settings) = &self.settings else {
            return Ok(());
        };

        // Copy files first
        if let Some(files) = &settings.copy_files {
            self.copy_files(files, worktree)?;
        }

        // Then run setup commands
        if let Some(commands) = &settings.setup_commands {
            for cmd in commands {
                self.run_command(cmd, worktree)?;
            }
        }

        Ok(())
    }

    /// Copy files from main worktree to new worktree
    fn copy_files(&self, files: &[String], worktree: &Worktree) -> Result<(), HookError> {
        let Some(main_path) = &self.main_worktree_path else {
            return Err(HookError::CopyFailed(
                "Main worktree path not set".to_string(),
            ));
        };

        for file_pattern in files {
            self.copy_file_or_pattern(main_path, file_pattern, &worktree.path)?;
        }

        Ok(())
    }

    /// Copy a single file or pattern from source to destination
    fn copy_file_or_pattern(
        &self,
        source_base: &Path,
        pattern: &str,
        dest_base: &Path,
    ) -> Result<(), HookError> {
        let source_path = source_base.join(pattern);
        let dest_path = dest_base.join(pattern);

        if source_path.exists() {
            // Create parent directories if needed
            if let Some(parent) = dest_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            // Copy the file
            std::fs::copy(&source_path, &dest_path).map_err(|e| {
                HookError::CopyFailed(format!(
                    "Failed to copy '{}' to '{}': {}",
                    source_path.display(),
                    dest_path.display(),
                    e
                ))
            })?;
        }
        // Silently skip if source doesn't exist (file is optional)

        Ok(())
    }

    /// Execute a command in the worktree directory
    fn run_command(&self, cmd: &str, worktree: &Worktree) -> Result<(), HookError> {
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

    /// Expand variables in command
    fn expand_variables(&self, cmd: &str, worktree: &Worktree) -> String {
        cmd.replace("$WORKTREE_NAME", &worktree.name)
            .replace("$WORKTREE_PATH", &worktree.path.to_string_lossy())
            .replace("$WORKTREE_BRANCH", worktree.branch.as_deref().unwrap_or(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_worktree() -> Worktree {
        Worktree {
            name: "feature-test".to_string(),
            path: PathBuf::from("/repo/worktrees/feature-test"),
            branch: Some("feature/test".to_string()),
            is_main: false,
        }
    }

    #[test]
    fn test_expand_worktree_name() {
        let runner = SetupRunner::new(None);
        let worktree = create_test_worktree();

        let expanded = runner.expand_variables("echo $WORKTREE_NAME", &worktree);

        assert_eq!(expanded, "echo feature-test");
    }

    #[test]
    fn test_expand_worktree_path() {
        let runner = SetupRunner::new(None);
        let worktree = create_test_worktree();

        let expanded = runner.expand_variables("cd $WORKTREE_PATH", &worktree);

        assert_eq!(expanded, "cd /repo/worktrees/feature-test");
    }

    #[test]
    fn test_expand_worktree_branch() {
        let runner = SetupRunner::new(None);
        let worktree = create_test_worktree();

        let expanded = runner.expand_variables("git checkout $WORKTREE_BRANCH", &worktree);

        assert_eq!(expanded, "git checkout feature/test");
    }

    #[test]
    fn test_expand_worktree_branch_when_none() {
        let runner = SetupRunner::new(None);
        let worktree = Worktree {
            name: "detached".to_string(),
            path: PathBuf::from("/repo/worktrees/detached"),
            branch: None,
            is_main: false,
        };

        let expanded = runner.expand_variables("branch is $WORKTREE_BRANCH end", &worktree);

        assert_eq!(expanded, "branch is  end");
    }

    #[test]
    fn test_no_settings() {
        let runner = SetupRunner::new(None);
        let worktree = create_test_worktree();

        let result = runner.run_setup(&worktree);
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_setup_commands() {
        let runner = SetupRunner::new(Some(RepositorySettings {
            repository: "test".to_string(),
            copy_files: None,
            setup_commands: None,
        }));
        let worktree = create_test_worktree();

        let result = runner.run_setup(&worktree);
        assert!(result.is_ok());
    }

    // ========== Copy Files Tests ==========

    #[test]
    fn test_with_main_worktree_builder() {
        let runner = SetupRunner::new(None).with_main_worktree(PathBuf::from("/repo/main"));

        assert!(runner.main_worktree_path.is_some());
        assert_eq!(
            runner.main_worktree_path.unwrap(),
            PathBuf::from("/repo/main")
        );
    }

    #[test]
    fn test_copy_files_without_main_worktree_path() {
        let runner = SetupRunner::new(Some(RepositorySettings {
            repository: "test".to_string(),
            copy_files: Some(vec![".env".to_string()]),
            setup_commands: None,
        }));
        let worktree = create_test_worktree();

        let result = runner.run_setup(&worktree);

        // Should fail because main_worktree_path is not set
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HookError::CopyFailed(_)));
    }

    #[test]
    fn test_copy_file_success() {
        use std::fs;

        // Create temporary directories
        let temp_dir = std::env::temp_dir().join("gwm_test_copy");
        let main_dir = temp_dir.join("main");
        let worktree_dir = temp_dir.join("worktree");

        // Clean up and create directories
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&worktree_dir).unwrap();

        // Create source file
        let source_file = main_dir.join(".env");
        fs::write(&source_file, "TEST=value").unwrap();

        let runner = SetupRunner::new(Some(RepositorySettings {
            repository: "test".to_string(),
            copy_files: Some(vec![".env".to_string()]),
            setup_commands: None,
        }))
        .with_main_worktree(main_dir.clone());

        let worktree = Worktree {
            name: "test-worktree".to_string(),
            path: worktree_dir.clone(),
            branch: Some("test".to_string()),
            is_main: false,
        };

        let result = runner.run_setup(&worktree);
        assert!(result.is_ok());

        // Verify file was copied
        let dest_file = worktree_dir.join(".env");
        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(&dest_file).unwrap(), "TEST=value");

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_file_source_not_exists() {
        use std::fs;

        // Create temporary directories
        let temp_dir = std::env::temp_dir().join("gwm_test_copy_missing");
        let main_dir = temp_dir.join("main");
        let worktree_dir = temp_dir.join("worktree");

        // Clean up and create directories
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&worktree_dir).unwrap();

        // Don't create source file - it doesn't exist

        let runner = SetupRunner::new(Some(RepositorySettings {
            repository: "test".to_string(),
            copy_files: Some(vec!["nonexistent.env".to_string()]),
            setup_commands: None,
        }))
        .with_main_worktree(main_dir.clone());

        let worktree = Worktree {
            name: "test-worktree".to_string(),
            path: worktree_dir.clone(),
            branch: Some("test".to_string()),
            is_main: false,
        };

        let result = runner.run_setup(&worktree);

        // Should succeed (file is optional)
        assert!(result.is_ok());

        // Destination file should not exist
        let dest_file = worktree_dir.join("nonexistent.env");
        assert!(!dest_file.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_file_nested_directory() {
        use std::fs;

        // Create temporary directories
        let temp_dir = std::env::temp_dir().join("gwm_test_copy_nested");
        let main_dir = temp_dir.join("main");
        let worktree_dir = temp_dir.join("worktree");

        // Clean up and create directories
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&main_dir.join("config")).unwrap();
        fs::create_dir_all(&worktree_dir).unwrap();

        // Create source file in nested directory
        let source_file = main_dir.join("config").join("settings.json");
        fs::write(&source_file, r#"{"key": "value"}"#).unwrap();

        let runner = SetupRunner::new(Some(RepositorySettings {
            repository: "test".to_string(),
            copy_files: Some(vec!["config/settings.json".to_string()]),
            setup_commands: None,
        }))
        .with_main_worktree(main_dir.clone());

        let worktree = Worktree {
            name: "test-worktree".to_string(),
            path: worktree_dir.clone(),
            branch: Some("test".to_string()),
            is_main: false,
        };

        let result = runner.run_setup(&worktree);
        assert!(result.is_ok());

        // Verify file was copied (and directory was created)
        let dest_file = worktree_dir.join("config").join("settings.json");
        assert!(dest_file.exists());
        assert_eq!(
            fs::read_to_string(&dest_file).unwrap(),
            r#"{"key": "value"}"#
        );

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_multiple_files() {
        use std::fs;

        // Create temporary directories
        let temp_dir = std::env::temp_dir().join("gwm_test_copy_multi");
        let main_dir = temp_dir.join("main");
        let worktree_dir = temp_dir.join("worktree");

        // Clean up and create directories
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&worktree_dir).unwrap();

        // Create source files
        fs::write(main_dir.join(".env"), "ENV=test").unwrap();
        fs::write(main_dir.join(".env.local"), "LOCAL=value").unwrap();

        let runner = SetupRunner::new(Some(RepositorySettings {
            repository: "test".to_string(),
            copy_files: Some(vec![".env".to_string(), ".env.local".to_string()]),
            setup_commands: None,
        }))
        .with_main_worktree(main_dir.clone());

        let worktree = Worktree {
            name: "test-worktree".to_string(),
            path: worktree_dir.clone(),
            branch: Some("test".to_string()),
            is_main: false,
        };

        let result = runner.run_setup(&worktree);
        assert!(result.is_ok());

        // Verify both files were copied
        assert!(worktree_dir.join(".env").exists());
        assert!(worktree_dir.join(".env.local").exists());
        assert_eq!(
            fs::read_to_string(worktree_dir.join(".env")).unwrap(),
            "ENV=test"
        );
        assert_eq!(
            fs::read_to_string(worktree_dir.join(".env.local")).unwrap(),
            "LOCAL=value"
        );

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
