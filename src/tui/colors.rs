//! Color definitions using 256-color palette
//!
//! This module centralizes all colors used in the TUI.
//! All colors use the 256-color (indexed) mode for broad terminal compatibility.

use ratatui::style::Color;

/// Primary accent color (cyan-like)
pub const PRIMARY: Color = Color::Indexed(73); // Steel blue

/// Secondary/muted text color
pub const MUTED: Color = Color::Indexed(243); // Gray

/// Background for selected items
pub const SELECTION_BG: Color = Color::Indexed(236); // Dark gray

/// Highlight color (yellow-like)
pub const HIGHLIGHT: Color = Color::Indexed(179); // Light goldenrod

/// Error/danger color
pub const ERROR: Color = Color::Indexed(167); // Indian red

/// Success color
pub const SUCCESS: Color = Color::Indexed(108); // Dark sea green

/// Warning color
pub const WARNING: Color = Color::Indexed(179); // Light goldenrod

/// Header/footer background
pub const BAR_BG: Color = Color::Indexed(236); // Dark gray

/// Main tag indicator
pub const MAIN_TAG: Color = Color::Indexed(179); // Light goldenrod

/// Border color for dialogs
pub const BORDER: Color = Color::Indexed(243); // Gray

/// Border color for focused/active elements
pub const BORDER_FOCUS: Color = Color::Indexed(73); // Steel blue
