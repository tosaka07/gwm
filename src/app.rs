use crate::config::Config;
use crate::git::{Branch, GitManager, Worktree, WorktreeDetail};
use crate::hooks::HookRunner;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Git error: {0}")]
    Git(#[from] crate::git::GitError),
    #[error("Hook error: {0}")]
    Hook(#[from] crate::hooks::HookError),
    #[error("Config error: {0}")]
    Config(#[from] crate::config::ConfigError),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Create,
    Confirm,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfirmAction {
    DeleteSingle,
    Prune,
}

pub struct App {
    pub mode: AppMode,
    pub worktrees: Vec<Worktree>,
    pub filtered_worktrees: Vec<Worktree>,
    pub branches: Vec<Branch>,
    pub filtered_branches: Vec<Branch>,
    pub selected_worktree: usize,
    pub selected_branch: usize,
    pub input: String,
    pub confirm_action: Option<ConfirmAction>,
    pub merged_worktrees: Vec<Worktree>,
    pub message: Option<String>,
    pub should_quit: bool,
    pub selected_worktree_path: Option<String>,
    config: Config,
    git: GitManager,
    hook_runner: HookRunner,
}

impl App {
    pub fn new(config: Config, git: GitManager) -> Result<Self, AppError> {
        let hook_runner = HookRunner::new(config.hooks.clone());
        let worktrees = git.list_worktrees()?;
        let branches = git.list_branches()?;

        Ok(Self {
            mode: AppMode::Normal,
            worktrees: worktrees.clone(),
            filtered_worktrees: worktrees,
            branches: branches.clone(),
            filtered_branches: branches,
            selected_worktree: 0,
            selected_branch: 0,
            input: String::new(),
            confirm_action: None,
            merged_worktrees: Vec::new(),
            message: None,
            should_quit: false,
            selected_worktree_path: None,
            config,
            git,
            hook_runner,
        })
    }

    pub fn refresh_worktrees(&mut self) -> Result<(), AppError> {
        self.worktrees = self.git.list_worktrees()?;
        self.filter_worktrees();
        Ok(())
    }

    pub fn refresh_branches(&mut self) -> Result<(), AppError> {
        self.branches = self.git.list_branches()?;
        self.filter_branches();
        Ok(())
    }

    pub fn filter_worktrees(&mut self) {
        if self.input.is_empty() {
            self.filtered_worktrees = self.worktrees.clone();
        } else {
            let query = self.input.to_lowercase();
            self.filtered_worktrees = self
                .worktrees
                .iter()
                .filter(|w| {
                    w.name.to_lowercase().contains(&query)
                        || w.branch
                            .as_ref()
                            .map(|b| b.to_lowercase().contains(&query))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();
        }
        if self.selected_worktree >= self.filtered_worktrees.len() {
            self.selected_worktree = self.filtered_worktrees.len().saturating_sub(1);
        }
    }

    pub fn filter_branches(&mut self) {
        if self.input.is_empty() {
            self.filtered_branches = self.branches.clone();
        } else {
            let query = self.input.to_lowercase();
            self.filtered_branches = self
                .branches
                .iter()
                .filter(|b| b.name.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
        if self.selected_branch >= self.filtered_branches.len() {
            self.selected_branch = self.filtered_branches.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        match self.mode {
            AppMode::Normal => {
                if self.selected_worktree > 0 {
                    self.selected_worktree -= 1;
                }
            }
            AppMode::Create => {
                if self.selected_branch > 0 {
                    self.selected_branch -= 1;
                }
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.mode {
            AppMode::Normal => {
                if self.selected_worktree < self.filtered_worktrees.len().saturating_sub(1) {
                    self.selected_worktree += 1;
                }
            }
            AppMode::Create => {
                // +1 for "Create new branch" option at index 0
                let max_index = self.filtered_branches.len();
                if self.selected_branch < max_index {
                    self.selected_branch += 1;
                }
            }
            _ => {}
        }
    }

    pub fn enter_create_mode(&mut self) -> Result<(), AppError> {
        self.input.clear();
        self.refresh_branches()?;
        self.mode = AppMode::Create;
        // Select "Create new branch" by default (index 0)
        self.selected_branch = 0;
        Ok(())
    }

    pub fn enter_normal_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.input.clear();
        self.confirm_action = None;
        self.filter_worktrees();
    }

    pub fn enter_help_mode(&mut self) {
        self.mode = AppMode::Help;
    }

    pub fn enter_confirm_delete(&mut self) {
        if !self.filtered_worktrees.is_empty() {
            let worktree = &self.filtered_worktrees[self.selected_worktree];
            if !worktree.is_main {
                self.mode = AppMode::Confirm;
                self.confirm_action = Some(ConfirmAction::DeleteSingle);
            } else {
                self.message = Some("Cannot delete main worktree".to_string());
            }
        }
    }

    pub fn enter_confirm_prune(&mut self) -> Result<(), AppError> {
        self.merged_worktrees = self.git.find_merged_worktrees()?;
        if self.merged_worktrees.is_empty() {
            self.message = Some("No merged worktrees to prune".to_string());
        } else {
            self.mode = AppMode::Confirm;
            self.confirm_action = Some(ConfirmAction::Prune);
        }
        Ok(())
    }

    pub fn confirm_action(&mut self, delete_branch: bool) -> Result<(), AppError> {
        match self.confirm_action {
            Some(ConfirmAction::DeleteSingle) => {
                self.delete_selected_worktree(delete_branch)?;
            }
            Some(ConfirmAction::Prune) => {
                self.prune_merged_worktrees(delete_branch)?;
            }
            None => {}
        }
        self.enter_normal_mode();
        Ok(())
    }

    fn delete_selected_worktree(&mut self, delete_branch: bool) -> Result<(), AppError> {
        if self.filtered_worktrees.is_empty() {
            return Ok(());
        }

        let worktree = self.filtered_worktrees[self.selected_worktree].clone();
        if worktree.is_main {
            self.message = Some("Cannot delete main worktree".to_string());
            return Ok(());
        }

        // Run pre_delete hook
        let _ = self.hook_runner.run_pre_delete(&worktree);

        // Get branch name before deleting worktree
        let branch_name = worktree.branch.clone();

        // Delete the worktree
        self.git.delete_worktree(&worktree.name)?;

        // Delete the branch if requested
        if delete_branch {
            if let Some(ref branch) = branch_name {
                if let Err(e) = self.git.delete_branch(branch) {
                    self.message = Some(format!(
                        "Deleted worktree '{}', but failed to delete branch '{}': {}",
                        worktree.name, branch, e
                    ));
                    self.refresh_worktrees()?;
                    return Ok(());
                }
            }
        }

        // Run post_delete hook
        let _ = self.hook_runner.run_post_delete(&worktree);

        if delete_branch {
            if let Some(ref branch) = branch_name {
                self.message = Some(format!(
                    "Deleted worktree '{}' and branch '{}'",
                    worktree.name, branch
                ));
            } else {
                self.message = Some(format!("Deleted worktree: {}", worktree.name));
            }
        } else {
            self.message = Some(format!("Deleted worktree: {}", worktree.name));
        }
        self.refresh_worktrees()?;

        Ok(())
    }

    fn prune_merged_worktrees(&mut self, delete_branch: bool) -> Result<(), AppError> {
        let count = self.merged_worktrees.len();
        let mut deleted_branches = 0;

        for worktree in &self.merged_worktrees.clone() {
            let _ = self.hook_runner.run_pre_delete(worktree);
            let branch_name = worktree.branch.clone();
            self.git.delete_worktree(&worktree.name)?;

            if delete_branch {
                if let Some(ref branch) = branch_name {
                    if self.git.delete_branch(branch).is_ok() {
                        deleted_branches += 1;
                    }
                }
            }

            let _ = self.hook_runner.run_post_delete(worktree);
        }

        if delete_branch && deleted_branches > 0 {
            self.message = Some(format!(
                "Pruned {} worktree(s) and {} branch(es)",
                count, deleted_branches
            ));
        } else {
            self.message = Some(format!("Pruned {} merged worktree(s)", count));
        }
        self.merged_worktrees.clear();
        self.refresh_worktrees()?;

        Ok(())
    }

    pub fn create_worktree(&mut self) -> Result<(), AppError> {
        // Check if "Create new branch" is selected (index 0)
        if self.selected_branch == 0 {
            // Creating a new branch requires input
            if self.input.is_empty() {
                self.message = Some("Please enter a branch name".to_string());
                return Ok(());
            }

            let branch_name = self.input.clone();
            let worktree_name = branch_name.replace('/', "-");

            // Create worktree with a new branch (atomic operation)
            let worktree = match self.git.create_worktree_with_new_branch(
                &worktree_name,
                &branch_name,
                self.config.worktree_base(),
            ) {
                Ok(wt) => wt,
                Err(e) => {
                    self.message = Some(format!("Failed to create: {}", e));
                    return Ok(());
                }
            };

            // Run post_create hook
            let _ = self.hook_runner.run_post_create(&worktree);

            self.message = Some(format!(
                "Created branch '{}' and worktree '{}'",
                branch_name, worktree_name
            ));
            self.enter_normal_mode();
            self.refresh_worktrees()?;

            return Ok(());
        }

        // Existing branch selected (index 1+ maps to filtered_branches[index-1])
        let branch_index = self.selected_branch - 1;
        if branch_index >= self.filtered_branches.len() {
            self.message = Some("No branch selected".to_string());
            return Ok(());
        }

        let branch = &self.filtered_branches[branch_index];
        let branch_name = if branch.is_remote {
            // Extract branch name from remote (e.g., "origin/feature" -> "feature")
            branch
                .name
                .split('/')
                .skip(1)
                .collect::<Vec<_>>()
                .join("/")
        } else {
            branch.name.clone()
        };

        // Use input as worktree name, or branch name if input is empty
        let worktree_name = if self.input.is_empty() {
            branch_name.replace('/', "-")
        } else {
            self.input.clone()
        };

        let worktree = self.git.create_worktree(
            &worktree_name,
            &branch_name,
            self.config.worktree_base(),
        )?;

        // Run post_create hook
        let _ = self.hook_runner.run_post_create(&worktree);

        self.message = Some(format!("Created worktree: {}", worktree_name));
        self.enter_normal_mode();
        self.refresh_worktrees()?;

        Ok(())
    }

    pub fn select_worktree(&mut self) {
        if !self.filtered_worktrees.is_empty() {
            let worktree = &self.filtered_worktrees[self.selected_worktree];
            self.selected_worktree_path = Some(worktree.path.to_string_lossy().to_string());
            self.should_quit = true;
        }
    }

    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
        if self.mode == AppMode::Normal {
            self.filter_worktrees();
        }
    }

    pub fn delete_char(&mut self) {
        self.input.pop();
        if self.mode == AppMode::Normal {
            self.filter_worktrees();
        }
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }

    pub fn get_selected_worktree_detail(&self) -> Option<WorktreeDetail> {
        if self.filtered_worktrees.is_empty() {
            return None;
        }
        let worktree = &self.filtered_worktrees[self.selected_worktree];
        Some(self.git.get_worktree_details(worktree))
    }
}
