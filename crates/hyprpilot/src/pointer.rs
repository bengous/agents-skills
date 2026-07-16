//! Mouse clicks and wheel scrolls through a native `zwlr_virtual_pointer_v1`,
//! warping over the whole monitor layout (Hyprland maps unbound absolute
//! motion to the layout bounding box), then restoring the user's cursor
//! position and focus.

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
/// Under the usual 300-500 ms multi-click interval of GUI toolkits.
const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(80);
const DETENT_GAP: Duration = Duration::from_millis(20);
/// One standard wheel detent in `wl_pointer` continuous-axis units.
const DETENT_VALUE: f64 = 15.0;
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
                what: "target",
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

    fn double_click(&mut self, button: MouseButton) -> Result<(), Error> {
        self.click(button)?;
        thread::sleep(DOUBLE_CLICK_GAP);
        self.click(button)
    }

    /// Emits one `axis_discrete` + `frame` per wheel detent, paced like a
    /// real wheel.
    fn scroll(&mut self, plan: &[(wl_pointer::Axis, f64, i32)]) -> Result<(), Error> {
        for (index, &(axis, value, discrete)) in plan.iter().enumerate() {
            if index > 0 {
                thread::sleep(DETENT_GAP);
            }
            self.pointer
                .axis_discrete(self.timestamp(), axis, value, discrete);
            self.pointer.frame();
            self.flush()?;
        }
        Ok(())
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

pub fn click(
    x: i32,
    y: i32,
    button: MouseButton,
    double: bool,
    absolute: bool,
) -> Result<String, Error> {
    let (_, window) = session::current_window()?;
    let (gx, gy) = resolve_target(&window, x, y, absolute)?;
    let (cursor_note, focus_note) = at_target(gx, gy, |pointer| {
        if double {
            pointer.double_click(button)
        } else {
            pointer.click(button)
        }
    })?;
    let verb = if double { "double-clicked" } else { "clicked" };
    Ok(format!(
        "{verb} {} at ({gx}, {gy}) — {cursor_note}, {focus_note}",
        button.label()
    ))
}

pub fn scroll(x: i32, y: i32, dx: i32, dy: i32, absolute: bool) -> Result<String, Error> {
    let plan = detent_plan(dx, dy)?;
    let (_, window) = session::current_window()?;
    let (gx, gy) = resolve_target(&window, x, y, absolute)?;
    let (cursor_note, focus_note) = at_target(gx, gy, |pointer| pointer.scroll(&plan))?;
    let mut amounts = Vec::new();
    if dy != 0 {
        amounts.push(format!("dy {dy}"));
    }
    if dx != 0 {
        amounts.push(format!("dx {dx}"));
    }
    Ok(format!(
        "scrolled {} at ({gx}, {gy}) — {cursor_note}, {focus_note}",
        amounts.join(", ")
    ))
}

/// Maps window-relative (or `--absolute` global) coordinates to global layout
/// coordinates, rejecting relative targets outside the window.
fn resolve_target(
    window: &hypr::Client,
    x: i32,
    y: i32,
    absolute: bool,
) -> Result<(i32, i32), Error> {
    if absolute {
        return Ok((x, y));
    }
    if x < 0 || y < 0 || x >= window.size[0] || y >= window.size[1] {
        return Err(Error::Invalid {
            what: "target",
            value: format!("({x}, {y})"),
            hint: format!(
                "outside the window (size {}x{}); use --absolute for global coordinates",
                window.size[0], window.size[1]
            ),
        });
    }
    Ok((window.at[0] + x, window.at[1] + y))
}

/// One `(axis, continuous value, discrete steps)` entry per wheel detent,
/// vertical detents first. Positive = down / right (`wl_pointer` convention).
fn detent_plan(dx: i32, dy: i32) -> Result<Vec<(wl_pointer::Axis, f64, i32)>, Error> {
    if dx == 0 && dy == 0 {
        return Err(Error::Invalid {
            what: "scroll amount",
            value: format!("dy {dy}, dx {dx}"),
            hint: "at least one wheel detent is required (positive = down/right)".to_owned(),
        });
    }
    let mut plan = Vec::new();
    for (axis, detents) in [
        (wl_pointer::Axis::VerticalScroll, dy),
        (wl_pointer::Axis::HorizontalScroll, dx),
    ] {
        let step = detents.signum();
        for _ in 0..detents.abs() {
            plan.push((axis, DETENT_VALUE * f64::from(step), step));
        }
    }
    Ok(plan)
}

/// Shared warp envelope: records the user's cursor and focus, warps to the
/// target, verifies the landing position, runs `act`, then always attempts to
/// warp back and re-focus — even when `act` fails — before reporting the
/// primary error. Returns the cursor and focus restoration notes.
fn at_target(
    gx: i32,
    gy: i32,
    act: impl FnOnce(&mut VirtualPointer) -> Result<(), Error>,
) -> Result<(String, String), Error> {
    let monitors = hypr::monitors()?;
    let layout = hypr::layout_box(&monitors)?;
    let before_cursor = hypr::cursor_pos()?;
    let before_active = hypr::active_window()?;

    let mut pointer = VirtualPointer::connect(&layout)?;
    pointer.warp(gx, gy)?;

    // From here the cursor has moved: whatever happens next, always attempt
    // to warp back and re-focus before reporting the primary error.
    let action = verify_and_act(&mut pointer, gx, gy, act);
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
    Ok((cursor_note, focus_note))
}

fn verify_and_act(
    pointer: &mut VirtualPointer,
    gx: i32,
    gy: i32,
    act: impl FnOnce(&mut VirtualPointer) -> Result<(), Error>,
) -> Result<(), Error> {
    let warped = hypr::cursor_pos()?;
    if !near(warped, (gx, gy)) {
        return Err(Error::Pointer(format!(
            "warp landed at {warped:?} instead of ({gx}, {gy}) — absolute motion mapping mismatch"
        )));
    }
    act(pointer)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> hypr::Client {
        hypr::Client {
            address: "0xabc".to_owned(),
            at: [100, 200],
            size: [800, 600],
            workspace: hypr::WorkspaceRef {
                name: "special:pilot".to_owned(),
            },
            class: String::new(),
            title: "App".to_owned(),
            pid: 1,
        }
    }

    #[test]
    fn resolve_target_offsets_relative_coordinates_by_window_position() {
        assert_eq!(
            resolve_target(&window(), 10, 20, false).unwrap(),
            (110, 220)
        );
        assert_eq!(resolve_target(&window(), 0, 0, false).unwrap(), (100, 200));
        assert_eq!(
            resolve_target(&window(), 799, 599, false).unwrap(),
            (899, 799)
        );
    }

    #[test]
    fn resolve_target_passes_absolute_coordinates_through_unchecked() {
        assert_eq!(
            resolve_target(&window(), 5000, -3, true).unwrap(),
            (5000, -3)
        );
    }

    #[test]
    fn resolve_target_rejects_relative_coordinates_outside_the_window() {
        for (x, y) in [(-1, 0), (0, -1), (800, 0), (0, 600)] {
            assert!(matches!(
                resolve_target(&window(), x, y, false),
                Err(Error::Invalid { .. })
            ));
        }
    }

    #[test]
    fn detent_plan_rejects_zero_detents() {
        assert!(matches!(detent_plan(0, 0), Err(Error::Invalid { .. })));
    }

    #[test]
    fn detent_plan_emits_one_entry_per_detent_with_the_sign_carried() {
        let plan = detent_plan(0, 2).unwrap();
        assert_eq!(
            plan,
            vec![
                (wl_pointer::Axis::VerticalScroll, 15.0, 1),
                (wl_pointer::Axis::VerticalScroll, 15.0, 1),
            ]
        );
        let plan = detent_plan(-1, 0).unwrap();
        assert_eq!(plan, vec![(wl_pointer::Axis::HorizontalScroll, -15.0, -1)]);
    }

    #[test]
    fn detent_plan_orders_vertical_before_horizontal() {
        let plan = detent_plan(1, -2).unwrap();
        assert_eq!(
            plan,
            vec![
                (wl_pointer::Axis::VerticalScroll, -15.0, -1),
                (wl_pointer::Axis::VerticalScroll, -15.0, -1),
                (wl_pointer::Axis::HorizontalScroll, 15.0, 1),
            ]
        );
    }
}
