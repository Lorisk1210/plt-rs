//! What `AppKit`'s calls on pltrs's views do: drawing from the window's
//! surface, and input and window changes queued as events, on the
//! per-thread registry of windows.
//!
//! These calls come from within the `AppKit` calls the backend makes
//! (dispatching an event, resizing, showing or closing a window), so the
//! registry is never borrowed across one.

use std::cell::RefCell;

use super::convert::{PointRect, pixel_size};
use super::keys::{self, KeyKind, KeyPress};
use super::pointer::{self, Pointer};
use super::sys::{EventRef, Image, ViewRef};
use crate::event::{ButtonState, MouseButton, WindowEvent};
use crate::registry::Registry;
use crate::surface::Surface;
use crate::window::CursorIcon;

/// What the backend keeps for each window.
#[derive(Debug)]
pub(super) struct ViewState {
    /// The window's contents, the size of its view in pixels.
    pub(super) surface: Surface,
    /// The window's scale factor (`backingScaleFactor`).
    pub(super) scale: f64,
    /// The cursor shown over the view.
    pub(super) cursor: CursorIcon,
    /// Whether the pointer is over the view.
    pub(super) pointer_inside: bool,
    /// The button the left button's last press stood for (see
    /// [`pointer::emulated_button`]), which its release reports too.
    left_button: MouseButton,
    focused: bool,
}

impl ViewState {
    /// The state of a new window showing `surface` at `scale`.
    pub(super) fn new(surface: Surface, scale: f64) -> Self {
        Self {
            surface,
            scale,
            cursor: CursorIcon::Default,
            pointer_inside: false,
            left_button: MouseButton::Left,
            focused: false,
        }
    }
}

thread_local! {
    /// The windows of this thread (the main thread: `AppKit`'s only one).
    static VIEWS: RefCell<Registry<ViewState>> = const { RefCell::new(Registry::new()) };
}

/// Runs `f` on this thread's windows.
pub(super) fn with_views<T>(f: impl FnOnce(&mut Registry<ViewState>) -> T) -> T {
    VIEWS.with_borrow_mut(f)
}

/// A change of a window, which its delegate (the view) hears of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowChange {
    /// The window was resized.
    Resized,
    /// The window became (`true`) or stopped being (`false`) the key
    /// window, the one with the keyboard focus.
    Focused(bool),
    /// The window's backing store changed: its scale factor may have.
    Backing,
}

/// `drawRect:`: draws the `dirty` part of `view` from the surface.
pub(crate) fn draw(view: ViewRef<'_>, dirty: PointRect) {
    let cropped = with_views(|views| {
        let state = views.state_mut(view.id())?;
        Some((
            state.scale,
            state.surface.crop(dirty.to_pixels(state.scale)),
        ))
    });
    let Some((scale, (region, pixels))) = cropped else {
        return;
    };
    if let Some(image) = Image::opaque_bgra(region.width, region.height, pixels) {
        image.draw(PointRect::from_pixels(region, scale));
    }
}

/// The mouse methods: buttons, motion, entering and leaving.
pub(crate) fn pointer(view: ViewRef<'_>, event: EventRef<'_>) {
    let Some(kind) = pointer::pointer(event.kind(), event.button_number()) else {
        return;
    };
    let (x, y) = view.location(event);
    let modifiers = keys::modifiers(event.modifier_flags());
    let click_count = u32::try_from(event.click_count()).unwrap_or(1).max(1);
    let id = view.id();
    with_views(|views| {
        let Some(state) = views.state_mut(id) else {
            return;
        };
        let position = pointer::position(x, y, state.scale);
        let reported = match kind {
            Pointer::Button {
                button,
                state: pressed,
            } => {
                let button = match (button, pressed) {
                    (MouseButton::Left, ButtonState::Pressed) => {
                        state.left_button = pointer::emulated_button(modifiers);
                        state.left_button
                    }
                    (MouseButton::Left, ButtonState::Released) => state.left_button,
                    (other, _) => other,
                };
                WindowEvent::MouseInput {
                    button,
                    state: pressed,
                    position,
                    modifiers,
                    click_count: if pressed.is_pressed() { click_count } else { 1 },
                }
            }
            Pointer::Moved => {
                views.push_motion(
                    id,
                    WindowEvent::CursorMoved {
                        position,
                        modifiers,
                    },
                );
                return;
            }
            Pointer::Entered if !std::mem::replace(&mut state.pointer_inside, true) => {
                WindowEvent::CursorEntered { position }
            }
            Pointer::Exited if std::mem::take(&mut state.pointer_inside) => WindowEvent::CursorLeft,
            Pointer::Entered | Pointer::Exited => return,
        };
        views.push(id, reported);
    });
}

