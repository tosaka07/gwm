use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Mode;
use crate::config::{parse_key, parse_modifiers, Config, KeyBinding};

use super::Action;

/// Dispatches key events to actions based on configuration
pub struct ActionDispatcher {
    bindings: Vec<KeyBinding>,
}

impl ActionDispatcher {
    /// Create a new dispatcher from configuration
    pub fn new(config: &Config) -> Self {
        Self {
            bindings: config.bindings.clone(),
        }
    }

    /// Dispatch a key event to an action
    pub fn dispatch(&self, key: KeyEvent, mode: &Mode) -> Option<Action> {
        // First, check configured bindings
        for binding in &self.bindings {
            if self.matches(binding, &key, mode) {
                return self.to_action(binding);
            }
        }

        // Handle character input in Insert/Search mode
        match mode {
            Mode::Insert | Mode::Search => {
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                    if let KeyCode::Char(c) = key.code {
                        return Some(Action::InsertChar(c));
                    }
                }
                // Backspace in input modes
                if key.code == KeyCode::Backspace {
                    return Some(Action::DeleteChar);
                }
            }
            Mode::Dialog => {
                // Ctrl+n/p for branch navigation in CreateWorktree form
                if key.modifiers == KeyModifiers::CONTROL {
                    match key.code {
                        KeyCode::Char('n') => return Some(Action::MoveDown),
                        KeyCode::Char('p') => return Some(Action::MoveUp),
                        _ => {}
                    }
                }
                // Character input for form fields (no modifiers or SHIFT only)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                    if let KeyCode::Char(c) = key.code {
                        return Some(Action::InsertChar(c));
                    }
                }
                // Backspace in dialog mode
                if key.code == KeyCode::Backspace {
                    return Some(Action::DeleteChar);
                }
            }
            _ => {}
        }

        None
    }

    /// Check if a binding matches the key event and mode
    fn matches(&self, binding: &KeyBinding, key: &KeyEvent, mode: &Mode) -> bool {
        // Parse the binding key
        let Ok(binding_key) = parse_key(&binding.key) else {
            return false;
        };

        // Check key code
        if binding_key != key.code {
            return false;
        }

        // Check modifiers
        let binding_mods = parse_modifiers(binding.mods.as_deref());
        if binding_mods != key.modifiers {
            return false;
        }

        // Check mode restriction
        if let Some(mode_str) = &binding.mode {
            if !self.mode_matches(mode_str, mode) {
                return false;
            }
        }

        true
    }

    /// Check if mode restriction matches current mode
    fn mode_matches(&self, mode_str: &str, current_mode: &Mode) -> bool {
        let mode_str = mode_str.trim();

        // Handle negation (~)
        if let Some(stripped) = mode_str.strip_prefix('~') {
            return !self.mode_name_matches(stripped, current_mode);
        }

        // Handle multiple modes (|)
        if mode_str.contains('|') {
            return mode_str
                .split('|')
                .any(|m| self.mode_matches(m.trim(), current_mode));
        }

        self.mode_name_matches(mode_str, current_mode)
    }

    /// Check if a mode name matches
    fn mode_name_matches(&self, name: &str, current_mode: &Mode) -> bool {
        match name.to_lowercase().as_str() {
            "normal" => matches!(current_mode, Mode::Normal),
            "insert" => matches!(current_mode, Mode::Insert),
            "search" => matches!(current_mode, Mode::Search),
            "dialog" => matches!(current_mode, Mode::Dialog),
            _ => false,
        }
    }

    /// Convert a binding to an action
    fn to_action(&self, binding: &KeyBinding) -> Option<Action> {
        // Check for action
        if let Some(action_str) = &binding.action {
            return Action::from_str(action_str);
        }

        // Check for command
        if let Some(command) = &binding.command {
            return Some(Action::RunCommand(command.clone()));
        }

        // Check for chars (in insert mode, this is handled differently)
        if let Some(chars) = &binding.chars {
            // For now, just insert the first character
            if let Some(c) = chars.chars().next() {
                return Some(Action::InsertChar(c));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_bindings;

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
    fn test_dispatch_ctrl_n_moves_down() {
        let config = make_config();
        let dispatcher = ActionDispatcher::new(&config);

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        let action = dispatcher.dispatch(key, &Mode::Normal);

        assert_eq!(action, Some(Action::MoveDown));
    }

    #[test]
    fn test_mode_restriction() {
        let config = make_config();
        let dispatcher = ActionDispatcher::new(&config);

        // Escape in Normal mode should not match Insert-mode binding
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

        // In Insert mode, Esc should return to Normal
        let action = dispatcher.dispatch(key, &Mode::Insert);
        assert_eq!(action, Some(Action::EnterNormalMode));
    }
}
