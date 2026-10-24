//! Replaying the selector steps of `test_mpl_widgets.py`: synthetic mouse
//! and key events placed as `MouseEvent._from_ax_coords` places them,
//! with the canvas' dead reckoning of the held button and key, and the
//! `call`, `set` and `get` steps each selector op answers through
//! [`Tool`].

use plt::artist::ArtistHandle;
use plt::axes::Axes;
use plt::events::{Event, KeyEvent, Location, MouseEvent};
use plt::navigation::PointerButton;

use super::{bad, button};
use crate::json::Json;
use crate::probes::ProbeError;

/// Whether plt-rs accepted a `call` or `set` step; refusals are recorded
/// as matplotlib's exceptions are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Attempt {
    /// The step was carried out.
    Done,
    /// plt-rs refused it.
    Refused,
}

impl<T, E> From<Result<T, E>> for Attempt {
    fn from(result: Result<T, E>) -> Self {
        if result.is_ok() {
            Self::Done
        } else {
            Self::Refused
        }
    }
}

/// A selector driven by the steps.
pub(super) trait Tool {
    /// The axes the events are placed in (matplotlib's `tool.ax`).
    fn ax(&self) -> Axes;

    /// `(x0, x1, y0, y1)`, for `drag_handle` steps.
    fn extents(&self) -> Result<[f64; 4], ProbeError> {
        Err(bad("drag_handle on a selector without extents"))
    }

    /// `tool.method(args)`.
    fn call(&self, method: &str, args: &[Json]) -> Result<Attempt, ProbeError>;

    /// `tool.attribute = value`.
    fn set(&self, attribute: &str, value: &Json) -> Result<Attempt, ProbeError>;

    /// The value of `query`.
    fn get(&self, query: &str) -> Result<Json, ProbeError>;
}

/// An unknown method, attribute or query.
pub(super) fn unknown(what: &str, name: &str) -> ProbeError {
    ProbeError::BadArgs(format!("unknown {what} {name:?}"))
}

/// What the canvas remembers between events (matplotlib's `_key_handler`
/// and `_mouse_handler`): the held key and the pressed button.
struct Canvas {
    ax: Axes,
    key: Option<String>,
    button: Option<PointerButton>,
}

/// The kinds of mouse event a step makes.
#[derive(Clone, Copy)]
enum Mouse {
    Press,
    Motion,
    Release,
}

impl Canvas {
    /// A canvas with no key held and no button pressed, for events in
    /// `ax`.
    fn new(ax: &Axes) -> Self {
        Self {
            ax: ax.clone(),
            key: None,
            button: None,
        }
    }

    fn location(&self, xy: (f64, f64)) -> Result<Location, ProbeError> {
        Location::from_ax_coords(&self.ax, xy).map_err(ProbeError::rejected)
    }

    /// A mouse event at data point `xy`; motions without a button get the
    /// pressed one, and every event the held key.
    fn mouse(
        &mut self,
        kind: Mouse,
        xy: (f64, f64),
        button: Option<PointerButton>,
    ) -> Result<(), ProbeError> {
        let button = match kind {
            Mouse::Press => {
                self.button = button;
                button
            }
            Mouse::Release => {
                self.button = None;
                button
            }
            Mouse::Motion => button.or(self.button),
        };
        let event = MouseEvent {
            location: self.location(xy)?,
            button,
            key: self.key.clone(),
            step: 0.0,
            dblclick: false,
        };
        self.ax.figure().process_event(&match kind {
            Mouse::Press => Event::ButtonPress(event),
            Mouse::Motion => Event::Motion(event),
            Mouse::Release => Event::ButtonRelease(event),
        });
        Ok(())
    }

    /// A key event at data point `xy`, or at the canvas origin over no
    /// axes (`KeyEvent(name, canvas, key)`).
    fn key(&mut self, press: bool, key: &str, xy: Option<(f64, f64)>) -> Result<(), ProbeError> {
        let location = match xy {
            Some(xy) => self.location(xy)?,
            None => Location {
                x: 0.0,
                y: 0.0,
                inaxes: None,
                xdata: None,
                ydata: None,
            },
        };
        let event = KeyEvent {
            location,
            key: key.to_owned(),
        };
        self.key = press.then(|| key.to_owned());
        self.ax.figure().process_event(&if press {
            Event::KeyPress(event)
        } else {
            Event::KeyRelease(event)
        });
        Ok(())
    }

