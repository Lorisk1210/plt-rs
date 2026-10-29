//! The `SpanSelector` ops.

use std::cell::RefCell;
use std::rc::Rc;

use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::rc::Aspect;
use pltrs::widgets::{Orientation, SpanSelector, SpanSelectorOptions};

use super::region::{alpha, per_line, state};
use super::steps::{Attempt, Tool, replay, unknown};
use super::{button, get_ax, malformed, numbers, props, resolved_rgba};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The name of `direction`.
fn direction_name(direction: Orientation) -> Json {
    Json::from(match direction {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    })
}

impl Tool for SpanSelector {
    fn ax(&self) -> Axes {
        SpanSelector::ax(self)
    }

    fn call(&self, method: &str, args: &[Json]) -> Result<Attempt, ProbeError> {
        let arg = args.first();
        Ok(match method {
            "add_state" => match state(arg) {
                Some(state) => self.add_state(state).into(),
                None => Attempt::Refused,
            },
            "clear" => {
                self.clear();
                Attempt::Done
            }
            "set_props" => {
                self.set_props(props::patch(arg.ok_or_else(|| malformed("set_props"))?)?);
                Attempt::Done
            }
            "set_handle_props" => self
                .set_handle_props(props::line(
                    arg.ok_or_else(|| malformed("set_handle_props"))?,
                )?)
                .into(),
            other => return Err(unknown("method", other)),
        })
    }

    fn set(&self, attribute: &str, value: &Json) -> Result<Attempt, ProbeError> {
        Ok(match attribute {
            "extents" => match value.as_f64_vec().as_deref() {
                Some(&[vmin, vmax]) => {
                    self.set_extents((vmin, vmax));
                    Attempt::Done
                }
                _ => return Err(malformed("extents")),
            },
            "direction" => match value.as_str().map(str::parse::<Orientation>) {
                Some(Ok(direction)) => {
                    self.set_direction(direction);
                    Attempt::Done
                }
                Some(Err(_)) => Attempt::Refused,
                None => return Err(malformed("direction")),
            },
            "snap_values" => {
                self.set_snap_values(value.as_f64_vec());
                Attempt::Done
            }
            other => return Err(unknown("attribute", other)),
        })
    }

    fn get(&self, query: &str) -> Result<Json, ProbeError> {
        let handles = self.handles();
        let bound = |(a, b): (f64, f64)| numbers([a.min(b), a.max(b)]);
        Ok(match query {
            "extents" => {
                let (vmin, vmax) = self.extents();
                numbers([vmin, vmax])
            }
            "direction" => direction_name(self.direction()),
            // A handle along x is a vertical line, of equal x data.
            "_edge_handles.direction" => {
                let handle = handles.first().ok_or_else(|| malformed("edge handles"))?;
                let xdata = handle.xdata();
                direction_name(if xdata.first() == xdata.last() {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                })
            }
            "_edge_handles.positions" => {
                let positions: Vec<f64> = handles
                    .iter()
                    .filter_map(|handle| match self.direction() {
                        Orientation::Horizontal => handle.xdata().first().copied(),
                        Orientation::Vertical => handle.ydata().first().copied(),
                    })
                    .collect();
                numbers(&positions)
            }
            "snap_values" => self
                .snap_values()
                .map_or(Json::Null, |values| numbers(&values)),
            "_selection_completed" => Json::from(self.selection_completed()),
            "get_visible" => Json::from(self.visible()),
            "_selection_artist.get_facecolor" => {
                encode::rgba(self.artist().colors().map_err(ProbeError::rejected)?.face)
            }
            "_handles_artists.get_color" => {
                per_line(&handles, |line| resolved_rgba(&line.color()))?
            }
            "_handles_artists.get_alpha" => per_line(&handles, |line| Ok(alpha(line)))?,
            "ax.get_xbound" => bound(self.ax().xlim()),
            "ax.get_ybound" => bound(self.ax().ylim()),
            other => return Err(unknown("query", other)),
        })
    }
}

