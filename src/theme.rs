//! Color theme system for gwm
//!
//! Supports preset themes and customizable colors via configuration.
//!
//! # Preset Themes
//!
//! - `default`: 256-color/True Color theme with modern colors
//! - `classic`: 8-bit 16-color theme (original gwm colors)

use ratatui::style::Color;
use serde::Deserialize;
use std::str::FromStr;

/// All color definitions for the UI
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // UI elements
    pub header: Color,
    pub selected: Color,
    pub branch: Color,
    pub remote: Color,
    pub main_worktree: Color,
    pub key: Color,
    pub description: Color,

    // Semantic colors
    pub text: Color,
    pub text_muted: Color,
    pub separator: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self::classic()
    }
}

impl ThemeColors {
    /// Classic theme - original 8-bit 16-color scheme
    pub fn classic() -> Self {
        Self {
            header: Color::Cyan,
            selected: Color::Yellow,
            branch: Color::Green,
            remote: Color::Magenta,
            main_worktree: Color::Blue,
            key: Color::Yellow,
            description: Color::DarkGray,

            text: Color::White,
            text_muted: Color::DarkGray,
            separator: Color::DarkGray,
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
        }
    }

    /// Default theme - 256-color/True Color modern scheme
    pub fn default_theme() -> Self {
        Self {
            header: Color::Rgb(6, 182, 212),         // Cyan 400 (#06B6D4)
            selected: Color::Rgb(251, 191, 36),      // Amber 400 (#FBBF24)
            branch: Color::Rgb(34, 197, 94),         // Green 500 (#22C55E)
            remote: Color::Rgb(168, 85, 247),        // Purple 500 (#A855F7)
            main_worktree: Color::Rgb(59, 130, 246), // Blue 500 (#3B82F6)
            key: Color::Rgb(245, 158, 11),           // Amber 500 (#F59E0B)
            description: Color::Rgb(156, 163, 175),  // Gray 400 (#9CA3AF)

            text: Color::Rgb(243, 244, 246), // Gray 100 (#F3F4F6)
            text_muted: Color::Rgb(156, 163, 175), // Gray 400 (#9CA3AF)
            separator: Color::Rgb(107, 114, 128), // Gray 500 (#6B7280)
            success: Color::Rgb(34, 197, 94), // Green 500 (#22C55E)
            error: Color::Rgb(239, 68, 68),  // Red 500 (#EF4444)
            warning: Color::Rgb(245, 158, 11), // Amber 500 (#F59E0B)
        }
    }

    /// Create ThemeColors from a preset name
    pub fn from_preset(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "default" => Some(Self::default_theme()),
            "classic" => Some(Self::classic()),
            _ => None,
        }
    }
}

/// Theme configuration for deserialization
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeColorsConfig {
    pub header: Option<String>,
    pub selected: Option<String>,
    pub branch: Option<String>,
    pub remote: Option<String>,
    pub main_worktree: Option<String>,
    pub key: Option<String>,
    pub description: Option<String>,
    pub text: Option<String>,
    pub text_muted: Option<String>,
    pub separator: Option<String>,
    pub success: Option<String>,
    pub error: Option<String>,
    pub warning: Option<String>,
}

impl ThemeColorsConfig {
    /// Apply color overrides to a ThemeColors instance
    pub fn apply_to(&self, base: &mut ThemeColors) {
        if let Some(ref c) = self.header {
            if let Some(color) = parse_color(c) {
                base.header = color;
            }
        }
        if let Some(ref c) = self.selected {
            if let Some(color) = parse_color(c) {
                base.selected = color;
            }
        }
        if let Some(ref c) = self.branch {
            if let Some(color) = parse_color(c) {
                base.branch = color;
            }
        }
        if let Some(ref c) = self.remote {
            if let Some(color) = parse_color(c) {
                base.remote = color;
            }
        }
        if let Some(ref c) = self.main_worktree {
            if let Some(color) = parse_color(c) {
                base.main_worktree = color;
            }
        }
        if let Some(ref c) = self.key {
            if let Some(color) = parse_color(c) {
                base.key = color;
            }
        }
        if let Some(ref c) = self.description {
            if let Some(color) = parse_color(c) {
                base.description = color;
            }
        }
        if let Some(ref c) = self.text {
            if let Some(color) = parse_color(c) {
                base.text = color;
            }
        }
        if let Some(ref c) = self.text_muted {
            if let Some(color) = parse_color(c) {
                base.text_muted = color;
            }
        }
        if let Some(ref c) = self.separator {
            if let Some(color) = parse_color(c) {
                base.separator = color;
            }
        }
        if let Some(ref c) = self.success {
            if let Some(color) = parse_color(c) {
                base.success = color;
            }
        }
        if let Some(ref c) = self.error {
            if let Some(color) = parse_color(c) {
                base.error = color;
            }
        }
        if let Some(ref c) = self.warning {
            if let Some(color) = parse_color(c) {
                base.warning = color;
            }
        }
    }
}

