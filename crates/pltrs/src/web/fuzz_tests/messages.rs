//! The JSON messages of the browser client: every message type, unknown
//! ones and types that are not strings, with fields valid, missing, of
//! the wrong type, huge, at the edges of `f64`, deeply nested or not JSON
//! at all. They go to the parser, and those that parse to the canvas of a
//! figure, which must answer with valid JSON events and frames.

use pltrs_core::json::Json;

use crate::figure::{Figure, SaveFormat};
use crate::fuzz::{Gen, fuzz};
use crate::web::canvas::{Next, Output, WebCanvas};
use crate::web::protocol::{ClientMessage, MouseInput};

/// The message types of the client, some it does not send, and odd ones.
const TYPES: [&str; 22] = [
    "supports_binary",
    "refresh",
    "draw",
    "ack",
    "send_image_mode",
    "resize",
    "set_device_pixel_ratio",
    "set_dpi_ratio",
    "button_press",
    "button_release",
    "dblclick",
    "motion_notify",
    "scroll",
    "figure_enter",
    "figure_leave",
    "key_press",
    "key_release",
    "toolbar_button",
    "close",
    "",
    "Draw",
    "scroll\u{0}",
];

/// The fields of the client's messages.
const FIELDS: [&str; 15] = [
    "x",
    "y",
    "button",
    "step",
    "modifiers",
    "guiEvent",
    "key",
    "name",
    "value",
    "width",
    "height",
    "device_pixel_ratio",
    "dpi_ratio",
    "figure_id",
    "buttons",
];

/// Keys as the client spells them, with matplotlib's shortcuts among
/// them (`q` closes the figure).
const KEYS: [&str; 22] = [
    "ka",
    "shift+kA",
    "ctrl+kc",
    "kArrowLeft",
    "alt+kArrowUp",
    "kp",
    "ko",
    "kh",
    "kc",
    "kv",
    "ks",
    "kg",
    "kG",
    "kl",
    "kL",
    "kk",
    "kx",
    "ky",
    "kq",
    "kControl",
    "kEscape",
    "k",
];

/// Pieces of JSON text, to break messages with.
const PIECES: [&str; 16] = [
    "{", "}", "[", "]", "\"", ":", ",", "\\", "\\u", "null", "true", "1e999", "-", ".", " ",
    "\"type\"",
];

/// Values that are not what any field wants, or JSON at its edges.
const ODD_VALUES: [&str; 18] = [
    "1e308",
    "-1.7976931348623157e308",
    "1e999",
    "-1e400",
    "1e-400",
    "-0",
    "123456789012345678901234567890",
    "9007199254740993",
    "NaN",
    "Infinity",
    "\"\\ud800\"",
    "\"\\u0000\"",
    "\"\u{1}\"",
    "\"\u{1F600}\"",
    "\"\"",
    "[]",
    "{}",
    "null",
];

/// Whether `field` belongs to messages of type `kind`.
fn relevant(kind: &str, field: &str) -> bool {
    match field {
        "x" | "y" | "button" | "modifiers" | "buttons" => {
            kind.contains("button")
                || kind.contains("click")
                || kind.contains("figure_")
                || kind.contains("motion")
                || kind.contains("scroll")
        }
        "step" | "guiEvent" => kind.starts_with("scroll") || kind.starts_with("key"),
        "key" => kind.starts_with("key"),
        "name" => kind == "toolbar_button",
        "value" | "figure_id" => kind == "supports_binary",
        "width" | "height" => kind == "resize",
        "device_pixel_ratio" => kind == "set_device_pixel_ratio",
        "dpi_ratio" => kind == "set_dpi_ratio",
        _ => false,
    }
}

/// A number as JSON text: mostly in `[low, high]`, sometimes any number
/// (`NaN` and the infinities are not JSON).
fn number(g: &mut Gen, low: f64, high: f64) -> String {
    let value = g.number_in(low, high);
    if g.chance(0.5) {
        format!("{value}")
    } else {
        format!("{value:e}")
    }
}

/// A value `field` takes in the client's messages.
fn valid_value(g: &mut Gen, field: &str) -> String {
    match field {
        "x" | "y" => number(g, -20.0, 420.0),
        "button" | "buttons" => g.pick(&["0", "1", "2", "3", "-1", "0.5"]).to_owned(),
        "step" => g.pick(&["1", "-1", "0", "3", "-1e9"]).to_owned(),
        "modifiers" => {
            let held: Vec<&str> = ["\"ctrl\"", "\"shift\"", "\"alt\"", "\"meta\""]
                .into_iter()
                .filter(|_| g.chance(0.3))
                .collect();
            format!("[{}]", held.join(","))
        }
        "guiEvent" => format!(
            "{{\"deltaY\": {}, \"deltaMode\": {}}}",
            number(g, -300.0, 300.0),
            g.pick(&["0", "1", "2", "3", "\"0\""])
        ),
        "key" => format!("\"{}\"", g.pick(&KEYS)),
        "name" => format!(
            "\"{}\"",
            g.pick(&[
                "home", "back", "forward", "pan", "zoom", "download", "subplots"
            ])
        ),
        "value" => g.pick(&["true", "false"]).to_owned(),
        "width" | "height" => number(g, 0.0, 900.0),
        "device_pixel_ratio" | "dpi_ratio" => number(g, 0.0, 4.0),
        _ => g.below(10).to_string(),
    }
}

