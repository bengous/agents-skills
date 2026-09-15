use std::io::{self, Write};
use std::process::ExitCode;

mod capture;
mod cli;
mod error;
mod guard;
mod host;
mod isolated;
mod keys;
mod pointer;
mod session;

/// `hypr` moved under `host` so the mutating calls could be narrowed to
/// `pub(in crate::host)` — a visibility that has to name an ancestor module, and
/// `crate::host` is a sibling of `crate::hypr` (E0742). Re-exported here so the
/// crate's `use crate::hypr::{self, Ctl}` keep resolving; a re-export does not
/// widen what it re-exports, so the five mutating calls stay unreachable from
/// outside `src/host/`.
pub(crate) use host::hypr;

fn main() -> ExitCode {
    match cli::run() {
        Ok(message) => {
            if !message.is_empty() {
                let _ = writeln!(io::stdout(), "{message}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(io::stderr(), "hyprpilot: {error}");
            ExitCode::FAILURE
        }
    }
}
