//! What window messages do to pltrs windows: the window procedure's
//! logic, and the per-thread registry of windows it works on.
//!
//! Win32 calls can dispatch messages (sent ones, and those of the
//! window's own changes) and so re-enter the procedure: the registry is
//! never borrowed across one.

use std::cell::RefCell;

use super::convert::scale_factor;
use super::icon::WindowIcon;
use super::keys::{self, AltGrTracker, KeyMessage, KeyRole, Typed};
use super::message::{Message, ScreenRect};
use super::sys::{self, Hwnd, Painter, Restore};
use crate::click::ClickCounter;
use crate::event::{ButtonState, MouseButton, ScrollDelta, WindowEvent};
use crate::geometry::{PhysicalPosition, PhysicalSize, Size};
use crate::registry::Registry;
use crate::surface::Surface;
use crate::window::{CursorIcon, WindowId};

/// How the window procedure answers a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reply {
    /// The system's handling (`DefWindowProcW`).
    Default,
    /// Handled, with this result.
    Handled(isize),
    /// `WM_GETMINMAXINFO` handled: the smallest window size (borders
    /// included) the user may drag the window to.
    MinTrackSize {
        /// The smallest width.
        width: i32,
        /// The smallest height.
        height: i32,
    },
}

/// What the backend keeps for each window.
#[derive(Debug)]
pub(super) struct WindowState {
    /// The window's contents, the size of its client area.
    pub(super) surface: Surface,
    /// The DPI of the window's monitor.
    pub(super) dpi: u32,
    /// The cursor shown over the client area.
    pub(super) cursor: CursorIcon,
    /// The smallest client size the user may drag the window to.
    min_size: Option<Size>,
    /// How to restore the window, while it is full screen.
    pub(super) fullscreen: Option<Restore>,
    /// The window's icon, drawn for its DPI.
    icon: Option<WindowIcon>,
    /// Whether the pointer is over the client area.
    pub(super) pointer_inside: bool,
    focused: bool,
    clicks: ClickCounter,
    alt_gr: AltGrTracker,
}

impl WindowState {
    /// The state of a new window showing `surface`.
    pub(super) fn new(
        surface: Surface,
        dpi: u32,
        min_size: Option<Size>,
        icon: Option<WindowIcon>,
    ) -> Self {
        Self {
            surface,
            dpi,
            cursor: CursorIcon::Default,
            min_size,
            fullscreen: None,
            icon,
            pointer_inside: false,
            focused: false,
            clicks: ClickCounter::new(sys::double_click_time()),
            alt_gr: AltGrTracker::default(),
        }
    }
}

thread_local! {
    /// The windows of this thread.
    static WINDOWS: RefCell<Registry<WindowState>> = const { RefCell::new(Registry::new()) };
}

/// Runs `f` on this thread's windows.
pub(super) fn with_windows<T>(f: impl FnOnce(&mut Registry<WindowState>) -> T) -> T {
    WINDOWS.with_borrow_mut(f)
}

