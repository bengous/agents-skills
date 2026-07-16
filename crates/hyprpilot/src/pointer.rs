//! Mouse clicks through a native `zwlr_virtual_pointer_v1`, warping over the
//! whole monitor layout (Hyprland maps unbound absolute motion to the layout
//! bounding box), then restoring the user's cursor position and focus.

use std::thread;
use std::time::{Duration, Instant};

use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_pointer, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

use crate::error::Error;
use crate::hypr;
use crate::session;

const VIRTUAL_POINTER_INTERFACE: &str = "zwlr_virtual_pointer_manager_v1";
const BUTTON_GAP: Duration = Duration::from_millis(30);
const WARP_TOLERANCE: i32 = 1;

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    /// Linux input event codes (`BTN_LEFT`…).
    fn code(self) -> u32 {
        match self {
            Self::Left => 0x110,
            Self::Right => 0x111,
            Self::Middle => 0x112,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Middle => "middle",
        }
    }
}

struct State;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(State: ignore ZwlrVirtualPointerManagerV1);
wayland_client::delegate_noop!(State: ignore ZwlrVirtualPointerV1);
wayland_client::delegate_noop!(State: ignore wl_seat::WlSeat);

/// True if the compositor exposes the virtual pointer protocol.
pub fn probe_virtual_pointer() -> Result<bool, Error> {
    let conn = Connection::connect_to_env()
        .map_err(|e| Error::Pointer(format!("connecting to the Wayland display: {e}")))?;
    let (globals, _queue) = registry_queue_init::<State>(&conn)
        .map_err(|e| Error::Pointer(format!("listing Wayland globals: {e}")))?;
    Ok(globals.contents().with_list(|list| {
        list.iter()
            .any(|g| g.interface == VIRTUAL_POINTER_INTERFACE)
    }))
}

struct VirtualPointer {
    pointer: ZwlrVirtualPointerV1,
    queue: EventQueue<State>,
    state: State,
    started: Instant,
    extent: (u32, u32),
    origin: (f64, f64),
}

impl VirtualPointer {
    fn connect(layout: &hypr::LayoutBox) -> Result<Self, Error> {
        let conn = Connection::connect_to_env()
            .map_err(|e| Error::Pointer(format!("connecting to the Wayland display: {e}")))?;
        let (globals, queue) = registry_queue_init::<State>(&conn)
            .map_err(|e| Error::Pointer(format!("listing Wayland globals: {e}")))?;
        let qh = queue.handle();
        let manager: ZwlrVirtualPointerManagerV1 = globals.bind(&qh, 1..=2, ()).map_err(|_| {
            Error::Pointer(format!(
                "compositor does not expose {VIRTUAL_POINTER_INTERFACE}"
            ))
        })?;
        let pointer = manager.create_virtual_pointer(Option::<&wl_seat::WlSeat>::None, &qh, ());
        Ok(Self {
            pointer,
            queue,
            state: State,
            started: Instant::now(),
            extent: (f64_to_u32(layout.width), f64_to_u32(layout.height)),
            origin: (layout.x, layout.y),
        })
    }

    fn timestamp(&self) -> u32 {
        u32::try_from(self.started.elapsed().as_millis()).unwrap_or(u32::MAX)
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.queue
            .roundtrip(&mut self.state)
            .map(|_| ())
            .map_err(|e| Error::Pointer(format!("waiting for the compositor: {e}")))
    }

    /// Warps to global layout coordinates.
    fn warp(&mut self, x: i32, y: i32) -> Result<(), Error> {
        let rel_x = f64::from(x) - self.origin.0;
        let rel_y = f64::from(y) - self.origin.1;
        if rel_x < 0.0
            || rel_y < 0.0
            || rel_x > f64::from(self.extent.0)
            || rel_y > f64::from(self.extent.1)
        {
            return Err(Error::Invalid {
                what: "click target",
                value: format!("({x}, {y})"),
                hint: "outside the monitor layout".to_owned(),
            });
        }
        self.pointer.motion_absolute(
            self.timestamp(),
            f64_to_u32(rel_x),
            f64_to_u32(rel_y),
            self.extent.0,
            self.extent.1,
        );
        self.pointer.frame();
        self.flush()
    }