/// The selector options the probe gives.
fn options(given: &Json) -> Result<SpanSelectorOptions, ProbeError> {
    let mut options = SpanSelectorOptions::default();
    let entries = given.as_object().ok_or_else(|| malformed("options"))?;
    let flag = |value: &Json, name: &str| value.as_bool().ok_or_else(|| malformed(name));
    for (name, value) in entries {
        match name.as_str() {
            // Blitting only speeds drawing up; selections are the same.
            "useblit" => {}
            "button" => options.buttons = Some(vec![button(value)?]),
            "minspan" => options.minspan = value.as_f64().ok_or_else(|| malformed(name))?,
            "props" => options.props = props::patch(value)?,
            "handle_props" => options.handle_props = props::line(value)?,
            "interactive" => options.interactive = flag(value, name)?,
            "drag_from_anywhere" => options.drag_from_anywhere = flag(value, name)?,
            "ignore_event_outside" => options.ignore_event_outside = flag(value, name)?,
            "snap_values" => options.snap_values = value.as_f64_vec(),
            other => return Err(unknown("option", other)),
        }
    }
    Ok(options)
}

/// The axes the probe's `axes` names.
fn span_axes(name: &str) -> Result<Axes, ProbeError> {
    match name {
        "bound" => {
            let fig = Figure::new();
            let ax = fig.subplot();
            ax.plot(vec![10.0, 20.0], vec![10.0, 30.0])
                .map_err(ProbeError::rejected)?;
            fig.layout();
            Ok(ax)
        }
        "get_ax" => get_ax(),
        "twin" | "inset" => {
            let ax = get_ax()?;
            ax.set_aspect(Aspect::Auto).map_err(ProbeError::rejected)?;
            let _twin = ax.twinx();
            let child = ax.inset_axes([0.0, 1.0, 1.0, 1.0]);
            child
                .set_xlim(0.0, 200.0)
                .and_then(|ax| ax.set_ylim(0.0, 200.0))
                .map_err(ProbeError::rejected)?;
            Ok(if name == "inset" { child } else { ax })
        }
        other => Err(unknown("axes", other)),
    }
}

/// The ends a callback received, in order.
type Log = Rc<RefCell<Vec<[f64; 2]>>>;

fn spans(log: &Log) -> Json {
    Json::Array(log.borrow().iter().map(numbers).collect())
}

/// `SpanSelector(ax, onselect, direction, **options)` driven by `steps`.
pub(super) fn span(args: Args<'_>) -> Outcome {
    styled(args, || {
        let direction: Orientation = args
            .str("direction")?
            .parse()
            .map_err(ProbeError::rejected)?;
        let ax = span_axes(args.opt_str("axes")?.unwrap_or("get_ax"))?;
        let tool = SpanSelector::new(&ax, direction, options(args.json("options")?)?);
        let (selects, moves): (Log, Log) = (Rc::default(), Rc::default());
        let sink = Rc::clone(&selects);
        tool.on_select(move |vmin, vmax| sink.borrow_mut().push([vmin, vmax]));
        let onmove = args.opt_bool("onmove")?.unwrap_or(false);
        if onmove {
            let sink = Rc::clone(&moves);
            tool.on_move(move |vmin, vmax| sink.borrow_mut().push([vmin, vmax]));
        }
        let steps = replay(&tool, args.json("steps")?)?;
        let mut result = vec![
            ("steps".to_owned(), steps),
            ("selects".to_owned(), spans(&selects)),
        ];
        if onmove {
            result.push(("moves".to_owned(), spans(&moves)));
        }
        Ok(Json::Object(result))
    })
}

/// `SpanSelector._snap(values, snap_values)`, through the `extents`
/// setter of a selector snapping to `snap_values`.
pub(super) fn snap(args: Args<'_>) -> Outcome {
    styled(args, || {
        let options = SpanSelectorOptions {
            snap_values: Some(args.f64_vec("snap_values")?),
            ..SpanSelectorOptions::default()
        };
        let tool = SpanSelector::new(&get_ax()?, Orientation::Horizontal, options);
        let snapped: Vec<f64> = args
            .f64_vec("values")?
            .into_iter()
            .map(|value| {
                tool.set_extents((value, value));
                tool.extents().0
            })
            .collect();
        Ok(numbers(&snapped))
    })
}