/// Complete theme containing colors
#[derive(Debug, Clone)]
pub struct Theme {
    #[allow(dead_code)]
    pub name: String,
    pub colors: ThemeColors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            colors: ThemeColors::default_theme(),
        }
    }
}

impl Theme {
    /// Create a theme from preset name
    #[allow(dead_code)]
    pub fn from_preset(name: &str) -> Self {
        let colors = ThemeColors::from_preset(name).unwrap_or_else(ThemeColors::default_theme);
        Self {
            name: name.to_string(),
            colors,
        }
    }

    /// Create a theme from config settings
    pub fn from_config(
        theme_name: Option<&str>,
        colors_config: Option<&ThemeColorsConfig>,
    ) -> Self {
        let preset_name = theme_name.unwrap_or("default");
        let mut colors =
            ThemeColors::from_preset(preset_name).unwrap_or_else(ThemeColors::default_theme);

        // Apply color overrides if provided
        if let Some(config) = colors_config {
            config.apply_to(&mut colors);
        }

        Self {
            name: preset_name.to_string(),
            colors,
        }
    }

    /// Create classic theme
    #[allow(dead_code)]
    pub fn classic() -> Self {
        Self {
            name: "classic".to_string(),
            colors: ThemeColors::classic(),
        }
    }
}

/// Parse a color string into a ratatui Color
///
/// Supports:
/// - Hex colors: "#RRGGBB" or "#RGB"
/// - Named colors: "red", "green", "blue", etc.
/// - 256-color index: "0" to "255"
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();

    // Hex color
    if s.starts_with('#') {
        return parse_hex_color(s);
    }

    // 256-color index
    if let Ok(index) = u8::from_str(s) {
        return Some(Color::Indexed(index));
    }

    // Named color
    parse_named_color(s)
}