/// Acts on `message` to `window`.
pub(crate) fn handle(window: Hwnd, message: Message) -> Reply {
    let id = window.id();
    let handled = |known: bool| {
        if known {
            Reply::Handled(0)
        } else {
            Reply::Default
        }
    };
    match message {
        Message::Wake => Reply::Handled(0),
        Message::Paint => {
            paint(window);
            Reply::Handled(0)
        }
        Message::EraseBackground => Reply::Handled(1),
        Message::Resized(size) => handled(with_windows(|windows| resized(windows, id, size))),
        Message::DpiChanged { dpi, suggested } => handled(dpi_changed(window, dpi, suggested)),
        Message::MinMaxInfo => min_track_size(window),
        Message::Close => handled(with_windows(|windows| {
            windows.push(id, WindowEvent::CloseRequested)
        })),
        Message::Focus(focused) => {
            with_windows(|windows| focus(windows, id, focused));
            Reply::Default
        }
        Message::SetCursor { client: true } => {
            match with_windows(|windows| windows.state_mut(id).map(|state| state.cursor)) {
                Some(cursor) => {
                    sys::show_cursor(cursor);
                    Reply::Handled(1)
                }
                None => Reply::Default,
            }
        }
        Message::MouseMove(position) => handled(pointer_moved(window, position)),
        Message::MouseLeave => {
            let position = sys::cursor_in_client(window)
                .map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
            handled(with_windows(|windows| pointer_left(windows, id, position)))
        }
        Message::Button {
            button,
            state,
            position,
            any_held,
            time,
        } => {
            let known = button_input(id, button, state, position, time);
            // The window keeps the pointer while a button is held.
            match state {
                ButtonState::Pressed => sys::capture(window),
                ButtonState::Released if !any_held => sys::release_capture(),
                ButtonState::Released => {}
            }
            // X buttons answer TRUE, the others 0.
            let x_button = matches!(button, MouseButton::Back | MouseButton::Forward);
            if known {
                Reply::Handled(isize::from(x_button))
            } else {
                Reply::Default
            }
        }
        Message::Wheel { delta, screen } => handled(wheel(window, delta, screen)),
        Message::Key { key, system } => key_input(window, key, system),
        // Alt+Space opens the system menu; other Alt+characters, which no
        // menu takes, would beep.
        Message::SystemChar(code) if code != usize::from(b' ') => Reply::Handled(0),
        Message::SetCursor { client: false } | Message::SystemChar(_) | Message::Other => {
            Reply::Default
        }
    }
}

/// Paints the invalid part of `window` from its surface.
fn paint(window: Hwnd) {
    let painter = Painter::begin(window);
    let invalid = painter.region();
    let cropped = with_windows(|windows| {
        windows
            .state_mut(window.id())
            .map(|state| state.surface.crop(invalid))
    });
    if let Some((region, pixels)) = cropped {
        painter.draw(region, &pixels);
    }
}

/// Follows a new client size (`None`: minimized, which changes nothing);
/// returns whether the window is known.
fn resized(
    windows: &mut Registry<WindowState>,
    window: WindowId,
    size: Option<PhysicalSize>,
) -> bool {
    let Some(state) = windows.state_mut(window) else {
        return false;
    };
    if let Some(size) = size
        && state.surface.size() != size
        && state.surface.resize(size).is_ok()
    {
        windows.push(window, WindowEvent::Resized(size));
    }
    true
}

/// Follows a new DPI, drawing the icon at its sizes and moving the
/// window to the place the system suggests; returns whether the window
/// is known.
fn dpi_changed(window: Hwnd, dpi: u32, suggested: ScreenRect) -> bool {
    let id = window.id();
    let known = with_windows(|windows| {
        let state = windows.state_mut(id)?;
        let changed = std::mem::replace(&mut state.dpi, dpi) != dpi;
        let icon = changed
            .then(|| state.icon.as_ref().map(|shown| shown.icon().clone()))
            .flatten();
        if changed {
            windows.push(id, WindowEvent::ScaleFactorChanged(scale_factor(dpi)));
        }
        Some(icon)
    });
    let Some(icon) = known else {
        return false;
    };
    // `WM_SETICON` may re-enter the procedure: the registry is not
    // borrowed meanwhile. An icon that cannot be drawn keeps the old one.
    if let Some(Ok(Some(shown))) = icon.map(|icon| WindowIcon::show(&icon, window, dpi)) {
        with_windows(|windows| {
            if let Some(state) = windows.state_mut(id) {
                state.icon = Some(shown);
            }
        });
    }
    // The new client size follows as WM_SIZE.
    window.place(suggested);
    true
}

/// The smallest window size for the minimum client size, if one was set.
fn min_track_size(window: Hwnd) -> Reply {
    let limits = with_windows(|windows| {
        windows
            .state_mut(window.id())
            .map(|state| (state.min_size, state.dpi))
    });
    let Some((Some(min_size), dpi)) = limits else {
        return Reply::Default;
    };
    match window.outer_size(min_size.to_physical(scale_factor(dpi)), dpi) {
        Ok((width, height)) => Reply::MinTrackSize { width, height },
        Err(_) => Reply::Default,
    }
}

