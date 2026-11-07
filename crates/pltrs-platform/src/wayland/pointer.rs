//! The pointer: crossing, motion, buttons and scrolling over window
//! content become window events; over client-side decorations they move,
//! resize, minimize, maximize and close the window, and clicks on the
//! title bar do what the desktop's settings say.

use super::cursor::Shape;
use super::decoration::layout::{Hit, TITLE_BAR_HEIGHT};
use super::decoration::{ClientDecorations, Part};
use super::protocol::shell;
use super::protocol::{Event, Interface};
use super::window::Window;
use super::wire::Request;
use super::{WaylandBackend, emit};
use crate::click::ClickCounter;
use crate::decoration::{TitleBarAction, TitleBarButton};
use crate::error::Error;
use crate::event::{ButtonState, Event as LoopEvent, MouseButton, ScrollDelta, WindowEvent};
use crate::geometry::PhysicalPosition;
use crate::window::WindowId;

/// `BTN_LEFT`.
const BTN_LEFT: u32 = 0x110;
/// `BTN_RIGHT`.
const BTN_RIGHT: u32 = 0x111;
/// `BTN_MIDDLE`.
const BTN_MIDDLE: u32 = 0x112;
/// `BTN_SIDE`: "back" on most mice.
const BTN_SIDE: u32 = 0x113;
/// `BTN_EXTRA`: "forward" on most mice.
const BTN_EXTRA: u32 = 0x114;
/// `BTN_FORWARD`.
const BTN_FORWARD: u32 = 0x115;
/// `BTN_BACK`.
const BTN_BACK: u32 = 0x116;
/// One wheel notch in `axis_value120` units.
const NOTCH: f64 = 120.0;
/// The first `wl_pointer` version grouping events into frames.
const FRAMES_VERSION: u32 = 5;

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// The window's content.
    Content,
    /// A part of its client-side decorations.
    Decoration(Part),
}

/// Scrolling gathered over one pointer frame: per axis (vertical,
/// horizontal), surface units and wheel steps in 120ths.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Scroll {
    distance: [f64; 2],
    steps: [i32; 2],
    /// Whether `axis_value120` arrives (then `axis_discrete` is ignored).
    high_resolution: bool,
}

impl Scroll {
    /// Adds continuous scrolling along `axis` (0 vertical, 1 horizontal).
    fn add_distance(&mut self, axis: u32, value: f64) {
        if let Some(distance) = self.distance.get_mut(axis as usize) {
            *distance += value;
        }
    }

    /// Adds wheel notches (`axis_discrete`), unless 120ths arrive.
    fn add_notches(&mut self, axis: u32, notches: i32) {
        if !self.high_resolution {
            self.add_steps(axis, notches.saturating_mul(NOTCH as i32));
        }
    }

    /// Adds wheel steps in 120ths (`axis_value120`).
    fn add_value120(&mut self, axis: u32, value120: i32) {
        if !self.high_resolution {
            self.steps = [0; 2];
            self.high_resolution = true;
        }
        self.add_steps(axis, value120);
    }

    fn add_steps(&mut self, axis: u32, steps: i32) {
        if let Some(total) = self.steps.get_mut(axis as usize) {
            *total = total.saturating_add(steps);
        }
    }

    /// The frame's scroll; starts the next frame.
    fn take(&mut self) -> Self {
        let frame = *self;
        *self = Self {
            high_resolution: self.high_resolution,
            ..Self::default()
        };
        frame
    }
}

/// The seat's pointer.
#[derive(Debug)]
pub(crate) struct Pointer {
    /// The `wl_pointer` object.
    pub(crate) id: u32,
    /// The serial of the latest enter, for cursor changes.
    pub(crate) enter_serial: u32,
    /// The window (surface id) and part the pointer is over.
    pub(crate) focus: Option<(u32, Target)>,
    /// The position in coordinates of the surface entered.
    position: (f64, f64),
    scroll: Scroll,
    clicks: ClickCounter,
}

