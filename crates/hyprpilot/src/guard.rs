use std::panic::{self, AssertUnwindSafe};

use crate::error::{Error, RestoreFailure};
use crate::hypr;

pub const WARP_TOLERANCE: i32 = 1;

pub struct Snapshot {
    cursor: (i32, i32),
    focus: Option<String>,
}

pub fn snapshot() -> Result<Snapshot, Error> {
    Ok(Snapshot {
        cursor: hypr::cursor_pos()?,
        focus: hypr::active_window()?.map(|window| window.address),
    })
}

fn run_with<S, P, R, E>(
    snapshot: impl FnOnce() -> Result<S, E>,
    prepare: impl FnOnce() -> Result<P, E>,
    focus: impl FnOnce() -> Result<(), E>,
    action: impl FnOnce(&P) -> Result<(), E>,
    restore: impl FnOnce(S, &P, Result<(), E>) -> Result<R, E>,
) -> Result<R, E> {
    let snapshot = snapshot()?;
    let prepared = prepare()?;
    // A panic between the focus change and the restoration would otherwise
    // unwind straight past it, leaving the user's desktop focused on the
    // session window with the cursor parked wherever the action left it. The
    // desktop is put back first, then the panic resumes on its way out.
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        focus().and_then(|()| action(&prepared))
    }));
    match outcome {
        Ok(action) => restore(snapshot, &prepared, action),
        Err(panicked) => {
            let _ = restore(snapshot, &prepared, Ok(()));
            panic::resume_unwind(panicked)
        }
    }
}

pub fn run<P>(
    focus_address: Option<&str>,
    prepare: impl FnOnce() -> Result<P, Error>,
    action: impl FnOnce(&P) -> Result<(), Error>,
    restore_cursor: impl FnOnce(&P, (i32, i32)) -> Result<(), Error>,
) -> Result<(String, String), Error> {
    run_with(
        snapshot,
        prepare,
        || {
            focus_address.map_or(Ok(()), |address| {
                hypr::dispatch(&["focuswindow", &format!("address:{address}")])
            })
        },
        action,
        |snapshot, prepared, action| {
            restore(snapshot, action, |cursor| restore_cursor(prepared, cursor))
        },
    )
}

pub fn restore_cursor(cursor: (i32, i32)) -> Result<(), Error> {
    hypr::dispatch(&["movecursor", &cursor.0.to_string(), &cursor.1.to_string()])
}

pub fn cursor_near(actual: (i32, i32), expected: (i32, i32)) -> bool {
    (actual.0 - expected.0).abs() <= WARP_TOLERANCE
        && (actual.1 - expected.1).abs() <= WARP_TOLERANCE
}

pub fn restore(
    snapshot: Snapshot,
    action: Result<(), Error>,
    restore_cursor: impl FnOnce((i32, i32)) -> Result<(), Error>,
) -> Result<(String, String), Error> {
    let expected_focus = snapshot.focus.as_ref().map_or_else(
        || "no focused window".to_owned(),
        |a| format!("address:{a}"),
    );
    let focus_restore = snapshot.focus.as_deref().map_or(Ok(()), |address| {
        hypr::dispatch(&["focuswindow", &format!("address:{address}")])
    });

    // Hyprland's `focuswindow` dispatcher re-warps the cursor
    // (`ConfigActions.cpp:454-470`, `Window.cpp:1609-1620`), so focus must be
    // restored before the cursor.
    let cursor_restore = restore_cursor(snapshot.cursor);

    let actual_focus = hypr::active_window();
    let actual_cursor = hypr::cursor_pos();
    let focus_matches = matches!(
        (&snapshot.focus, &actual_focus),
        (Some(expected), Ok(Some(actual))) if expected == &actual.address
    ) || matches!((&snapshot.focus, &actual_focus), (None, Ok(None)));
    let cursor_matches = matches!(
        &actual_cursor,
        Ok(actual) if cursor_near(*actual, snapshot.cursor)
    );

    let mut restore = Vec::new();
    if focus_restore.is_err() || !focus_matches {
        let observed = match &actual_focus {
            Ok(Some(actual)) if snapshot.focus.is_none() => format!(
                "address:{}; cannot restore prior no-focus state",
                actual.address
            ),
            Ok(Some(actual)) => format!("address:{}", actual.address),
            Ok(None) => "no focused window".to_owned(),
            Err(error) => format!("verification failed: {error}"),
        };
        let actual = match focus_restore {
            Ok(()) => observed,
            Err(error) => format!("restore failed: {error}; observed {observed}"),
        };
        restore.push(RestoreFailure {
            what: "focus",
            expected: expected_focus,
            actual,
        });
    }

    if cursor_restore.is_err() || !cursor_matches {
        let observed = match &actual_cursor {
            Ok(actual) => format!("{actual:?}"),
            Err(error) => format!("verification failed: {error}"),
        };
        let actual = match cursor_restore {
            Ok(()) => observed,
            Err(error) => format!("restore failed: {error}; observed {observed}"),
        };
        restore.push(RestoreFailure {
            what: "cursor",
            expected: format!("{:?}", snapshot.cursor),
            actual,
        });
    }

    let action = action.err().map(Box::new);
    if action.is_some() || !restore.is_empty() {
        return Err(Error::Guarded { action, restore });
    }

    let cursor_note = actual_cursor.map_or_else(
        |_| String::new(),
        |actual| format!("cursor restored to {actual:?}"),
    );
    let focus_note = snapshot.focus.map_or_else(
        || "no previous focus to restore".to_owned(),
        |address| format!("focus restored to {address}"),
    );
    Ok((cursor_note, focus_note))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::run_with;

    #[test]
    fn guard_composes_snapshot_focus_action_and_restore_in_order() {
        let calls = RefCell::new(Vec::new());

        let result = run_with(
            || {
                calls.borrow_mut().push("snapshot");
                Ok::<_, ()>("desktop")
            },
            || {
                calls.borrow_mut().push("prepare");
                Ok::<_, ()>("target")
            },
            || {
                calls.borrow_mut().push("focus");
                Ok(())
            },
            |target| {
                assert_eq!(*target, "target");
                calls.borrow_mut().push("action");
                Ok(())
            },
            |desktop, target, action| {
                assert_eq!(desktop, "desktop");
                assert_eq!(*target, "target");
                assert_eq!(action, Ok(()));
                calls.borrow_mut().push("restore");
                Ok("restored")
            },
        );

        assert_eq!(result, Ok("restored"));
        assert_eq!(
            calls.into_inner(),
            ["snapshot", "prepare", "focus", "action", "restore"]
        );
    }

    /// The desktop is the user's, whatever happens to the action: a panic must
    /// still give the focus and the cursor back before it leaves the process.
    #[test]
    #[expect(clippy::panic, reason = "the panicking action is what this covers")]
    fn a_panicking_action_still_restores_the_desktop_before_unwinding() {
        let restored = AtomicBool::new(false);

        let panicked = panic::catch_unwind(AssertUnwindSafe(|| {
            run_with(
                || Ok::<_, ()>("desktop"),
                || Ok::<_, ()>("target"),
                || Ok(()),
                |_| panic!("the action blew up"),
                |_, _, _| {
                    restored.store(true, Ordering::SeqCst);
                    Ok("restored")
                },
            )
        }));

        assert!(panicked.is_err(), "the panic must not be swallowed");
        assert!(
            restored.load(Ordering::SeqCst),
            "the desktop was not restored"
        );
    }
}
