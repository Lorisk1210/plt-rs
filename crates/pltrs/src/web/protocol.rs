//! matplotlib's `WebAgg` protocol: the JSON messages of the browser client
//! (`mpl.js`), and in `events` those the server answers with
//! (`backend_webagg_core.py`).

mod events;

use pltrs_core::json::{Json, ParseError};

use crate::navigation::PointerButton;

pub(crate) use events::{ImageMode, ServerEvent};

/// What a mouse message reports (the client's event names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MouseKind {
    /// `button_press`.
    Press,
    /// `button_release`.
    Release,
    /// `dblclick`: a second press, after its release.
    DoubleClick,
    /// `motion_notify`.
    Motion,
    /// `scroll`.
    Scroll,
    /// `figure_enter`.
    Enter,
    /// `figure_leave`.
    Leave,
}

/// A mouse message, in physical pixels from the top-left corner of the
/// canvas, as the client measures them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MouseInput {
    /// What happened.
    pub(crate) kind: MouseKind,
    /// Pixels from the left.
    pub(crate) x: f64,
    /// Pixels from the top.
    pub(crate) y: f64,
    /// The button pressed or released, if one navigation knows.
    pub(crate) button: Option<PointerButton>,
    /// Wheel steps, `+1` up and `-1` down (matplotlib's `step`).
    pub(crate) step: f64,
    /// How far the wheel turned in notches, from the browser's deltas.
    pub(crate) wheel: f64,
    /// Whether control was held.
    pub(crate) ctrl: bool,
    /// Whether shift was held.
    pub(crate) shift: bool,
}

/// A toolbar button of the client (the methods of matplotlib's
/// `NavigationToolbar2WebAgg.toolitems`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolbarButton {
    /// `home`.
    Home,
    /// `back`.
    Back,
    /// `forward`.
    Forward,
    /// `pan`.
    Pan,
    /// `zoom`.
    Zoom,
}

/// A message of the browser client.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ClientMessage {
    /// Whether this client takes binary frames (`supports_binary`).
    SupportsBinary(bool),
    /// The client needs everything again: label, buttons and a full frame
    /// (`refresh`).
    Refresh,
    /// Send a frame (`draw`).
    Draw,
    /// A frame arrived (`ack`).
    Ack,
    /// The canvas is `width × height` CSS pixels (`resize`).
    Resize {
        /// The width in CSS pixels.
        width: f64,
        /// The height in CSS pixels.
        height: f64,
    },
    /// Which image mode is current (`send_image_mode`).
    SendImageMode,
    /// Physical pixels per CSS pixel (`set_device_pixel_ratio`).
    SetDevicePixelRatio(f64),
    /// Mouse input.
    Mouse(MouseInput),
    /// A key went down (`true`) or up, in matplotlib's naming.
    Key {
        /// Whether the key went down.
        pressed: bool,
        /// The key, e.g. `"a"`, `"ctrl+s"` or `"left"`.
        key: String,
    },
    /// A toolbar button was clicked; `None` for one the server does not
    /// handle.
    Toolbar(Option<ToolbarButton>),
    /// A message type the server does not handle.
    Unknown,
}

/// Why a message could not be understood.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MessageError {
    /// The text is not JSON.
    Json(ParseError),
    /// The JSON is not an object with a string `type`.
    NoType,
    /// A field is missing or of the wrong type.
    BadField(&'static str),
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(err) => err.fmt(f),
            Self::NoType => f.write_str("the message has no type"),
            Self::BadField(name) => write!(f, "missing or invalid field {name:?}"),
        }
    }
}

impl std::error::Error for MessageError {}

/// The keys the client names differently from matplotlib
/// (`_SPECIAL_KEYS_LUT`).
const SPECIAL_KEYS: [(&str, &str); 38] = [
    ("Alt", "alt"),
    ("AltGraph", "alt"),
    ("CapsLock", "caps_lock"),
    ("Control", "control"),
    ("Meta", "meta"),
    ("NumLock", "num_lock"),
    ("ScrollLock", "scroll_lock"),
    ("Shift", "shift"),
    ("Super", "super"),
    ("Enter", "enter"),
    ("Tab", "tab"),
    ("ArrowDown", "down"),
    ("ArrowLeft", "left"),
    ("ArrowRight", "right"),
    ("ArrowUp", "up"),
    ("End", "end"),
    ("Home", "home"),
    ("PageDown", "pagedown"),
    ("PageUp", "pageup"),
    ("Backspace", "backspace"),
    ("Delete", "delete"),
    ("Insert", "insert"),
    ("Escape", "escape"),
    ("Pause", "pause"),
    ("Select", "select"),
    ("Dead", "dead"),
    ("F1", "f1"),
    ("F2", "f2"),
    ("F3", "f3"),
    ("F4", "f4"),
    ("F5", "f5"),
    ("F6", "f6"),
    ("F7", "f7"),
    ("F8", "f8"),
    ("F9", "f9"),
    ("F10", "f10"),
    ("F11", "f11"),
    ("F12", "f12"),
];