impl Pointer {
    /// A pointer over nothing yet.
    pub(crate) fn new(id: u32) -> Self {
        Self {
            id,
            enter_serial: 0,
            focus: None,
            position: (0.0, 0.0),
            scroll: Scroll::default(),
            clicks: ClickCounter::default(),
        }
    }
}

/// The crate's button for a Linux `BTN_*` code.
fn mouse_button(code: u32) -> MouseButton {
    match code {
        BTN_LEFT => MouseButton::Left,
        BTN_RIGHT => MouseButton::Right,
        BTN_MIDDLE => MouseButton::Middle,
        BTN_SIDE | BTN_BACK => MouseButton::Back,
        BTN_EXTRA | BTN_FORWARD => MouseButton::Forward,
        other => MouseButton::Other(u16::try_from(other).unwrap_or(u16::MAX)),
    }
}

/// The scroll of a frame, positive up and right; `None` when nothing
/// scrolled.
fn scroll_delta(scroll: Scroll, scale: f64) -> Option<ScrollDelta> {
    let [vertical, horizontal] = scroll.steps;
    if vertical != 0 || horizontal != 0 {
        return Some(ScrollDelta::Lines {
            x: f64::from(horizontal) / NOTCH,
            y: -f64::from(vertical) / NOTCH,
        });
    }
    let [vertical, horizontal] = scroll.distance;
    (vertical != 0.0 || horizontal != 0.0).then(|| ScrollDelta::Pixels {
        x: horizontal * scale,
        y: -vertical * scale,
    })
}

impl WaylandBackend {
    /// Handles an event of the pointer.
    pub(super) fn pointer_event(
        &mut self,
        event: &Event,
        out: &mut Vec<LoopEvent>,
    ) -> Result<(), Error> {
        if self.pointer().is_none() {
            return Ok(());
        }
        match *event {
            Event::PointerEnter {
                serial,
                surface,
                x,
                y,
            } => {
                let focus = self.target(surface);
                if let Some(pointer) = self.pointer_mut() {
                    pointer.enter_serial = serial;
                    pointer.position = (x, y);
                    pointer.focus = focus;
                }
                self.pointer_entered(out)?;
            }
            Event::PointerLeave => {
                let focus = self.pointer_mut().and_then(|pointer| pointer.focus.take());
                self.pointer_left(focus, out)?;
            }
            Event::Motion { x, y } => {
                if let Some(pointer) = self.pointer_mut() {
                    pointer.position = (x, y);
                }
                self.pointer_moved(out)?;
            }
            Event::Button {
                serial,
                time,
                button,
                pressed,
            } => {
                self.note_serial(serial);
                self.pointer_button(serial, time, button, pressed, out)?;
            }
            Event::Axis { axis, value } => {
                self.scroll(out, |scroll| scroll.add_distance(axis, value));
            }
            Event::AxisDiscrete { axis, discrete } => {
                self.scroll(out, |scroll| scroll.add_notches(axis, discrete));
            }
            Event::AxisValue120 { axis, value120 } => {
                self.scroll(out, |scroll| scroll.add_value120(axis, value120));
            }
            Event::Frame => self.pointer_frame(out),
            _ => {}
        }
        Ok(())
    }

    /// The window and part `surface` belongs to.
    fn target(&self, surface: u32) -> Option<(u32, Target)> {
        if self.windows.contains_key(&surface) {
            return Some((surface, Target::Content));
        }
        self.windows.iter().find_map(|(&id, window)| {
            let part = window.decorations.as_ref()?.part(surface)?;
            Some((id, Target::Decoration(part)))
        })
    }

    fn pointer(&self) -> Option<&Pointer> {
        self.seat.as_ref()?.pointer.as_ref()
    }

    fn pointer_mut(&mut self) -> Option<&mut Pointer> {
        self.seat.as_mut()?.pointer.as_mut()
    }