    /// matplotlib's `testing.widgets.click_and_drag`.
    fn drag(
        &mut self,
        start: (f64, f64),
        end: (f64, f64),
        key: Option<&str>,
    ) -> Result<(), ProbeError> {
        if let Some(key) = key {
            self.key(true, key, Some(start))?;
        }
        self.mouse(Mouse::Press, start, Some(PointerButton::Left))?;
        self.mouse(Mouse::Motion, end, Some(PointerButton::Left))?;
        self.mouse(Mouse::Release, end, Some(PointerButton::Left))?;
        if let Some(key) = key {
            self.key(false, key, Some(end))?;
        }
        Ok(())
    }
}

/// A press of the left button at data point `xy` of `ax`.
pub(super) fn press(ax: &Axes, xy: (f64, f64)) -> Result<(), ProbeError> {
    Canvas::new(ax).mouse(Mouse::Press, xy, Some(PointerButton::Left))
}

/// A motion to data point `xy` of `ax` without a button.
pub(super) fn motion(ax: &Axes, xy: (f64, f64)) -> Result<(), ProbeError> {
    Canvas::new(ax).mouse(Mouse::Motion, xy, None)
}

/// A press of `key` on the canvas of `ax`, over no axes.
pub(super) fn key_press(ax: &Axes, key: &str) -> Result<(), ProbeError> {
    Canvas::new(ax).key(true, key, None)
}

/// A data point `[x, y]`.
fn point(value: Option<&Json>) -> Result<(f64, f64), ProbeError> {
    match value.and_then(Json::as_f64_vec).as_deref() {
        Some(&[x, y]) => Ok((x, y)),
        _ => Err(bad("point")),
    }
}

/// A key name, or `None` for `null`.
fn key_name(value: Option<&Json>) -> Result<Option<&str>, ProbeError> {
    match value {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value.as_str().map(Some).ok_or_else(|| bad("key")),
    }
}

/// Where handle `name` of a rectangle of `extents` is, as the
/// `test_rectangle_resize*` tests compute it.
fn handle_at([x0, x1, y0, y1]: [f64; 4], name: &str) -> Result<(f64, f64), ProbeError> {
    let ymid = y0 + (y1 - y0) / 2.0;
    match name {
        "NE" => Ok((x1, y1)),
        "E" => Ok((x1, ymid)),
        "W" => Ok((x0, ymid)),
        "SW" => Ok((x0, y0)),
        other => Err(unknown("handle", other)),
    }
}

/// What a `call` or `set` step records.
fn recorded(attempt: Attempt) -> Json {
    match attempt {
        Attempt::Done => Json::Null,
        Attempt::Refused => Json::from("error"),
    }
}

/// Replays `steps` on `tool`; returns what the `call`, `set` and `get`
/// steps recorded, in order.
pub(super) fn replay(tool: &impl Tool, steps: &Json) -> Result<Json, ProbeError> {
    let mut canvas = Canvas::new(&tool.ax());
    let mut recorded_steps = Vec::new();
    for step in steps.as_array().ok_or_else(|| bad("steps"))? {
        let parts = step.as_array().ok_or_else(|| bad("step"))?;
        let (kind, rest) = parts.split_first().ok_or_else(|| bad("step"))?;
        let arg = |i: usize| rest.get(i);
        let text = |i: usize| arg(i).and_then(Json::as_str).ok_or_else(|| bad("step"));
        let mouse_button = |i: usize| match arg(i) {
            None | Some(Json::Null) => Ok(None),
            Some(value) => button(value).map(Some),
        };
        match kind.as_str().ok_or_else(|| bad("step"))? {
            "press" => canvas.mouse(Mouse::Press, point(arg(0))?, mouse_button(1)?)?,
            "move" => canvas.mouse(Mouse::Motion, point(arg(0))?, mouse_button(1)?)?,
            "release" => canvas.mouse(Mouse::Release, point(arg(0))?, mouse_button(1)?)?,
            "drag" => canvas.drag(point(arg(0))?, point(arg(1))?, key_name(arg(2))?)?,
            "drag_handle" => {
                let start = handle_at(tool.extents()?, text(0)?)?;
                let (dx, dy) = point(arg(1))?;
                canvas.drag(start, (start.0 + dx, start.1 + dy), key_name(arg(2))?)?;
            }
            "key" => canvas.key(true, text(0)?, None)?,
            "keyup" => canvas.key(false, text(0)?, None)?,
            "call" => {
                let attempt = tool.call(text(0)?, rest.get(1..).unwrap_or_default())?;
                recorded_steps.push(recorded(attempt));
            }
            "set" => {
                let value = arg(1).ok_or_else(|| bad("set step"))?;
                recorded_steps.push(recorded(tool.set(text(0)?, value)?));
            }
            "get" => recorded_steps.push(tool.get(text(0)?)?),
            other => return Err(unknown("step", other)),
        }
    }
    Ok(Json::Array(recorded_steps))
}
