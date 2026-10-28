//! The events the `WebAgg` server sends its clients
//! (`backend_webagg_core.py`), as JSON text.

use plt_core::Bbox;
use plt_core::json;

use crate::events::CursorShape;
use crate::navigation::NavMode;

/// How a frame relates to the previous one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ImageMode {
    /// The whole canvas.
    #[default]
    Full,
    /// Only the pixels that changed; the others are transparent.
    Diff,
}

/// An event the server sends the clients of a figure.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ServerEvent {
    /// The figure's title.
    FigureLabel(String),
    /// The mode of the frames that follow.
    ImageMode(ImageMode),
    /// The pointer shape over the canvas.
    Cursor(CursorShape),
    /// The status line (the coordinate readout).
    Message(String),
    /// The zoom rectangle in display pixels, or `None` to remove it.
    Rubberband(Option<Bbox>),
    /// Whether the back and forward buttons are enabled.
    HistoryButtons {
        /// Whether there is a previous view.
        back: bool,
        /// Whether there is a next view.
        forward: bool,
    },
    /// The active navigation mode.
    NavigateMode(NavMode),
    /// The canvas size in CSS pixels; `forward` resizes the client's canvas.
    Resize {
        /// The width in CSS pixels.
        width: f64,
        /// The height in CSS pixels.
        height: f64,
        /// Whether the client resizes its canvas element.
        forward: bool,
    },
    /// Whether the canvas keeps the wheel from scrolling the page.
    CaptureScroll(bool),
    /// The figure changed: the client asks for a frame when it is ready.
    Draw,
    /// Download the figure in the format chosen in the toolbar.
    Save,
}

/// The CSS cursor of a cursor shape (`FigureCanvasWebAggCore.set_cursor`).
fn css_cursor(shape: CursorShape) -> &'static str {
    match shape {
        CursorShape::Hand => "pointer",
        CursorShape::Pointer => "default",
        CursorShape::SelectRegion => "crosshair",
        CursorShape::Move => "move",
        CursorShape::Wait => "wait",
        CursorShape::ResizeHorizontal => "ew-resize",
        CursorShape::ResizeVertical => "ns-resize",
    }
}

impl ServerEvent {
    /// The event as the JSON text the client reads.
    pub(crate) fn to_json(&self) -> String {
        match self {
            Self::FigureLabel(label) => {
                format!(
                    r#"{{"type": "figure_label", "label": {}}}"#,
                    json::string(label)
                )
            }
            Self::ImageMode(mode) => format!(
                r#"{{"type": "image_mode", "mode": "{}"}}"#,
                match mode {
                    ImageMode::Full => "full",
                    ImageMode::Diff => "diff",
                }
            ),
            Self::Cursor(shape) => {
                format!(
                    r#"{{"type": "cursor", "cursor": "{}"}}"#,
                    css_cursor(*shape)
                )
            }
            Self::Message(text) => {
                format!(
                    r#"{{"type": "message", "message": {}}}"#,
                    json::string(text)
                )
            }
            Self::Rubberband(rect) => {
                let [x0, y0, x1, y1] = rect.map_or([-1.0; 4], |b| [b.x0(), b.y0(), b.x1(), b.y1()]);
                format!(
                    r#"{{"type": "rubberband", "x0": {}, "y0": {}, "x1": {}, "y1": {}}}"#,
                    json::number(x0),
                    json::number(y0),
                    json::number(x1),
                    json::number(y1)
                )
            }
            Self::HistoryButtons { back, forward } => {
                format!(r#"{{"type": "history_buttons", "Back": {back}, "Forward": {forward}}}"#)
            }
            Self::NavigateMode(mode) => format!(
                r#"{{"type": "navigate_mode", "mode": "{}"}}"#,
                match mode {
                    NavMode::None => "NONE",
                    NavMode::Pan => "PAN",
                    NavMode::Zoom => "ZOOM",
                }
            ),
            Self::Resize {
                width,
                height,
                forward,
            } => format!(
                r#"{{"type": "resize", "size": [{}, {}], "forward": {forward}}}"#,
                json::number(*width),
                json::number(*height)
            ),
            Self::CaptureScroll(capture) => {
                format!(r#"{{"type": "capture_scroll", "capture_scroll": {capture}}}"#)
            }
            Self::Draw => r#"{"type": "draw"}"#.to_owned(),
            Self::Save => r#"{"type": "save"}"#.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use plt_core::json::Json;

    use super::*;

    #[test]
    fn events_serialize_like_matplotlib() {
        assert_eq!(
            ServerEvent::FigureLabel("Figure \"1\"".into()).to_json(),
            r#"{"type": "figure_label", "label": "Figure \"1\""}"#
        );
        assert_eq!(
            ServerEvent::HistoryButtons {
                back: true,
                forward: false
            }
            .to_json(),
            r#"{"type": "history_buttons", "Back": true, "Forward": false}"#
        );
        assert_eq!(
            ServerEvent::Rubberband(None).to_json(),
            r#"{"type": "rubberband", "x0": -1, "y0": -1, "x1": -1, "y1": -1}"#
        );
        let band = Bbox::from_extents(1.0, 2.5, 30.0, 40.0);
        assert_eq!(
            ServerEvent::Rubberband(Some(band)).to_json(),
            r#"{"type": "rubberband", "x0": 1, "y0": 2.5, "x1": 30, "y1": 40}"#
        );
        assert_eq!(
            ServerEvent::Resize {
                width: 640.0,
                height: 480.0,
                forward: true
            }
            .to_json(),
            r#"{"type": "resize", "size": [640, 480], "forward": true}"#
        );
        assert_eq!(
            ServerEvent::Cursor(CursorShape::Move).to_json(),
            r#"{"type": "cursor", "cursor": "move"}"#
        );
        assert_eq!(
            ServerEvent::NavigateMode(NavMode::Zoom).to_json(),
            r#"{"type": "navigate_mode", "mode": "ZOOM"}"#
        );
        for event in [
            ServerEvent::ImageMode(ImageMode::Diff),
            ServerEvent::Message("(x, y) = (1, 2)".into()),
            ServerEvent::CaptureScroll(true),
            ServerEvent::Draw,
            ServerEvent::Save,
        ] {
            let parsed = Json::parse_strict(&event.to_json()).expect("valid JSON");
            assert!(parsed.get("type").and_then(Json::as_str).is_some());
        }
    }
}
