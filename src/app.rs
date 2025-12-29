use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;
use tachyonfx::Interpolation;

use crate::action::{ActionDispatcher, ActionHandler};
use crate::config::Config;
use crate::error::Result;
use crate::git::{Worktree, WorktreeManager};
use crate::tui::{render, Event, EventHandler, Terminal};

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Search,
    Dialog,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "Normal"),
            Mode::Insert => write!(f, "Insert"),
            Mode::Search => write!(f, "Search"),
            Mode::Dialog => write!(f, "Dialog"),
        }
    }
}

/// Pending operation that will be executed after dialog interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOperation {
    /// Rebasing current worktree onto selected branch
    Rebase,
}

/// Create worktree form state (Telescope/fzf style)
#[derive(Debug, Clone, Default)]
pub struct CreateWorktreeForm {
    pub branch_idx: usize,
    pub name: String,
}

impl CreateWorktreeForm {
    pub fn new(branch_idx: usize, default_name: &str) -> Self {
        Self {
            branch_idx,
            name: default_name.to_string(),
        }
    }
}

/// Dialog types (UI only - what to display)
#[derive(Debug, Clone, Default)]
pub enum DialogKind {
    #[default]
    None,
    ConfirmDelete {
        worktree_path: String,
    },
    CreateWorktree(CreateWorktreeForm),
    BranchSelect {
        title: String,
    },
}

/// Notification level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Error,
}

/// Notification duration
const NOTIFICATION_DURATION: Duration = Duration::from_secs(5);

/// Slide-out animation duration
const SLIDE_OUT_DURATION_MS: u128 = 300;

/// Notification with slide-out animation
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub expires_at: Instant,
}

impl Notification {
    pub fn new(message: impl Into<String>, level: NotificationLevel) -> Self {
        Self {
            message: message.into(),
            level,
            expires_at: Instant::now() + NOTIFICATION_DURATION,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Info)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Error)
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Calculate slide-out offset (0 to popup_width) for the last 300ms
    /// Returns 0 when not sliding, increases as it slides out
    pub fn slide_offset(&self, popup_width: u16) -> u16 {
        let now = Instant::now();
        if now >= self.expires_at {
            return popup_width;
        }
        let remaining = self.expires_at.duration_since(now);
        let remaining_ms = remaining.as_millis();

        if remaining_ms >= SLIDE_OUT_DURATION_MS {
            return 0;
        }

        // Calculate progress (0.0 to 1.0)
        let progress = 1.0 - (remaining_ms as f32 / SLIDE_OUT_DURATION_MS as f32);

        // Apply easing (QuadOut: 1 - (1-t)^2)
        let eased = Interpolation::QuadOut.alpha(progress);

        (popup_width as f32 * eased) as u16
    }
}

/// Main application state
pub struct App {
    pub config: Config,
    pub mode: Mode,
    pub dialog: DialogKind,

    // Pending operation (what to do after dialog interaction)
    pub pending_operation: Option<PendingOperation>,

    // Worktree list
    pub worktrees: Vec<Worktree>,
    pub list_state: ListState,

    // Branch list (for dialogs)
    pub branches: Vec<String>,
    pub branch_list_state: ListState,

    // Input state
    pub input_buffer: String,
    pub search_query: String,

    // Notifications (stacked, newest at end)
    pub notifications: Vec<Notification>,

    // Exit flag
    pub should_quit: bool,

    // Repository info
    pub repo_root: PathBuf,

    // Managers
    worktree_manager: WorktreeManager,
    action_dispatcher: ActionDispatcher,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Result<Self> {
        let worktree_manager = WorktreeManager::new()?;
        let repo_root = worktree_manager.repo_root().to_path_buf();
        let action_dispatcher = ActionDispatcher::new(&config);

        let mut app = Self {
            config,
            mode: Mode::Normal,
            dialog: DialogKind::None,
            pending_operation: None,
            worktrees: Vec::new(),
            list_state: ListState::default(),
            branches: Vec::new(),
            branch_list_state: ListState::default(),
            input_buffer: String::new(),
            search_query: String::new(),
            notifications: Vec::new(),
            should_quit: false,
            repo_root,
            worktree_manager,
            action_dispatcher,
        };

        // Load initial data
        app.refresh_worktrees()?;
        app.refresh_branches()?;

        // Select first item
        if !app.worktrees.is_empty() {
            app.list_state.select(Some(0));
        }

        Ok(app)
    }