/// Parse a hex color string (#RRGGBB or #RGB)
fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.trim_start_matches('#');

    match hex.len() {
        // #RGB -> #RRGGBB
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        // #RRGGBB
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

/// Parse a named color
fn parse_named_color(s: &str) -> Option<Color> {
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "lightred" | "light_red" => Some(Color::LightRed),
        "lightgreen" | "light_green" => Some(Color::LightGreen),
        "lightyellow" | "light_yellow" => Some(Color::LightYellow),
        "lightblue" | "light_blue" => Some(Color::LightBlue),
        "lightmagenta" | "light_magenta" => Some(Color::LightMagenta),
        "lightcyan" | "light_cyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        "reset" | "default" => Some(Color::Reset),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Color Parsing Tests ==========

    #[test]
    fn test_parse_hex_color_6_digits() {
        let color = parse_color("#FF5733");
        assert_eq!(color, Some(Color::Rgb(255, 87, 51)));
    }

    #[test]
    fn test_parse_hex_color_3_digits() {
        let color = parse_color("#F53");
        assert_eq!(color, Some(Color::Rgb(255, 85, 51)));
    }

    #[test]
    fn test_parse_hex_color_lowercase() {
        let color = parse_color("#aabbcc");
        assert_eq!(color, Some(Color::Rgb(170, 187, 204)));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_color("#GG0000"), None);
        assert_eq!(parse_color("#FF"), None);
        assert_eq!(parse_color("#FFFFFFF"), None);
    }

    #[test]
    fn test_parse_named_color() {
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("GREEN"), Some(Color::Green));
        assert_eq!(parse_color("Blue"), Some(Color::Blue));
        assert_eq!(parse_color("darkgray"), Some(Color::DarkGray));
        assert_eq!(parse_color("dark_gray"), Some(Color::DarkGray));
    }

    #[test]
    fn test_parse_256_color_index() {
        assert_eq!(parse_color("0"), Some(Color::Indexed(0)));
        assert_eq!(parse_color("255"), Some(Color::Indexed(255)));
        assert_eq!(parse_color("34"), Some(Color::Indexed(34)));
    }

    #[test]
    fn test_parse_color_with_whitespace() {
        assert_eq!(parse_color("  red  "), Some(Color::Red));
        assert_eq!(parse_color("  #FF0000  "), Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn test_parse_invalid_color() {
        assert_eq!(parse_color("invalid"), None);
        assert_eq!(parse_color(""), None);
        assert_eq!(parse_color("256"), None); // 256 > u8::MAX
    }

    // ========== ThemeColors Tests ==========

    #[test]
    fn test_theme_colors_classic() {
        let colors = ThemeColors::classic();
        assert_eq!(colors.header, Color::Cyan);
        assert_eq!(colors.selected, Color::Yellow);
        assert_eq!(colors.branch, Color::Green);
        assert_eq!(colors.remote, Color::Magenta);
        assert_eq!(colors.main_worktree, Color::Blue);
    }

    #[test]
    fn test_theme_colors_default() {
        let colors = ThemeColors::default_theme();
        // Should use RGB colors
        assert!(matches!(colors.header, Color::Rgb(_, _, _)));
        assert!(matches!(colors.selected, Color::Rgb(_, _, _)));
        assert!(matches!(colors.branch, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_theme_colors_from_preset() {
        assert!(ThemeColors::from_preset("default").is_some());
        assert!(ThemeColors::from_preset("classic").is_some());
        assert!(ThemeColors::from_preset("DEFAULT").is_some());
        assert!(ThemeColors::from_preset("CLASSIC").is_some());
        assert!(ThemeColors::from_preset("nonexistent").is_none());
    }

    // ========== ThemeColorsConfig Tests ==========

    #[test]
    fn test_theme_colors_config_apply() {
        let config = ThemeColorsConfig {
            header: Some("#FF0000".to_string()),
            selected: Some("blue".to_string()),
            branch: Some("34".to_string()),
            ..Default::default()
        };

        let mut colors = ThemeColors::classic();
        config.apply_to(&mut colors);

        assert_eq!(colors.header, Color::Rgb(255, 0, 0));
        assert_eq!(colors.selected, Color::Blue);
        assert_eq!(colors.branch, Color::Indexed(34));
        // Unchanged colors should remain
        assert_eq!(colors.remote, Color::Magenta);
    }

    #[test]
    fn test_theme_colors_config_invalid_colors_ignored() {
        let config = ThemeColorsConfig {
            header: Some("invalid_color".to_string()),
            selected: Some("#GGG".to_string()),
            ..Default::default()
        };

        let mut colors = ThemeColors::classic();
        let original_header = colors.header;
        let original_selected = colors.selected;
        config.apply_to(&mut colors);

        // Invalid colors should be ignored
        assert_eq!(colors.header, original_header);
        assert_eq!(colors.selected, original_selected);
    }

    // ========== Theme Tests ==========

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme.name, "default");
        // Should use default_theme colors (RGB)
        assert!(matches!(theme.colors.header, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_theme_from_preset() {
        let theme = Theme::from_preset("classic");
        assert_eq!(theme.name, "classic");
        assert_eq!(theme.colors.header, Color::Cyan);

        let theme = Theme::from_preset("default");
        assert_eq!(theme.name, "default");
        assert!(matches!(theme.colors.header, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_theme_from_preset_unknown() {
        let theme = Theme::from_preset("unknown");
        // Should fall back to default theme
        assert_eq!(theme.name, "unknown");
        assert!(matches!(theme.colors.header, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_theme_from_config() {
        let colors_config = ThemeColorsConfig {
            header: Some("#00FF00".to_string()),
            ..Default::default()
        };

        let theme = Theme::from_config(Some("classic"), Some(&colors_config));
        assert_eq!(theme.name, "classic");
        // Header should be overridden
        assert_eq!(theme.colors.header, Color::Rgb(0, 255, 0));
        // Other colors should be from classic preset
        assert_eq!(theme.colors.selected, Color::Yellow);
    }

    #[test]
    fn test_theme_from_config_no_preset() {
        let theme = Theme::from_config(None, None);
        // Should default to "default" preset
        assert_eq!(theme.name, "default");
        assert!(matches!(theme.colors.header, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_theme_classic() {
        let theme = Theme::classic();
        assert_eq!(theme.name, "classic");
        assert_eq!(theme.colors.header, Color::Cyan);
    }
}
