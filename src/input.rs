use crate::app::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum InputResult {
    Continue,
    Quit,
}

pub fn handle_key_event(app: &mut App, key: KeyEvent) -> InputResult {
    // Clear any previous message
    app.clear_message();

    match app.mode {
        AppMode::Normal => handle_normal_mode(app, key),
        AppMode::Create => handle_create_mode(app, key),
        AppMode::Confirm => handle_confirm_mode(app, key),
        AppMode::Help => handle_help_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) -> InputResult {
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => InputResult::Quit,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            // If searching, clear input; otherwise quit
            if !app.input.is_empty() {
                app.input.clear();
                app.filter_worktrees();
                InputResult::Continue
            } else {
                InputResult::Quit
            }
        }
        (KeyCode::Esc, _) => {
            // If searching, clear input; otherwise quit
            if !app.input.is_empty() {
                app.input.clear();
                app.filter_worktrees();
                InputResult::Continue
            } else {
                InputResult::Quit
            }
        }

        // Navigation
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            app.move_up();
            InputResult::Continue
        }
        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            app.move_down();
            InputResult::Continue
        }

        // Select worktree
        (KeyCode::Enter, _) => {
            app.select_worktree();
            if app.should_quit {
                InputResult::Quit
            } else {
                InputResult::Continue
            }
        }

        // Create mode
        (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
            if let Err(e) = app.enter_create_mode() {
                app.message = Some(format!("Error: {}", e));
            }
            InputResult::Continue
        }

        // Delete
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            app.enter_confirm_delete();
            InputResult::Continue
        }

        // Prune (Ctrl+Shift+D)
        (KeyCode::Char('D'), KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
            if let Err(e) = app.enter_confirm_prune() {
                app.message = Some(format!("Error: {}", e));
            }
            InputResult::Continue
        }

        // Help
        (KeyCode::Char('?'), _) => {
            app.enter_help_mode();
            InputResult::Continue
        }

        // Text input for search
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            app.input_char(c);
            InputResult::Continue
        }
        (KeyCode::Backspace, _) => {
            app.delete_char();
            InputResult::Continue
        }

        _ => InputResult::Continue,
    }
}

fn handle_create_mode(app: &mut App, key: KeyEvent) -> InputResult {
    match (key.code, key.modifiers) {
        // Cancel
        (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.enter_normal_mode();
            InputResult::Continue
        }

        // Navigation
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            app.move_up();
            InputResult::Continue
        }
        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            app.move_down();
            InputResult::Continue
        }

        // Create worktree
        (KeyCode::Enter, _) => {
            if let Err(e) = app.create_worktree() {
                app.message = Some(format!("Error: {}", e));
            }
            InputResult::Continue
        }

        // Text input
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            app.input_char(c);
            InputResult::Continue
        }
        (KeyCode::Backspace, _) => {
            app.delete_char();
            InputResult::Continue
        }

        _ => InputResult::Continue,
    }
}

fn handle_confirm_mode(app: &mut App, key: KeyEvent) -> InputResult {
    match key.code {
        // Confirm (worktree only)
        KeyCode::Enter | KeyCode::Char('y') => {
            if let Err(e) = app.confirm_action(false) {
                app.message = Some(format!("Error: {}", e));
            }
            InputResult::Continue
        }

        // Confirm (worktree and branch)
        KeyCode::Char('Y') => {
            if let Err(e) = app.confirm_action(true) {
                app.message = Some(format!("Error: {}", e));
            }
            InputResult::Continue
        }

        // Cancel
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.enter_normal_mode();
            InputResult::Continue
        }

        _ => InputResult::Continue,
    }
}

