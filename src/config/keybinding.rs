use crossterm::event::{KeyCode, KeyModifiers};

use crate::error::{Error, Result};

/// Parse a key string into KeyCode
pub fn parse_key(key: &str) -> Result<KeyCode> {
    let key = key.trim();

    // Single character
    if key.len() == 1 {
        let c = key.chars().next().unwrap();
        return Ok(KeyCode::Char(c));
    }

    // Named keys
    match key.to_lowercase().as_str() {
        "enter" | "return" => Ok(KeyCode::Enter),
        "escape" | "esc" => Ok(KeyCode::Esc),
        "backspace" | "back" => Ok(KeyCode::Backspace),
        "tab" => Ok(KeyCode::Tab),
        "space" => Ok(KeyCode::Char(' ')),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" => Ok(KeyCode::PageUp),
        "pagedown" => Ok(KeyCode::PageDown),
        "insert" => Ok(KeyCode::Insert),
        "delete" | "del" => Ok(KeyCode::Delete),
        "f1" => Ok(KeyCode::F(1)),
        "f2" => Ok(KeyCode::F(2)),
        "f3" => Ok(KeyCode::F(3)),
        "f4" => Ok(KeyCode::F(4)),
        "f5" => Ok(KeyCode::F(5)),
        "f6" => Ok(KeyCode::F(6)),
        "f7" => Ok(KeyCode::F(7)),
        "f8" => Ok(KeyCode::F(8)),
        "f9" => Ok(KeyCode::F(9)),
        "f10" => Ok(KeyCode::F(10)),
        "f11" => Ok(KeyCode::F(11)),
        "f12" => Ok(KeyCode::F(12)),
        // Numpad keys
        "numpadenter" => Ok(KeyCode::Enter),
        "numpadadd" => Ok(KeyCode::Char('+')),
        "numpadsubtract" => Ok(KeyCode::Char('-')),
        "numpadmultiply" => Ok(KeyCode::Char('*')),
        "numpaddivide" => Ok(KeyCode::Char('/')),
        "numpad0" => Ok(KeyCode::Char('0')),
        "numpad1" => Ok(KeyCode::Char('1')),
        "numpad2" => Ok(KeyCode::Char('2')),
        "numpad3" => Ok(KeyCode::Char('3')),
        "numpad4" => Ok(KeyCode::Char('4')),
        "numpad5" => Ok(KeyCode::Char('5')),
        "numpad6" => Ok(KeyCode::Char('6')),
        "numpad7" => Ok(KeyCode::Char('7')),
        "numpad8" => Ok(KeyCode::Char('8')),
        "numpad9" => Ok(KeyCode::Char('9')),
        _ => Err(Error::InvalidKeyBinding(format!("Unknown key: {}", key))),
    }
}

/// Parse modifier string into KeyModifiers
pub fn parse_modifiers(mods: Option<&str>) -> KeyModifiers {
    let Some(mods) = mods else {
        return KeyModifiers::NONE;
    };

    let mut result = KeyModifiers::NONE;

    for part in mods.split('|') {
        let part = part.trim().to_lowercase();
        match part.as_str() {
            "control" | "ctrl" => result |= KeyModifiers::CONTROL,
            "shift" => result |= KeyModifiers::SHIFT,
            "alt" | "option" => result |= KeyModifiers::ALT,
            "super" | "command" | "cmd" => result |= KeyModifiers::SUPER,
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_char() {
        assert_eq!(parse_key("j").unwrap(), KeyCode::Char('j'));
        assert_eq!(parse_key("k").unwrap(), KeyCode::Char('k'));
        assert_eq!(parse_key("G").unwrap(), KeyCode::Char('G'));
    }

    #[test]
    fn test_parse_named_keys() {
        assert_eq!(parse_key("Enter").unwrap(), KeyCode::Enter);
        assert_eq!(parse_key("Escape").unwrap(), KeyCode::Esc);
        assert_eq!(parse_key("esc").unwrap(), KeyCode::Esc);
        assert_eq!(parse_key("Up").unwrap(), KeyCode::Up);
        assert_eq!(parse_key("F1").unwrap(), KeyCode::F(1));
    }

    #[test]
    fn test_parse_modifiers() {
        assert_eq!(parse_modifiers(None), KeyModifiers::NONE);
        assert_eq!(parse_modifiers(Some("Control")), KeyModifiers::CONTROL);
        assert_eq!(parse_modifiers(Some("Shift")), KeyModifiers::SHIFT);
        assert_eq!(
            parse_modifiers(Some("Control|Shift")),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        );
    }
}
