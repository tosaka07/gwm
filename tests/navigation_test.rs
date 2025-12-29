//! Tests for navigation logic in App

mod common;

use common::TestRepo;
use gwm::app::{App, DialogKind, Mode};
use gwm::config::Config;

/// Create an App instance for testing with a temporary git repo
fn create_test_app() -> (App, TestRepo) {
    let repo = TestRepo::new();
    repo.create_branch("feature-1");
    repo.create_branch("feature-2");
    repo.create_branch("develop");

    // Change to repo directory and create App
    std::env::set_current_dir(repo.path()).unwrap();
    let config = Config::default();
    let app = App::new(config).expect("Failed to create App");

    (app, repo)
}

// ===================
// Basic navigation tests
// ===================

#[test]
fn test_app_initial_state() {
    let (app, _repo) = create_test_app();

    assert_eq!(app.mode, Mode::Normal);
    assert!(matches!(app.dialog, DialogKind::None));
    assert!(!app.should_quit);
    assert!(!app.worktrees.is_empty());
}

#[test]
fn test_app_has_branches() {
    let (app, _repo) = create_test_app();

    // Should have main (or master) plus the branches we created
    assert!(app.branches.len() >= 3);
    assert!(app.branches.iter().any(|b| b == "feature-1"));
    assert!(app.branches.iter().any(|b| b == "feature-2"));
    assert!(app.branches.iter().any(|b| b == "develop"));
}

#[test]
fn test_move_down_from_first() {
    let (mut app, _repo) = create_test_app();

    // This test verifies move_down behavior.
    // With only 1 worktree (main), moving down from 0 wraps to 0.
    // This is expected wrap-around behavior.
    app.list_state.select(Some(0));

    let initial_len = app.worktrees.len();
    app.move_down();

    if initial_len > 1 {
        assert_eq!(app.list_state.selected(), Some(1));
    } else {
        // With 1 item, wraps back to 0
        assert_eq!(app.list_state.selected(), Some(0));
    }
}

#[test]
fn test_move_up_from_second() {
    let (mut app, _repo) = create_test_app();

    // Only test if we have at least 2 worktrees
    if app.worktrees.len() > 1 {
        app.list_state.select(Some(1));
        app.move_up();
        assert_eq!(app.list_state.selected(), Some(0));
    } else {
        // With only 1 worktree, test that move_up from 0 wraps to 0
        app.list_state.select(Some(0));
        app.move_up();
        assert_eq!(app.list_state.selected(), Some(0));
    }
}

#[test]
fn test_move_down_wraps_at_end() {
    let (mut app, _repo) = create_test_app();

    let len = app.worktrees.len();
    app.list_state.select(Some(len - 1));

    app.move_down();

    // Should wrap to first item
    assert_eq!(app.list_state.selected(), Some(0));
}

#[test]
fn test_move_up_wraps_at_start() {
    let (mut app, _repo) = create_test_app();

    let len = app.worktrees.len();
    app.list_state.select(Some(0));

    app.move_up();

    // Should wrap to last item
    assert_eq!(app.list_state.selected(), Some(len - 1));
}

#[test]
fn test_move_top() {
    let (mut app, _repo) = create_test_app();

    app.list_state.select(Some(app.worktrees.len() - 1));

    app.move_top();

    assert_eq!(app.list_state.selected(), Some(0));
}

#[test]
fn test_move_bottom() {
    let (mut app, _repo) = create_test_app();

    app.list_state.select(Some(0));

    app.move_bottom();

    assert_eq!(app.list_state.selected(), Some(app.worktrees.len() - 1));
}

// ===================
// Navigation in BranchSelect dialog
// ===================

#[test]
fn test_move_down_in_branch_select() {
    let (mut app, _repo) = create_test_app();

    // Enter BranchSelect dialog
    app.dialog = DialogKind::BranchSelect {
        title: "Test".to_string(),
    };
    app.branch_list_state.select(Some(0));

    app.move_down();

    // Should move in branch list, not worktree list
    assert_eq!(app.branch_list_state.selected(), Some(1));
}

#[test]
fn test_move_up_in_branch_select() {
    let (mut app, _repo) = create_test_app();

    app.dialog = DialogKind::BranchSelect {
        title: "Test".to_string(),
    };
    app.branch_list_state.select(Some(1));

    app.move_up();

    assert_eq!(app.branch_list_state.selected(), Some(0));
}

#[test]
fn test_move_top_in_branch_select() {
    let (mut app, _repo) = create_test_app();

    app.dialog = DialogKind::BranchSelect {
        title: "Test".to_string(),
    };
    let len = app.branches.len();
    app.branch_list_state.select(Some(len - 1));

    app.move_top();

    assert_eq!(app.branch_list_state.selected(), Some(0));
}

#[test]
fn test_move_bottom_in_branch_select() {
    let (mut app, _repo) = create_test_app();

    app.dialog = DialogKind::BranchSelect {
        title: "Test".to_string(),
    };
    app.branch_list_state.select(Some(0));

    app.move_bottom();

    assert_eq!(
        app.branch_list_state.selected(),
        Some(app.branches.len() - 1)
    );
}

// ===================
// Selected item getters
// ===================

#[test]
fn test_selected_worktree() {
    let (mut app, _repo) = create_test_app();

    app.list_state.select(Some(0));
    let selected = app.selected_worktree();

    assert!(selected.is_some());
}

#[test]
fn test_selected_worktree_none_when_empty_selection() {
    let (mut app, _repo) = create_test_app();

    app.list_state.select(None);
    let selected = app.selected_worktree();

    assert!(selected.is_none());
}

#[test]
fn test_selected_branch() {
    let (mut app, _repo) = create_test_app();

    app.branch_list_state.select(Some(0));
    let selected = app.selected_branch();

    assert!(selected.is_some());
}

#[test]
fn test_selected_branch_none_when_empty_selection() {
    let (mut app, _repo) = create_test_app();

    app.branch_list_state.select(None);
    let selected = app.selected_branch();

    assert!(selected.is_none());
}

// ===================
// Dialog state tests
// ===================

#[test]
fn test_has_active_dialog_false_when_none() {
    let (app, _repo) = create_test_app();

    assert!(!app.has_active_dialog());
}

#[test]
fn test_has_active_dialog_true_when_branch_select() {
    let (mut app, _repo) = create_test_app();

    app.dialog = DialogKind::BranchSelect {
        title: "Test".to_string(),
    };

    assert!(app.has_active_dialog());
}

#[test]
fn test_has_active_dialog_true_when_confirm_delete() {
    let (mut app, _repo) = create_test_app();

    app.dialog = DialogKind::ConfirmDelete {
        worktree_path: "/test".to_string(),
    };

    assert!(app.has_active_dialog());
}

// ===================
// Notification tests
// ===================

#[test]
fn test_show_error_adds_notification() {
    let (mut app, _repo) = create_test_app();

    assert!(app.notifications.is_empty());

    app.show_error("Test error");

    assert_eq!(app.notifications.len(), 1);
    assert_eq!(app.notifications[0].message, "Test error");
}

#[test]
fn test_notify_adds_notification() {
    use gwm::app::Notification;

    let (mut app, _repo) = create_test_app();

    app.notify(Notification::info("Test info"));

    assert_eq!(app.notifications.len(), 1);
    assert_eq!(app.notifications[0].message, "Test info");
}

#[test]
fn test_multiple_notifications_stack() {
    let (mut app, _repo) = create_test_app();

    app.show_error("Error 1");
    app.show_error("Error 2");
    app.show_error("Error 3");

    assert_eq!(app.notifications.len(), 3);
}