/// matplotlib's name of a key the client spells `[modifier+]k<key>`
/// (`_handle_key`): `"shift+kA"` is `"A"`, `"ctrl+kArrowLeft"` is
/// `"ctrl+left"`.
fn key_name(sent: &str) -> Option<String> {
    let (_, value) = sent.split_once('k')?;
    let key = if sent.contains("shift+") && value.chars().count() == 1 {
        sent.replace("shift+", "")
    } else {
        sent.to_owned()
    };
    let value = SPECIAL_KEYS
        .iter()
        .find_map(|(js, name)| (*js == value).then_some(*name))
        .unwrap_or(value);
    let (prefix, _) = key.split_once('k')?;
    Some(format!("{prefix}{value}"))
}

/// A required finite number field.
fn number(message: &Json, name: &'static str) -> Result<f64, MessageError> {
    message
        .get(name)
        .and_then(Json::as_f64)
        .ok_or(MessageError::BadField(name))
}

/// An optional number field, `default` when absent.
fn number_or(message: &Json, name: &'static str, default: f64) -> Result<f64, MessageError> {
    match message.get(name) {
        None => Ok(default),
        Some(value) => value.as_f64().ok_or(MessageError::BadField(name)),
    }
}

/// Wheel notches from the browser's `deltaY` and `deltaMode` (pixels,
/// lines or pages), or matplotlib's `step` without them.
fn wheel_notches(message: &Json, step: f64) -> f64 {
    let gui = message.get("guiEvent");
    let delta = gui.and_then(|gui| gui.get("deltaY")).and_then(Json::as_f64);
    let mode = gui
        .and_then(|gui| gui.get("deltaMode"))
        .and_then(Json::as_f64);
    let notches = match (delta, mode) {
        (Some(delta), Some(0.0)) => -delta / 100.0,
        (Some(delta), Some(1.0)) => -delta / 3.0,
        (Some(delta), Some(2.0)) => -delta,
        _ => step,
    };
    notches.clamp(-3.0, 3.0)
}

fn mouse(message: &Json, kind: MouseKind) -> Result<MouseInput, MessageError> {
    let button = number_or(message, "button", 0.0)?;
    let modifiers = match message.get("modifiers") {
        None => &[][..],
        Some(list) => list.as_array().ok_or(MessageError::BadField("modifiers"))?,
    };
    let held = |name: &str| modifiers.iter().any(|m| m.as_str() == Some(name));
    let step = if kind == MouseKind::Scroll {
        number(message, "step")?
    } else {
        0.0
    };
    Ok(MouseInput {
        kind,
        x: number(message, "x")?,
        y: number(message, "y")?,
        // The browser numbers buttons from 0: left, middle, right, back
        // and forward (matplotlib adds 1, which makes the side buttons 4
        // and 5 rather than its BACK and FORWARD).
        button: match button {
            0.0 => Some(PointerButton::Left),
            1.0 => Some(PointerButton::Middle),
            2.0 => Some(PointerButton::Right),
            3.0 => Some(PointerButton::Back),
            4.0 => Some(PointerButton::Forward),
            _ => None,
        },
        step,
        wheel: wheel_notches(message, step),
        ctrl: held("ctrl"),
        shift: held("shift"),
    })
}

