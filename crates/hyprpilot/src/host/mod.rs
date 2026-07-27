//! The one place a durable change to the user's compositor is posed.
//!
//! `hyprctl` has five mutating calls this crate uses and two mutating
//! dispatchers. The calls are `pub(in crate::host)`, so outside `src/host/` they
//! do not exist and the wrappers below are the only way to reach them — the
//! invariant is the compiler's, not a convention. `hypr::dispatch` has to stay
//! visible to the crate (21 legitimate, read-only-in-effect sites), so for the
//! two dispatchers the same invariant is held by a source sweep instead
//! (`no_module_outside_host_dispatches_renameworkspace_or_moveworkspacetomonitor`).
//!
//! What each mutation costs to undo lives next door, in `ledger`.

pub mod hypr;
// The ledger has no caller until the session state carries it: the type and its
// undo land first, so the mutations can move behind it one at a time.
#[expect(
    dead_code,
    reason = "persisted and drained by the following lots; the type lands first"
)]
pub mod ledger;

use crate::error::Error;

pub fn output_create_headless(output: &str) -> Result<(), Error> {
    hypr::output_create_headless(output)
}

pub fn output_remove(output: &str) -> Result<(), Error> {
    hypr::output_remove(output)
}

pub fn keyword_monitor(output: &str, width: u32, height: u32) -> Result<(), Error> {
    hypr::keyword_monitor(output, width, height)
}

pub fn keyword_monitor_at(
    output: &str,
    width: u32,
    height: u32,
    at: (i32, i32),
    scale: f64,
) -> Result<(), Error> {
    hypr::keyword_monitor_at(output, width, height, at, scale)
}

pub fn keyword_workspace(workspace: &str, monitor: &str) -> Result<(), Error> {
    hypr::keyword_workspace(workspace, monitor)
}

/// Takes the workspace *id*, never its name: the name is what the rename is
/// about to change, and the id is what survives it.
pub fn rename_workspace(id: i64, to: &str) -> Result<(), Error> {
    hypr::dispatch(&["renameworkspace", &id.to_string(), to])
}

/// Takes a workspace *selector* (`session::workspace_selector`), because the
/// dispatcher does.
pub fn move_workspace_to_monitor(selector: &str, monitor: &str) -> Result<(), Error> {
    hypr::dispatch(&["moveworkspacetomonitor", selector, monitor])
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// The two mutating dispatchers, as a call site spells them. Backticks in a
    /// doc comment and the `hyprctl dispatch renameworkspace` of an error
    /// message do not match: only a quoted argument does.
    const MUTATING_DISPATCHERS: [&str; 2] = ["\"renameworkspace\"", "\"moveworkspacetomonitor\""];

    fn rust_files(dir: &Path, skip: &Path, into: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path == skip {
                continue;
            }
            if path.is_dir() {
                rust_files(&path, skip, into)?;
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                into.push(path);
            }
        }
        Ok(())
    }

    /// `pub(in crate::host)` keeps the five mutating `hyprctl` calls inside this
    /// module; the two dispatchers go through `hypr::dispatch`, which the crate
    /// needs for its twenty-one harmless uses. This is what stands in for the
    /// compiler there.
    #[test]
    fn no_module_outside_host_dispatches_renameworkspace_or_moveworkspacetomonitor()
    -> Result<(), Box<dyn StdError>> {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &src.join("host"), &mut files)?;
        assert!(
            files.len() > 5,
            "the sweep found {} files under {} — it is looking in the wrong place",
            files.len(),
            src.display()
        );

        for file in files {
            let text = fs::read_to_string(&file)?;
            for dispatcher in MUTATING_DISPATCHERS {
                assert!(
                    !text.contains(dispatcher),
                    "{} dispatches {dispatcher} itself; a durable host mutation is posed from \
                     src/host/ or not at all",
                    file.display()
                );
            }
        }
        Ok(())
    }
}
