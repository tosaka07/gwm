use std::process::Command;

use crate::app::{App, DialogKind, Mode, Notification};
use crate::error::Result;
use crate::tui::Terminal;

use super::Action;

/// Handles action execution
pub struct ActionHandler;

impl ActionHandler {
    /// Handle an action
    pub async fn handle(app: &mut App, action: Action, terminal: &mut Terminal) -> Result<()> {
        match action {
            // Navigation
            Action::MoveUp => Self::handle_move_up(app),
            Action::MoveDown => Self::handle_move_down(app),
            Action::MoveTop => app.move_top(),
            Action::MoveBottom => app.move_bottom(),
            Action::PageUp => {
                for _ in 0..10 {
                    Self::handle_move_up(app);
                }
            }
            Action::PageDown => {
                for _ in 0..10 {
                    Self::handle_move_down(app);
                }
            }
            Action::Select => Self::handle_select(app, terminal)?,
            Action::Back => Self::handle_back(app),

            // Input navigation
            Action::MoveLineStart => {
                // Move cursor to start (for input buffer)
                // Currently not implemented - would need cursor position tracking
            }
            Action::MoveLineEnd => {
                // Move cursor to end (for input buffer)
                // Currently not implemented
            }

            // Worktree operations
            Action::OpenShell => Self::handle_open_shell(app, terminal)?,
            Action::CreateWorktree => Self::handle_create_worktree(app),
            Action::DeleteWorktree => Self::handle_delete_worktree(app),
            Action::DeleteMergedWorktrees => Self::handle_delete_merged(app)?,
            Action::RebaseWorktree => Self::handle_rebase(app),
            Action::Refresh => {
                app.refresh_worktrees()?;
                app.refresh_branches()?;
                app.notify(Notification::info("Refreshed"));
            }

            // Mode switching
            Action::EnterInsertMode => {
                app.mode = Mode::Insert;
            }
            Action::EnterSearchMode => {
                app.mode = Mode::Search;
                app.search_query.clear();
            }
            Action::EnterNormalMode => {
                app.mode = Mode::Normal;
                app.dialog = DialogKind::None;
            }

            // Dialog
            Action::Confirm => Self::handle_confirm(app, terminal).await?,
            Action::Cancel => Self::handle_cancel(app),

            // Input
            Action::InsertChar(c) => Self::handle_insert_char(app, c),
            Action::DeleteChar => Self::handle_delete_char(app),
            Action::DeleteWord => {
                if matches!(app.mode, Mode::Insert | Mode::Search) {
                    // Delete word backwards
                    let trimmed = app.input_buffer.trim_end();
                    if let Some(pos) = trimmed.rfind(char::is_whitespace) {
                        app.input_buffer.truncate(pos + 1);
                    } else {
                        app.input_buffer.clear();
                    }
                }
            }

            // Other
            Action::ToggleHelp => {
                // TODO: Implement help screen toggle
                app.notify(Notification::info("Help: Press ? for help, q to quit"));
            }
            Action::Quit => {
                if app.has_active_dialog() {
                    app.dialog = DialogKind::None;
                    app.mode = Mode::Normal;
                } else {
                    app.should_quit = true;
                }
            }
            Action::ForceQuit => {
                app.should_quit = true;
            }

            // Custom command
            Action::RunCommand(cmd) => {
                Self::run_command(app, terminal, &cmd)?;
            }
        }

        Ok(())
    }

