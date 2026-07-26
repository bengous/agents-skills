//! Mouse clicks and wheel scrolls through a native `zwlr_virtual_pointer_v1`,
//! warping over the whole monitor layout (Hyprland maps unbound absolute
//! motion to the layout bounding box), then restoring the user's cursor
//! position and focus.
//!
//! In an isolated session the pointer is created on the nested compositor of
//! the agent desktop instead, over its own single-output layout, and nothing is
//! restored: that seat has no human on it (`Route` and `Seat` below).

use std::env;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
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
use crate::guard;
use crate::hypr::{self, Ctl};
use crate::session::{self, Instance, Isolated, ModeState};

const VIRTUAL_POINTER_INTERFACE: &str = "zwlr_virtual_pointer_manager_v1";
const BUTTON_GAP: Duration = Duration::from_millis(30);
/// Under the usual 300-500 ms multi-click interval of GUI toolkits.
const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(80);
const DETENT_GAP: Duration = Duration::from_millis(20);
/// One standard wheel detent in `wl_pointer` continuous-axis units.
const DETENT_VALUE: f64 = 15.0;

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

/// Which compositor the virtual pointer is created on: the user's session, from
/// the environment, or an agent desktop's nested compositor, by socket path.
/// The socket is passed explicitly because the process environment is never
/// modified — a shared-mode command in this same binary must keep seeing the
/// user's `WAYLAND_DISPLAY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seat<'a> {
    User,
    Agent(&'a Path),
}

impl Seat<'_> {
    fn connect(self) -> Result<Connection, Error> {
        match self {
            Self::User => Connection::connect_to_env()
                .map_err(|e| Error::Pointer(format!("connecting to the Wayland display: {e}"))),
            Self::Agent(socket) => {
                let stream = UnixStream::connect(socket).map_err(|e| {
                    Error::Pointer(format!(
                        "connecting to the agent desktop socket {}: {e}",
                        socket.display()
                    ))
                })?;
                Connection::from_socket(stream).map_err(|e| {
                    Error::Pointer(format!(
                        "opening a Wayland connection on {}: {e}",
                        socket.display()
                    ))
                })
            }
        }
    }
}

/// True if the compositor exposes the virtual pointer protocol.
pub fn probe_virtual_pointer() -> Result<bool, Error> {
    let conn = Seat::User.connect()?;
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
    fn connect(seat: Seat<'_>, layout: &hypr::LayoutBox) -> Result<Self, Error> {
        let conn = seat.connect()?;
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

/// Which seat the pointer acts on, and whose layout the target coordinates
/// belong to. An isolated session resolves every read and every pointer event
/// to its own nested compositor: the user's cursor is never moved.
enum Route {
    Shared {
        window: hypr::Client,
    },
    Isolated {
        signature: String,
        socket: PathBuf,
        window: hypr::Client,
    },
}

/// The nested compositor an isolated session drives, read from its state alone.
#[derive(Debug, PartialEq, Eq)]
struct AgentTarget {
    signature: String,
    socket: PathBuf,
    address: String,
}

impl AgentTarget {
    fn resolve(name: &str, isolated: &Isolated, runtime_dir: &Path) -> Result<Self, Error> {
        let Instance::Live {
            signature,
            wayland_display,
            ..
        } = &isolated.instance
        else {
            return Err(instance_pending(name));
        };
        let address = isolated
            .active_address
            .clone()
            .ok_or_else(|| no_target(name))?;
        Ok(Self {
            signature: signature.clone(),
            socket: socket_path(runtime_dir, wayland_display),
            address,
        })
    }
}

impl Route {
    fn resolve(name: &str, pending: session::Pending) -> Result<Self, Error> {
        match session::load(name)?.state {
            // Shared mode goes through `session::current_window`, which re-reads
            // the state and stays the single definition of the window a session
            // drives on the user's desktop.
            ModeState::Shared(_) => {
                let (_, window) = session::current_window(name, pending)?;
                Ok(Self::Shared { window })
            }
            ModeState::Isolated(isolated) => {
                let target = AgentTarget::resolve(name, &isolated, &runtime_root()?)?;
                let window = agent_window(&target)?;
                Ok(Self::Isolated {
                    signature: target.signature,
                    socket: target.socket,
                    window,
                })
            }
        }
    }

    fn window(&self) -> &hypr::Client {
        match self {
            Self::Shared { window } | Self::Isolated { window, .. } => window,
        }
    }

    fn ctl(&self) -> Ctl<'_> {
        match self {
            Self::Shared { .. } => Ctl::Host,
            Self::Isolated { signature, .. } => Ctl::Instance(signature),
        }
    }

    fn seat(&self) -> Seat<'_> {
        match self {
            Self::Shared { .. } => Seat::User,
            Self::Isolated { socket, .. } => Seat::Agent(socket),
        }
    }

    /// The window to focus before acting, if any. `--focus` is accepted and
    /// ignored on an agent desktop: its target window already is the focused
    /// window of that seat.
    fn focus_address(&self, focus: bool) -> Option<&str> {
        match self {
            Self::Shared { window } => focus.then_some(window.address.as_str()),
            Self::Isolated { .. } => None,
        }
    }
}

