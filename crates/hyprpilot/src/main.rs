use std::io::{self, Write};
use std::process::ExitCode;

mod capture;
mod cli;
mod error;
mod hypr;
mod keys;
mod pointer;
mod session;

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
