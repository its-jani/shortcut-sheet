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
    /// The spare keys a keyboard carries and almost nobody presses. They matter here because
    /// they are what a *hold* trigger wants: a key that does nothing else can be held while
    /// the sheet is up without leaking a character into the app behind it.
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    /// The menu key, drawn on some keyboards as a list or a context-menu glyph.
    Apps,
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
            CapsLock => 0x14,
            NumLock => 0x90,
            ScrollLock => 0x91,
            PrintScreen => 0x2C,
            Pause => 0x13,
            Apps => 0x5D,
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
            CapsLock => "Caps".into(),
            NumLock => "Num".into(),
            ScrollLock => "Scroll".into(),
            PrintScreen => "PrtSc".into(),
            Pause => "Pause".into(),
            Apps => "Menu".into(),
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
            "capslock" | "caps" => Key::CapsLock,
            "numlock" | "num" => Key::NumLock,
            "scrolllock" | "scroll" => Key::ScrollLock,
            "printscreen" | "print" | "prtsc" | "snapshot" => Key::PrintScreen,
            "pause" | "break" => Key::Pause,
            "apps" | "menu" | "contextmenu" => Key::Apps,
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

/// Which spelling a chord is being read for. The grammar is one grammar everywhere; what
/// changes is what a chord is *allowed to be made of*, and that depends on what happens to
/// it afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// A shortcut this app fires into another app with `SendInput`, so there has to be a key
    /// to press: modifiers alone would inject nothing.
    Shortcut,
    /// The instant summon chord. Watched, never fired, so `Win+Alt` is legal - but a lone
    /// `Ctrl` would pop the sheet over every copy-paste, so it takes two keys to be a chord.
    Hotkey,
    /// The key set a *hold* is measured on. Here one key is not a bug but the whole design
    /// (`trigger: "ctrl"`), because the delay is what turns a key into a lookup. A set of
    /// modifiers, or any key at all, is equally a trigger.
    Trigger,
}

impl Form {
    /// Whether a chord with no ordinary key in it is something this form can use.
    fn modifiers_ok(self) -> bool {
        !matches!(self, Form::Shortcut)
    }

    /// Whether one modifier on its own is enough. Only the hold, and only because of `hold_ms`.
    fn lone_modifier_ok(self) -> bool {
        matches!(self, Form::Trigger)
    }
}

impl Combo {
    /// Parses `"Ctrl+Shift+Z"`. Case-insensitive, `+` separated.
    ///
    /// Fails loudly: a malformed dataset must stop startup with the offending string,
    /// not silently render an empty keycap later.
    pub fn parse(s: &str) -> Result<Self, ComboError> {
        Self::read(s, Form::Shortcut)
    }

    /// Parse the chord that summons the overlay itself, where `"Win+Alt"` is legal.
    ///
    /// [`Combo::parse`] refuses a modifiers-only chord on purpose: there would be no key to
    /// fire into another app. A hotkey is the other case - it is never sent anywhere, only
    /// watched for - and the observed configuration of the target uses exactly `Win+Alt`
    /// (`StaticShortcutKeys: [91, 18]`), two modifiers and nothing else. Same token grammar,
    /// same canonical order, one rule switched off.
    pub fn parse_hotkey(s: &str) -> Result<Self, ComboError> {
        Self::read(s, Form::Hotkey)
    }

    /// Parse the key set a hold is measured on, where `"ctrl"` is the stock answer and
    /// `"f6"`, `"capslock"` or `"ctrl+shift"` are the customised ones.
    ///
    /// The instant chord's grammar with its two-key minimum removed: a hold is not a chord the
    /// user taps on the way to something else, it is a key they deliberately keep down for
    /// `hold_ms`, and one key is the common case. Everything else still applies - unknown
    /// tokens fail, and so does a set with two ordinary keys in it, which is a sequence
    /// nobody can hold.
    pub fn parse_trigger(s: &str) -> Result<Self, ComboError> {
        Self::read(s, Form::Trigger)
    }

