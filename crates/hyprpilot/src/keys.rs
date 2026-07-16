//! Keyboard input via the `sendshortcut` dispatcher (no focus required).
//!
//! Hyprland resolves the key name to an XKB keysym, then scans the active
//! keymap for a keycode whose *unmodified* level produces that keysym, and
//! sends the requested modifiers alongside. Consequences encoded here:
//! shifted characters must be written as `SHIFT` + the base key of a US
//! keymap (`!` → `SHIFT,1`), and accented characters resolve only on keymaps
//! that expose them unshifted (e.g. `fr`).

use std::thread;
use std::time::Duration;

use crate::error::Error;
use crate::hypr;
use crate::session;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Super,
}

impl Modifier {
    fn hypr_name(self) -> &'static str {
        match self {
            Self::Ctrl => "CTRL",
            Self::Shift => "SHIFT",
            Self::Alt => "ALT",
            Self::Super => "SUPER",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(Self::Ctrl),
            "shift" => Some(Self::Shift),
            "alt" => Some(Self::Alt),
            "super" | "meta" | "win" => Some(Self::Super),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub mods: Vec<Modifier>,
    pub keysym: String,
}

impl Chord {
    fn mods_string(&self) -> String {
        let mut mods: Vec<&str> = self.mods.iter().map(|m| m.hypr_name()).collect();
        mods.sort_unstable();
        mods.dedup();
        mods.join(" ")
    }
}

/// Parses `Ctrl+Shift+a`, `Down`, `F5`, `!`… Single characters go through
/// the character table; longer key parts are passed to Hyprland verbatim as
/// XKB keysym names.
pub fn parse_chord(raw: &str) -> Result<Chord, Error> {
    let parts: Vec<&str> = raw.split('+').collect();
    let invalid = || Error::InvalidChord(raw.to_owned());

    let (key_part, mod_parts) = parts.split_last().ok_or_else(invalid)?;
    let mut mods = Vec::new();
    for part in mod_parts {
        mods.push(Modifier::parse(part).ok_or_else(invalid)?);
    }

    let mut chars = key_part.chars();
    let keysym = match (chars.next(), chars.next()) {
        (None, _) => return Err(invalid()),
        (Some(c), None) => {
            let (shift, keysym) = char_to_keysym(c)?;
            if shift {
                mods.push(Modifier::Shift);
            }
            keysym.to_owned()
        }
        _ => (*key_part).to_owned(),
    };

    mods.sort_unstable();
    mods.dedup();
    Ok(Chord { mods, keysym })
}

/// Maps a character to `(needs_shift, keysym)` under a US keymap.
/// Accented characters map to their own keysym: they only resolve if the
/// active keymap exposes them on an unmodified key (e.g. layout `fr`).
pub fn char_to_keysym(c: char) -> Result<(bool, &'static str), Error> {
    const LETTERS: [&str; 26] = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
        "s", "t", "u", "v", "w", "x", "y", "z",
    ];
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

    let letter = |c: char| LETTERS[(c.to_ascii_lowercase() as usize) - ('a' as usize)];
    let digit = |c: char| DIGITS[(c as usize) - ('0' as usize)];

    let mapped = match c {
        'a'..='z' => (false, letter(c)),
        'A'..='Z' => (true, letter(c)),
        '0'..='9' => (false, digit(c)),
        ' ' => (false, "space"),
        '\n' => (false, "Return"),
        '\t' => (false, "Tab"),
        // Unshifted US punctuation: the character's own keysym.
        '-' => (false, "minus"),
        '=' => (false, "equal"),
        '[' => (false, "bracketleft"),
        ']' => (false, "bracketright"),
        '\\' => (false, "backslash"),
        ';' => (false, "semicolon"),
        '\'' => (false, "apostrophe"),
        '`' => (false, "grave"),
        ',' => (false, "comma"),
        '.' => (false, "period"),
        '/' => (false, "slash"),
        // Shifted US punctuation: SHIFT + the base key's keysym.
        '!' => (true, "1"),
        '@' => (true, "2"),
        '#' => (true, "3"),
        '$' => (true, "4"),
        '%' => (true, "5"),
        '^' => (true, "6"),
        '&' => (true, "7"),
        '*' => (true, "8"),
        '(' => (true, "9"),
        ')' => (true, "0"),
        '_' => (true, "minus"),
        '+' => (true, "equal"),
        '{' => (true, "bracketleft"),
        '}' => (true, "bracketright"),
        '|' => (true, "backslash"),
        ':' => (true, "semicolon"),
        '"' => (true, "apostrophe"),
        '~' => (true, "grave"),
        '<' => (true, "comma"),
        '>' => (true, "period"),
        '?' => (true, "slash"),
        // Common French accents — keymap-dependent, see module docs.
        'é' => (false, "eacute"),
        'è' => (false, "egrave"),
        'ê' => (false, "ecircumflex"),
        'ë' => (false, "ediaeresis"),
        'à' => (false, "agrave"),
        'â' => (false, "acircumflex"),
        'î' => (false, "icircumflex"),
        'ï' => (false, "idiaeresis"),
        'ô' => (false, "ocircumflex"),
        'ù' => (false, "ugrave"),
        'û' => (false, "ucircumflex"),
        'ü' => (false, "udiaeresis"),
        'ç' => (false, "ccedilla"),
        'œ' => (false, "oe"),
        _ => return Err(Error::UnmappedChar(c)),
    };
    Ok(mapped)
}

