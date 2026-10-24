//! The `RectangleSelector` and `EllipseSelector` ops.

use std::cell::RefCell;
use std::rc::Rc;

use plt::artist::ArtistHandle;
use plt::axes::Axes;
use plt::lines::Line2D;
use plt::rc::Aspect;
use plt::transforms::Point;
use plt::widgets::{
    EllipseSelector, RectangleSelector, RectangleSelectorOptions, SelectorState, SelectorUnits,
};

use super::steps::{Attempt, Tool, replay, unknown};
use super::{bad, button, get_ax, markerfacecolor, numbers, props, rgba};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// A rectangle or an ellipse selector.
enum Region {
    Rectangle(RectangleSelector),
    Ellipse(EllipseSelector),
}

/// Evaluates `$body` with `$selector` bound to whichever selector
/// `$region` holds (both have the same methods).
macro_rules! with_selector {
    ($region:expr, $selector:ident => $body:expr) => {
        match $region {
            Region::Rectangle($selector) => $body,
            Region::Ellipse($selector) => $body,
        }
    };
}

/// `SelectorState` from matplotlib's name, as the probe's `call` steps
/// give it.
pub(super) fn state(value: Option<&Json>) -> Option<SelectorState> {
    value?.as_str()?.parse().ok()
}

/// The names of `states`, sorted (the spec's record of `_state`).
fn state_names(states: &[SelectorState]) -> Json {
    let mut names: Vec<&str> = states.iter().map(|state| state.name()).collect();
    names.sort_unstable();
    Json::Array(names.into_iter().map(Json::from).collect())
}

/// `values` of each line as JSON.
pub(super) fn per_line(
    lines: &[Line2D],
    value: impl Fn(&Line2D) -> Result<Json, ProbeError>,
) -> Result<Json, ProbeError> {
    lines
        .iter()
        .map(value)
        .collect::<Result<_, _>>()
        .map(Json::Array)
}

/// A line's alpha (`null` when unset).
pub(super) fn alpha(line: &Line2D) -> Json {
    line.alpha().map_or(Json::Null, Json::from)
}

/// matplotlib's `geometry` rows, `[ys, xs]` (it swaps them), of the
/// outline `points`.
fn geometry_rows(points: &[Point]) -> [Vec<f64>; 2] {
    [
        points.iter().map(|p| p.y).collect(),
        points.iter().map(|p| p.x).collect(),
    ]
}

impl Tool for Region {
    fn ax(&self) -> Axes {
        with_selector!(self, s => s.ax())
    }

    fn extents(&self) -> Result<[f64; 4], ProbeError> {
        let (x0, x1, y0, y1) = with_selector!(self, s => s.extents());
        Ok([x0, x1, y0, y1])
    }

    fn call(&self, method: &str, args: &[Json]) -> Result<Attempt, ProbeError> {
        let arg = args.first();
        Ok(match method {
            "add_state" => match state(arg) {
                Some(state) => with_selector!(self, s => s.add_state(state)).into(),
                None => Attempt::Refused,
            },
            "remove_state" => match state(arg) {
                Some(state) => with_selector!(self, s => s.remove_state(state)).into(),
                None => Attempt::Refused,
            },
            "clear" => {
                with_selector!(self, s => s.clear());
                Attempt::Done
            }
            "set_props" => {
                let props = props::patch(arg.ok_or_else(|| bad("set_props"))?)?;
                with_selector!(self, s => s.set_props(props));
                Attempt::Done
            }
            "set_handle_props" => {
                let props = props::line(arg.ok_or_else(|| bad("set_handle_props"))?)?;
                with_selector!(self, s => s.set_handle_props(props)).into()
            }
            other => return Err(unknown("method", other)),
        })
    }

    fn set(&self, attribute: &str, value: &Json) -> Result<Attempt, ProbeError> {
        match attribute {
            "extents" => match value.as_f64_vec().as_deref() {
                Some(&[x0, x1, y0, y1]) => {
                    with_selector!(self, s => s.set_extents((x0, x1, y0, y1)));
                    Ok(Attempt::Done)
                }
                _ => Err(bad("extents")),
            },
            "rotation" => {
                let degrees = value.as_f64().ok_or_else(|| bad("rotation"))?;
                with_selector!(self, s => s.set_rotation(degrees));
                Ok(Attempt::Done)
            }
            other => Err(unknown("attribute", other)),
        }
    }

