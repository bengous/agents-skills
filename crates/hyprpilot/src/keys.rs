//! Keyboard input via the `sendshortcut` dispatcher (no focus required).
//!
//! Hyprland resolves the key name to an XKB keysym, then scans the active
//! keymap for a keycode whose *unmodified* level produces that keysym, and
//! sends the requested modifiers alongside. Consequences encoded here:
//! shifted characters must be written as `SHIFT` + the base key of a US
//! keymap (`!` → `SHIFT,1`), and accented characters resolve only on keymaps
//! that expose them unshifted (e.g. `fr`).
//!
//! Both modes use that one dispatcher, on different compositors: a shared
//! session addresses the host, an isolated one the nested compositor of its
//! agent desktop (`Route` below).

use std::thread;
use std::time::Duration;

use crate::error::Error;
use crate::guard;
use crate::hypr::{self, Ctl};
use crate::isolated;
use crate::session::{self, Isolated, ModeState};

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

/// Which compositor input goes to, and to which window. An isolated session
/// resolves to `Ctl::Instance`, so no dispatch made here can reach a window of
/// the user's desktop.
#[derive(Debug, PartialEq, Eq)]
enum Route {
    Shared { address: String },
    Isolated { signature: String, address: String },
}

impl Route {
    fn resolve(name: &str, command: &'static str) -> Result<Self, Error> {
        match session::load(name)?.state {
            // Shared mode goes through `session::current_window`, which re-reads
            // the state and stays the single definition of the window a session
            // drives on the user's desktop.
            ModeState::Shared(_) => {
                let (_, window) = session::current_window(name, command)?;
                Ok(Self::Shared {
                    address: window.address,
                })
            }
            ModeState::Isolated(isolated) => Self::agent(name, &isolated),
        }
    }

    /// The target of an agent desktop: the signature of its nested compositor
    /// and the window address recorded when the app was launched inside it.
    /// `isolated::live_instance` is the one gate for a dead or unfinished
    /// desktop, so no dispatch here can go looking for the user's own windows.
    fn agent(name: &str, isolated: &Isolated) -> Result<Self, Error> {
        let instance = isolated::live_instance(name, isolated)?;
        let address = isolated::recorded_window(name, isolated)?;
        ensure_window(instance.signature, address)?;
        Ok(Self::Isolated {
            signature: instance.signature.to_owned(),
            address: address.to_owned(),
        })
    }

    fn ctl(&self) -> Ctl<'_> {
        match self {
            Self::Shared { .. } => Ctl::Host,
            Self::Isolated { signature, .. } => Ctl::Instance(signature),
        }
    }

    fn address(&self) -> &str {
        match self {
            Self::Shared { address } | Self::Isolated { address, .. } => address,
        }
    }

    /// Only the user's seat is guarded. `--focus` is accepted and ignored on an
    /// agent desktop: its target window already is the focused window of that
    /// seat, and no human has a cursor there to snapshot or restore.
    fn guarded(&self, focus: bool) -> bool {
        match self {
            Self::Shared { .. } => focus,
            Self::Isolated { .. } => false,
        }
    }

    /// How the target reads back in command output: an agent desktop's address
    /// only means something together with the instance it lives in.
    fn target(&self) -> String {
        match self {
            Self::Shared { address } => address.clone(),
            Self::Isolated { signature, address } => {
                format!("{address} in agent desktop instance {signature}")
            }
        }
    }

    fn send(&self, chords: &[Chord], delay_ms: u64, focus: bool) -> Result<(), Error> {
        let action = || {
            for (index, chord) in chords.iter().enumerate() {
                if index > 0 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                send_chord(chord, self.ctl(), self.address())?;
            }
            Ok(())
        };
        if self.guarded(focus) {
            guard::run(
                Some(self.address()),
                || Ok(()),
                |()| action(),
                |&(), cursor| guard::restore_cursor(cursor),
            )?;
            Ok(())
        } else {
            action()
        }
    }
}

/// The window is read back from the instance before anything is dispatched, so
/// a stale address fails here with the same error as in shared mode instead of
/// depending on what the dispatcher answers for a window it cannot find.
fn ensure_window(signature: &str, address: &str) -> Result<(), Error> {
    let present = hypr::clients_on(Ctl::Instance(signature))?
        .iter()
        .any(|client| client.address == address);
    if present {
        Ok(())
    } else {
        Err(Error::WindowGone(address.to_owned()))
    }
}

fn send_chord(chord: &Chord, ctl: Ctl<'_>, address: &str) -> Result<(), Error> {
    hypr::dispatch_on(ctl, &["sendshortcut", &shortcut_arg(chord, address)])
        .map_err(|error| enrich_key_error(error, chord))
}

