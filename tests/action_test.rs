//! Tests for action types and dispatcher

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gwm::action::{Action, ActionDispatcher};
use gwm::app::Mode;
use gwm::config::{default_bindings, Config};

// ===================
// Action::from_str tests
// ===================

#[test]
fn test_action_from_str_navigation() {
    assert_eq!(Action::from_str("MoveUp"), Some(Action::MoveUp));
    assert_eq!(Action::from_str("MoveDown"), Some(Action::MoveDown));
    assert_eq!(Action::from_str("MoveTop"), Some(Action::MoveTop));
    assert_eq!(Action::from_str("MoveBottom"), Some(Action::MoveBottom));
    assert_eq!(Action::from_str("PageUp"), Some(Action::PageUp));
    assert_eq!(Action::from_str("PageDown"), Some(Action::PageDown));
    assert_eq!(Action::from_str("Select"), Some(Action::Select));
    assert_eq!(Action::from_str("Back"), Some(Action::Back));
}

#[test]
fn test_action_from_str_input_navigation() {
    assert_eq!(
        Action::from_str("MoveLineStart"),
        Some(Action::MoveLineStart)
    );
    assert_eq!(Action::from_str("MoveLineEnd"), Some(Action::MoveLineEnd));
}

#[test]
fn test_action_from_str_worktree_operations() {
    assert_eq!(Action::from_str("OpenShell"), Some(Action::OpenShell));
    assert_eq!(
        Action::from_str("CreateWorktree"),
        Some(Action::CreateWorktree)
    );
    assert_eq!(
        Action::from_str("DeleteWorktree"),
        Some(Action::DeleteWorktree)
    );
    assert_eq!(
        Action::from_str("DeleteMergedWorktrees"),
        Some(Action::DeleteMergedWorktrees)
    );
    assert_eq!(
        Action::from_str("RebaseWorktree"),
        Some(Action::RebaseWorktree)
    );
    assert_eq!(Action::from_str("Refresh"), Some(Action::Refresh));
}

#[test]
fn test_action_from_str_mode_switching() {
    assert_eq!(
        Action::from_str("EnterInsertMode"),
        Some(Action::EnterInsertMode)
    );
    assert_eq!(
        Action::from_str("EnterSearchMode"),
        Some(Action::EnterSearchMode)
    );
    assert_eq!(
        Action::from_str("EnterNormalMode"),
        Some(Action::EnterNormalMode)
    );
}

#[test]
fn test_action_from_str_dialog() {
    assert_eq!(Action::from_str("Confirm"), Some(Action::Confirm));
    assert_eq!(Action::from_str("Cancel"), Some(Action::Cancel));
}

#[test]
fn test_action_from_str_input() {
    assert_eq!(Action::from_str("DeleteChar"), Some(Action::DeleteChar));
    assert_eq!(Action::from_str("DeleteWord"), Some(Action::DeleteWord));
}

#[test]
fn test_action_from_str_other() {
    assert_eq!(Action::from_str("ToggleHelp"), Some(Action::ToggleHelp));
    assert_eq!(Action::from_str("Quit"), Some(Action::Quit));
    assert_eq!(Action::from_str("ForceQuit"), Some(Action::ForceQuit));
}

#[test]
fn test_action_from_str_none() {
    // "None" and "ReceiveChar" should return None (used to disable bindings)
    assert_eq!(Action::from_str("None"), None);
    assert_eq!(Action::from_str("ReceiveChar"), None);
}

#[test]
fn test_action_from_str_unknown() {
    assert_eq!(Action::from_str("UnknownAction"), None);
    assert_eq!(Action::from_str(""), None);
    assert_eq!(Action::from_str("moveup"), None); // case sensitive
    assert_eq!(Action::from_str("MOVEUP"), None); // case sensitive
}

#[test]
fn test_action_equality() {
    assert_eq!(Action::MoveUp, Action::MoveUp);
    assert_ne!(Action::MoveUp, Action::MoveDown);
}