    /// Run the application main loop
    pub async fn run(&mut self, mut terminal: Terminal) -> Result<()> {
        let event_handler = EventHandler::default();

        while !self.should_quit {
            // Draw UI
            terminal.draw(|frame| render(frame, self))?;

            // Handle events
            if let Some(event) = event_handler.poll()? {
                self.handle_event(event, &mut terminal).await?;
            }

            // Clear expired notifications
            self.notifications.retain(|n| !n.is_expired());
        }

        Ok(())
    }

    /// Handle an event
    async fn handle_event(&mut self, event: Event, terminal: &mut Terminal) -> Result<()> {
        match event {
            Event::Key(key) => {
                if let Some(action) = self.action_dispatcher.dispatch(key, &self.mode) {
                    ActionHandler::handle(self, action, terminal).await?;
                }
            }
            Event::Resize => {
                // Terminal handles resize automatically
            }
            Event::Tick => {
                // Periodic updates if needed
            }
        }
        Ok(())
    }

    /// Refresh worktree list
    pub fn refresh_worktrees(&mut self) -> Result<()> {
        self.worktrees = self.worktree_manager.list()?;
        Ok(())
    }

    /// Refresh branch list
    pub fn refresh_branches(&mut self) -> Result<()> {
        self.branches = self.worktree_manager.list_branches()?;
        Ok(())
    }

    /// Get selected worktree
    pub fn selected_worktree(&self) -> Option<&Worktree> {
        self.list_state
            .selected()
            .and_then(|idx| self.worktrees.get(idx))
    }

    /// Get selected branch
    pub fn selected_branch(&self) -> Option<&String> {
        self.branch_list_state
            .selected()
            .and_then(|idx| self.branches.get(idx))
    }

    /// Check if there's an active dialog
    pub fn has_active_dialog(&self) -> bool {
        !matches!(self.dialog, DialogKind::None)
    }

