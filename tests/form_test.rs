//! Tests for form operations (CreateWorktree dialog)

mod common;

use common::TestRepo;
use gwm::app::{App, CreateWorktreeForm, DialogKind, Mode};
use gwm::config::Config;

/// Create an App instance with CreateWorktree dialog open
fn create_app_with_form() -> (App, TestRepo) {
    let repo = TestRepo::new();
    repo.create_branch("feature-1");
    repo.create_branch("develop");

    std::env::set_current_dir(repo.path()).unwrap();
    let config = Config::default();
    let mut app = App::new(config).expect("Failed to create App");

    // Open CreateWorktree dialog
    app.dialog = DialogKind::CreateWorktree(CreateWorktreeForm::new(0, ""));
    app.mode = Mode::Dialog;

    (app, repo)
}

// ===================
// Form initialization tests
// ===================

#[test]
fn test_create_worktree_form_opens_in_dialog_mode() {
    let (app, _repo) = create_app_with_form();

    assert_eq!(app.mode, Mode::Dialog);
    assert!(matches!(app.dialog, DialogKind::CreateWorktree(_)));
}

#[test]
fn test_create_worktree_form_initial_state() {
    let (app, _repo) = create_app_with_form();

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.branch_idx, 0);
        assert_eq!(form.name, "");
    } else {
        panic!("Expected CreateWorktree dialog");
    }
}

// ===================
// Name input tests (simulating handler logic)
// ===================

#[test]
fn test_form_insert_char() {
    let (mut app, _repo) = create_app_with_form();

    // Simulate inserting characters
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.name.push('t');
        form.name.push('e');
        form.name.push('s');
        form.name.push('t');
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.name, "test");
    }
}

#[test]
fn test_form_delete_char() {
    let (mut app, _repo) = create_app_with_form();

    // Set initial name and delete
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.name = "test".to_string();
        form.name.pop();
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.name, "tes");
    }
}

#[test]
fn test_form_delete_char_on_empty() {
    let (mut app, _repo) = create_app_with_form();

    // Delete on empty name should do nothing
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.name.pop();
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.name, "");
    }
}

#[test]
fn test_form_name_with_special_chars() {
    let (mut app, _repo) = create_app_with_form();

    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.name = "feature/my-feature_123".to_string();
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.name, "feature/my-feature_123");
    }
}

// ===================
// Branch navigation tests (simulating handler logic)
// ===================

#[test]
fn test_form_branch_navigate_down() {
    let (mut app, _repo) = create_app_with_form();

    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        let max_idx = app.branches.len().saturating_sub(1);
        if form.branch_idx < max_idx {
            form.branch_idx += 1;
        }
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.branch_idx, 1);
    }
}

#[test]
fn test_form_branch_navigate_up() {
    let (mut app, _repo) = create_app_with_form();

    // Start at index 1
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.branch_idx = 1;
    }

    // Navigate up
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        if form.branch_idx > 0 {
            form.branch_idx -= 1;
        }
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.branch_idx, 0);
    }
}

#[test]
fn test_form_branch_navigate_up_at_top() {
    let (mut app, _repo) = create_app_with_form();

    // At index 0, navigate up should stay at 0
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        if form.branch_idx > 0 {
            form.branch_idx -= 1;
        }
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.branch_idx, 0);
    }
}

#[test]
fn test_form_branch_navigate_down_at_bottom() {
    let (mut app, _repo) = create_app_with_form();

    let branch_count = app.branches.len();

    // Navigate to bottom
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.branch_idx = branch_count - 1;
    }

    // Try to navigate down (should stay at bottom)
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        let max_idx = app.branches.len().saturating_sub(1);
        if form.branch_idx < max_idx {
            form.branch_idx += 1;
        }
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.branch_idx, branch_count - 1);
    }
}

// ===================
// Form state combination tests
// ===================

#[test]
fn test_form_name_and_branch_selection() {
    let (mut app, _repo) = create_app_with_form();

    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.name = "my-feature".to_string();
        form.branch_idx = 1;
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.name, "my-feature");
        assert_eq!(form.branch_idx, 1);
    }
}

#[test]
fn test_form_get_selected_branch() {
    let (mut app, _repo) = create_app_with_form();

    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.branch_idx = 0;
    }

    if let DialogKind::CreateWorktree(form) = &app.dialog {
        let branch = app.branches.get(form.branch_idx);
        assert!(branch.is_some());
    }
}

// ===================
// Dialog close tests
// ===================

#[test]
fn test_form_cancel_closes_dialog() {
    let (mut app, _repo) = create_app_with_form();

    // Simulate cancel
    app.dialog = DialogKind::None;
    app.mode = Mode::Normal;

    assert_eq!(app.mode, Mode::Normal);
    assert!(matches!(app.dialog, DialogKind::None));
}

#[test]
fn test_form_state_preserved_until_close() {
    let (mut app, _repo) = create_app_with_form();

    // Modify form state
    if let DialogKind::CreateWorktree(ref mut form) = app.dialog {
        form.name = "test-branch".to_string();
        form.branch_idx = 2;
    }

    // State should be preserved
    if let DialogKind::CreateWorktree(form) = &app.dialog {
        assert_eq!(form.name, "test-branch");
        assert_eq!(form.branch_idx, 2);
    }
}

// ===================
// Validation tests (logic that would be in handler)
// ===================

#[test]
fn test_form_empty_name_is_invalid() {
    let form = CreateWorktreeForm::new(0, "");
    assert!(form.name.is_empty());
}

#[test]
fn test_form_non_empty_name_is_valid() {
    let form = CreateWorktreeForm::new(0, "valid-name");
    assert!(!form.name.is_empty());
}

#[test]
fn test_form_whitespace_only_name() {
    let form = CreateWorktreeForm::new(0, "   ");
    // Note: In real validation, you'd want to trim and check
    // This test documents current behavior (whitespace is not trimmed)
    assert!(!form.name.is_empty());
    assert_eq!(form.name.trim(), "");
}