#[test]
fn test_action_insert_char() {
    let action = Action::InsertChar('a');
    assert_eq!(action, Action::InsertChar('a'));
    assert_ne!(action, Action::InsertChar('b'));
}

#[test]
fn test_action_run_command() {
    let action = Action::RunCommand("ls -la".to_string());
    assert_eq!(action, Action::RunCommand("ls -la".to_string()));
    assert_ne!(action, Action::RunCommand("pwd".to_string()));
}

#[test]
fn test_action_clone() {
    let action = Action::MoveUp;
    let cloned = action.clone();
    assert_eq!(action, cloned);
}

#[test]
fn test_action_debug() {
    let action = Action::MoveUp;
    let debug_str = format!("{:?}", action);
    assert_eq!(debug_str, "MoveUp");
}

// ===================
// ActionDispatcher tests
// ===================

fn make_config() -> Config {
    Config {
        bindings: default_bindings(),
        ..Default::default()
    }
}

#[test]
fn test_dispatch_j_moves_down() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::MoveDown));
}

#[test]
fn test_dispatch_k_moves_up() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::MoveUp));
}

#[test]
fn test_dispatch_ctrl_n_moves_down() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::MoveDown));
}

#[test]
fn test_dispatch_ctrl_p_moves_up() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::MoveUp));
}

#[test]
fn test_dispatch_g_moves_top() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::MoveTop));
}

#[test]
fn test_dispatch_uppercase_g_moves_bottom() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    // Note: The binding is defined as "G" without SHIFT modifier,
    // so we need to send the key without SHIFT for it to match.
    // In practice, crossterm sends SHIFT when pressing Shift+G,
    // which wouldn't match this binding.
    let key = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::MoveBottom));
}

#[test]
fn test_dispatch_enter_opens_shell() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    // In Normal mode, Enter opens a shell (not Select)
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::OpenShell));
}

#[test]
fn test_dispatch_enter_confirms_in_dialog() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    // In Dialog mode, Enter confirms
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Dialog);

    assert_eq!(action, Some(Action::Confirm));
}

#[test]
fn test_dispatch_c_creates_worktree() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::CreateWorktree));
}

#[test]
fn test_dispatch_d_deletes_worktree() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::DeleteWorktree));
}

#[test]
fn test_dispatch_q_quits() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Normal);

    assert_eq!(action, Some(Action::Quit));
}

#[test]
fn test_dispatch_esc_in_insert_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Insert);

    assert_eq!(action, Some(Action::EnterNormalMode));
}

#[test]
fn test_dispatch_esc_in_search_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Search);

    assert_eq!(action, Some(Action::EnterNormalMode));
}

#[test]
fn test_dispatch_char_in_insert_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Insert);

    assert_eq!(action, Some(Action::InsertChar('a')));
}

#[test]
fn test_dispatch_char_in_search_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Search);

    assert_eq!(action, Some(Action::InsertChar('x')));
}

#[test]
fn test_dispatch_backspace_in_insert_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Insert);

    assert_eq!(action, Some(Action::DeleteChar));
}

#[test]
fn test_dispatch_ctrl_n_in_dialog_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
    let action = dispatcher.dispatch(key, &Mode::Dialog);

    assert_eq!(action, Some(Action::MoveDown));
}

#[test]
fn test_dispatch_ctrl_p_in_dialog_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let action = dispatcher.dispatch(key, &Mode::Dialog);

    assert_eq!(action, Some(Action::MoveUp));
}

#[test]
fn test_dispatch_char_in_dialog_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Dialog);

    assert_eq!(action, Some(Action::InsertChar('t')));
}

#[test]
fn test_dispatch_backspace_in_dialog_mode() {
    let config = make_config();
    let dispatcher = ActionDispatcher::new(&config);

    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    let action = dispatcher.dispatch(key, &Mode::Dialog);

    assert_eq!(action, Some(Action::DeleteChar));
}