/// The dispatcher's window argument is the address: a title can change between
/// the read and the dispatch.
fn shortcut_arg(chord: &Chord, address: &str) -> String {
    format!("{},{},address:{address}", chord.mods_string(), chord.keysym)
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

pub fn send_keys(
    name: &str,
    raw_chords: &[String],
    delay_ms: u64,
    focus: bool,
) -> Result<String, Error> {
    let route = Route::resolve(name, "key")?;
    let chords = raw_chords
        .iter()
        .map(|raw| parse_chord(raw))
        .collect::<Result<Vec<_>, _>>()?;
    route.send(&chords, delay_ms, focus)?;
    Ok(format!(
        "sent {} key(s) to {}",
        chords.len(),
        route.target()
    ))
}

pub fn type_text(name: &str, text: &str, delay_ms: u64, focus: bool) -> Result<String, Error> {
    let route = Route::resolve(name, "type")?;
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
    route.send(&chords, delay_ms, focus)?;
    Ok(format!(
        "typed {} character(s) into {}",
        chords.len(),
        route.target()
    ))
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::{Chord, Ctl, Isolated, Modifier, Route, char_to_keysym, parse_chord, shortcut_arg};
    use crate::error::Error;
    use crate::session::Instance;

    const SIGNATURE: &str = "abcdef_1730000000";

    fn live() -> Instance {
        Instance::Live {
            signature: SIGNATURE.to_owned(),
            wayland_display: "wayland-2".to_owned(),
            pid: 4242,
            console_address: "0xc0ff33".to_owned(),
        }
    }

    fn agent_state(instance: Instance, active_address: Option<&str>) -> Isolated {
        Isolated {
            output: "hyprpilot-alpha".to_owned(),
            workspace: "agent-alpha".to_owned(),
            size: [1600, 1000],
            shown: false,
            active_address: active_address.map(str::to_owned),
            instance,
        }
    }

    fn agent_route() -> Route {
        Route::Isolated {
            signature: SIGNATURE.to_owned(),
            address: "0xdead".to_owned(),
        }
    }

    fn shared_route() -> Route {
        Route::Shared {
            address: "0xabc".to_owned(),
        }
    }

    #[test]
    fn an_agent_desktop_chord_is_addressed_to_its_instance_never_the_host() -> Result<(), Error> {
        let route = agent_route();
        assert_eq!(route.ctl(), Ctl::Instance(SIGNATURE));
        assert_eq!(
            shortcut_arg(&parse_chord("Ctrl+Shift+a")?, route.address()),
            "CTRL SHIFT,a,address:0xdead"
        );
        assert_eq!(
            shortcut_arg(&parse_chord("Return")?, route.address()),
            ",Return,address:0xdead"
        );
        assert_eq!(shared_route().ctl(), Ctl::Host);
        Ok(())
    }

    #[test]
    fn focus_is_a_no_op_on_an_agent_desktop_seat() {
        assert!(!agent_route().guarded(true));
        assert!(!agent_route().guarded(false));
        assert!(shared_route().guarded(true));
        assert!(!shared_route().guarded(false));
    }

    #[test]
    fn output_names_the_instance_an_agent_desktop_window_lives_in() {
        assert_eq!(
            agent_route().target(),
            format!("0xdead in agent desktop instance {SIGNATURE}")
        );
        assert_eq!(shared_route().target(), "0xabc");
    }

    #[test]
    fn a_pending_instance_is_refused_with_a_teardown_hint() -> Result<(), Box<dyn StdError>> {
        let state = agent_state(Instance::Pending, Some("0xdead"));
        let Err(error) = Route::agent("alpha", &state) else {
            return Err("a pending instance has no compositor to send to".into());
        };
        assert!(
            matches!(error, Error::AgentDesktopUnready { .. }),
            "{error:?}"
        );
        let message = error.to_string();
        assert!(message.contains("never spawned"), "{message}");
        assert!(message.contains("--session alpha teardown"), "{message}");
        Ok(())
    }

    #[test]
    fn a_dead_instance_is_refused_instead_of_reaching_the_host() -> Result<(), Box<dyn StdError>> {
        // Pid 4242 does not carry this session's marker in the real `/proc`,
        // which is exactly what a crashed nested compositor looks like; the probe
        // itself is asserted against a fake `/proc` in `isolated`.
        let state = agent_state(live(), Some("0xdead"));
        let Err(error) = Route::agent("alpha", &state) else {
            return Err("a dead instance has no compositor to send chords to".into());
        };
        assert!(matches!(error, Error::AgentDesktopDead { .. }), "{error:?}");
        let message = error.to_string();
        assert!(message.contains("is dead"), "{message}");
        assert!(message.contains("--session alpha teardown"), "{message}");
        Ok(())
    }

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
