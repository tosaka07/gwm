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

pub struct GitManager {
    repo: Repository,
    repo_root: PathBuf,
}

impl GitManager {
    pub fn new() -> Result<Self, GitError> {
        let current_dir = std::env::current_dir()?;
        let repo = Repository::discover(&current_dir)?;

        let repo_root = repo
            .workdir()
            .or_else(|| repo.path().parent())
            .ok_or(GitError::PathError)?
            .to_path_buf();

        Ok(Self { repo, repo_root })
    }

    #[allow(dead_code)]
    pub fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }

    /// Get all worktrees
    pub fn list_worktrees(&self) -> Result<Vec<Worktree>, GitError> {
        let mut worktrees = Vec::new();

        // Add main worktree
        if let Some(workdir) = self.repo.workdir() {
            let branch = self.get_head_branch()?;
            worktrees.push(Worktree {
                name: workdir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "main".to_string()),
                path: workdir.to_path_buf(),
                branch,
                is_main: true,
            });
        }

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

    /// Get the current HEAD branch name
    fn get_head_branch(&self) -> Result<Option<String>, GitError> {
        let head = self.repo.head()?;
        if head.is_branch() {
            Ok(head.shorthand().map(|s| s.to_string()))
        } else {
            Ok(None)
        }
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
        let worktree_path = self.repo_root.parent().unwrap_or(&self.repo_root).join(base_path).join(name);

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

        let worktree_path = self
            .repo_root
            .parent()
            .unwrap_or(&self.repo_root)
            .join(base_path)
            .join(name);

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
