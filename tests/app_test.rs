//! Tests for app state management

mod common;

use gwm::app::{
    CreateWorktreeForm, DialogKind, Mode, Notification, NotificationLevel, PendingOperation,
};

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
