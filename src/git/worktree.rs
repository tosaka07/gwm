use std::path::{Path, PathBuf};

use git2::{BranchType, Repository};

use crate::error::{Error, Result};

/// Represents a Git worktree
#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub commit_hash: String,
    pub commit_message: String,
    pub commit_author: String,
    pub commit_date: String,
    pub is_main: bool,
}

/// Manages Git worktrees using git2
pub struct WorktreeManager {
    repo: Repository,
    repo_root: PathBuf,
}

impl WorktreeManager {
    /// Create a new WorktreeManager by finding the repository
    pub fn new() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let repo = Repository::discover(&current_dir)?;
        let repo_root = repo
            .workdir()
            .or_else(|| repo.path().parent())
            .ok_or_else(|| Error::NotInRepository)?
            .to_path_buf();

        Ok(Self { repo, repo_root })
    }

    /// Get the repository root path
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// List all worktrees
    pub fn list(&self) -> Result<Vec<Worktree>> {
        let mut worktrees = Vec::new();

        // Add main worktree first
        if let Some(main_wt) = self.get_main_worktree()? {
            worktrees.push(main_wt);
        }

        // Get linked worktrees
        let worktree_names = self.repo.worktrees()?;
        for name in worktree_names.iter() {
            if let Some(name) = name {
                if let Ok(wt) = self.get_worktree_info(name) {
                    worktrees.push(wt);
                }
            }
        }

        Ok(worktrees)
    }

    /// Get main worktree info
    fn get_main_worktree(&self) -> Result<Option<Worktree>> {
        let workdir = match self.repo.workdir() {
            Some(path) => path.to_path_buf(),
            None => return Ok(None), // Bare repository
        };

        let (branch, commit_hash, commit_message, commit_author, commit_date) =
            self.get_head_info()?;

        Ok(Some(Worktree {
            path: workdir,
            branch,
            commit_hash,
            commit_message,
            commit_author,
            commit_date,
            is_main: true,
        }))
    }

    /// Get worktree info by name
    fn get_worktree_info(&self, name: &str) -> Result<Worktree> {
        let wt = self.repo.find_worktree(name)?;
        let wt_path = wt.path().to_path_buf();

        // Open the worktree as a repository to get HEAD info
        let wt_repo = Repository::open(&wt_path)?;
        let (branch, commit_hash, commit_message, commit_author, commit_date) =
            Self::get_head_info_from_repo(&wt_repo)?;

        Ok(Worktree {
            path: wt_path,
            branch,
            commit_hash,
            commit_message,
            commit_author,
            commit_date,
            is_main: false,
        })
    }

    /// Get HEAD info from the main repository
    fn get_head_info(&self) -> Result<(Option<String>, String, String, String, String)> {
        Self::get_head_info_from_repo(&self.repo)
    }

    /// Get HEAD info from any repository
    fn get_head_info_from_repo(
        repo: &Repository,
    ) -> Result<(Option<String>, String, String, String, String)> {
        let head = repo.head()?;

        let branch = if head.is_branch() {
            head.shorthand().map(|s| s.to_string())
        } else {
            None
        };

        let commit = head.peel_to_commit()?;
        let commit_hash = commit.id().to_string();
        let commit_message = commit.summary().unwrap_or("").to_string();
        let commit_author = commit.author().name().unwrap_or("").to_string();

        let time = commit.time();
        let offset = chrono::FixedOffset::east_opt(time.offset_minutes() * 60)
            .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
        let datetime = chrono::DateTime::from_timestamp(time.seconds(), 0)
            .map(|dt| dt.with_timezone(&offset))
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        Ok((branch, commit_hash, commit_message, commit_author, datetime))
    }

    /// Create a new worktree
    pub fn create(
        &self,
        path: &Path,
        branch_name: &str,
        base_branch: Option<&str>,
    ) -> Result<Worktree> {
        // Get base commit
        let base_commit = if let Some(base) = base_branch {
            let branch = self.repo.find_branch(base, BranchType::Local)?;
            branch.get().peel_to_commit()?
        } else {
            self.repo.head()?.peel_to_commit()?
        };

        // Create branch if it doesn't exist
        let branch = match self.repo.find_branch(branch_name, BranchType::Local) {
            Ok(b) => b,
            Err(_) => self.repo.branch(branch_name, &base_commit, false)?,
        };

        // Create worktree
        let reference = branch.into_reference();
        self.repo.worktree(
            branch_name,
            path,
            Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
        )?;

        // Get worktree info
        self.get_worktree_info(branch_name)
    }

    /// Remove a worktree
    pub fn remove(&self, path: &Path, force: bool) -> Result<()> {
        // Find the worktree by path
        let worktree_names = self.repo.worktrees()?;
        let mut found_name = None;

        for name in worktree_names.iter() {
            if let Some(name) = name {
                if let Ok(wt) = self.repo.find_worktree(name) {
                    if wt.path() == path {
                        found_name = Some(name.to_string());
                        break;
                    }
                }
            }
        }

        let name = found_name.ok_or_else(|| Error::WorktreeNotFound(path.display().to_string()))?;

        // Prune the worktree
        let wt = self.repo.find_worktree(&name)?;

        if force {
            // Force remove - delete directory and prune
            if path.exists() {
                std::fs::remove_dir_all(path)?;
            }
            wt.prune(Some(
                git2::WorktreePruneOptions::new()
                    .valid(true)
                    .working_tree(true),
            ))?;
        } else {
            wt.prune(None)?;
        }

        Ok(())
    }

    /// List all branches
    pub fn list_branches(&self) -> Result<Vec<String>> {
        let mut branches = Vec::new();

        for branch in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                branches.push(name.to_string());
            }
        }

        Ok(branches)
    }

    /// Find merged worktrees (branches merged into base_branch)
    pub fn find_merged_worktrees(&self, base_branch: &str) -> Result<Vec<Worktree>> {
        let base = self.repo.find_branch(base_branch, BranchType::Local)?;
        let base_commit = base.get().peel_to_commit()?;

        let all_worktrees = self.list()?;
        let mut merged = Vec::new();

        for wt in all_worktrees {
            if wt.is_main {
                continue; // Skip main worktree
            }

            if let Some(branch_name) = &wt.branch {
                if let Ok(branch) = self.repo.find_branch(branch_name, BranchType::Local) {
                    if let Ok(branch_commit) = branch.get().peel_to_commit() {
                        // Check if branch is merged into base
                        if let Ok(merge_base) =
                            self.repo.merge_base(base_commit.id(), branch_commit.id())
                        {
                            if merge_base == branch_commit.id() {
                                merged.push(wt);
                            }
                        }
                    }
                }
            }
        }

        Ok(merged)
    }

    /// Delete all merged worktrees
    pub fn delete_merged_worktrees(&self, base_branch: &str) -> Result<Vec<String>> {
        let merged = self.find_merged_worktrees(base_branch)?;
        let mut deleted = Vec::new();

        for wt in merged {
            if let Err(e) = self.remove(&wt.path, true) {
                eprintln!("Failed to delete {}: {}", wt.path.display(), e);
            } else {
                deleted.push(wt.path.display().to_string());
            }
        }

        Ok(deleted)
    }
}