    fn get(&self, query: &str) -> Result<Json, ProbeError> {
        let rows = |(xs, ys): (Vec<f64>, Vec<f64>)| Json::Array(vec![numbers(&xs), numbers(&ys)]);
        let handles = with_selector!(self, s => s.handles());
        let geometry = || with_selector!(self, s => s.geometry());
        Ok(match query {
            "extents" => numbers(&self.extents()?),
            "center" => {
                let (x, y) = with_selector!(self, s => s.center());
                numbers(&[x, y])
            }
            "corners" => rows(with_selector!(self, s => s.corners())),
            "edge_centers" => rows(with_selector!(self, s => s.edge_centers())),
            "rotation" => Json::from(with_selector!(self, s => s.rotation())),
            "geometry" => Json::Array(
                geometry_rows(&geometry())
                    .iter()
                    .map(|row| numbers(row))
                    .collect(),
            ),
            "geometry.shape" => numbers(&[2.0, geometry().len() as f64]),
            "geometry[:, 0]" => {
                let first = geometry().first().copied().ok_or_else(|| bad("geometry"))?;
                numbers(&[first.y, first.x])
            }
            "_state" => state_names(&with_selector!(self, s => s.states())),
            "_selection_completed" => {
                Json::from(with_selector!(self, s => s.selection_completed()))
            }
            "get_visible" => Json::from(with_selector!(self, s => s.visible())),
            "_selection_artist.get_facecolor" => {
                let colors = with_selector!(self, s => s.artist().colors());
                encode::rgba(colors.map_err(ProbeError::rejected)?.face)
            }
            "_handles_artists.get_markeredgecolor" => {
                per_line(&handles, |line| rgba(&line.effective_markeredgecolor()))?
            }
            "_handles_artists.get_alpha" => per_line(&handles, |line| Ok(alpha(line)))?,
            "_corner_handles.artists.get_markerfacecolor" => {
                per_line(corners(&handles)?, markerfacecolor)?
            }
            "_corner_handles.artists.get_markeredgecolor" => {
                per_line(corners(&handles)?, |line| {
                    rgba(&line.effective_markeredgecolor())
                })?
            }
            other => return Err(unknown("query", other)),
        })
    }
}

/// The corner handle markers among `handles` (centre, corners, edges).
fn corners(handles: &[Line2D]) -> Result<&[Line2D], ProbeError> {
    handles.get(1..2).ok_or_else(|| bad("corner handles"))
}

/// The selector options the probe gives, on `ax`.
fn options(ax: &Axes, given: &Json) -> Result<RectangleSelectorOptions, ProbeError> {
    let mut options = RectangleSelectorOptions::default();
    let entries = given.as_object().ok_or_else(|| bad("options"))?;
    let number = |value: &Json, name: &str| value.as_f64().ok_or_else(|| bad(name));
    let flag = |value: &Json, name: &str| value.as_bool().ok_or_else(|| bad(name));
    for (name, value) in entries {
        match name.as_str() {
            // Blitting only speeds drawing up; selections are the same.
            "useblit" => {}
            "button" => options.buttons = Some(vec![button(value)?]),
            "minspanx" => options.minspanx = number(value, name)?,
            "minspany" => options.minspany = number(value, name)?,
            "minspan_between" => {
                let corners = value.as_array().ok_or_else(|| bad(name))?;
                let [start, end] = corners else {
                    return Err(bad(name));
                };
                let display = |corner: &Json| match corner.as_f64_vec().as_deref() {
                    Some(&[x, y]) => {
                        let trans = ax
                            .figure()
                            .resolve_transform(&ax.trans_data())
                            .map_err(ProbeError::rejected)?;
                        Ok(trans.transform_point(Point::new(x, y)))
                    }
                    _ => Err(bad("minspan_between")),
                };
                let (start, end) = (display(start)?, display(end)?);
                options.minspanx = end.x - start.x;
                options.minspany = end.y - start.y;
            }
            "spancoords" => options.spancoords = units(value)?,
            "props" => options.props = props::patch(value)?,
            "handle_props" => options.handle_props = props::line(value)?,
            "interactive" => options.interactive = flag(value, name)?,
            "drag_from_anywhere" => options.drag_from_anywhere = flag(value, name)?,
            "ignore_event_outside" => options.ignore_event_outside = flag(value, name)?,
            "grab_range" => options.grab_range = number(value, name)?,
            "use_data_coordinates" => {
                options.square_units = if flag(value, name)? {
                    SelectorUnits::Data
                } else {
                    SelectorUnits::Pixels
                };
            }
            other => return Err(unknown("option", other)),
        }
    }
    Ok(options)
}

/// `"data"` or `"pixels"`.
fn units(value: &Json) -> Result<SelectorUnits, ProbeError> {
    match value.as_str() {
        Some("data") => Ok(SelectorUnits::Data),
        Some("pixels") => Ok(SelectorUnits::Pixels),
        _ => Err(bad("spancoords")),
    }
}

/// The selectors' `onselect` arguments: the press and release points.
type Selections = Rc<RefCell<Vec<[f64; 4]>>>;

fn run(args: Args<'_>, make: fn(&Axes, RectangleSelectorOptions) -> Region) -> Outcome {
    styled(args, || {
        let ax = get_ax()?;
        if let Some(aspect) = args.opt_f64("aspect")? {
            ax.set_aspect(Aspect::Ratio(aspect))
                .map_err(ProbeError::rejected)?;
        }
        let region = make(&ax, options(&ax, args.json("options")?)?);
        let selects: Selections = Rc::default();
        let sink = Rc::clone(&selects);
        with_selector!(&region, s => s.on_select(move |press, release| {
            sink.borrow_mut()
                .push([press.xdata, press.ydata, release.xdata, release.ydata]);
        }));
        let steps = replay(&region, args.json("steps")?)?;
        let selects = Json::Array(selects.borrow().iter().map(|four| numbers(four)).collect());
        Ok(encode::object([("steps", steps), ("selects", selects)]))
    })
}

/// `RectangleSelector(ax, onselect, **options)` driven by `steps`.
pub(super) fn rectangle(args: Args<'_>) -> Outcome {
    run(args, |ax, options| {
        Region::Rectangle(RectangleSelector::new(ax, options))
    })
}

/// `EllipseSelector(ax, onselect, **options)` driven by `steps`.
pub(super) fn ellipse(args: Args<'_>) -> Outcome {
    run(args, |ax, options| {
        Region::Ellipse(EllipseSelector::new(ax, options))
    })
}