/// `keyDown:`, `keyUp:` and `flagsChanged:`.
pub(crate) fn key(view: ViewRef<'_>, event: EventRef<'_>) {
    let code = event.key_code();
    let flags = event.modifier_flags();
    let reported = match keys::key_kind(event.kind()) {
        Some(KeyKind::Press(state)) => {
            let unmodified = event.characters_ignoring_modifiers();
            let typed = event.characters();
            Some(keys::key_event(KeyPress {
                code,
                state,
                repeat: event.is_repeat(),
                flags,
                unmodified: &unmodified,
                typed: &typed,
            }))
        }
        Some(KeyKind::FlagsChanged) => keys::flags_changed(code, flags),
        None => None,
    };
    if let Some(key) = reported {
        with_views(|views| views.push(view.id(), WindowEvent::Key(key)));
    }
}

/// `scrollWheel:`.
pub(crate) fn scroll(view: ViewRef<'_>, event: EventRef<'_>) {
    let (dx, dy, precise) = event.scroll();
    let (x, y) = view.location(event);
    let modifiers = keys::modifiers(event.modifier_flags());
    let id = view.id();
    with_views(|views| {
        let Some(state) = views.state_mut(id) else {
            return;
        };
        let position = pointer::position(x, y, state.scale);
        if let Some(delta) = pointer::scroll(dx, dy, precise, state.scale) {
            views.push(
                id,
                WindowEvent::MouseWheel {
                    delta,
                    position,
                    modifiers,
                },
            );
        }
    });
}

/// `resetCursorRects`: the window's cursor over the whole view.
pub(crate) fn reset_cursor_rects(view: ViewRef<'_>) {
    let cursor = with_views(|views| views.state_mut(view.id()).map(|state| state.cursor));
    if let Some(cursor) = cursor {
        view.add_cursor_rect(view.bounds(), cursor);
    }
}

/// The window notifications: focus, size and scale changes.
pub(crate) fn window_changed(view: ViewRef<'_>, change: WindowChange) {
    let id = view.id();
    let WindowChange::Focused(focused) = change else {
        let scale = view.scale_factor();
        let bounds = view.bounds();
        let size = pixel_size(bounds.width, bounds.height, scale);
        with_views(|views| {
            let Some(state) = views.state_mut(id) else {
                return;
            };
            let rescaled = state.scale.to_bits() != scale.to_bits();
            state.scale = scale;
            let resized = state.surface.size() != size && state.surface.resize(size).is_ok();
            if rescaled {
                views.push(id, WindowEvent::ScaleFactorChanged(scale));
            }
            if resized {
                views.push(id, WindowEvent::Resized(size));
            }
        });
        return;
    };
    with_views(|views| {
        if let Some(state) = views.state_mut(id)
            && std::mem::replace(&mut state.focused, focused) != focused
        {
            views.push(id, WindowEvent::Focused(focused));
        }
    });
}

/// `windowShouldClose:`: the user asked to close the window.
pub(crate) fn close_requested(view: ViewRef<'_>) {
    with_views(|views| views.push(view.id(), WindowEvent::CloseRequested));
}

/// `applicationShouldTerminate:`: the user asked to quit (⌘Q, the Dock):
/// every window is asked to close.
pub(crate) fn quit_requested() {
    with_views(|views| {
        for window in views.all_windows() {
            views.push(window, WindowEvent::CloseRequested);
        }
    });
}
