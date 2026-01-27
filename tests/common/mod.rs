//! Common test utilities for integration tests

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// A helper struct for creating test git repositories
pub struct GitTestRepo {
    pub temp_dir: TempDir,
    pub path: PathBuf,
}

impl GitTestRepo {
    /// Create a new git repository for testing
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_path_buf();

        // Initialize git repo with explicit main branch
        run_git(&path, &["init", "-b", "main"]);

        // Configure git user (required for commits)
        run_git(&path, &["config", "user.email", "test@example.com"]);
        run_git(&path, &["config", "user.name", "Test User"]);

        // Create initial commit
        std::fs::write(path.join("README.md"), "# Test Repository").unwrap();
        run_git(&path, &["add", "."]);
        run_git(&path, &["commit", "-m", "Initial commit"]);

        Self { temp_dir, path }
    }

    /// Create a new branch at the current HEAD
    pub fn create_branch(&self, name: &str) {
        run_git(&self.path, &["branch", name]);
    }

    /// Create a branch with a commit (makes it diverge from main)
    pub fn create_branch_with_commit(&self, name: &str) {
        run_git(&self.path, &["checkout", "-b", name]);

        // Create a unique file and commit
        let file_name = format!("{}.txt", name);
        std::fs::write(self.path.join(&file_name), name).unwrap();
        run_git(&self.path, &["add", "."]);
        run_git(
            &self.path,
            &["commit", "-m", &format!("Commit on {}", name)],
        );

        // Return to main
        self.checkout_main();
    }

    /// Create a branch, commit on it, and merge it back to main
    pub fn create_merged_branch(&self, name: &str) {
        self.create_branch_with_commit(name);

        // Merge back to main
        run_git(
            &self.path,
            &["merge", name, "--no-ff", "-m", &format!("Merge {}", name)],
        );
    }

    /// Checkout main branch
    pub fn checkout_main(&self) {
        run_git(&self.path, &["checkout", "main"]);
    }

    /// Checkout a specific branch
    pub fn checkout(&self, branch: &str) {
        run_git(&self.path, &["checkout", branch]);
    }

    /// Create a worktree
    pub fn create_worktree(&self, name: &str, branch: &str) {
        let worktree_path = self.path.join(name);
        run_git(
            &self.path,
            &["worktree", "add", worktree_path.to_str().unwrap(), branch],
        );
    }

    /// Create a worktree with a new branch
    pub fn create_worktree_with_new_branch(&self, name: &str, branch: &str) {
        let worktree_path = self.path.join(name);
        run_git(
            &self.path,
            &[
                "worktree",
                "add",
                "-b",
                branch,
                worktree_path.to_str().unwrap(),
            ],
        );
    }

    /// Remove a worktree
    pub fn remove_worktree(&self, name: &str) {
        run_git(&self.path, &["worktree", "remove", name, "--force"]);
    }

    /// List worktrees
    pub fn list_worktrees(&self) -> String {
        let output = Command::new("git")
            .args(["worktree", "list"])
            .current_dir(&self.path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// Get list of branches
    pub fn list_branches(&self) -> Vec<String> {
        let output = Command::new("git")
            .args(["branch", "--format=%(refname:short)"])
            .current_dir(&self.path)
            .output()
            .unwrap();

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect()
    }

    /// Add a commit on the current branch
    pub fn add_commit(&self, message: &str) {
        let file_name = format!("{}.txt", message.replace(" ", "_"));
        std::fs::write(self.path.join(&file_name), message).unwrap();
        run_git(&self.path, &["add", "."]);
        run_git(&self.path, &["commit", "-m", message]);
    }
}

/// Run a git command in the specified directory
fn run_git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("Failed to execute git command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Git command failed: git {} - {}", args.join(" "), stderr);
    }
}