/// Any value: an odd one, a valid one of another field, or an array or
/// object of them, nested up to `depth` more levels (or far deeper).
fn any_value(g: &mut Gen, depth: usize) -> String {
    match g.below(8) {
        0 | 1 => g.pick(&ODD_VALUES).to_owned(),
        2 => format!("{:?}", g.soup(&["a", "\\", "\"", "é"], 4)),
        3 if depth > 0 => {
            let items: Vec<String> = (0..g.len(4)).map(|_| any_value(g, depth - 1)).collect();
            format!("[{}]", items.join(","))
        }
        4 if depth > 0 => {
            let members: Vec<String> = (0..g.len(4))
                .map(|_| format!("\"{}\": {}", g.pick(&FIELDS), any_value(g, depth - 1)))
                .collect();
            format!("{{{}}}", members.join(","))
        }
        5 => {
            let levels = 28 + g.below(10);
            "[".repeat(levels) + &"]".repeat(levels)
        }
        _ => {
            let field = g.pick(&FIELDS);
            valid_value(g, field)
        }
    }
}

/// A message of the client as JSON text, mostly well formed.
pub(super) fn message(g: &mut Gen) -> String {
    let kind = g.pick(&TYPES);
    let mut members = vec![if g.chance(0.95) {
        format!("\"type\": \"{kind}\"")
    } else {
        format!("\"type\": {}", any_value(g, 1))
    }];
    for field in FIELDS {
        let wanted = if relevant(kind, field) { 0.8 } else { 0.05 };
        if g.chance(wanted) {
            let value = if g.chance(0.85) {
                valid_value(g, field)
            } else {
                any_value(g, 2)
            };
            members.push(format!("\"{field}\": {value}"));
        }
    }
    let text = format!("{{{}}}", members.join(", "));
    if g.chance(0.15) {
        g.mutate(&text, &PIECES)
    } else {
        text
    }
}

/// Checks what parsing a message gave.
fn check_parsed(message: &ClientMessage) {
    if let ClientMessage::Mouse(MouseInput { wheel, .. }) = message {
        assert!((-3.0..=3.0).contains(wheel), "wheel notches {wheel}");
    }
}

#[test]
fn client_messages() {
    fuzz("web messages", 5_000, message, |text| {
        if let Ok(message) = ClientMessage::parse(text) {
            check_parsed(&message);
        }
    });
}

/// A small figure with a curve, a title and a legend, to serve.
fn served_figure() -> Figure {
    let figure = Figure::with_size(2.0, 1.5, 50.0);
    let ax = figure.subplot();
    let line = crate::lines::LineProps::default().label("a");
    ax.plot_with(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0], line)
        .expect("a valid curve");
    ax.set_title("t");
    let _ = ax.legend(crate::legend::LegendOptions::default());
    figure
}

/// Checks the outputs of a canvas: every event is JSON the client reads.
fn check_outputs(canvas: &mut WebCanvas) {
    for output in canvas.take_outputs() {
        match output {
            Output::Event(event) => {
                let text = event.to_json();
                assert!(Json::parse_strict(&text).is_ok(), "invalid event {text}");
            }
            Output::Frame(png) => assert!(png.starts_with(b"\x89PNG")),
        }
    }
}

#[test]
fn canvas_messages() {
    fuzz(
        "web canvas",
        150,
        |g| {
            let messages: Vec<String> = (0..g.len(30)).map(|_| message(g)).collect();
            let download = g
                .chance(0.1)
                .then(|| g.pick(&[SaveFormat::Png, SaveFormat::Svg]));
            (messages, download)
        },
        |(messages, download)| {
            let figure = served_figure();
            let mut canvas = WebCanvas::new(&figure, "Figure 1");
            canvas.connected();
            for text in messages {
                let Ok(message) = ClientMessage::parse(text) else {
                    continue;
                };
                check_parsed(&message);
                let next = canvas.handle(message);
                canvas.idle();
                check_outputs(&mut canvas);
                if next != Next::Continue {
                    break;
                }
            }
            if let Some(format) = download {
                let _ = canvas.download(*format);
            }
        },
    );
}