    /// The position of the pointer in buffer pixels of `window`'s content.
    fn content_position(&self, window: u32) -> PhysicalPosition {
        let (x, y) = self
            .pointer()
            .map_or((0.0, 0.0), |pointer| pointer.position);
        self.windows
            .get(&window)
            .map_or(PhysicalPosition::new(x, y), |w| w.scale.position(x, y))
    }

    fn pointer_entered(&mut self, out: &mut Vec<LoopEvent>) -> Result<(), Error> {
        let Some((window, target)) = self.pointer().and_then(|pointer| pointer.focus) else {
            return Ok(());
        };
        if target == Target::Content {
            let position = self.content_position(window);
            if let Some(cursor) = self.windows.get(&window).map(|state| state.cursor) {
                emit(out, window, WindowEvent::CursorEntered { position });
                // The pointer came there by moving, as X11 and Windows
                // report it with the crossing.
                let modifiers = self.modifiers();
                emit(
                    out,
                    window,
                    WindowEvent::CursorMoved {
                        position,
                        modifiers,
                    },
                );
                self.show_cursor(window, Shape::Icon(cursor))?;
            }
            return Ok(());
        }
        self.decoration_hover(window)
    }

    fn pointer_left(
        &mut self,
        focus: Option<(u32, Target)>,
        out: &mut Vec<LoopEvent>,
    ) -> Result<(), Error> {
        let Some((window, target)) = focus else {
            return Ok(());
        };
        let Some(state) = self.windows.get_mut(&window) else {
            return Ok(());
        };
        if target == Target::Content {
            // The compositor does not tell where the pointer went.
            emit(out, window, WindowEvent::CursorLeft { position: None });
        } else if let Some(decorations) = &mut state.decorations
            && decorations.set_buttons(None, None)
        {
            state.request_commit();
            self.commit_window(window, false)?;
        }
        Ok(())
    }

    fn pointer_moved(&mut self, out: &mut Vec<LoopEvent>) -> Result<(), Error> {
        let Some((window, target)) = self.pointer().and_then(|pointer| pointer.focus) else {
            return Ok(());
        };
        if target != Target::Content {
            return self.decoration_hover(window);
        }
        let event = WindowEvent::CursorMoved {
            position: self.content_position(window),
            modifiers: self.modifiers(),
        };
        let id = WindowId::from_raw(u64::from(window));
        if let Some(LoopEvent::Window {
            window: last,
            event: WindowEvent::CursorMoved { .. },
        }) = out.last()
            && *last == id
        {
            out.pop();
        }
        emit(out, window, event);
        Ok(())
    }