fn send_chord(chord: &Chord, address: &str) -> Result<(), Error> {
    let arg = format!("{},{},address:{address}", chord.mods_string(), chord.keysym);
    hypr::dispatch(&["sendshortcut", &arg]).map_err(|error| enrich_key_error(error, chord))
}

/// Hyprland's `key not found` means the keysym is not reachable unmodified on
/// the active keymap — surface that instead of the raw dispatcher error.
fn enrich_key_error(error: Error, chord: &Chord) -> Error {
    match &error {
        Error::Tool { message, .. } if message.contains("key not found") => Error::Invalid {
            what: "key",
            value: chord.keysym.clone(),
            hint: "keysym not reachable without modifiers on the active keymap \
                   (see `hyprpilot doctor` for the layout)"
                .to_owned(),
        },
        _ => error,
    }
}

pub fn send_keys(raw_chords: &[String], delay_ms: u64) -> Result<String, Error> {
    let (_, window) = session::current_window()?;
    let chords = raw_chords
        .iter()
        .map(|raw| parse_chord(raw))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, chord) in chords.iter().enumerate() {
        if index > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }
        send_chord(chord, &window.address)?;
    }
    Ok(format!(
        "sent {} key(s) to {}",
        chords.len(),
        window.address
    ))
}

pub fn type_text(text: &str, delay_ms: u64) -> Result<String, Error> {
    let (_, window) = session::current_window()?;
    let chords = text
        .chars()
        .map(|c| {
            let (shift, keysym) = char_to_keysym(c)?;
            Ok(Chord {
                mods: if shift { vec![Modifier::Shift] } else { vec![] },
                keysym: keysym.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    for (index, chord) in chords.iter().enumerate() {
        if index > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }
        send_chord(chord, &window.address)?;
    }
    Ok(format!(
        "typed {} character(s) into {}",
        chords.len(),
        window.address
    ))
}

#[cfg(test)]
mod tests {
    use super::{Chord, Modifier, char_to_keysym, parse_chord};
    use crate::error::Error;

    #[test]
    fn every_printable_ascii_char_is_mapped() {
        for code in 0x20..=0x7Eu8 {
            let c = char::from(code);
            assert!(
                char_to_keysym(c).is_ok(),
                "printable ASCII {c:?} must be mapped"
            );
        }
    }

    #[test]
    fn letters_map_with_shift_for_uppercase() -> Result<(), Error> {
        assert_eq!(char_to_keysym('a')?, (false, "a"));
        assert_eq!(char_to_keysym('z')?, (false, "z"));
        assert_eq!(char_to_keysym('A')?, (true, "a"));
        assert_eq!(char_to_keysym('Z')?, (true, "z"));
        Ok(())
    }

    #[test]
    fn digits_and_us_shift_pairs() -> Result<(), Error> {
        assert_eq!(char_to_keysym('7')?, (false, "7"));
        assert_eq!(char_to_keysym('!')?, (true, "1"));
        assert_eq!(char_to_keysym('?')?, (true, "slash"));
        assert_eq!(char_to_keysym(':')?, (true, "semicolon"));
        assert_eq!(char_to_keysym('"')?, (true, "apostrophe"));
        assert_eq!(char_to_keysym('~')?, (true, "grave"));
        assert_eq!(char_to_keysym('.')?, (false, "period"));
        assert_eq!(char_to_keysym(' ')?, (false, "space"));
        Ok(())
    }

    #[test]
    fn french_accents_map_to_their_keysym() -> Result<(), Error> {
        assert_eq!(char_to_keysym('é')?, (false, "eacute"));
        assert_eq!(char_to_keysym('ç')?, (false, "ccedilla"));
        assert_eq!(char_to_keysym('œ')?, (false, "oe"));
        Ok(())
    }

    #[test]
    fn unmapped_characters_error_clearly() {
        assert!(matches!(char_to_keysym('€'), Err(Error::UnmappedChar('€'))));
        assert!(matches!(char_to_keysym('É'), Err(Error::UnmappedChar('É'))));
    }

    #[test]
    fn parses_plain_and_modified_chords() -> Result<(), Error> {
        assert_eq!(
            parse_chord("a")?,
            Chord {
                mods: vec![],
                keysym: "a".to_owned()
            }
        );
        assert_eq!(
            parse_chord("Down")?,
            Chord {
                mods: vec![],
                keysym: "Down".to_owned()
            }
        );
        assert_eq!(
            parse_chord("Ctrl+c")?,
            Chord {
                mods: vec![Modifier::Ctrl],
                keysym: "c".to_owned()
            }
        );
        assert_eq!(
            parse_chord("ctrl+shift+Escape")?,
            Chord {
                mods: vec![Modifier::Ctrl, Modifier::Shift],
                keysym: "Escape".to_owned()
            }
        );
        Ok(())
    }

    #[test]
    fn single_char_chords_go_through_the_table() -> Result<(), Error> {
        // `Ctrl+A` implies SHIFT because 'A' is shifted on a US keymap.
        assert_eq!(
            parse_chord("Ctrl+A")?,
            Chord {
                mods: vec![Modifier::Ctrl, Modifier::Shift],
                keysym: "a".to_owned()
            }
        );
        assert_eq!(
            parse_chord("!")?,
            Chord {
                mods: vec![Modifier::Shift],
                keysym: "1".to_owned()
            }
        );
        Ok(())
    }

    #[test]
    fn rejects_malformed_chords() {
        assert!(parse_chord("").is_err());
        assert!(parse_chord("Bogus+a").is_err());
        assert!(parse_chord("Ctrl+").is_err());
        assert!(parse_chord("Ctrl++").is_err());
    }
}
