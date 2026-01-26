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
