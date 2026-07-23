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
