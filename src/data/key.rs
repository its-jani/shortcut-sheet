//! Key and combo model. Deliberately free of any Win32 type so the parser and the
//! rendering rules can be unit-tested without a Windows API surface.

use std::fmt;

/// A physical key, named the way a user writes it down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Ctrl,
    Shift,
    Alt,
    /// The Windows key.
    Win,
    Char(char),
    Digit(u8),
    F(u8),
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Minus,
    Equals,
    Comma,
    Period,
    Semicolon,
    Quote,
    Slash,
    Backslash,
    BracketOpen,
    BracketClose,
    Grave,
}

impl Key {
    /// Windows virtual-key code. Kept as a plain number so this module stays
    /// independent of the `windows` crate; `platform::win32` converts it at the boundary.
    pub fn vk(&self) -> u16 {
        use Key::*;
        match self {
            // Modifiers are injected as the generic codes; Windows delivers the correct
            // left/right variant from the scan code we send alongside.
            Ctrl => 0x11,
            Shift => 0x10,
            Alt => 0x12,
            Win => 0x5B,
            Char(c) => match c {
                'a'..='z' => (*c as u16) - 32,
                'A'..='Z' => *c as u16,
                _ => 0,
            },
            Digit(d) => 0x30 + u16::from(*d),
            F(n) => 0x6F + u16::from(*n),
            Enter => 0x0D,
            Escape => 0x1B,
            Tab => 0x09,
            Space => 0x20,
            Backspace => 0x08,
            Delete => 0x2E,
            Insert => 0x2D,
            Home => 0x24,
            End => 0x23,
            PageUp => 0x21,
            PageDown => 0x22,
            Up => 0x26,
            Down => 0x28,
            Left => 0x25,
            Right => 0x27,
            Minus => 0xBD,
            Equals => 0xBB,
            Comma => 0xBC,
            Period => 0xBE,
            Semicolon => 0xBA,
            Quote => 0xDE,
            Slash => 0xBF,
            Backslash => 0xDC,
            BracketOpen => 0xDB,
            BracketClose => 0xDD,
            Grave => 0xC0,
        }
    }

    pub fn is_modifier(&self) -> bool {
        matches!(self, Key::Ctrl | Key::Shift | Key::Alt | Key::Win)
    }

    /// The label shown inside a keycap. Our own rule, applied consistently: every
    /// modifier is a word. (The original mixes the word `CTRL` with a Shift glyph.)
    pub fn label(&self) -> String {
        use Key::*;
        match self {
            Ctrl => "Ctrl".into(),
            Shift => "Shift".into(),
            Alt => "Alt".into(),
            Win => "Win".into(),
            Char(c) => c.to_uppercase().to_string(),
            Digit(d) => d.to_string(),
            F(n) => format!("F{n}"),
            Enter => "Enter".into(),
            Escape => "Esc".into(),
            Tab => "Tab".into(),
            Space => "Space".into(),
            Backspace => "Bksp".into(),
            Delete => "Del".into(),
            Insert => "Ins".into(),
            Home => "Home".into(),
            End => "End".into(),
            PageUp => "PgUp".into(),
            PageDown => "PgDn".into(),
            Up => "\u{2191}".into(),
            Down => "\u{2193}".into(),
            Left => "\u{2190}".into(),
            Right => "\u{2192}".into(),
            Minus => "-".into(),
            Equals => "=".into(),
            Comma => ",".into(),
            Period => ".".into(),
            Semicolon => ";".into(),
            Quote => "'".into(),
            Slash => "/".into(),
            Backslash => "\\".into(),
            BracketOpen => "[".into(),
            BracketClose => "]".into(),
            Grave => "`".into(),
        }
    }