    fn pointer_button(
        &mut self,
        serial: u32,
        time: u32,
        code: u32,
        pressed: bool,
        out: &mut Vec<LoopEvent>,
    ) -> Result<(), Error> {
        let Some((window, target)) = self.pointer().and_then(|pointer| pointer.focus) else {
            return Ok(());
        };
        if target != Target::Content {
            return self.decoration_button(window, (serial, time), code, pressed, out);
        }
        let position = self.content_position(window);
        let button = mouse_button(code);
        let state = if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };
        let click_count = match self.pointer_mut() {
            Some(pointer) if pressed => pointer.clicks.press(button, time, position),
            _ => 1,
        };
        let event = WindowEvent::MouseInput {
            button,
            state,
            position,
            modifiers: self.modifiers(),
            click_count,
        };
        emit(out, window, event);
        Ok(())
    }

    /// Gathers scrolling; pointers without frames report it at once.
    fn scroll(&mut self, out: &mut Vec<LoopEvent>, add: impl FnOnce(&mut Scroll)) {
        if let Some(pointer) = self.pointer_mut() {
            add(&mut pointer.scroll);
        }
        let framed = self
            .seat
            .as_ref()
            .is_some_and(|seat| seat.version >= FRAMES_VERSION);
        if !framed {
            self.pointer_frame(out);
        }
    }

    /// Reports the scroll gathered over a frame.
    fn pointer_frame(&mut self, out: &mut Vec<LoopEvent>) {
        let Some(pointer) = self.pointer_mut() else {
            return;
        };
        let scroll = pointer.scroll.take();
        let Some((window, Target::Content)) = pointer.focus else {
            return;
        };
        let Some(scale) = self.windows.get(&window).map(|w| w.scale.factor()) else {
            return;
        };
        if let Some(delta) = scroll_delta(scroll, scale) {
            let event = WindowEvent::MouseWheel {
                delta,
                position: self.content_position(window),
                modifiers: self.modifiers(),
            };
            emit(out, window, event);
        }
    }

    /// What the pointer is over in `window`'s decorations.
    fn decoration_hit(&self, window: u32) -> Option<Hit> {
        let pointer = self.pointer()?;
        let Some((_, Target::Decoration(part))) = pointer.focus else {
            return None;
        };
        let state = self.windows.get(&window)?;
        let layout = state.buttons(&self.desktop.get().layout);
        ClientDecorations::hit(part, pointer.position, state.logical, &layout)
    }

    /// Updates the hovered button and the cursor over the decorations.
    fn decoration_hover(&mut self, window: u32) -> Result<(), Error> {
        let hit = self.decoration_hit(window);
        let hovered = match hit {
            Some(Hit::Button(button)) => Some(button),
            _ => None,
        };
        let shape = match hit {
            Some(Hit::Resize(edges)) => Shape::Resize(edges),
            _ => Shape::Icon(crate::window::CursorIcon::Default),
        };
        self.show_cursor(window, shape)?;
        let Some(state) = self.windows.get_mut(&window) else {
            return Ok(());
        };
        if let Some(decorations) = &mut state.decorations
            && decorations.set_buttons(hovered, decorations.pressed())
        {
            state.request_commit();
            self.commit_window(window, false)?;
        }
        Ok(())
    }

    /// Acts on a press or release over the decorations.
    fn decoration_button(
        &mut self,
        window: u32,
        (serial, time): (u32, u32),
        code: u32,
        pressed: bool,
        out: &mut Vec<LoopEvent>,
    ) -> Result<(), Error> {
        let hit = self.decoration_hit(window);
        let Some(seat) = self.seat.as_ref().map(|seat| seat.id) else {
            return Ok(());
        };
        let position = self
            .pointer()
            .map_or((0.0, 0.0), |pointer| pointer.position);
        let Some(state) = self.windows.get_mut(&window) else {
            return Ok(());
        };
        let toplevel = state.toplevel;
        let held = state
            .decorations
            .as_ref()
            .and_then(ClientDecorations::pressed);
        match (hit, pressed, code) {
            (Some(Hit::Button(button)), true, BTN_LEFT) => {
                if let Some(decorations) = &mut state.decorations {
                    decorations.set_buttons(Some(button), Some(button));
                }
                state.request_commit();
                self.commit_window(window, false)?;
            }
            (_, false, BTN_LEFT) if held.is_some() => {
                let hovered = hit.and_then(|hit| match hit {
                    Hit::Button(button) => Some(button),
                    _ => None,
                });
                if let Some(decorations) = &mut state.decorations {
                    decorations.set_buttons(hovered, None);
                }
                state.request_commit();
                let action = match held.filter(|&button| hovered == Some(button)) {
                    Some(TitleBarButton::Close) => {
                        emit(out, window, WindowEvent::CloseRequested);
                        TitleBarAction::Nothing
                    }
                    Some(TitleBarButton::Maximize) => TitleBarAction::ToggleMaximize,
                    Some(TitleBarButton::Minimize) => TitleBarAction::Minimize,
                    _ => TitleBarAction::Nothing,
                };
                if let Some(request) = title_bar_request(state, action, (seat, serial), position) {
                    self.conn.send(request)?;
                }
                self.commit_window(window, false)?;
            }
            (Some(Hit::Title), true, BTN_LEFT) => {
                let at = PhysicalPosition::new(position.0, position.1);
                let double = self
                    .pointer_mut()
                    .is_some_and(|pointer| pointer.clicks.press(MouseButton::Left, time, at) == 2);
                let action = self.desktop.get().double_click;
                let request = self
                    .windows
                    .get(&window)
                    .filter(|_| double)
                    .and_then(|state| title_bar_request(state, action, (seat, serial), position))
                    .unwrap_or_else(|| shell::start_move(toplevel, seat, serial));
                self.conn.send(request)?;
            }
            (Some(Hit::Title), true, BTN_MIDDLE | BTN_RIGHT) => {
                let settings = self.desktop.get();
                let action = if code == BTN_MIDDLE {
                    settings.middle_click
                } else {
                    settings.right_click
                };
                if let Some(request) = title_bar_request(state, action, (seat, serial), position) {
                    self.conn.send(request)?;
                }
            }
            (Some(Hit::Resize(edges)), true, BTN_LEFT) => {
                self.conn
                    .send(shell::start_resize(toplevel, seat, serial, edges.0))?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Shows `shape` as the cursor over `window`.
    pub(super) fn show_cursor(&mut self, window: u32, shape: Shape) -> Result<(), Error> {
        let Some((pointer, serial)) = self.pointer().map(|p| (p.id, p.enter_serial)) else {
            return Ok(());
        };
        let scale = self
            .windows
            .get(&window)
            .map_or(1, |w| w.scale.factor().ceil() as u32);
        let compositor = self.globals.require(Interface::Compositor)?;
        let shm = self.globals.require(Interface::Shm)?;
        self.cursors.show(
            &mut self.conn,
            (compositor, shm),
            pointer,
            serial,
            shape,
            scale,
        )
    }
}

/// The request carrying out `action` on `window`, for a click (with the
/// seat and serial of its press) at `position` of the title bar, if the
/// compositor can carry it out.
fn title_bar_request(
    window: &Window,
    action: TitleBarAction,
    (seat, serial): (u32, u32),
    position: (f64, f64),
) -> Option<Request> {
    let toplevel = window.toplevel;
    match action {
        TitleBarAction::Nothing => None,
        TitleBarAction::ToggleMaximize => window
            .can_maximize
            .then(|| shell::set_maximized(toplevel, !window.states.maximized)),
        TitleBarAction::Minimize => window.can_minimize.then(|| shell::set_minimized(toplevel)),
        TitleBarAction::Menu => {
            // The position in the window's surface, above which the title
            // bar is.
            let (x, y) = (
                position.0 as i32,
                position.1 as i32 - TITLE_BAR_HEIGHT as i32,
            );
            Some(shell::show_window_menu(toplevel, seat, serial, x, y))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_follow_linux_codes() {
        assert_eq!(mouse_button(BTN_LEFT), MouseButton::Left);
        assert_eq!(mouse_button(BTN_RIGHT), MouseButton::Right);
        assert_eq!(mouse_button(BTN_MIDDLE), MouseButton::Middle);
        assert_eq!(mouse_button(BTN_SIDE), MouseButton::Back);
        assert_eq!(mouse_button(BTN_BACK), MouseButton::Back);
        assert_eq!(mouse_button(BTN_EXTRA), MouseButton::Forward);
        assert_eq!(mouse_button(BTN_FORWARD), MouseButton::Forward);
        assert_eq!(mouse_button(0x117), MouseButton::Other(0x117));
        assert_eq!(mouse_button(u32::MAX), MouseButton::Other(u16::MAX));
    }

    #[test]
    fn scrolling_prefers_wheel_steps() {
        let wheel = Scroll {
            distance: [15.0, 0.0],
            steps: [120, -60],
            high_resolution: true,
        };
        assert_eq!(
            scroll_delta(wheel, 2.0),
            Some(ScrollDelta::Lines { x: -0.5, y: -1.0 }),
            "down one notch, left half a notch"
        );
        let touchpad = Scroll {
            distance: [-3.0, 1.5],
            ..Scroll::default()
        };
        assert_eq!(
            scroll_delta(touchpad, 2.0),
            Some(ScrollDelta::Pixels { x: 3.0, y: 6.0 })
        );
        assert_eq!(scroll_delta(Scroll::default(), 1.0), None);
    }
}