impl ClientMessage {
    /// Parses a message of the client.
    pub(crate) fn parse(text: &str) -> Result<Self, MessageError> {
        let message = Json::parse_strict(text).map_err(MessageError::Json)?;
        let kind = message
            .get("type")
            .and_then(Json::as_str)
            .ok_or(MessageError::NoType)?;
        Ok(match kind {
            "supports_binary" => Self::SupportsBinary(
                message
                    .get("value")
                    .and_then(Json::as_bool)
                    .ok_or(MessageError::BadField("value"))?,
            ),
            "refresh" => Self::Refresh,
            "draw" => Self::Draw,
            "ack" => Self::Ack,
            "send_image_mode" => Self::SendImageMode,
            "resize" => Self::Resize {
                width: number_or(&message, "width", 800.0)?,
                height: number_or(&message, "height", 800.0)?,
            },
            "set_device_pixel_ratio" => {
                Self::SetDevicePixelRatio(number_or(&message, "device_pixel_ratio", 1.0)?)
            }
            // Older ipympl clients.
            "set_dpi_ratio" => Self::SetDevicePixelRatio(number_or(&message, "dpi_ratio", 1.0)?),
            "button_press" => Self::Mouse(mouse(&message, MouseKind::Press)?),
            "button_release" => Self::Mouse(mouse(&message, MouseKind::Release)?),
            "dblclick" => Self::Mouse(mouse(&message, MouseKind::DoubleClick)?),
            "motion_notify" => Self::Mouse(mouse(&message, MouseKind::Motion)?),
            "scroll" => Self::Mouse(mouse(&message, MouseKind::Scroll)?),
            "figure_enter" => Self::Mouse(mouse(&message, MouseKind::Enter)?),
            "figure_leave" => Self::Mouse(mouse(&message, MouseKind::Leave)?),
            "key_press" | "key_release" => Self::Key {
                pressed: kind == "key_press",
                key: message
                    .get("key")
                    .and_then(Json::as_str)
                    .and_then(key_name)
                    .ok_or(MessageError::BadField("key"))?,
            },
            "toolbar_button" => Self::Toolbar(
                match message
                    .get("name")
                    .and_then(Json::as_str)
                    .ok_or(MessageError::BadField("name"))?
                {
                    "home" => Some(ToolbarButton::Home),
                    "back" => Some(ToolbarButton::Back),
                    "forward" => Some(ToolbarButton::Forward),
                    "pan" => Some(ToolbarButton::Pan),
                    "zoom" => Some(ToolbarButton::Zoom),
                    _ => None,
                },
            ),
            _ => Self::Unknown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_named_like_matplotlib() {
        for (sent, name) in [
            ("ka", "a"),
            ("shift+kA", "A"),
            ("ctrl+kc", "ctrl+c"),
            ("alt+kArrowLeft", "alt+left"),
            ("shift+kArrowUp", "shift+up"),
            ("kControl", "control"),
            ("kF5", "f5"),
            ("k ", " "),
            ("shift+kÉ", "É"),
        ] {
            assert_eq!(key_name(sent).as_deref(), Some(name), "{sent}");
        }
        assert_eq!(key_name("ctrl+"), None);
    }

    #[test]
    fn parses_the_client_messages() {
        let parse = |text: &str| ClientMessage::parse(text).unwrap();
        assert_eq!(
            parse(r#"{"type": "supports_binary", "value": true, "figure_id": 1}"#),
            ClientMessage::SupportsBinary(true)
        );
        assert_eq!(parse(r#"{"type": "refresh"}"#), ClientMessage::Refresh);
        assert_eq!(
            parse(r#"{"type": "resize", "width": 640.5, "height": 480}"#),
            ClientMessage::Resize {
                width: 640.5,
                height: 480.0
            }
        );
        assert_eq!(
            parse(r#"{"type": "set_device_pixel_ratio", "device_pixel_ratio": 2}"#),
            ClientMessage::SetDevicePixelRatio(2.0)
        );
        assert_eq!(
            parse(r#"{"type": "key_press", "key": "shift+kA", "guiEvent": {}}"#),
            ClientMessage::Key {
                pressed: true,
                key: "A".into()
            }
        );
        assert_eq!(
            parse(r#"{"type": "toolbar_button", "name": "pan"}"#),
            ClientMessage::Toolbar(Some(ToolbarButton::Pan))
        );
        assert_eq!(
            parse(r#"{"type": "toolbar_button", "name": "subplots"}"#),
            ClientMessage::Toolbar(None)
        );
        assert_eq!(parse(r#"{"type": "nonsense"}"#), ClientMessage::Unknown);
        let ClientMessage::Mouse(scroll) = parse(
            r#"{"type": "scroll", "x": 10, "y": 20, "button": 0, "step": -1, "buttons": 0,
                "modifiers": ["ctrl"], "guiEvent": {"deltaY": 50, "deltaMode": 0}}"#,
        ) else {
            panic!("a mouse message");
        };
        assert_eq!(
            (scroll.kind, scroll.step, scroll.wheel),
            (MouseKind::Scroll, -1.0, -0.5)
        );
        assert!(scroll.ctrl && !scroll.shift);
        let buttons = [
            (2, Some(PointerButton::Right)),
            (3, Some(PointerButton::Back)),
            (4, Some(PointerButton::Forward)),
            (5, None),
        ];
        for (number, button) in buttons {
            let text = format!(r#"{{"type": "button_press", "x": 1, "y": 2, "button": {number}}}"#);
            let ClientMessage::Mouse(press) = parse(&text) else {
                panic!("a mouse message");
            };
            assert_eq!(press.button, button);
        }
    }

    #[test]
    fn malformed_messages_are_errors() {
        for text in [
            "not json",
            "[]",
            r#"{"type": 3}"#,
            r#"{"type": "supports_binary"}"#,
            r#"{"type": "motion_notify", "y": 1}"#,
            r#"{"type": "motion_notify", "x": "1", "y": 1}"#,
            r#"{"type": "scroll", "x": 1, "y": 1}"#,
            r#"{"type": "resize", "width": "wide"}"#,
            r#"{"type": "key_press", "key": "ctrl+"}"#,
            r#"{"type": "button_press", "x": 1, "y": 1, "modifiers": "ctrl"}"#,
            r#"{"type": "toolbar_button"}"#,
        ] {
            assert!(ClientMessage::parse(text).is_err(), "{text}");
        }
    }
}