    fn parse_token(t: &str) -> Option<Key> {
        let lower = t.to_ascii_lowercase();
        let k = match lower.as_str() {
            "ctrl" | "control" => Key::Ctrl,
            "shift" => Key::Shift,
            "alt" | "option" => Key::Alt,
            "win" | "windows" | "super" | "cmd" => Key::Win,
            "enter" | "return" => Key::Enter,
            "esc" | "escape" => Key::Escape,
            "tab" => Key::Tab,
            "space" | "spacebar" => Key::Space,
            "backspace" | "bksp" => Key::Backspace,
            "delete" | "del" => Key::Delete,
            "insert" | "ins" => Key::Insert,
            "home" => Key::Home,
            "end" => Key::End,
            "pgup" | "pageup" => Key::PageUp,
            "pgdn" | "pagedown" => Key::PageDown,
            "up" => Key::Up,
            "down" => Key::Down,
            "left" => Key::Left,
            "right" => Key::Right,
            "minus" | "-" => Key::Minus,
            "equals" | "=" => Key::Equals,
            "comma" | "," => Key::Comma,
            "period" | "." => Key::Period,
            "semicolon" | ";" => Key::Semicolon,
            "quote" | "apos" | "'" => Key::Quote,
            "slash" | "/" => Key::Slash,
            "backslash" | "\\" => Key::Backslash,
            "bracketopen" | "[" => Key::BracketOpen,
            "bracketclose" | "]" => Key::BracketClose,
            "grave" | "backtick" | "`" => Key::Grave,
            other => {
                let mut ch = other.chars();
                let a = ch.next()?;
                if ch.next().is_some() {
                    if let Some(n) = other.strip_prefix('f').and_then(|s| s.parse::<u8>().ok()) {
                        if (1..=24).contains(&n) {
                            return Some(Key::F(n));
                        }
                    }
                    return None;
                }
                match a {
                    'a'..='z' | 'A'..='Z' => Key::Char(a.to_ascii_lowercase()),
                    '0'..='9' => Key::Digit(a.to_digit(10)? as u8),
                    _ => return None,
                }
            }
        };
        Some(k)
    }
}

/// An ordered chord, e.g. `Ctrl + Shift + Z`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Combo(pub Vec<Key>);

impl Combo {
    /// Parses `"Ctrl+Shift+Z"`. Case-insensitive, `+` separated.
    ///
    /// Fails loudly: a malformed dataset must stop startup with the offending string,
    /// not silently render an empty keycap later.
    pub fn parse(s: &str) -> Result<Self, ComboError> {
        let mut keys = Vec::new();
        for tok in s.split('+') {
            let tok = tok.trim();
            if tok.is_empty() {
                return Err(ComboError::EmptyToken(s.to_string()));
            }
            // "Ctrl++" means Ctrl plus the literal plus key; we do not support the
            // Equals-plus ambiguity, so a bare empty token is an error above.
            let k = Key::parse_token(tok).ok_or_else(|| ComboError::Unknown {
                combo: s.to_string(),
                token: tok.to_string(),
            })?;
            keys.push(k);
        }
        if keys.is_empty() {
            return Err(ComboError::Empty(s.to_string()));
        }
        if keys.iter().all(Key::is_modifier) {
            return Err(ComboError::ModifiersOnly(s.to_string()));
        }
        // A chord like Ctrl+K,S is two presses in sequence, not one simultaneous press.
        // Firing it as a single SendInput burst would do something different and wrong,
        // so we refuse it at load time rather than mis-fire at the user.
        if keys.iter().filter(|k| !k.is_modifier()).count() > 1 {
            return Err(ComboError::NotAChord(s.to_string()));
        }
        // Modifiers first, in canonical order, then the trigger key. Keeps rendering
        // and de-duplication stable regardless of how the author wrote it.
        keys.sort_by_key(|k| match k {
            Key::Ctrl => 0,
            Key::Win => 1,
            Key::Alt => 2,
            Key::Shift => 3,
            _ => 4,
        });
        Ok(Self(keys))
    }

    /// The key the user actually presses, once the modifiers are accounted for.
    /// Read by the drill screen (S05), which is a `should` feature and not built yet.
    #[allow(dead_code)]
    pub fn trigger(&self) -> Option<&Key> {
        self.0.iter().find(|k| !k.is_modifier())
    }