    fn read(s: &str, form: Form) -> Result<Self, ComboError> {
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
            // A key written twice is one key pressed once. This matters more than tidiness:
            // the hook watches a set it either has or has not seen complete, so a duplicate
            // left in a trigger would be a key that can never come down twice at once - a
            // trigger that silently never fires, from a settings file that looks reasonable.
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        if keys.is_empty() {
            return Err(ComboError::Empty(s.to_string()));
        }
        if !form.modifiers_ok() && keys.iter().all(Key::is_modifier) {
            return Err(ComboError::ModifiersOnly(s.to_string()));
        }
        // A chord like Ctrl+K,S is two presses in sequence, not one simultaneous press.
        // Firing it as a single SendInput burst would do something different and wrong,
        // so we refuse it at load time rather than mis-fire at the user. A hotkey is
        // watched, not fired, but the same ambiguity would make it unpressable, so the
        // rule holds for both spellings.
        if keys.iter().filter(|k| !k.is_modifier()).count() > 1 {
            return Err(ComboError::NotAChord(s.to_string()));
        }
        if !form.lone_modifier_ok() && keys.iter().all(Key::is_modifier) && keys.len() < 2 {
            // A one-key hotkey (`"Ctrl"`) is the hold mode with the delay taken to zero: it
            // would summon the sheet on every press of a key the user types hundreds of times
            // a day. Two keys minimum is what makes an instant chord a chord.
            return Err(ComboError::ModifiersOnly(s.to_string()));
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
    fn a_hotkey_may_be_modifiers_only_because_it_is_never_fired() {
        // The instant trigger the observed config uses. `parse` refuses it, `parse_hotkey`
        // does not, and neither is wrong: one sends a chord, the other watches for one.
        assert!(matches!(
            Combo::parse("Win+Alt").unwrap_err(),
            ComboError::ModifiersOnly(_)
        ));
        let hot = Combo::parse_hotkey("alt+win").unwrap();
        assert_eq!(hot.0, vec![Key::Win, Key::Alt], "canonical modifier order");
    }

    #[test]
    fn a_hotkey_may_end_on_an_ordinary_key() {
        // Both shapes the target's own settings log shows: [91, 18] and [91, 8].
        assert_eq!(Combo::parse_hotkey("Win+Backspace").unwrap().0, vec![Key::Win, Key::Backspace]);
        assert_eq!(Combo::parse_hotkey("Win+Alt+`").unwrap().0, vec![Key::Win, Key::Alt, Key::Grave]);
    }

    #[test]
    fn a_single_modifier_is_not_a_hotkey() {
        // Watching for a bare Ctrl to appear instantly would pop the sheet over every Ctrl+C.
        assert!(matches!(
            Combo::parse_hotkey("Ctrl").unwrap_err(),
            ComboError::ModifiersOnly(_)
        ));
    }

    #[test]
    fn a_hotkey_still_rejects_two_sequential_presses() {
        assert!(matches!(
            Combo::parse_hotkey("Win+K+S").unwrap_err(),
            ComboError::NotAChord(_)
        ));
    }

    #[test]
    fn a_broken_hotkey_names_the_string_that_is_broken() {
        // The settings file is hand-edited, so the error has to say which value is at fault.
        let err = Combo::parse_hotkey("Win+Banan").unwrap_err();
        assert!(err.to_string().contains("Banan"), "{err}");
    }

    // -- the hold trigger -----------------------------------------------------------------

    #[test]
    fn a_lone_modifier_is_a_trigger_even_though_it_is_not_a_chord() {
        // The stock configuration, which is what `"trigger": "ctrl"` has always meant. The
        // two other forms refuse it for good reasons; the hold is the case those reasons
        // do not apply to, because the delay is what makes it deliberate.
        assert_eq!(Combo::parse_trigger("ctrl").unwrap().0, vec![Key::Ctrl]);
        assert_eq!(Combo::parse_trigger("ALT").unwrap().0, vec![Key::Alt]);
    }

    #[test]
    fn any_key_at_all_can_be_a_trigger() {
        // The whole feature: the hold is not limited to the four modifiers. Function keys,
        // the spare keys nobody else uses, and punctuation all work, spelled the way every
        // other chord in this project spells them.
        assert_eq!(Combo::parse_trigger("f13").unwrap().0, vec![Key::F(13)]);
        assert_eq!(Combo::parse_trigger("capslock").unwrap().0, vec![Key::CapsLock]);
        assert_eq!(Combo::parse_trigger("`").unwrap().0, vec![Key::Grave]);
        assert_eq!(Combo::parse_trigger("menu").unwrap().0, vec![Key::Apps]);
    }

    #[test]
    fn a_trigger_may_be_a_set_of_keys_held_together() {
        // "Hold Ctrl+Shift for a second and a half" is a trigger a one-key hook could not
        // offer, and it reads the way the instant chord reads. Canonical order, so
        // "shift+ctrl" and "ctrl+shift" are the same instruction.
        let t = Combo::parse_trigger("shift+ctrl").unwrap();
        assert_eq!(t.0, vec![Key::Ctrl, Key::Shift]);
        assert_eq!(t, Combo::parse_trigger("Ctrl+Shift").unwrap());
        // ...including a full shortcut, which is a key set like any other to hold down.
        assert_eq!(
            Combo::parse_trigger("Ctrl+Shift+P").unwrap().0,
            vec![Key::Ctrl, Key::Shift, Key::Char('p')]
        );
    }

    #[test]
    fn a_trigger_keeps_the_rules_that_protect_it_from_itself() {
        // Two ordinary keys are a sequence, not a gesture, and an unknown word is a typo in a
        // hand-edited file. Both answers are the ones the instant chord gives.
        assert!(matches!(
            Combo::parse_trigger("K+S").unwrap_err(),
            ComboError::NotAChord(_)
        ));
        assert!(matches!(
            Combo::parse_trigger("Banan").unwrap_err(),
            ComboError::Unknown { .. }
        ));
        assert!(Combo::parse_trigger("").is_err());
    }

    #[test]
    fn the_spare_keys_are_known_by_every_name_people_use() {
        for (spelling, key) in [
            ("caps", Key::CapsLock),
            ("capslock", Key::CapsLock),
            ("numlock", Key::NumLock),
            ("scroll", Key::ScrollLock),
            ("prtsc", Key::PrintScreen),
            ("printscreen", Key::PrintScreen),
            ("pause", Key::Pause),
            ("apps", Key::Apps),
            ("menu", Key::Apps),
        ] {
            assert_eq!(Key::parse_token(spelling), Some(key), "'{spelling}'");
        }
    }

    #[test]
    fn the_spare_keys_have_the_windows_vk_codes_and_short_labels() {
        // Labels go inside a keycap, so they have to stay narrow; the codes are what the hook
        // and SendInput match on.
        assert_eq!(Key::CapsLock.vk(), 0x14);
        assert_eq!(Key::NumLock.vk(), 0x90);
        assert_eq!(Key::ScrollLock.vk(), 0x91);
        assert_eq!(Key::PrintScreen.vk(), 0x2C);
        assert_eq!(Key::Pause.vk(), 0x13);
        assert_eq!(Key::Apps.vk(), 0x5D);
        assert_eq!(Key::CapsLock.label(), "Caps");
        assert_eq!(Key::PrintScreen.label(), "PrtSc");
        // And none of them is a modifier: CapsLock held down is still a key of its own.
        assert!(!Key::CapsLock.is_modifier());
    }

    #[test]
    fn a_key_written_twice_is_one_key_pressed_once() {
        // A typo in the hand-edited settings file must not become a dead trigger, and must not
        // become a modifier pressed twice on the way into an app either.
        assert_eq!(Combo::parse_trigger("ctrl+ctrl").unwrap().0, vec![Key::Ctrl]);
        assert_eq!(Combo::parse("Ctrl+Ctrl+C").unwrap().0, vec![Key::Ctrl, Key::Char('c')]);
        assert_eq!(Combo::parse_hotkey("Win+Alt+Win").unwrap().0, vec![Key::Win, Key::Alt]);
        // And the deduplication happens before the two-key rules, so a doubled ordinary key is
        // one key held down, not a sequence nobody can press.
        assert_eq!(Combo::parse_trigger("K+K").unwrap().0, vec![Key::Char('k')]);
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