    fn handle_select(app: &mut App, terminal: &mut Terminal) -> Result<()> {
        match &app.dialog {
            DialogKind::CreateWorktree(_) => {
                // Submit form (Enter always submits in Telescope/fzf style)
                Self::submit_create_worktree_form(app)?;
            }
            DialogKind::BranchSelect { .. } => {
                use crate::app::PendingOperation;

                if let Some(branch) = app.selected_branch().cloned() {
                    match app.pending_operation {
                        Some(PendingOperation::Rebase) => {
                            // TODO: Implement actual rebase
                            app.notify(Notification::info(format!(
                                "Rebase onto {} (not yet implemented)",
                                branch
                            )));
                            app.pending_operation = None;
                            app.dialog = DialogKind::None;
                            app.mode = Mode::Normal;
                        }
                        None => {
                            app.dialog = DialogKind::None;
                            app.mode = Mode::Normal;
                        }
                    }
                }
            }
            DialogKind::None => {
                // Normal mode - open shell
                Self::handle_open_shell(app, terminal)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_back(app: &mut App) {
        match &app.dialog {
            DialogKind::None => {
                // Already in main view
            }
            _ => {
                app.dialog = DialogKind::None;
                app.mode = Mode::Normal;
            }
        }
    }

    fn handle_move_up(app: &mut App) {
        if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
            // Navigate branch list
            if form.branch_idx > 0 {
                form.branch_idx -= 1;
            }
        } else {
            app.move_up();
        }
    }

    fn handle_move_down(app: &mut App) {
        if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
            // Navigate branch list
            if form.branch_idx < app.branches.len().saturating_sub(1) {
                form.branch_idx += 1;
            }
        } else {
            app.move_down();
        }
    }

    fn handle_insert_char(app: &mut App, c: char) {
        match &mut app.dialog {
            DialogKind::CreateWorktree(ref mut form) => {
                // Always insert to name field (Telescope/fzf style)
                form.name.push(c);
            }
            _ => {
                if matches!(app.mode, Mode::Insert | Mode::Search) {
                    app.input_buffer.push(c);
                }
            }
        }
    }

    fn handle_delete_char(app: &mut App) {
        match &mut app.dialog {
            DialogKind::CreateWorktree(ref mut form) => {
                // Always delete from name field (Telescope/fzf style)
                form.name.pop();
            }
            _ => {
                if matches!(app.mode, Mode::Insert | Mode::Search) {
                    app.input_buffer.pop();
                }
            }
        }
    }

    fn handle_open_shell(app: &mut App, terminal: &mut Terminal) -> Result<()> {
        let Some(worktree) = app.selected_worktree() else {
            return Ok(());
        };

        let path = worktree.path.clone();

        // Suspend terminal and spawn shell
        terminal.suspend(|| {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

            let status = Command::new(&shell).current_dir(&path).status();

            match status {
                Ok(s) if s.success() => {}
                Ok(s) => {
                    eprintln!("Shell exited with status: {}", s);
                }
                Err(e) => {
                    eprintln!("Failed to spawn shell: {}", e);
                }
            }
        })?;

        // Refresh after returning from shell
        app.refresh_worktrees()?;

        Ok(())
    }

    fn handle_create_worktree(app: &mut App) {
        use crate::app::CreateWorktreeForm;

        // Default to first branch, empty name
        let form = CreateWorktreeForm::new(0, "");
        app.dialog = DialogKind::CreateWorktree(form);
        app.mode = Mode::Dialog;
    }

    fn handle_delete_worktree(app: &mut App) {
        let Some(worktree) = app.selected_worktree() else {
            return;
        };

        if worktree.is_main {
            app.show_error("Cannot delete main worktree");
            return;
        }

        app.dialog = DialogKind::ConfirmDelete {
            worktree_path: worktree.path.display().to_string(),
        };
        app.mode = Mode::Dialog;
    }

    fn handle_delete_merged(app: &mut App) -> Result<()> {
        // Find default branch
        let base_branch = app
            .branches
            .iter()
            .find(|b| *b == "main" || *b == "master")
            .cloned()
            .unwrap_or_else(|| "main".to_string());

        let merged = app.worktree_manager().find_merged_worktrees(&base_branch)?;

        if merged.is_empty() {
            app.notify(Notification::info("No merged worktrees found"));
        } else {
            let count = merged.len();
            app.dialog = DialogKind::ConfirmDelete {
                worktree_path: format!("{} merged worktrees", count),
            };
            app.mode = Mode::Dialog;
        }

        Ok(())
    }

    fn handle_rebase(app: &mut App) {
        app.branch_list_state.select(Some(0));
        app.pending_operation = Some(crate::app::PendingOperation::Rebase);
        app.dialog = DialogKind::BranchSelect {
            title: "Rebase Onto".to_string(),
        };
        app.mode = Mode::Dialog;
    }

    async fn handle_confirm(app: &mut App, _terminal: &mut Terminal) -> Result<()> {
        match &app.dialog {
            DialogKind::ConfirmDelete { worktree_path } => {
                let path = worktree_path.clone();

                // Check if it's a batch delete
                if path.contains("merged worktrees") {
                    let base_branch = app
                        .branches
                        .iter()
                        .find(|b| *b == "main" || *b == "master")
                        .cloned()
                        .unwrap_or_else(|| "main".to_string());

                    match app.worktree_manager().delete_merged_worktrees(&base_branch) {
                        Ok(deleted) => {
                            let msg = format!("Deleted {} worktrees", deleted.len());
                            app.notify(Notification::info(msg));
                        }
                        Err(e) => {
                            app.notify(Notification::error(format!("Failed: {}", e)));
                        }
                    }
                } else {
                    // Single worktree delete
                    let path = std::path::PathBuf::from(&path);
                    match app.worktree_manager().remove(&path, true) {
                        Ok(_) => {
                            app.notify(Notification::info("Worktree deleted"));
                        }
                        Err(e) => {
                            app.notify(Notification::error(format!("Failed: {}", e)));
                        }
                    }
                }

                app.refresh_worktrees()?;
                app.dialog = DialogKind::None;
                app.mode = Mode::Normal;
            }
            DialogKind::BranchSelect { .. } => {
                use crate::app::PendingOperation;

                if let Some(branch) = app.selected_branch().cloned() {
                    match app.pending_operation {
                        Some(PendingOperation::Rebase) => {
                            // TODO: Implement actual rebase
                            app.notify(Notification::info(format!(
                                "Rebase onto {} (not yet implemented)",
                                branch
                            )));
                            app.pending_operation = None;
                            app.dialog = DialogKind::None;
                            app.mode = Mode::Normal;
                        }
                        None => {
                            // No pending operation, just close dialog
                            app.dialog = DialogKind::None;
                            app.mode = Mode::Normal;
                        }
                    }
                }
            }
            DialogKind::CreateWorktree(_) => {
                // Submit form
                Self::submit_create_worktree_form(app)?;
            }
            DialogKind::None => {}
        }

        Ok(())
    }

    fn handle_cancel(app: &mut App) {
        app.pending_operation = None;
        app.dialog = DialogKind::None;
        app.mode = Mode::Normal;
        app.input_buffer.clear();
    }

    fn submit_create_worktree_form(app: &mut App) -> Result<()> {
        let (branch_idx, name) = if let DialogKind::CreateWorktree(ref form) = app.dialog {
            (form.branch_idx, form.name.clone())
        } else {
            return Ok(());
        };

        if name.is_empty() {
            app.notify(Notification::error("Name cannot be empty"));
            return Ok(());
        }

        let branch = app.branches.get(branch_idx).cloned().unwrap_or_default();

        // Create worktree
        let base_dir = app.config.worktree.base_dir.clone();
        let worktree_path = if let Some(template) = base_dir {
            let path = template.replace("{name}", &name).replace(
                "{repo}",
                &app.repo_root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            );

            if path.starts_with('/') {
                std::path::PathBuf::from(path)
            } else {
                app.repo_root.join(path)
            }
        } else {
            app.repo_root.parent().unwrap_or(&app.repo_root).join(&name)
        };

        // Use selected branch as base, create new branch with the given name
        let base_branch = if branch.is_empty() {
            None
        } else {
            Some(branch.as_str())
        };

        match app
            .worktree_manager()
            .create(&worktree_path, &name, base_branch)
        {
            Ok(_) => {
                app.notify(Notification::info(format!(
                    "Created worktree: {}",
                    worktree_path.display()
                )));
            }
            Err(e) => {
                app.notify(Notification::error(format!("Failed: {}", e)));
            }
        }

        app.refresh_worktrees()?;
        app.dialog = DialogKind::None;
        app.mode = Mode::Normal;

        Ok(())
    }

    fn run_command(app: &mut App, terminal: &mut Terminal, cmd: &str) -> Result<()> {
        let Some(worktree) = app.selected_worktree() else {
            return Ok(());
        };

        let cwd = worktree.path.clone();

        // Suspend terminal and run command
        terminal.suspend(|| {
            let status = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(&cwd)
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("\nCommand completed successfully. Press Enter to continue...");
                }
                Ok(s) => {
                    println!(
                        "\nCommand exited with status: {}. Press Enter to continue...",
                        s
                    );
                }
                Err(e) => {
                    println!("\nFailed to run command: {}. Press Enter to continue...", e);
                }
            }

            // Wait for user input
            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);
        })?;

        app.refresh_worktrees()?;

        Ok(())
    }
}
