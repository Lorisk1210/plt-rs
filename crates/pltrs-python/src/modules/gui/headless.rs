//! Hosts without a display, for tests: what a window system would send
//! their windows (pointer, keys, close requests) and what their windows
//! show. Windows are the display's (`window_system_id` of a hosted
//! window, or one of `windows`, the file chooser and help windows
//! included), by their raw numbers.

use pltrs::gui::{HostedWindowId, WindowHost};
use pltrs_platform::{
    ButtonState, HeadlessHandle, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition,
    ScrollDelta, WindowEvent, WindowId,
};

use crate::binding::prelude::*;
use crate::binding::wrap_native;

impl IntoPy for HeadlessHandle {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        wrap_native(py, self)
    }
}

/// A host without a display and the handle driving its windows.
fn connect_headless() -> PyResult<(WindowHost, HeadlessHandle)> {
    Ok(WindowHost::headless())
}

/// The raw number of the display's window showing hosted window
/// `window`, `None` until it is shown.
fn window_system_id(host: Native<WindowHost>, window: u64) -> PyResult<Option<u64>> {
    Ok(host
        .borrow()
        .window_system_id(HostedWindowId::from_raw(window))
        .map(WindowId::to_raw))
}

/// The raw numbers of the display's open windows, bottom to top.
fn windows(handle: Native<HeadlessHandle>) -> PyResult<Vec<u64>> {
    Ok(handle
        .borrow()
        .stacking_order()
        .into_iter()
        .map(WindowId::to_raw)
        .collect())
}

/// The display's open window with the raw number `raw`.
fn find(handle: &HeadlessHandle, raw: u64) -> PyResult<WindowId> {
    handle
        .windows()
        .into_iter()
        .find(|window| window.to_raw() == raw)
        .ok_or_else(|| PyErr::runtime_error(format!("no window {raw} is open")))
}

/// Makes the display send `event` to its window `window`.
fn send(handle: &Native<HeadlessHandle>, window: u64, event: WindowEvent) -> PyResult<()> {
    let handle = handle.borrow();
    let target = find(&handle, window)?;
    handle
        .inject(target, event)
        .map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// The modifiers of matplotlib's names `names` (`ctrl`, `alt`, `shift`,
/// `super` or `cmd`); others hold nothing.
fn held(names: &[String]) -> Modifiers {
    names
        .iter()
        .fold(Modifiers::NONE, |held, name| match name.as_str() {
            "ctrl" => held | Modifiers::CONTROL,
            "alt" => held | Modifiers::ALT,
            "shift" => held | Modifiers::SHIFT,
            "super" | "cmd" => held | Modifiers::SUPER,
            _ => held,
        })
}

/// The key of a one-character name or of one of a few named keys.
fn key(name: &str) -> PyResult<Key> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Ok(Key::Character(c));
    }
    Ok(match name {
        "enter" => Key::Enter,
        "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "tab" => Key::Tab,
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        "shift" => Key::Shift,
        "control" => Key::Control,
        other => return Err(PyErr::value_error(format!("no key named {other:?}"))),
    })
}

/// The mouse button of matplotlib's number `number`.
fn mouse_button(number: u16) -> MouseButton {
    match number {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        8 => MouseButton::Back,
        9 => MouseButton::Forward,
        other => MouseButton::Other(other),
    }
}