/// The nested compositor's socket sits directly in `$XDG_RUNTIME_DIR`, next to
/// the user's own, not under the crate's session directory.
fn runtime_root() -> Result<PathBuf, Error> {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or(Error::Env("XDG_RUNTIME_DIR"))
}

fn socket_path(runtime_dir: &Path, wayland_display: &str) -> PathBuf {
    runtime_dir.join(wayland_display)
}

/// The target window as the nested compositor sees it: `at` and `size` are then
/// in the agent desktop's layout, which is what the warp needs.
fn agent_window(target: &AgentTarget) -> Result<hypr::Client, Error> {
    hypr::clients_on(Ctl::Instance(&target.signature))?
        .into_iter()
        .find(|client| client.address == target.address)
        .ok_or_else(|| Error::WindowGone(target.address.clone()))
}

/// An interrupted `session start --isolated` leaves a state with no nested
/// compositor: refuse it, never fall back to the user's seat.
fn instance_pending(name: &str) -> Error {
    Error::Invalid {
        what: "agent desktop",
        value: name.to_owned(),
        hint: format!(
            "its nested compositor was never spawned (`session start --isolated` did not \
             finish); run `hyprpilot --session {name} teardown`"
        ),
    }
}

fn no_target(name: &str) -> Error {
    Error::Invalid {
        what: "agent desktop",
        value: name.to_owned(),
        hint: format!(
            "no window is tracked in it, its app never appeared; run \
             `hyprpilot --session {name} teardown`"
        ),
    }
}

pub fn click(
    name: &str,
    x: i32,
    y: i32,
    button: MouseButton,
    double: bool,
    absolute: bool,
    focus: bool,
) -> Result<String, Error> {
    let route = Route::resolve(name, session::Pending::CLICK)?;
    let (gx, gy) = resolve_target(route.window(), x, y, absolute)?;
    let note = at_target(&route, gx, gy, focus, |pointer| {
        if double {
            pointer.double_click(button)
        } else {
            pointer.click(button)
        }
    })?;
    let verb = if double { "double-clicked" } else { "clicked" };
    Ok(format!(
        "{verb} {} at ({gx}, {gy}) — {note}",
        button.label()
    ))
}