fn handle_help_mode(app: &mut App, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
            app.enter_normal_mode();
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::git::{Branch, Worktree};
    use std::path::PathBuf;

    /// Create a test App without Git dependencies
    fn create_test_app() -> App {
        App::new_for_test(
            Config::default(),
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
            ],
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
                    name: "origin/feature/b".to_string(),
                    is_remote: true,
                    is_head: false,
                },
            ],
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    // ========== Normal Mode Tests ==========

    #[test]
    fn test_normal_mode_move_up() {
        let mut app = create_test_app();
        app.selected_worktree = 2;

        let result = handle_key_event(&mut app, key(KeyCode::Up));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.selected_worktree, 1);
    }

    #[test]
    fn test_normal_mode_move_down() {
        let mut app = create_test_app();
        app.selected_worktree = 0;

        let result = handle_key_event(&mut app, key(KeyCode::Down));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.selected_worktree, 1);
    }

    #[test]
    fn test_normal_mode_quit_ctrl_q() {
        let mut app = create_test_app();

        let result = handle_key_event(&mut app, key_ctrl('q'));

        assert!(matches!(result, InputResult::Quit));
    }

    #[test]
    fn test_normal_mode_quit_esc() {
        let mut app = create_test_app();

        let result = handle_key_event(&mut app, key(KeyCode::Esc));

        assert!(matches!(result, InputResult::Quit));
    }

    #[test]
    fn test_normal_mode_esc_clears_input_first() {
        let mut app = create_test_app();
        app.input = "search".to_string();

        let result = handle_key_event(&mut app, key(KeyCode::Esc));

        assert!(matches!(result, InputResult::Continue));
        assert!(app.input.is_empty());
    }

    #[test]
    fn test_normal_mode_enter_help() {
        let mut app = create_test_app();

        let result = handle_key_event(&mut app, key(KeyCode::Char('?')));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Help);
    }

    #[test]
    fn test_normal_mode_input_char() {
        let mut app = create_test_app();

        handle_key_event(&mut app, key(KeyCode::Char('a')));

        assert_eq!(app.input, "a");
    }

    #[test]
    fn test_normal_mode_delete_char() {
        let mut app = create_test_app();
        app.input = "abc".to_string();

        handle_key_event(&mut app, key(KeyCode::Backspace));

        assert_eq!(app.input, "ab");
    }

    #[test]
    fn test_normal_mode_select_worktree() {
        let mut app = create_test_app();
        app.selected_worktree = 1;

        let result = handle_key_event(&mut app, key(KeyCode::Enter));

        assert!(matches!(result, InputResult::Quit));
        assert!(app.should_quit);
        assert_eq!(
            app.selected_worktree_path,
            Some("/repo/feature-a".to_string())
        );
    }

    // ========== Create Mode Tests ==========

    #[test]
    fn test_create_mode_move_up_down() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.selected_branch = 2;

        handle_key_event(&mut app, key(KeyCode::Up));
        assert_eq!(app.selected_branch, 1);

        handle_key_event(&mut app, key(KeyCode::Down));
        assert_eq!(app.selected_branch, 2);
    }

    #[test]
    fn test_create_mode_cancel_esc() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.input = "some input".to_string();

        let result = handle_key_event(&mut app, key(KeyCode::Esc));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.input.is_empty());
    }

    #[test]
    fn test_create_mode_cancel_ctrl_c() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;

        let result = handle_key_event(&mut app, key_ctrl('c'));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_create_mode_input_char() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;

        handle_key_event(&mut app, key(KeyCode::Char('t')));
        handle_key_event(&mut app, key(KeyCode::Char('e')));
        handle_key_event(&mut app, key(KeyCode::Char('s')));
        handle_key_event(&mut app, key(KeyCode::Char('t')));

        assert_eq!(app.input, "test");
    }

    #[test]
    fn test_create_mode_delete_char() {
        let mut app = create_test_app();
        app.mode = AppMode::Create;
        app.input = "test".to_string();

        handle_key_event(&mut app, key(KeyCode::Backspace));

        assert_eq!(app.input, "tes");
    }

    // ========== Confirm Mode Tests ==========

    #[test]
    fn test_confirm_mode_cancel_n() {
        let mut app = create_test_app();
        app.mode = AppMode::Confirm;

        let result = handle_key_event(&mut app, key(KeyCode::Char('n')));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_confirm_mode_cancel_esc() {
        let mut app = create_test_app();
        app.mode = AppMode::Confirm;

        let result = handle_key_event(&mut app, key(KeyCode::Esc));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Normal);
    }

    // ========== Help Mode Tests ==========

    #[test]
    fn test_help_mode_exit_esc() {
        let mut app = create_test_app();
        app.mode = AppMode::Help;

        let result = handle_key_event(&mut app, key(KeyCode::Esc));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_help_mode_exit_enter() {
        let mut app = create_test_app();
        app.mode = AppMode::Help;

        let result = handle_key_event(&mut app, key(KeyCode::Enter));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn test_help_mode_exit_q() {
        let mut app = create_test_app();
        app.mode = AppMode::Help;

        let result = handle_key_event(&mut app, key(KeyCode::Char('q')));

        assert!(matches!(result, InputResult::Continue));
        assert_eq!(app.mode, AppMode::Normal);
    }
}
