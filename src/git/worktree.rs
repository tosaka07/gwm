use git2::{BranchType, Repository};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git2(#[from] git2::Error),
    #[allow(dead_code)]
    #[error("Not a git repository")]
    NotARepository,
    #[error("Failed to get repository path")]
    PathError,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Worktree already exists: {0}")]
    WorktreeExists(String),
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_main: bool,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub is_remote: bool,
    pub is_head: bool,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub short_id: String,
    pub message: String,
    #[allow(dead_code)]
    pub author: String,
    pub is_merge: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ChangedFilesSummary {
    pub added: usize,
    pub deleted: usize,
    pub modified: usize,
}

impl ChangedFilesSummary {
    pub fn is_empty(&self) -> bool {
        self.added == 0 && self.deleted == 0 && self.modified == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorktreeDetail {
    pub branch: Option<String>,
    pub path: String,
    pub changed_files: ChangedFilesSummary,
    pub recent_commits: Vec<CommitInfo>,
}

/// Repository information extracted from remote URL
#[derive(Debug, Clone, Default)]
pub struct RepoInfo {
    pub host: String,
    pub owner: String,
    pub repository: String,
}

impl RepoInfo {
    /// Parse repository info from a remote URL
    /// Supports:
    /// - SSH: git@github.com:owner/repo.git
    /// - HTTPS: https://github.com/owner/repo.git
    /// - HTTPS with user: https://user@github.com/owner/repo.git
    pub fn from_url(url: &str) -> Option<Self> {
        let url = url.trim();

        // SSH format: git@github.com:owner/repo.git
        if url.starts_with("git@") {
            let rest = url.strip_prefix("git@")?;
            let (host, path) = rest.split_once(':')?;
            let path = path.trim_end_matches(".git");
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 2 {
                return Some(Self {
                    host: host.to_string(),
                    owner: parts[0].to_string(),
                    repository: parts[1..].join("/"),
                });
            }
        }

        // HTTPS format: https://github.com/owner/repo.git
        if url.starts_with("https://") || url.starts_with("http://") {
            let url = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))?;

            // Remove user@ prefix if present
            let url = if let Some(at_pos) = url.find('@') {
                &url[at_pos + 1..]
            } else {
                url
            };

            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 3 {
                let host = parts[0].to_string();
                let owner = parts[1].to_string();
                let repo = parts[2..].join("/").trim_end_matches(".git").to_string();
                return Some(Self {
                    host,
                    owner,
                    repository: repo,
                });
            }
        }

        None
    }
}

pub struct GitManager {
    repo: Repository,
    repo_root: PathBuf,
}

impl GitManager {
    pub fn new() -> Result<Self, GitError> {
        let current_dir = std::env::current_dir()?;
        let repo = Repository::discover(&current_dir)?;

        // Use commondir() to get main repo root even when inside a worktree
        // commondir() returns the path to .git directory (or .git/worktrees/<name> for worktrees)
        // of the main repository, equivalent to `git rev-parse --git-common-dir`
        let repo_root = repo
            .commondir()
            .parent()
            .ok_or(GitError::PathError)?
            .to_path_buf();

        Ok(Self { repo, repo_root })
    }

    /// Create GitManager from a specific path (for testing)
    #[cfg(test)]
    pub fn from_path(path: &Path) -> Result<Self, GitError> {
        let repo = Repository::discover(path)?;

        // Use commondir() to get main repo root even when inside a worktree
        let repo_root = repo
            .commondir()
            .parent()
            .ok_or(GitError::PathError)?
            .to_path_buf();

        Ok(Self { repo, repo_root })
    }

    #[allow(dead_code)]
    pub fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }

    /// Get repository info from origin remote URL
    pub fn get_repo_info(&self) -> Option<RepoInfo> {
        // Try to get origin remote
        let remote = self.repo.find_remote("origin").ok()?;
        let url = remote.url()?;
        RepoInfo::from_url(url)
    }

    /// Get all worktrees
    pub fn list_worktrees(&self) -> Result<Vec<Worktree>, GitError> {
        let mut worktrees = Vec::new();

        // Add main worktree using repo_root (derived from commondir)
        // This ensures we always get the main repo path even when called from a worktree
        let branch = self.get_main_worktree_branch()?;
        worktrees.push(Worktree {
            name: self
                .repo_root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "main".to_string()),
            path: self.repo_root.clone(),
            branch,
            is_main: true,
        });

        // Get linked worktrees
        let worktree_names = self.repo.worktrees()?;
        for name in worktree_names.iter().flatten() {
            if let Ok(wt) = self.repo.find_worktree(name) {
                let path = wt.path().to_path_buf();
                let branch = self.get_worktree_branch(&path);

                worktrees.push(Worktree {
                    name: name.to_string(),
                    path,
                    branch,
                    is_main: false,
                });
            }
        }

        Ok(worktrees)
    }

    /// Get the current HEAD branch name for the current worktree
    fn get_head_branch(&self) -> Result<Option<String>, GitError> {
        let head = self.repo.head()?;
        if head.is_branch() {
            Ok(head.shorthand().map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Get the branch name for the main worktree
    /// This reads from the main repo's HEAD file directly to ensure we get
    /// the correct branch even when called from within a linked worktree
    fn get_main_worktree_branch(&self) -> Result<Option<String>, GitError> {
        let head_file = self.repo_root.join(".git").join("HEAD");
        if let Ok(content) = std::fs::read_to_string(&head_file) {
            if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
                return Ok(Some(branch.trim().to_string()));
            }
        }
        // Fallback to get_head_branch if we can't read the file
        // (e.g., if we're already in the main repo)
        self.get_head_branch()
    }

    /// Get branch for a worktree path
    fn get_worktree_branch(&self, path: &Path) -> Option<String> {
        let head_path = path.join(".git");

        // For linked worktrees, .git is a file containing "gitdir: ..."
        if head_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&head_path) {
                if let Some(gitdir) = content.strip_prefix("gitdir: ") {
                    let gitdir = PathBuf::from(gitdir.trim());
                    let head_file = gitdir.join("HEAD");
                    if let Ok(head_content) = std::fs::read_to_string(&head_file) {
                        if let Some(branch) = head_content.strip_prefix("ref: refs/heads/") {
                            return Some(branch.trim().to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Get all local branches
    pub fn list_branches(&self) -> Result<Vec<Branch>, GitError> {
        let mut branches = Vec::new();
        let head = self.repo.head().ok();
        let head_name = head.as_ref().and_then(|h| h.shorthand()).map(String::from);

        for branch_result in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                branches.push(Branch {
                    name: name.to_string(),
                    is_remote: false,
                    is_head: head_name.as_deref() == Some(name),
                });
            }
        }

        // Also include remote branches
        for branch_result in self.repo.branches(Some(BranchType::Remote))? {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                // Skip HEAD references
                if name.ends_with("/HEAD") {
                    continue;
                }
                branches.push(Branch {
                    name: name.to_string(),
                    is_remote: true,
                    is_head: false,
                });
            }
        }

        Ok(branches)
    }

    /// Create a new worktree
    pub fn create_worktree(
        &self,
        name: &str,
        branch_name: &str,
        base_path: &str,
    ) -> Result<Worktree, GitError> {
        let worktree_path = self.repo_root.join(base_path).join(name);

        if worktree_path.exists() {
            return Err(GitError::WorktreeExists(name.to_string()));
        }

        // Check if branch exists
        let branch = self.repo.find_branch(branch_name, BranchType::Local);

        let reference = if let Ok(branch) = branch {
            // Use existing local branch
            branch.into_reference()
        } else {
            // Try to find remote branch and create local tracking branch
            let remote_name = format!("origin/{}", branch_name);
            if let Ok(remote_branch) = self.repo.find_branch(&remote_name, BranchType::Remote) {
                let commit = remote_branch.get().peel_to_commit()?;
                let new_branch = self.repo.branch(branch_name, &commit, false)?;
                new_branch.into_reference()
            } else {
                return Err(GitError::BranchNotFound(branch_name.to_string()));
            }
        };

        // Create the worktree
        self.repo.worktree(
            name,
            &worktree_path,
            Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
        )?;

        Ok(Worktree {
            name: name.to_string(),
            path: worktree_path,
            branch: Some(branch_name.to_string()),
            is_main: false,
        })
    }

    /// Delete a worktree
    pub fn delete_worktree(&self, name: &str) -> Result<(), GitError> {
        let wt = self.repo.find_worktree(name)?;
        let path = wt.path().to_path_buf();

        // Prune the worktree from git
        wt.prune(Some(
            git2::WorktreePruneOptions::new()
                .valid(true)
                .working_tree(true),
        ))?;

        // Remove the directory
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }

        Ok(())
    }

    /// Delete a local branch (force delete, equivalent to `git branch -D`)
    pub fn delete_branch(&self, branch_name: &str) -> Result<(), GitError> {
        use std::process::Command;

        // Use git command for force delete (-D)
        let output = Command::new("git")
            .args(["branch", "-D", branch_name])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::Git2(git2::Error::from_str(&stderr)));
        }

        Ok(())
    }

    /// Create a worktree with a new branch (equivalent to `git worktree add -b <branch> <path>`)
    pub fn create_worktree_with_new_branch(
        &self,
        name: &str,
        branch_name: &str,
        base_path: &str,
    ) -> Result<Worktree, GitError> {
        use std::process::Command;

        let worktree_path = self.repo_root.join(base_path).join(name);

        if worktree_path.exists() {
            return Err(GitError::WorktreeExists(name.to_string()));
        }

        // Use git command directly for atomic branch creation + worktree add
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                branch_name,
                worktree_path.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::Git2(git2::Error::from_str(&stderr)));
        }

        Ok(Worktree {
            name: name.to_string(),
            path: worktree_path,
            branch: Some(branch_name.to_string()),
            is_main: false,
        })
    }

    /// Get the default branch name (usually main or master)
    pub fn get_default_branch(&self) -> Result<String, GitError> {
        // Try to find origin/HEAD
        if let Ok(reference) = self.repo.find_reference("refs/remotes/origin/HEAD") {
            if let Some(target) = reference.symbolic_target() {
                if let Some(branch) = target.strip_prefix("refs/remotes/origin/") {
                    return Ok(branch.to_string());
                }
            }
        }

        // Fallback: check for main or master
        if self.repo.find_branch("main", BranchType::Local).is_ok() {
            return Ok("main".to_string());
        }
        if self.repo.find_branch("master", BranchType::Local).is_ok() {
            return Ok("master".to_string());
        }

        // Last resort: return current branch
        self.get_head_branch()?
            .ok_or_else(|| GitError::BranchNotFound("default".to_string()))
    }

    /// Find merged branches (branches that are fully merged into the default branch)
    pub fn find_merged_branches(&self) -> Result<Vec<String>, GitError> {
        let default_branch = self.get_default_branch()?;
        let default_commit = self
            .repo
            .find_branch(&default_branch, BranchType::Local)?
            .get()
            .peel_to_commit()?;

        let mut merged = Vec::new();

        for branch_result in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                // Skip the default branch itself
                if name == default_branch {
                    continue;
                }

                let branch_commit = branch.get().peel_to_commit()?;

                // Check if branch is an ancestor of default (i.e., merged)
                if self
                    .repo
                    .graph_descendant_of(default_commit.id(), branch_commit.id())?
                {
                    merged.push(name.to_string());
                }
            }
        }

        Ok(merged)
    }

    /// Find worktrees with merged branches
    pub fn find_merged_worktrees(&self) -> Result<Vec<Worktree>, GitError> {
        let merged_branches = self.find_merged_branches()?;
        let worktrees = self.list_worktrees()?;

        Ok(worktrees
            .into_iter()
            .filter(|wt| {
                !wt.is_main
                    && wt
                        .branch
                        .as_ref()
                        .map(|b| merged_branches.contains(b))
                        .unwrap_or(false)
            })
            .collect())
    }

    /// Get detailed information for a worktree
    pub fn get_worktree_details(&self, worktree: &Worktree) -> WorktreeDetail {
        let changed_files = self.get_changed_files(&worktree.path);
        let recent_commits = self.get_recent_commits(worktree);

        WorktreeDetail {
            branch: worktree.branch.clone(),
            path: worktree.path.to_string_lossy().to_string(),
            changed_files,
            recent_commits,
        }
    }

    /// Get changed files summary in a worktree
    fn get_changed_files(&self, path: &Path) -> ChangedFilesSummary {
        // Try to open the repository at the worktree path
        let repo = match Repository::open(path) {
            Ok(r) => r,
            Err(_) => return ChangedFilesSummary::default(),
        };

        let mut summary = ChangedFilesSummary::default();

        // Get status
        if let Ok(statuses) = repo.statuses(None) {
            for entry in statuses.iter() {
                let status = entry.status();
                if status.is_wt_new() || status.is_index_new() {
                    summary.added += 1;
                } else if status.is_wt_deleted() || status.is_index_deleted() {
                    summary.deleted += 1;
                } else {
                    summary.modified += 1;
                }
            }
        }

        summary
    }

    /// Get recent commits for a worktree
    fn get_recent_commits(&self, worktree: &Worktree) -> Vec<CommitInfo> {
        // Try to open the repository at the worktree path
        let repo = match Repository::open(&worktree.path) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let mut commits = Vec::new();

        // Get HEAD
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let oid = match head.target() {
            Some(o) => o,
            None => return Vec::new(),
        };

        // Walk commits
        let mut revwalk = match repo.revwalk() {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        if revwalk.push(oid).is_err() {
            return Vec::new();
        }

        for (i, oid_result) in revwalk.enumerate() {
            if i >= 5 {
                break; // Limit to 5 commits
            }

            if let Ok(oid) = oid_result {
                if let Ok(commit) = repo.find_commit(oid) {
                    let short_id = commit
                        .as_object()
                        .short_id()
                        .map(|b| b.as_str().unwrap_or("").to_string())
                        .unwrap_or_else(|_| format!("{:.7}", oid));

                    let message = commit
                        .summary()
                        .unwrap_or("")
                        .chars()
                        .take(50)
                        .collect::<String>();

                    let author = commit.author().name().unwrap_or("").to_string();
                    let is_merge = commit.parent_count() > 1;

                    commits.push(CommitInfo {
                        short_id,
                        message,
                        author,
                        is_merge,
                    });
                }
            }
        }

        commits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    /// Builder for creating test git repositories with various configurations
    pub struct TestRepoBuilder {
        temp_dir: TempDir,
        branches: Vec<String>,
        merged_branches: Vec<String>,
        commits: Vec<String>,
    }

    impl TestRepoBuilder {
        pub fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let repo_path = temp_dir.path();

            // Initialize git repo with explicit main branch
            Command::new("git")
                .args(["init", "-b", "main"])
                .current_dir(repo_path)
                .output()
                .unwrap();

            // Configure git user for commits
            Command::new("git")
                .args(["config", "user.email", "test@test.com"])
                .current_dir(repo_path)
                .output()
                .unwrap();
            Command::new("git")
                .args(["config", "user.name", "Test User"])
                .current_dir(repo_path)
                .output()
                .unwrap();

            // Create initial commit on main branch
            std::fs::write(repo_path.join("README.md"), "# Test").unwrap();
            Command::new("git")
                .args(["add", "."])
                .current_dir(repo_path)
                .output()
                .unwrap();
            Command::new("git")
                .args(["commit", "-m", "Initial commit"])
                .current_dir(repo_path)
                .output()
                .unwrap();

            Self {
                temp_dir,
                branches: Vec::new(),
                merged_branches: Vec::new(),
                commits: Vec::new(),
            }
        }

        /// Add a branch (not merged into main)
        pub fn with_branch(mut self, name: &str) -> Self {
            self.branches.push(name.to_string());
            self
        }

        /// Add a branch that is merged into main
        pub fn with_merged_branch(mut self, name: &str) -> Self {
            self.merged_branches.push(name.to_string());
            self
        }

        /// Add a commit on the current branch
        pub fn with_commit(mut self, message: &str) -> Self {
            self.commits.push(message.to_string());
            self
        }

        /// Build the repository and return TempDir and GitManager
        pub fn build(self) -> (TempDir, GitManager) {
            let repo_path = self.temp_dir.path();

            // Create unmerged branches
            for branch_name in &self.branches {
                // Create and checkout branch
                Command::new("git")
                    .args(["checkout", "-b", branch_name])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();

                // Add a unique commit so branch is ahead
                let file_name = format!("{}.txt", branch_name);
                std::fs::write(repo_path.join(&file_name), branch_name).unwrap();
                Command::new("git")
                    .args(["add", "."])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();
                Command::new("git")
                    .args(["commit", "-m", &format!("Commit on {}", branch_name)])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();

                // Go back to main
                Command::new("git")
                    .args(["checkout", "main"])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();
            }

            // Create merged branches
            for branch_name in &self.merged_branches {
                // Create and checkout branch
                Command::new("git")
                    .args(["checkout", "-b", branch_name])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();

                // Add a commit on the feature branch
                let file_name = format!("{}.txt", branch_name);
                std::fs::write(repo_path.join(&file_name), branch_name).unwrap();
                Command::new("git")
                    .args(["add", "."])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();
                Command::new("git")
                    .args(["commit", "-m", &format!("Feature on {}", branch_name)])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();

                // Go back to main and merge
                Command::new("git")
                    .args(["checkout", "main"])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();

                Command::new("git")
                    .args([
                        "merge",
                        branch_name,
                        "--no-ff",
                        "-m",
                        &format!("Merge {}", branch_name),
                    ])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();
            }

            // Add additional commits if specified
            for message in &self.commits {
                let file_name = format!("{}.txt", message.replace(" ", "_"));
                std::fs::write(repo_path.join(&file_name), message).unwrap();
                Command::new("git")
                    .args(["add", "."])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();
                Command::new("git")
                    .args(["commit", "-m", message])
                    .current_dir(repo_path)
                    .output()
                    .unwrap();
            }

            // Change to repo directory and create GitManager
            std::env::set_current_dir(repo_path).unwrap();
            let git_manager = GitManager::from_path(repo_path).unwrap();

            (self.temp_dir, git_manager)
        }
    }

    /// Helper to create a test git repository (backward compatible)
    fn setup_test_repo() -> (TempDir, GitManager) {
        TestRepoBuilder::new().build()
    }

    #[test]
    fn test_list_worktrees_returns_main() {
        let (_temp_dir, git) = setup_test_repo();

        let worktrees = git.list_worktrees().unwrap();

        assert!(!worktrees.is_empty());
        assert!(worktrees.iter().any(|w| w.is_main));
    }

    #[test]
    fn test_list_branches() {
        let (_temp_dir, git) = setup_test_repo();

        let branches = git.list_branches().unwrap();

        assert!(!branches.is_empty());
        // Should have at least one local branch (main or master)
        assert!(branches.iter().any(|b| !b.is_remote));
    }

    #[test]
    fn test_create_worktree_with_existing_branch() {
        let (temp_dir, git) = setup_test_repo();
        let repo_path = temp_dir.path();

        // Create a new branch first
        Command::new("git")
            .args(["branch", "feature-test"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create worktree with existing branch
        let result = git.create_worktree("test-wt", "feature-test", ".");

        assert!(result.is_ok());
        let worktree = result.unwrap();
        assert_eq!(worktree.name, "test-wt");
        assert_eq!(worktree.branch, Some("feature-test".to_string()));
        assert!(!worktree.is_main);
    }

    #[test]
    fn test_create_worktree_with_new_branch() {
        let (_temp_dir, git) = setup_test_repo();

        // Create worktree with new branch
        let result = git.create_worktree_with_new_branch("new-wt", "new-feature", ".");

        assert!(result.is_ok());
        let worktree = result.unwrap();
        assert_eq!(worktree.name, "new-wt");
        assert_eq!(worktree.branch, Some("new-feature".to_string()));
        assert!(!worktree.is_main);

        // Verify branch was created
        let branches = git.list_branches().unwrap();
        assert!(branches.iter().any(|b| b.name == "new-feature"));
    }

    #[test]
    fn test_create_worktree_with_new_branch_already_exists() {
        let (temp_dir, git) = setup_test_repo();
        let repo_path = temp_dir.path();

        // Create a branch first
        Command::new("git")
            .args(["branch", "existing-branch"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Try to create worktree with same branch name - should fail
        let result = git.create_worktree_with_new_branch("wt", "existing-branch", ".");

        assert!(result.is_err());
    }

    #[test]
    fn test_delete_worktree() {
        let (temp_dir, git) = setup_test_repo();
        let repo_path = temp_dir.path();

        // Create a branch and worktree
        Command::new("git")
            .args(["branch", "to-delete"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        git.create_worktree("delete-wt", "to-delete", ".").unwrap();

        // Delete the worktree
        let result = git.delete_worktree("delete-wt");

        assert!(result.is_ok());

        // Verify worktree is gone
        let worktrees = git.list_worktrees().unwrap();
        assert!(!worktrees.iter().any(|w| w.name == "delete-wt"));
    }

    #[test]
    fn test_delete_branch() {
        let (temp_dir, git) = setup_test_repo();
        let repo_path = temp_dir.path();

        // Create a branch
        Command::new("git")
            .args(["branch", "branch-to-delete"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Delete the branch
        let result = git.delete_branch("branch-to-delete");

        assert!(result.is_ok());

        // Verify branch is gone
        let branches = git.list_branches().unwrap();
        assert!(!branches.iter().any(|b| b.name == "branch-to-delete"));
    }

    #[test]
    fn test_delete_branch_not_found() {
        let (_temp_dir, git) = setup_test_repo();

        let result = git.delete_branch("nonexistent-branch");

        assert!(result.is_err());
    }

    #[test]
    fn test_changed_files_summary_is_empty() {
        let summary = ChangedFilesSummary::default();
        assert!(summary.is_empty());

        let summary_with_added = ChangedFilesSummary {
            added: 1,
            deleted: 0,
            modified: 0,
        };
        assert!(!summary_with_added.is_empty());
    }

    #[test]
    fn test_get_default_branch() {
        let (_temp_dir, git) = setup_test_repo();

        let result = git.get_default_branch();

        assert!(result.is_ok());
        // Should be "main" or "master"
        let branch = result.unwrap();
        assert!(branch == "main" || branch == "master");
    }

    // ========== RepoInfo Tests ==========

    #[test]
    fn test_repo_info_from_ssh_url() {
        let url = "git@github.com:owner/repo.git";
        let info = RepoInfo::from_url(url).unwrap();

        assert_eq!(info.host, "github.com");
        assert_eq!(info.owner, "owner");
        assert_eq!(info.repository, "repo");
    }

    #[test]
    fn test_repo_info_from_https_url() {
        let url = "https://github.com/owner/repo.git";
        let info = RepoInfo::from_url(url).unwrap();

        assert_eq!(info.host, "github.com");
        assert_eq!(info.owner, "owner");
        assert_eq!(info.repository, "repo");
    }

    #[test]
    fn test_repo_info_from_https_url_no_git_suffix() {
        let url = "https://github.com/owner/repo";
        let info = RepoInfo::from_url(url).unwrap();

        assert_eq!(info.host, "github.com");
        assert_eq!(info.owner, "owner");
        assert_eq!(info.repository, "repo");
    }

    #[test]
    fn test_repo_info_from_https_with_user() {
        let url = "https://user@github.com/owner/repo.git";
        let info = RepoInfo::from_url(url).unwrap();

        assert_eq!(info.host, "github.com");
        assert_eq!(info.owner, "owner");
        assert_eq!(info.repository, "repo");
    }

    #[test]
    fn test_repo_info_from_gitlab_url() {
        let url = "git@gitlab.com:group/subgroup/project.git";
        let info = RepoInfo::from_url(url).unwrap();

        assert_eq!(info.host, "gitlab.com");
        assert_eq!(info.owner, "group");
        assert_eq!(info.repository, "subgroup/project");
    }

    #[test]
    fn test_repo_info_invalid_url() {
        let url = "not-a-valid-url";
        let info = RepoInfo::from_url(url);

        assert!(info.is_none());
    }

    // ========== find_merged_branches Tests ==========

    #[test]
    fn test_find_merged_branches_returns_merged_branch() {
        let (_temp_dir, git) = TestRepoBuilder::new()
            .with_merged_branch("feature-merged")
            .build();

        let merged = git.find_merged_branches().unwrap();

        assert!(merged.contains(&"feature-merged".to_string()));
    }

    #[test]
    fn test_find_merged_branches_excludes_unmerged_branch() {
        let (_temp_dir, git) = TestRepoBuilder::new()
            .with_branch("feature-unmerged")
            .build();

        let merged = git.find_merged_branches().unwrap();

        assert!(!merged.contains(&"feature-unmerged".to_string()));
    }

    #[test]
    fn test_find_merged_branches_excludes_default_branch() {
        let (_temp_dir, git) = TestRepoBuilder::new()
            .with_merged_branch("feature-merged")
            .build();

        let merged = git.find_merged_branches().unwrap();
        let default_branch = git.get_default_branch().unwrap();

        // Default branch should not be in merged list
        assert!(!merged.contains(&default_branch));
    }

    #[test]
    fn test_find_merged_branches_with_no_other_branches() {
        let (_temp_dir, git) = TestRepoBuilder::new().build();

        let merged = git.find_merged_branches().unwrap();

        assert!(merged.is_empty());
    }

    #[test]
    fn test_find_merged_branches_mixed_branches() {
        let (_temp_dir, git) = TestRepoBuilder::new()
            .with_merged_branch("merged-1")
            .with_merged_branch("merged-2")
            .with_branch("unmerged-1")
            .build();

        let merged = git.find_merged_branches().unwrap();

        assert!(merged.contains(&"merged-1".to_string()));
        assert!(merged.contains(&"merged-2".to_string()));
        assert!(!merged.contains(&"unmerged-1".to_string()));
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_find_merged_worktrees_returns_worktrees_with_merged_branch() {
        let (temp_dir, git) = TestRepoBuilder::new()
            .with_merged_branch("merged-feature")
            .build();

        // Create a worktree with the merged branch
        let repo_path = temp_dir.path();
        Command::new("git")
            .args(["worktree", "add", "wt-merged", "merged-feature"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let merged_worktrees = git.find_merged_worktrees().unwrap();

        assert!(!merged_worktrees.is_empty());
        assert!(merged_worktrees.iter().any(|wt| wt.name == "wt-merged"));
    }

    #[test]
    fn test_find_merged_worktrees_excludes_unmerged() {
        let (temp_dir, git) = TestRepoBuilder::new()
            .with_branch("unmerged-feature")
            .build();

        // Create a worktree with an unmerged branch
        let repo_path = temp_dir.path();
        Command::new("git")
            .args(["worktree", "add", "wt-unmerged", "unmerged-feature"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let merged_worktrees = git.find_merged_worktrees().unwrap();

        assert!(merged_worktrees.is_empty());
    }

    #[test]
    fn test_find_merged_worktrees_excludes_main() {
        let (_temp_dir, git) = TestRepoBuilder::new()
            .with_merged_branch("merged-feature")
            .build();

        let merged_worktrees = git.find_merged_worktrees().unwrap();

        // Main worktree should never be in merged worktrees
        assert!(merged_worktrees.iter().all(|wt| !wt.is_main));
    }

    // ========== Error Cases Tests ==========

    #[test]
    fn test_delete_worktree_not_found() {
        let (_temp_dir, git) = setup_test_repo();

        let result = git.delete_worktree("nonexistent-worktree");

        assert!(result.is_err());
    }

    #[test]
    fn test_create_worktree_branch_not_found() {
        let (_temp_dir, git) = setup_test_repo();

        let result = git.create_worktree("test-wt", "nonexistent-branch", ".");

        assert!(result.is_err());
        match result {
            Err(GitError::BranchNotFound(name)) => {
                assert_eq!(name, "nonexistent-branch");
            }
            _ => panic!("Expected BranchNotFound error"),
        }
    }

    #[test]
    fn test_create_worktree_already_exists() {
        let (temp_dir, git) = setup_test_repo();
        let repo_path = temp_dir.path();

        // Create a branch
        Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create directory manually to simulate existing worktree
        std::fs::create_dir(repo_path.join("existing-wt")).unwrap();

        let result = git.create_worktree("existing-wt", "test-branch", ".");

        assert!(result.is_err());
        match result {
            Err(GitError::WorktreeExists(name)) => {
                assert_eq!(name, "existing-wt");
            }
            _ => panic!("Expected WorktreeExists error"),
        }
    }

    // ========== Worktree-relative Tests (commondir behavior) ==========

    /// Helper to canonicalize path for comparison (handles macOS /var -> /private/var symlink)
    fn canonicalize_path(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    #[test]
    fn test_git_manager_from_worktree_returns_main_repo_root() {
        let (temp_dir, _git) = setup_test_repo();
        let main_repo_path = canonicalize_path(temp_dir.path());

        // Create a worktree
        Command::new("git")
            .args(["worktree", "add", "-b", "feature-wt", "worktree-dir"])
            .current_dir(&main_repo_path)
            .output()
            .unwrap();

        let worktree_path = main_repo_path.join("worktree-dir");

        // Create GitManager from inside the worktree
        let git_from_worktree = GitManager::from_path(&worktree_path).unwrap();

        // repo_root should point to main repo, not the worktree
        assert_eq!(
            canonicalize_path(&git_from_worktree.repo_root),
            main_repo_path
        );
    }

    #[test]
    fn test_list_worktrees_from_worktree_returns_main_repo_list() {
        let (temp_dir, git) = setup_test_repo();
        let main_repo_path = canonicalize_path(temp_dir.path());

        // Create a worktree
        Command::new("git")
            .args(["worktree", "add", "-b", "feature-list", "worktree-list"])
            .current_dir(&main_repo_path)
            .output()
            .unwrap();

        let worktree_path = main_repo_path.join("worktree-list");

        // Get worktrees from main repo
        let worktrees_from_main = git.list_worktrees().unwrap();

        // Create GitManager from inside the worktree and list worktrees
        let git_from_worktree = GitManager::from_path(&worktree_path).unwrap();
        let worktrees_from_worktree = git_from_worktree.list_worktrees().unwrap();

        // Both should return the same worktrees
        assert_eq!(worktrees_from_main.len(), worktrees_from_worktree.len());

        // Main worktree path should be the main repo, not the worktree
        let main_wt = worktrees_from_worktree.iter().find(|w| w.is_main).unwrap();
        assert_eq!(canonicalize_path(&main_wt.path), main_repo_path);
    }

    #[test]
    fn test_create_worktree_from_worktree_uses_main_repo_basedir() {
        let (temp_dir, _git) = setup_test_repo();
        let main_repo_path = canonicalize_path(temp_dir.path());

        // Create an initial worktree
        Command::new("git")
            .args(["worktree", "add", "-b", "initial-wt", "initial-worktree"])
            .current_dir(&main_repo_path)
            .output()
            .unwrap();

        // Create a branch for the new worktree
        Command::new("git")
            .args(["branch", "second-feature"])
            .current_dir(&main_repo_path)
            .output()
            .unwrap();

        let worktree_path = main_repo_path.join("initial-worktree");

        // Create GitManager from inside the worktree
        let git_from_worktree = GitManager::from_path(&worktree_path).unwrap();

        // Create a new worktree from inside the first worktree
        let result = git_from_worktree.create_worktree("second-wt", "second-feature", ".");

        assert!(result.is_ok());
        let new_worktree = result.unwrap();

        // The new worktree should be created relative to main repo, not the worktree
        // Use canonicalize to handle symlinks and normalize paths
        let expected_path = main_repo_path.join("second-wt");
        assert_eq!(
            canonicalize_path(&new_worktree.path),
            canonicalize_path(&expected_path)
        );
        assert!(new_worktree.path.exists());
    }

    #[test]
    fn test_main_worktree_branch_from_worktree() {
        let (temp_dir, _git) = setup_test_repo();
        let main_repo_path = canonicalize_path(temp_dir.path());

        // Create a worktree with a different branch
        Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                "feature-branch",
                "feature-worktree",
            ])
            .current_dir(&main_repo_path)
            .output()
            .unwrap();

        let worktree_path = main_repo_path.join("feature-worktree");

        // Create GitManager from inside the worktree
        let git_from_worktree = GitManager::from_path(&worktree_path).unwrap();

        // List worktrees and check main worktree's branch
        let worktrees = git_from_worktree.list_worktrees().unwrap();
        let main_wt = worktrees.iter().find(|w| w.is_main).unwrap();

        // Main worktree should show "main" branch, not "feature-branch"
        assert_eq!(main_wt.branch, Some("main".to_string()));
    }
}