pub fn scroll(
    name: &str,
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    absolute: bool,
    focus: bool,
) -> Result<String, Error> {
    let plan = detent_plan(dx, dy)?;
    let route = Route::resolve(name, session::Pending::SCROLL)?;
    let (gx, gy) = resolve_target(route.window(), x, y, absolute)?;
    let note = at_target(&route, gx, gy, focus, |pointer| pointer.scroll(&plan))?;
    let mut amounts = Vec::new();
    if dy != 0 {
        amounts.push(format!("dy {dy}"));
    }
    if dx != 0 {
        amounts.push(format!("dx {dx}"));
    }
    Ok(format!(
        "scrolled {} at ({gx}, {gy}) — {note}",
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

/// Warps to the target and acts there, returning the note the command reports.
/// On the user's seat everything runs inside the guard, which restores cursor
/// and focus; on an agent desktop's seat there is no human, so nothing is
/// snapshotted and nothing is restored.
fn at_target(
    route: &Route,
    gx: i32,
    gy: i32,
    focus: bool,
    act: impl FnOnce(&mut VirtualPointer) -> Result<(), Error>,
) -> Result<String, Error> {
    let seat = route.seat();
    let ctl = route.ctl();
    match route {
        Route::Shared { .. } => {
            let (cursor_note, focus_note) = guard::run(
                route.focus_address(focus),
                || {
                    let monitors = hypr::monitors()?;
                    hypr::layout_box(&monitors)
                },
                |layout| {
                    let mut pointer = VirtualPointer::connect(seat, layout)?;
                    pointer.warp(gx, gy)?;
                    verify_and_act(&mut pointer, ctl, gx, gy, act)
                },
                |layout, cursor| {
                    let mut pointer = VirtualPointer::connect(seat, layout)?;
                    pointer.warp(cursor.0, cursor.1)
                },
            )?;
            Ok(format!("{cursor_note}, {focus_note}"))
        }
        Route::Isolated { .. } => {
            let monitors = hypr::monitors_on(ctl)?;
            let layout = hypr::layout_box(&monitors)?;
            let mut pointer = VirtualPointer::connect(seat, &layout)?;
            pointer.warp(gx, gy)?;
            verify_and_act(&mut pointer, ctl, gx, gy, act)?;
            Ok(format!(
                "cursor left at ({gx}, {gy}) on the agent desktop seat"
            ))
        }
    }
}

fn verify_and_act(
    pointer: &mut VirtualPointer,
    ctl: Ctl<'_>,
    gx: i32,
    gy: i32,
    act: impl FnOnce(&mut VirtualPointer) -> Result<(), Error>,
) -> Result<(), Error> {
    let warped = hypr::cursor_pos_on(ctl)?;
    if !guard::cursor_near(warped, (gx, gy)) {
        return Err(Error::Pointer(format!(
            "warp landed at {warped:?} instead of ({gx}, {gy}) — absolute motion mapping mismatch"
        )));
    }
    act(pointer)?;
    thread::sleep(BUTTON_GAP);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::*;

    const SIGNATURE: &str = "abcdef_1730000000";
    const RUNTIME_DIR: &str = "/run/user/1000";

    fn window() -> hypr::Client {
        hypr::Client {
            address: "0xabc".to_owned(),
            at: [100, 200],
            size: [800, 600],
            workspace: hypr::WorkspaceRef {
                name: "special:pilot".to_owned(),
            },
            floating: false,
            monitor: 0,
            class: String::new(),
            initial_class: String::new(),
            title: "App".to_owned(),
            initial_title: "App".to_owned(),
            pid: 1,
        }
    }

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
            socket: PathBuf::from("/run/user/1000/wayland-2"),
            window: window(),
        }
    }

    #[test]
    fn the_agent_socket_sits_beside_the_users_own_in_the_runtime_dir() {
        assert_eq!(
            socket_path(Path::new(RUNTIME_DIR), "wayland-2"),
            PathBuf::from("/run/user/1000/wayland-2")
        );
    }

    #[test]
    fn an_agent_target_carries_the_instance_signature_socket_and_window()
    -> Result<(), Box<dyn StdError>> {
        let state = agent_state(live(), Some("0xdead"));
        assert_eq!(
            AgentTarget::resolve("alpha", &state, Path::new(RUNTIME_DIR))?,
            AgentTarget {
                signature: SIGNATURE.to_owned(),
                socket: PathBuf::from("/run/user/1000/wayland-2"),
                address: "0xdead".to_owned(),
            }
        );
        Ok(())
    }

    #[test]
    fn an_agent_desktop_pointer_is_created_on_its_own_socket_and_read_from_its_own_instance() {
        let route = agent_route();
        assert_eq!(
            route.seat(),
            Seat::Agent(Path::new("/run/user/1000/wayland-2"))
        );
        assert_eq!(route.ctl(), Ctl::Instance(SIGNATURE));

        let shared = Route::Shared { window: window() };
        assert_eq!(shared.seat(), Seat::User);
        assert_eq!(shared.ctl(), Ctl::Host);
    }

    #[test]
    fn focus_is_a_no_op_on_an_agent_desktop_seat() {
        assert_eq!(agent_route().focus_address(true), None);
        assert_eq!(agent_route().focus_address(false), None);
        let shared = Route::Shared { window: window() };
        assert_eq!(shared.focus_address(true), Some("0xabc"));
        assert_eq!(shared.focus_address(false), None);
    }

    #[test]
    fn a_pending_instance_is_refused_with_a_teardown_hint() -> Result<(), Box<dyn StdError>> {
        let state = agent_state(Instance::Pending, Some("0xdead"));
        let Err(error) = AgentTarget::resolve("alpha", &state, Path::new(RUNTIME_DIR)) else {
            return Err("a pending instance has no seat to act on".into());
        };
        assert!(
            matches!(
                error,
                Error::Invalid {
                    what: "agent desktop",
                    ..
                }
            ),
            "{error:?}"
        );
        let message = error.to_string();
        assert!(message.contains("never spawned"), "{message}");
        assert!(message.contains("--session alpha teardown"), "{message}");
        Ok(())
    }

    #[test]
    fn a_live_instance_without_a_tracked_window_is_refused() -> Result<(), Box<dyn StdError>> {
        let state = agent_state(live(), None);
        let Err(error) = AgentTarget::resolve("alpha", &state, Path::new(RUNTIME_DIR)) else {
            return Err("no window address means no target".into());
        };
        let message = error.to_string();
        assert!(message.contains("no window is tracked"), "{message}");
        assert!(message.contains("--session alpha teardown"), "{message}");
        Ok(())
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