    /// Show error notification
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.notifications.push(Notification::error(message));
    }

    /// Show notification
    pub fn notify(&mut self, notification: Notification) {
        self.notifications.push(notification);
    }

    /// Get worktree manager reference
    pub fn worktree_manager(&self) -> &WorktreeManager {
        &self.worktree_manager
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let len = if matches!(self.dialog, DialogKind::BranchSelect { .. }) {
            self.branches.len()
        } else {
            self.worktrees.len()
        };

        if len == 0 {
            return;
        }

        let state = if matches!(self.dialog, DialogKind::BranchSelect { .. }) {
            &mut self.branch_list_state
        } else {
            &mut self.list_state
        };

        let current = state.selected().unwrap_or(0);
        let next = if current == 0 { len - 1 } else { current - 1 };
        state.select(Some(next));
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let len = if matches!(self.dialog, DialogKind::BranchSelect { .. }) {
            self.branches.len()
        } else {
            self.worktrees.len()
        };

        if len == 0 {
            return;
        }

        let state = if matches!(self.dialog, DialogKind::BranchSelect { .. }) {
            &mut self.branch_list_state
        } else {
            &mut self.list_state
        };

        let current = state.selected().unwrap_or(0);
        let next = (current + 1) % len;
        state.select(Some(next));
    }

    /// Move to top
    pub fn move_top(&mut self) {
        let state = if matches!(self.dialog, DialogKind::BranchSelect { .. }) {
            &mut self.branch_list_state
        } else {
            &mut self.list_state
        };
        state.select(Some(0));
    }

    /// Move to bottom
    pub fn move_bottom(&mut self) {
        let len = if matches!(self.dialog, DialogKind::BranchSelect { .. }) {
            self.branches.len()
        } else {
            self.worktrees.len()
        };

        if len == 0 {
            return;
        }

        let state = if matches!(self.dialog, DialogKind::BranchSelect { .. }) {
            &mut self.branch_list_state
        } else {
            &mut self.list_state
        };
        state.select(Some(len - 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===================
    // CreateWorktreeForm tests
    // ===================

    #[test]
    fn test_create_worktree_form_new() {
        let form = CreateWorktreeForm::new(2, "feature-branch");
        assert_eq!(form.branch_idx, 2);
        assert_eq!(form.name, "feature-branch");
    }

    #[test]
    fn test_create_worktree_form_default() {
        let form = CreateWorktreeForm::default();
        assert_eq!(form.branch_idx, 0);
        assert_eq!(form.name, "");
    }

    // ===================
    // Mode tests
    // ===================

    #[test]
    fn test_mode_display() {
        assert_eq!(format!("{}", Mode::Normal), "Normal");
        assert_eq!(format!("{}", Mode::Insert), "Insert");
        assert_eq!(format!("{}", Mode::Search), "Search");
        assert_eq!(format!("{}", Mode::Dialog), "Dialog");
    }

    #[test]
    fn test_mode_default() {
        let mode = Mode::default();
        assert_eq!(mode, Mode::Normal);
    }

    // ===================
    // DialogKind tests
    // ===================

    #[test]
    fn test_dialog_kind_default() {
        let dialog = DialogKind::default();
        assert!(matches!(dialog, DialogKind::None));
    }

    #[test]
    fn test_dialog_kind_confirm_delete() {
        let dialog = DialogKind::ConfirmDelete {
            worktree_path: "/path/to/worktree".to_string(),
        };
        if let DialogKind::ConfirmDelete { worktree_path } = dialog {
            assert_eq!(worktree_path, "/path/to/worktree");
        } else {
            panic!("Expected ConfirmDelete variant");
        }
    }

    #[test]
    fn test_dialog_kind_create_worktree() {
        let form = CreateWorktreeForm::new(1, "test-branch");
        let dialog = DialogKind::CreateWorktree(form);
        if let DialogKind::CreateWorktree(f) = dialog {
            assert_eq!(f.branch_idx, 1);
            assert_eq!(f.name, "test-branch");
        } else {
            panic!("Expected CreateWorktree variant");
        }
    }

    #[test]
    fn test_dialog_kind_branch_select() {
        let dialog = DialogKind::BranchSelect {
            title: "Select Branch".to_string(),
        };
        if let DialogKind::BranchSelect { title } = dialog {
            assert_eq!(title, "Select Branch");
        } else {
            panic!("Expected BranchSelect variant");
        }
    }

    // ===================
    // Notification tests
    // ===================

    #[test]
    fn test_notification_info() {
        let notification = Notification::info("Test message");
        assert_eq!(notification.message, "Test message");
        assert_eq!(notification.level, NotificationLevel::Info);
    }

    #[test]
    fn test_notification_error() {
        let notification = Notification::error("Error message");
        assert_eq!(notification.message, "Error message");
        assert_eq!(notification.level, NotificationLevel::Error);
    }

    #[test]
    fn test_notification_not_immediately_expired() {
        let notification = Notification::info("Test");
        // Notification should not be expired immediately after creation
        assert!(!notification.is_expired());
    }

    #[test]
    fn test_notification_slide_offset_not_sliding_initially() {
        let notification = Notification::info("Test");
        // Should return 0 when not in the slide-out phase (more than 300ms remaining)
        let offset = notification.slide_offset(100);
        assert_eq!(offset, 0);
    }

    // ===================
    // NotificationLevel tests
    // ===================

    #[test]
    fn test_notification_level_equality() {
        assert_eq!(NotificationLevel::Info, NotificationLevel::Info);
        assert_eq!(NotificationLevel::Error, NotificationLevel::Error);
        assert_ne!(NotificationLevel::Info, NotificationLevel::Error);
    }

    // ===================
    // PendingOperation tests
    // ===================

    #[test]
    fn test_pending_operation_rebase() {
        let op = PendingOperation::Rebase;
        assert_eq!(op, PendingOperation::Rebase);
    }
}
