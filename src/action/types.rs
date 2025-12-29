/// Actions that can be performed in the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    PageUp,
    PageDown,
    Select,
    Back,

    // Input navigation
    MoveLineStart,
    MoveLineEnd,

    // Worktree operations
    OpenShell,
    CreateWorktree,
    DeleteWorktree,
    DeleteMergedWorktrees,
    RebaseWorktree,
    Refresh,

    // Mode switching
    EnterInsertMode,
    EnterSearchMode,
    EnterNormalMode,

    // Dialog
    Confirm,
    Cancel,

    // Input
    InsertChar(char),
    DeleteChar,
    DeleteWord,

    // Other
    ToggleHelp,
    Quit,
    ForceQuit,

    // Custom command
    RunCommand(String),
}

impl Action {
    /// Parse action from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "MoveUp" => Some(Action::MoveUp),
            "MoveDown" => Some(Action::MoveDown),
            "MoveTop" => Some(Action::MoveTop),
            "MoveBottom" => Some(Action::MoveBottom),
            "PageUp" => Some(Action::PageUp),
            "PageDown" => Some(Action::PageDown),
            "Select" => Some(Action::Select),
            "Back" => Some(Action::Back),
            "MoveLineStart" => Some(Action::MoveLineStart),
            "MoveLineEnd" => Some(Action::MoveLineEnd),
            "OpenShell" => Some(Action::OpenShell),
            "CreateWorktree" => Some(Action::CreateWorktree),
            "DeleteWorktree" => Some(Action::DeleteWorktree),
            "DeleteMergedWorktrees" => Some(Action::DeleteMergedWorktrees),
            "RebaseWorktree" => Some(Action::RebaseWorktree),
            "Refresh" => Some(Action::Refresh),
            "EnterInsertMode" => Some(Action::EnterInsertMode),
            "EnterSearchMode" => Some(Action::EnterSearchMode),
            "EnterNormalMode" => Some(Action::EnterNormalMode),
            "Confirm" => Some(Action::Confirm),
            "Cancel" => Some(Action::Cancel),
            "DeleteChar" => Some(Action::DeleteChar),
            "DeleteWord" => Some(Action::DeleteWord),
            "ToggleHelp" => Some(Action::ToggleHelp),
            "Quit" => Some(Action::Quit),
            "ForceQuit" => Some(Action::ForceQuit),
            "None" | "ReceiveChar" => None, // Special actions to disable bindings
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===================
    // Navigation actions
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
        assert_eq!(Action::from_str("MoveLineStart"), Some(Action::MoveLineStart));
        assert_eq!(Action::from_str("MoveLineEnd"), Some(Action::MoveLineEnd));
    }

    // ===================
    // Worktree operations
    // ===================

    #[test]
    fn test_action_from_str_worktree_operations() {
        assert_eq!(Action::from_str("OpenShell"), Some(Action::OpenShell));
        assert_eq!(Action::from_str("CreateWorktree"), Some(Action::CreateWorktree));
        assert_eq!(Action::from_str("DeleteWorktree"), Some(Action::DeleteWorktree));
        assert_eq!(Action::from_str("DeleteMergedWorktrees"), Some(Action::DeleteMergedWorktrees));
        assert_eq!(Action::from_str("RebaseWorktree"), Some(Action::RebaseWorktree));
        assert_eq!(Action::from_str("Refresh"), Some(Action::Refresh));
    }

    // ===================
    // Mode switching
    // ===================

    #[test]
    fn test_action_from_str_mode_switching() {
        assert_eq!(Action::from_str("EnterInsertMode"), Some(Action::EnterInsertMode));
        assert_eq!(Action::from_str("EnterSearchMode"), Some(Action::EnterSearchMode));
        assert_eq!(Action::from_str("EnterNormalMode"), Some(Action::EnterNormalMode));
    }

    // ===================
    // Dialog actions
    // ===================

    #[test]
    fn test_action_from_str_dialog() {
        assert_eq!(Action::from_str("Confirm"), Some(Action::Confirm));
        assert_eq!(Action::from_str("Cancel"), Some(Action::Cancel));
    }

    // ===================
    // Input actions
    // ===================

    #[test]
    fn test_action_from_str_input() {
        assert_eq!(Action::from_str("DeleteChar"), Some(Action::DeleteChar));
        assert_eq!(Action::from_str("DeleteWord"), Some(Action::DeleteWord));
    }

    // ===================
    // Other actions
    // ===================

    #[test]
    fn test_action_from_str_other() {
        assert_eq!(Action::from_str("ToggleHelp"), Some(Action::ToggleHelp));
        assert_eq!(Action::from_str("Quit"), Some(Action::Quit));
        assert_eq!(Action::from_str("ForceQuit"), Some(Action::ForceQuit));
    }

    // ===================
    // Special cases
    // ===================

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

    // ===================
    // Action equality
    // ===================

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

    // ===================
    // Clone and Debug
    // ===================

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
}