fn focus(windows: &mut Registry<WindowState>, window: WindowId, focused: bool) {
    if let Some(state) = windows.state_mut(window)
        && std::mem::replace(&mut state.focused, focused) != focused
    {
        windows.push(window, WindowEvent::Focused(focused));
    }
}

/// Reports pointer motion, and the pointer entering; returns whether the
/// window is known.
fn pointer_moved(window: Hwnd, position: PhysicalPosition) -> bool {
    let id = window.id();
    let held = sys::held_modifiers();
    let entered = with_windows(|windows| {
        let state = windows.state_mut(id)?;
        let modifiers = held.modifiers(state.alt_gr.held());
        let entered = !std::mem::replace(&mut state.pointer_inside, true);
        if entered {
            windows.push(id, WindowEvent::CursorEntered { position });
        }
        let motion = WindowEvent::CursorMoved {
            position,
            modifiers,
        };
        windows.push_motion(id, motion);
        Some(entered)
    });
    if entered == Some(true) {
        sys::track_leave(window);
    }
    entered.is_some()
}

/// Reports the pointer leaving for `position` (client coordinates);
/// returns whether the window is known.
fn pointer_left(
    windows: &mut Registry<WindowState>,
    window: WindowId,
    position: Option<PhysicalPosition>,
) -> bool {
    let Some(state) = windows.state_mut(window) else {
        return false;
    };
    if std::mem::take(&mut state.pointer_inside) {
        windows.push(window, WindowEvent::CursorLeft { position });
    }
    true
}

/// Reports a button press or release; returns whether the window is
/// known.
fn button_input(
    window: WindowId,
    button: MouseButton,
    state: ButtonState,
    position: PhysicalPosition,
    time: u32,
) -> bool {
    let held = sys::held_modifiers();
    with_windows(|windows| {
        let Some(window_state) = windows.state_mut(window) else {
            return false;
        };
        let modifiers = held.modifiers(window_state.alt_gr.held());
        let click_count = if state.is_pressed() {
            window_state.clicks.press(button, time, position)
        } else {
            1
        };
        windows.push(
            window,
            WindowEvent::MouseInput {
                button,
                state,
                position,
                modifiers,
                click_count,
            },
        )
    })
}

/// Reports a wheel turn; returns whether the window is known.
fn wheel(window: Hwnd, delta: ScrollDelta, screen: (i32, i32)) -> bool {
    let (x, y) = sys::screen_to_client(window, screen);
    let position = PhysicalPosition::new(f64::from(x), f64::from(y));
    let held = sys::held_modifiers();
    let id = window.id();
    with_windows(|windows| {
        let Some(state) = windows.state_mut(id) else {
            return false;
        };
        let modifiers = held.modifiers(state.alt_gr.held());
        windows.push(
            id,
            WindowEvent::MouseWheel {
                delta,
                position,
                modifiers,
            },
        )
    })
}

/// Reports a key press or release, with the text it typed.
fn key_input(window: Hwnd, key: KeyMessage, system: bool) -> Reply {
    let id = window.id();
    let next = sys::next_key_message(window);
    let classified = with_windows(|windows| {
        let state = windows.state_mut(id)?;
        let role = state.alt_gr.classify(key, next);
        Some((role, state.alt_gr.held()))
    });
    let Some((role, alt_gr_held)) = classified else {
        return Reply::Default;
    };
    if role != KeyRole::FakeControl {
        let typed = if key.state.is_pressed() {
            sys::take_typed(window)
        } else {
            Typed::default()
        };
        let character = sys::layout_character(key, alt_gr_held);
        let held = sys::held_modifiers();
        let event = keys::key_event(key, role, alt_gr_held, held, character, typed);
        with_windows(|windows| windows.push(id, WindowEvent::Key(event)));
    }
    if system && !keys::hides_system_key(key.vk) {
        Reply::Default
    } else {
        Reply::Handled(0)
    }
}