/// Makes the display move the pointer to `(x, y)` (physical pixels from
/// the top-left corner of its window `window`) with the `modifiers`
/// held, then for `kind`: nothing (`"move"`), press or release mouse
/// button `button` (matplotlib's number; `"press"`, the `clicks`-th click
/// in a row, and `"release"`), or turn the wheel by `lines` (`"scroll"`,
/// up positive).
fn inject_pointer(
    handle: Native<HeadlessHandle>,
    window: u64,
    kind: String,
    (x, y): (f64, f64),
    modifiers: Vec<String>,
    (button, clicks, lines): (u16, u32, f64),
) -> PyResult<()> {
    let position = PhysicalPosition::new(x, y);
    let modifiers = held(&modifiers);
    let moved = WindowEvent::CursorMoved {
        position,
        modifiers,
    };
    send(&handle, window, moved)?;
    let state = match kind.as_str() {
        "move" => return Ok(()),
        "press" => ButtonState::Pressed,
        "release" => ButtonState::Released,
        "scroll" => {
            let wheel = WindowEvent::MouseWheel {
                delta: ScrollDelta::Lines { x: 0.0, y: lines },
                position,
                modifiers,
            };
            return send(&handle, window, wheel);
        }
        other => return Err(PyErr::value_error(format!("no pointer event {other:?}"))),
    };
    let input = WindowEvent::MouseInput {
        button: mouse_button(button),
        state,
        position,
        modifiers,
        click_count: clicks,
    };
    send(&handle, window, input)
}

/// Makes the display press (or release) the key `name` (a character, or
/// `enter`, `escape`, `backspace`, `tab`, `left`, `right`, `up`, `down`,
/// `shift`, `control`) in its window `window`, with the `modifiers` held.
fn inject_key(
    handle: Native<HeadlessHandle>,
    window: u64,
    name: String,
    pressed: bool,
    modifiers: Vec<String>,
) -> PyResult<()> {
    let event = KeyEvent {
        key: key(&name)?,
        state: if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
        repeat: false,
        text: None,
        modifiers: held(&modifiers),
        scancode: 0,
    };
    send(&handle, window, WindowEvent::Key(event))
}

/// Makes the display take the pointer out of its window `window`
/// (`"leave"`) or ask to close it (`"close"`).
fn inject_window(handle: Native<HeadlessHandle>, window: u64, kind: String) -> PyResult<()> {
    let event = match kind.as_str() {
        "leave" => WindowEvent::CursorLeft,
        "close" => WindowEvent::CloseRequested,
        other => return Err(PyErr::value_error(format!("no window event {other:?}"))),
    };
    send(&handle, window, event)
}

/// Makes the display's screen `scale_factor` physical pixels per logical
/// pixel, as when a window moves to a screen of another density.
fn set_scale_factor(handle: Native<HeadlessHandle>, scale_factor: f64) -> PyResult<()> {
    handle
        .borrow()
        .set_scale_factor(scale_factor)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// What the display's window `window` shows, toolbar included, as
/// `height × width × 4` RGBA bytes.
fn capture(handle: Native<HeadlessHandle>, window: u64) -> PyResult<NdArray<u8>> {
    let handle = handle.borrow();
    let frame = handle
        .surface(find(&handle, window)?)
        .ok_or_else(|| PyErr::runtime_error("the window shows nothing yet"))?;
    let size = frame.size();
    NdArray::with_shape(
        frame.into_pixels(),
        vec![size.height as usize, size.width as usize, 4],
    )
}

/// The title the display shows on its window `window`.
fn shown_title(handle: Native<HeadlessHandle>, window: u64) -> PyResult<String> {
    let handle = handle.borrow();
    let snapshot = handle
        .window(find(&handle, window)?)
        .ok_or_else(|| PyErr::runtime_error("the window is closed"))?;
    Ok(snapshot.title)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("connect_headless", connect_headless)?;
    module.add("window_system_id", window_system_id)?;
    module.add("windows", windows)?;
    module.add("inject_pointer", inject_pointer)?;
    module.add("inject_key", inject_key)?;
    module.add("inject_window", inject_window)?;
    module.add("set_scale_factor", set_scale_factor)?;
    module.add("capture", capture)?;
    module.add("shown_title", shown_title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_and_keys_have_matplotlibs_names() {
        let names = ["ctrl".to_owned(), "shift".to_owned(), "hyper".to_owned()];
        assert_eq!(held(&names), Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(key("q").unwrap(), Key::Character('q'));
        assert_eq!(key("escape").unwrap(), Key::Escape);
        assert!(key("pageup").is_err());
        assert_eq!(mouse_button(3), MouseButton::Right);
        assert_eq!(mouse_button(7), MouseButton::Other(7));
    }
}
