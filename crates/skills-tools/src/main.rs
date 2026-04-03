use std::env;
use std::io::{self, Write};

fn main() {
    match skills_tools::run_cli(env::args().skip(1)) {
        Ok(message) => {
            if !message.is_empty() {
                let _ = writeln!(io::stdout(), "{message}");
            }
        }
        Err(error) => {
            let _ = writeln!(io::stderr(), "{error}");
            std::process::exit(1);
        }
    }
}