    fn click(&mut self, button: MouseButton) -> Result<(), Error> {
        self.pointer.button(
            self.timestamp(),
            button.code(),
            wl_pointer::ButtonState::Pressed,
        );
        self.pointer.frame();
        self.flush()?;
        thread::sleep(BUTTON_GAP);
        self.pointer.button(
            self.timestamp(),
            button.code(),
            wl_pointer::ButtonState::Released,
        );
        self.pointer.frame();
        self.flush()
    }
}

impl Drop for VirtualPointer {
    fn drop(&mut self) {
        self.pointer.destroy();
        // Deliver the destroy request; the session is gone anyway if it fails.
        let _ = self.queue.roundtrip(&mut self.state);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are clamped to [0, u32::MAX] before the cast"
)]
fn f64_to_u32(value: f64) -> u32 {
    value.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

fn near(actual: (i32, i32), expected: (i32, i32)) -> bool {
    (actual.0 - expected.0).abs() <= WARP_TOLERANCE
        && (actual.1 - expected.1).abs() <= WARP_TOLERANCE
}

pub fn click(x: i32, y: i32, button: MouseButton, absolute: bool) -> Result<String, Error> {
    let (_, window) = session::current_window()?;
    let (gx, gy) = if absolute {
        (x, y)
    } else {
        if x < 0 || y < 0 || x >= window.size[0] || y >= window.size[1] {
            return Err(Error::Invalid {
                what: "click target",
                value: format!("({x}, {y})"),
                hint: format!(
                    "outside the window (size {}x{}); use --absolute for global coordinates",
                    window.size[0], window.size[1]
                ),
            });
        }
        (window.at[0] + x, window.at[1] + y)
    };

    let monitors = hypr::monitors()?;
    let layout = hypr::layout_box(&monitors)?;
    let before_cursor = hypr::cursor_pos()?;
    let before_active = hypr::active_window()?;

    let mut pointer = VirtualPointer::connect(&layout)?;
    pointer.warp(gx, gy)?;

    // From here the cursor has moved: whatever happens next, always attempt
    // to warp back and re-focus before reporting the primary error.
    let action = verify_and_click(&mut pointer, gx, gy, button);
    let warp_back = pointer.warp(before_cursor.0, before_cursor.1);
    drop(pointer);
    let focus_note = restore_focus(before_active.map(|w| w.address));
    action?;
    warp_back?;
    let focus_note = focus_note?;

    let restored_cursor = hypr::cursor_pos()?;
    let cursor_note = if near(restored_cursor, before_cursor) {
        format!("cursor restored to {restored_cursor:?}")
    } else {
        format!("cursor at {restored_cursor:?}, expected {before_cursor:?}")
    };

    Ok(format!(
        "clicked {} at ({gx}, {gy}) — {cursor_note}, {focus_note}",
        button.label()
    ))
}

fn verify_and_click(
    pointer: &mut VirtualPointer,
    gx: i32,
    gy: i32,
    button: MouseButton,
) -> Result<(), Error> {
    let warped = hypr::cursor_pos()?;
    if !near(warped, (gx, gy)) {
        return Err(Error::Pointer(format!(
            "warp landed at {warped:?} instead of ({gx}, {gy}) — absolute motion mapping mismatch"
        )));
    }
    pointer.click(button)?;
    thread::sleep(BUTTON_GAP);
    Ok(())
}

fn restore_focus(before_address: Option<String>) -> Result<String, Error> {
    let after_address = hypr::active_window()?.map(|w| w.address);
    if before_address == after_address {
        return Ok("focus unchanged".to_owned());
    }
    match before_address {
        Some(address) => {
            hypr::dispatch(&["focuswindow", &format!("address:{address}")])?;
            Ok(format!("focus restored to {address}"))
        }
        None => Ok("no previous focus to restore".to_owned()),
    }
}
