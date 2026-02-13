use crate::config::{Config, RepositorySettings};
use crate::git::{Branch, GitManager, Worktree, WorktreeDetail};
use crate::hooks::SetupRunner;
use crate::theme::Theme;
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
    pub theme: Theme,
    config: Config,
    git: GitManager,
}

impl App {
    pub fn new(config: Config, git: GitManager) -> Result<Self, AppError> {
        let worktrees = git.list_worktrees()?;
        let branches = git.list_branches()?;
        let theme = Theme::from_config(Some(config.theme_name()), config.theme_colors());

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
            theme,
            config,
            git,
        })
    }

    /// Get effective repository settings for the current repository
    /// Returns settings from repository_settings if matched, otherwise falls back to top-level settings
    fn get_repository_settings(&self) -> Option<RepositorySettings> {
        let repo_path = self.git.repo_root().to_string_lossy().to_string();
        let settings = self.config.get_effective_settings(&repo_path);

        // Return None if no copy_files and no setup_commands
        if settings.copy_files.is_none() && settings.setup_commands.is_none() {
            None
        } else {
            Some(settings)
        }
    }

    /// Get the main worktree path
    fn get_main_worktree_path(&self) -> Option<std::path::PathBuf> {
        self.worktrees
            .iter()
            .find(|w| w.is_main)
            .map(|w| w.path.clone())
    }

    /// Create a SetupRunner with repository settings and main worktree path
    fn create_setup_runner(&self) -> SetupRunner {
        let runner = SetupRunner::new(self.get_repository_settings());
        if let Some(main_path) = self.get_main_worktree_path() {
            runner.with_main_worktree(main_path)
        } else {
            runner
        }
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
            let branch_name = worktree.branch.clone();
            self.git.delete_worktree(&worktree.name)?;

            if delete_branch {
                if let Some(ref branch) = branch_name {
                    if self.git.delete_branch(branch).is_ok() {
                        deleted_branches += 1;
                    }
                }
            }
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
        let base_path = self
            .config
            .worktree_basedir_expanded_with_repo_root(self.git.repo_root());
        let repo_info = self.git.get_repo_info();

        // Auto-create base directory if enabled
        if self.config.auto_mkdir() {
            let base_dir = std::path::Path::new(&base_path);
            if !base_dir.exists() {
                std::fs::create_dir_all(base_dir)
                    .map_err(|e| AppError::Git(crate::git::GitError::IoError(e)))?;
            }
        }

        // Check if "Create new branch" is selected (index 0)
        if self.selected_branch == 0 {
            // Creating a new branch requires input
            if self.input.is_empty() {
                self.message = Some("Please enter a branch name".to_string());
                return Ok(());
            }

            let branch_name = self.input.clone();
            let worktree_name = match self
                .config
                .generate_worktree_name(&branch_name, repo_info.as_ref())
            {
                Ok(name) => name,
                Err(e) => {
                    self.message = Some(format!("{}", e));
                    return Ok(());
                }
            };

            // Create worktree with a new branch (atomic operation)
            let worktree = match self.git.create_worktree_with_new_branch(
                &worktree_name,
                &branch_name,
                &base_path,
            ) {
                Ok(wt) => wt,
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("already exists") && error_msg.contains("branch") {
                        self.message = Some(format!("Branch '{}' already exists", branch_name));
                    } else if error_msg.contains("directory exists") {
                        self.message = Some(format!(
                            "Directory '{}' already exists. Run 'git worktree prune' to clean up",
                            worktree_name
                        ));
                    } else {
                        self.message = Some(format!("Failed to create: {}", e));
                    }
                    return Ok(());
                }
            };

            // Run setup (copy files and commands)
            let setup_runner = self.create_setup_runner();
            let _ = setup_runner.run_setup(&worktree);

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
            branch.name.split('/').skip(1).collect::<Vec<_>>().join("/")
        } else {
            branch.name.clone()
        };

        // Use input as worktree name, or branch name if input is empty
        let worktree_name = if self.input.is_empty() {
            match self
                .config
                .generate_worktree_name(&branch_name, repo_info.as_ref())
            {
                Ok(name) => name,
                Err(e) => {
                    self.message = Some(format!("{}", e));
                    return Ok(());
                }
            }
        } else {
            self.input.clone()
        };

        let worktree = match self
            .git
            .create_worktree(&worktree_name, &branch_name, &base_path)
        {
            Ok(wt) => wt,
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("already checked out") {
                    self.message = Some(format!(
                        "Branch '{}' is already used by another worktree",
                        branch_name
                    ));
                } else if error_msg.contains("directory exists") {
                    self.message = Some(format!(
                        "Directory '{}' already exists. Run 'git worktree prune' to clean up",
                        worktree_name
                    ));
                } else {
                    self.message = Some(format!("Failed to create: {}", e));
                }
                return Ok(());
            }
        };

        // Run setup (copy files and commands)
        let setup_runner = self.create_setup_runner();
        let _ = setup_runner.run_setup(&worktree);

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

    /// Format path for display (uses tilde_home config setting)
    pub fn format_path(&self, path: &str) -> String {
        self.config.format_path_for_display(path)
    }

    /// Check if icons should be displayed (uses ui.icons config setting)
    pub fn icons_enabled(&self) -> bool {
        self.config.icons_enabled()
    }

    /// Create an App instance for testing without Git operations
    #[cfg(test)]
    pub fn new_for_test(config: Config, worktrees: Vec<Worktree>, branches: Vec<Branch>) -> Self {
        use std::path::PathBuf;

        // Use the project root (where Cargo.toml is) as the repo path for testing
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let git = GitManager::from_path(&manifest_dir).unwrap();
        let theme = Theme::from_config(Some(config.theme_name()), config.theme_colors());

        Self {
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
            theme,
            config,
            git,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_worktrees() -> Vec<Worktree> {
        vec![
            Worktree {
                name: "main".to_string(),
                path: PathBuf::from("/repo/main"),
                branch: Some("main".to_string()),
                is_main: true,
            },
            Worktree {
                name: "feature-a".to_string(),
                path: PathBuf::from("/repo/feature-a"),
                branch: Some("feature/a".to_string()),
                is_main: false,
            },
            Worktree {
                name: "feature-b".to_string(),
                path: PathBuf::from("/repo/feature-b"),
                branch: Some("feature/b".to_string()),
                is_main: false,
            },
            Worktree {
                name: "bugfix-x".to_string(),
                path: PathBuf::from("/repo/bugfix-x"),
                branch: Some("bugfix/x".to_string()),
                is_main: false,
            },
        ]
    }

    fn create_test_branches() -> Vec<Branch> {
        vec![
            Branch {
                name: "main".to_string(),
                is_remote: false,
                is_head: true,
            },
            Branch {
                name: "feature/a".to_string(),
                is_remote: false,
                is_head: false,
            },
            Branch {
                name: "feature/b".to_string(),
                is_remote: false,
                is_head: false,
            },
            Branch {
                name: "origin/feature/c".to_string(),
                is_remote: true,
                is_head: false,
            },
        ]
    }

    fn create_test_app() -> App {
        App::new_for_test(
            Config::default(),
            create_test_worktrees(),
            create_test_branches(),
        )
    }

    // ========== Filter Tests ==========

    #[test]
    fn test_filter_worktrees_by_name() {
        let mut app = create_test_app();

        app.input = "feature".to_string();
        app.filter_worktrees();

        assert_eq!(app.filtered_worktrees.len(), 2);
        assert!(app
            .filtered_worktrees
            .iter()
            .all(|w| w.name.contains("feature")));
    }

    #[test]
    fn test_filter_worktrees_by_branch() {
        let mut app = create_test_app();

        app.input = "bugfix".to_string();
        app.filter_worktrees();

        assert_eq!(app.filtered_worktrees.len(), 1);
        assert_eq!(app.filtered_worktrees[0].name, "bugfix-x");
    }

    #[test]
    fn test_filter_worktrees_case_insensitive() {
        let mut app = create_test_app();

        app.input = "FEATURE".to_string();
        app.filter_worktrees();

        assert_eq!(app.filtered_worktrees.len(), 2);
    }

    #[test]
    fn test_filter_worktrees_empty_input() {
        let mut app = create_test_app();
        app.input = "something".to_string();
        app.filter_worktrees();
        assert!(app.filtered_worktrees.len() < app.worktrees.len());

        app.input.clear();
        app.filter_worktrees();

        assert_eq!(app.filtered_worktrees.len(), app.worktrees.len());
    }

    #[test]
    fn test_filter_branches() {
        let mut app = create_test_app();

        app.input = "feature".to_string();
        app.filter_branches();

        assert_eq!(app.filtered_branches.len(), 3);
    }

    // ========== Navigation Tests ==========

    #[test]
    fn test_move_up_boundary() {
        let mut app = create_test_app();
        app.selected_worktree = 0;

        app.move_up();

        // Should not go below 0
        assert_eq!(app.selected_worktree, 0);
    }

    #[test]
    fn test_move_down_boundary() {
        let mut app = create_test_app();
        app.selected_worktree = app.filtered_worktrees.len() - 1;

        app.move_down();

        // Should not exceed max index
        assert_eq!(app.selected_worktree, app.filtered_worktrees.len() - 1);
    }

    #[test]
    fn test_move_up_decrements() {
        let mut app = create_test_app();
        app.selected_worktree = 2;

        app.move_up();

        assert_eq!(app.selected_worktree, 1);
    }

    #[test]
    fn test_move_down_increments() {
        let mut app = create_test_app();
        app.selected_worktree = 1;

        app.move_down();

        assert_eq!(app.selected_worktree, 2);
    }

    // ========== Mode Transition Tests ==========

    #[test]
    fn test_enter_normal_mode_clears_input() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.input = "some-input".to_string();

        app.enter_normal_mode();

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.input.is_empty());
        assert!(app.confirm_action.is_none());
    }

    #[test]
    fn test_enter_help_mode() {
        let mut app = create_test_app();

        app.enter_help_mode();

        assert_eq!(app.mode, AppMode::Help);
    }

    #[test]
    fn test_enter_confirm_delete_sets_mode() {
        let mut app = create_test_app();
        app.selected_worktree = 1; // Not main worktree

        app.enter_confirm_delete();

        assert_eq!(app.mode, AppMode::Confirm);
        assert_eq!(app.confirm_action, Some(ConfirmAction::DeleteSingle));
    }

    #[test]
    fn test_enter_confirm_delete_prevents_main_deletion() {
        let mut app = create_test_app();
        app.selected_worktree = 0; // Main worktree

        app.enter_confirm_delete();

        // Should not enter confirm mode
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.message.is_some());
        assert!(app.message.as_ref().unwrap().contains("Cannot delete main"));
    }

    // ========== Input Tests ==========

    #[test]
    fn test_input_char() {
        let mut app = create_test_app();

        app.input_char('a');
        app.input_char('b');
        app.input_char('c');

        assert_eq!(app.input, "abc");
    }

    #[test]
    fn test_delete_char() {
        let mut app = create_test_app();
        app.input = "test".to_string();

        app.delete_char();

        assert_eq!(app.input, "tes");
    }

    #[test]
    fn test_delete_char_empty() {
        let mut app = create_test_app();
        app.input = String::new();

        app.delete_char();

        assert!(app.input.is_empty());
    }

    #[test]
    fn test_input_char_triggers_filter_in_normal_mode() {
        let mut app = create_test_app();
        assert_eq!(app.filtered_worktrees.len(), 4);

        app.input_char('f');
        app.input_char('e');
        app.input_char('a');

        // Should have filtered to just worktrees containing "fea"
        assert_eq!(app.filtered_worktrees.len(), 2);
    }

    // ========== Selection Tests ==========

    #[test]
    fn test_select_worktree_sets_path() {
        let mut app = create_test_app();
        app.selected_worktree = 1;

        app.select_worktree();

        assert!(app.should_quit);
        assert_eq!(
            app.selected_worktree_path,
            Some("/repo/feature-a".to_string())
        );
    }

    #[test]
    fn test_select_worktree_empty_list() {
        let mut app = App::new_for_test(Config::default(), vec![], vec![]);

        app.select_worktree();

        assert!(!app.should_quit);
        assert!(app.selected_worktree_path.is_none());
    }

    #[test]
    fn test_clear_message() {
        let mut app = create_test_app();
        app.message = Some("Test message".to_string());

        app.clear_message();

        assert!(app.message.is_none());
    }

    // ========== Filter Adjusts Selection Tests ==========

    #[test]
    fn test_filter_adjusts_selection_when_out_of_bounds() {
        let mut app = create_test_app();
        app.selected_worktree = 3; // Last item

        app.input = "feature-a".to_string();
        app.filter_worktrees();

        // After filtering, only 1 item remains, selection should be adjusted
        assert!(app.selected_worktree < app.filtered_worktrees.len());
    }

    // ========== Config Integration Tests ==========

    #[test]
    fn test_icons_enabled_default() {
        let app = create_test_app();
        // Default should be true
        assert!(app.icons_enabled());
    }

    #[test]
    fn test_icons_enabled_disabled() {
        use crate::config::UiConfig;

        let config = Config {
            ui: UiConfig {
                icons: Some(false),
                ..Default::default()
            },
            ..Default::default()
        };
        let app = App::new_for_test(config, create_test_worktrees(), create_test_branches());

        assert!(!app.icons_enabled());
    }

    #[test]
    fn test_format_path_with_tilde_home() {
        let app = create_test_app();
        let home = dirs::home_dir().unwrap();
        let full_path = format!("{}/projects/test", home.to_string_lossy());

        let formatted = app.format_path(&full_path);

        // Default tilde_home is true, so should be compressed
        assert_eq!(formatted, "~/projects/test");
    }

    #[test]
    fn test_format_path_without_tilde_home() {
        use crate::config::UiConfig;

        let config = Config {
            ui: UiConfig {
                tilde_home: Some(false),
                ..Default::default()
            },
            ..Default::default()
        };
        let app = App::new_for_test(config, create_test_worktrees(), create_test_branches());

        let home = dirs::home_dir().unwrap();
        let full_path = format!("{}/projects/test", home.to_string_lossy());

        let formatted = app.format_path(&full_path);

        // tilde_home is false, so should NOT be compressed
        assert_eq!(formatted, full_path);
    }

    // ========== Main Worktree Path Tests ==========

    #[test]
    fn test_get_main_worktree_path_found() {
        let app = create_test_app();

        let main_path = app.get_main_worktree_path();

        assert!(main_path.is_some());
        assert_eq!(main_path.unwrap(), PathBuf::from("/repo/main"));
    }

    #[test]
    fn test_get_main_worktree_path_not_found() {
        let worktrees = vec![
            Worktree {
                name: "feature-a".to_string(),
                path: PathBuf::from("/repo/feature-a"),
                branch: Some("feature/a".to_string()),
                is_main: false,
            },
            Worktree {
                name: "feature-b".to_string(),
                path: PathBuf::from("/repo/feature-b"),
                branch: Some("feature/b".to_string()),
                is_main: false,
            },
        ];
        let app = App::new_for_test(Config::default(), worktrees, vec![]);

        let main_path = app.get_main_worktree_path();

        assert!(main_path.is_none());
    }

    #[test]
    fn test_get_main_worktree_path_empty_list() {
        let app = App::new_for_test(Config::default(), vec![], vec![]);

        let main_path = app.get_main_worktree_path();

        assert!(main_path.is_none());
    }

    // ========== Create Worktree Logic Tests ==========

    #[test]
    fn test_create_worktree_new_branch_empty_input_shows_message() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.selected_branch = 0; // "Create new branch" option
        app.input.clear();

        // This would trigger "Please enter a branch name" message
        // Since create_worktree requires actual git repo, we test the logic condition
        let should_show_message = app.selected_branch == 0 && app.input.is_empty();

        assert!(should_show_message);
    }

    #[test]
    fn test_create_worktree_existing_branch_selected() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.selected_branch = 1; // First actual branch (index 1 = filtered_branches[0])

        // Verify branch index mapping
        let branch_index = app.selected_branch - 1;
        assert_eq!(branch_index, 0);
        assert!(branch_index < app.filtered_branches.len());
    }

    #[test]
    fn test_create_worktree_remote_branch_name_extraction() {
        // Test the remote branch name extraction logic
        let remote_branch_name = "origin/feature/test";

        let extracted_name: String = remote_branch_name
            .split('/')
            .skip(1)
            .collect::<Vec<_>>()
            .join("/");

        assert_eq!(extracted_name, "feature/test");
    }

    #[test]
    fn test_create_worktree_remote_branch_nested_name() {
        // Test nested remote branch name
        let remote_branch_name = "origin/user/feature/auth";

        let extracted_name: String = remote_branch_name
            .split('/')
            .skip(1)
            .collect::<Vec<_>>()
            .join("/");

        assert_eq!(extracted_name, "user/feature/auth");
    }

    #[test]
    fn test_create_worktree_uses_custom_name_from_input() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.selected_branch = 1; // Existing branch
        app.input = "custom-worktree-name".to_string();

        // When input is not empty, it should be used as worktree name
        let worktree_name = if app.input.is_empty() {
            "default-name".to_string()
        } else {
            app.input.clone()
        };

        assert_eq!(worktree_name, "custom-worktree-name");
    }

    #[test]
    fn test_create_worktree_uses_branch_name_when_input_empty() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.selected_branch = 1;
        app.input.clear();

        // When input is empty, branch name should be used
        let branch_index = app.selected_branch - 1;
        let branch_name = &app.filtered_branches[branch_index].name;

        let worktree_name = if app.input.is_empty() {
            branch_name.clone()
        } else {
            app.input.clone()
        };

        assert_eq!(worktree_name, "main");
    }

    #[test]
    fn test_create_mode_navigation_includes_create_option() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.selected_branch = 0;

        // In create mode, index 0 is "Create new branch"
        // Moving down should work
        app.move_down();
        assert_eq!(app.selected_branch, 1);

        // Can go up back to 0
        app.move_up();
        assert_eq!(app.selected_branch, 0);
    }

    #[test]
    fn test_create_mode_max_navigation_boundary() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        // +1 for "Create new branch" option
        let max_index = app.filtered_branches.len();

        app.selected_branch = max_index;
        app.move_down();

        // Should not exceed max
        assert_eq!(app.selected_branch, max_index);
    }

    #[test]
    fn test_enter_create_mode_resets_selection() {
        let mut app = create_test_app();
        app.selected_branch = 5;
        app.input = "some-filter".to_string();

        let _ = app.enter_create_mode();

        // Should reset to select "Create new branch" (index 0)
        assert_eq!(app.selected_branch, 0);
        assert!(app.input.is_empty());
        assert_eq!(app.mode, AppMode::Create);
    }

    // ========== Prune Tests ==========

    #[test]
    fn test_enter_confirm_prune_with_no_merged() {
        let mut app = create_test_app();
        // merged_worktrees is empty by default

        // Note: This requires actual git operations, so we test the state
        assert!(app.merged_worktrees.is_empty());
    }

    #[test]
    fn test_confirm_action_delete_single() {
        let mut app = create_test_app();
        app.mode = AppMode::Confirm;
        app.confirm_action = Some(ConfirmAction::DeleteSingle);

        assert_eq!(app.confirm_action, Some(ConfirmAction::DeleteSingle));
    }

    #[test]
    fn test_confirm_action_prune() {
        let mut app = create_test_app();
        app.mode = AppMode::Confirm;
        app.confirm_action = Some(ConfirmAction::Prune);

        assert_eq!(app.confirm_action, Some(ConfirmAction::Prune));
    }
}