    /// Ctrl and friends in the order they should be drawn.
    #[allow(dead_code)]
    pub fn modifiers(&self) -> impl Iterator<Item = &Key> {
        self.0.iter().filter(|k| k.is_modifier())
    }
}

impl fmt::Display for Combo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(Key::label).collect();
        f.write_str(&parts.join("+"))
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ComboError {
    #[error("'{0}' has an empty key token (check for a double '+')")]
    EmptyToken(String),
    #[error("'{0}' is empty")]
    Empty(String),
    #[error("'{token}' in combo '{combo}' is not a key we know")]
    Unknown { combo: String, token: String },
    #[error("'{0}' has no non-modifier key; a shortcut must press something")]
    ModifiersOnly(String),
    #[error("'{0}' looks like a sequential chord (two real keys); this build fires only simultaneous combos")]
    NotAChord(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_three_key_chord() {
        let c = Combo::parse("Ctrl+Shift+Z").unwrap();
        assert_eq!(c.0, vec![Key::Ctrl, Key::Shift, Key::Char('z')]);
        assert_eq!(c.trigger(), Some(&Key::Char('z')));
    }

    #[test]
    fn normalises_modifier_order() {
        let c = Combo::parse("z+ctrl").unwrap();
        assert_eq!(c.0, vec![Key::Ctrl, Key::Char('z')]);
    }

    #[test]
    fn is_case_insensitive_and_trims() {
        // A lone `f` is the letter F, not F5 - the function-key form needs digits after it.
        assert_eq!(Combo::parse("  CTRL + f ").unwrap().0, vec![Key::Ctrl, Key::Char('f')]);
        assert_eq!(Combo::parse("ctrl + F5").unwrap().0, vec![Key::Ctrl, Key::F(5)]);
    }

    #[test]
    fn rejects_unknown_token() {
        assert!(matches!(
            Combo::parse("Ctrl+Banan").unwrap_err(),
            ComboError::Unknown { .. }
        ));
    }

    #[test]
    fn rejects_modifiers_only() {
        assert!(matches!(
            Combo::parse("Ctrl+Shift").unwrap_err(),
            ComboError::ModifiersOnly(_)
        ));
    }

    #[test]
    fn rejects_double_plus() {
        assert!(Combo::parse("Ctrl++").is_err());
    }

    #[test]
    fn rejects_a_sequential_chord() {
        // Ctrl+K then S is two presses. We must not fire it as one burst.
        assert!(matches!(
            Combo::parse("Ctrl+K+S").unwrap_err(),
            ComboError::NotAChord(_)
        ));
    }

    #[test]
    fn accepts_a_simultaneous_function_combo() {
        assert!(Combo::parse("Ctrl+Shift+F12").is_ok());
    }

    #[test]
    fn vk_codes_match_the_windows_constants() {
        assert_eq!(Key::Ctrl.vk(), 0x11);
        assert_eq!(Key::Shift.vk(), 0x10);
        assert_eq!(Key::Alt.vk(), 0x12);
        assert_eq!(Key::Win.vk(), 0x5B);
        assert_eq!(Key::Char('a').vk(), 0x41);
        assert_eq!(Key::Digit(7).vk(), 0x37);
        assert_eq!(Key::F(1).vk(), 0x70);
        assert_eq!(Key::Enter.vk(), 0x0D);
    }

    #[test]
    fn labels_are_render_ready() {
        assert_eq!(Key::Ctrl.label(), "Ctrl");
        assert_eq!(Key::Up.label(), "\u{2191}");
        assert_eq!(Combo::parse("Ctrl+Shift+Left").unwrap().to_string(), "Ctrl+Shift+\u{2190}");
    }

    #[test]
    fn normalisation_makes_equivalent_spellings_equal() {
        // The dataset is hand-written, so "Ctrl+C" and "c+ctrl" are the same chord and must
        // compare as one - that is what lets a user override a built-in entry by name.
        assert_eq!(Combo::parse("Ctrl+C").unwrap(), Combo::parse("c+CTRL").unwrap());
        assert_ne!(Combo::parse("Ctrl+C").unwrap(), Combo::parse("Ctrl+V").unwrap());
    }
}
