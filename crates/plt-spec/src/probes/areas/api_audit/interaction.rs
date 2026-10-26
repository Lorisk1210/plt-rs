//! Probe operations of the `api_audit` area for the interaction API the
//! audit found missing: picking axes, moving span selectors, pausing
//! animations, resizing windows, mouse buttons, toolbar toggles and
//! blocked callbacks.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use plt::animation::{AnimationOptions, Frames, FuncAnimation};
use plt::artist::{ArtistHandle, Picker};
use plt::axes::{Axes, Axis};
use plt::events::{Event, EventKind, EventSource};
use plt::figure::Figure;
use plt::gui::{ToolKind, ToolSpec};
use plt::navigation::PointerButton;
use plt::pyplot;
use plt::widgets::{Orientation, SpanSelector, SpanSelectorOptions};
use plt_core::Point;

use crate::json::Json;
use crate::probes::encode::{count, numbers, pair};
use crate::probes::{Args, Outcome, ProbeError};

/// A 4 × 3 inch figure at 100 dpi and its axes, 100 to 300 pixels across
/// and 75 to 225 up.
fn framed() -> Result<(Figure, Axes), ProbeError> {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig
        .add_axes([0.25, 0.25, 0.5, 0.5])
        .map_err(ProbeError::rejected)?;
    Ok((fig, ax))
}

/// The x or y axis of `ax`, as `name` says.
fn axis_named(ax: &Axes, name: &str) -> Result<Axis, ProbeError> {
    match name {
        "x" => Ok(ax.xaxis()),
        "y" => Ok(ax.yaxis()),
        other => Err(ProbeError::BadArgs(format!("unknown axis {other}"))),
    }
}

/// A list of booleans.
fn flags(values: impl IntoIterator<Item = bool>) -> Json {
    Json::Array(values.into_iter().map(Json::Bool).collect())
}

/// A list of strings.
fn names<S: Into<String>>(values: impl IntoIterator<Item = S>) -> Json {
    Json::Array(
        values
            .into_iter()
            .map(|value| Json::String(value.into()))
            .collect(),
    )
}

pub(super) fn pickradius(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let axis = axis_named(&ax, args.str("axis")?)?;
    let default = axis.pickradius();
    axis.set_pickradius(args.f64("radius")?)
        .map_err(ProbeError::rejected)?;
    Ok(numbers([default, axis.pickradius(), axis.pickradius()]))
}

pub(super) fn axis_contains(args: Args<'_>) -> Outcome {
    let (_fig, ax) = framed()?;
    let axis = axis_named(&ax, args.str("axis")?)?;
    axis.set_pickradius(args.f64("radius")?)
        .map_err(ProbeError::rejected)?;
    // A mouse event lies on whole pixels (matplotlib's `int(x)`).
    let points = args.pairs("points")?;
    Ok(flags(points.into_iter().map(|(x, y)| {
        axis.contains(Point::new(x.trunc(), y.trunc()))
    })))
}

pub(super) fn axis_pick(args: Args<'_>) -> Outcome {
    let (fig, ax) = framed()?;
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 1.0))
        .map_err(ProbeError::rejected)?;
    let pickers = args
        .json("pickers")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("pickers is not a list".into()))?;
    for name in pickers {
        let name = name
            .as_str()
            .ok_or_else(|| ProbeError::BadArgs("a picker is not a name".into()))?;
        axis_named(&ax, name)?.set_picker(Some(Picker::Default));
    }
    if args.bool("line")? {
        let line = ax
            .plot([0.5, 0.5], [-0.2, 0.5])
            .map_err(ProbeError::rejected)?;
        line.set_clip_on(false)
            .set_picker(Some(Picker::Radius(5.0)));
    }
    fig.layout();
    let picks = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&picks);
    fig.connect(EventKind::Pick, move |event| {
        if let Event::Pick(pick) = event {
            let name = match pick.picked_axis().map(|axis| axis.name()) {
                Some(plt::ticker::AxisName::X) => "xaxis",
                Some(plt::ticker::AxisName::Y) => "yaxis",
                None => "line",
            };
            log.borrow_mut().push(name);
        }
    });
    let (x, y) = pair_arg(args, "at")?;
    EventSource::new().button_press(&fig, x, y, PointerButton::Left, false);
    Ok(names(picks.take()))
}

/// Argument `key` as a pair of numbers.
fn pair_arg(args: Args<'_>, key: &str) -> Result<(f64, f64), ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        &[x, y] => Ok((x, y)),
        _ => Err(ProbeError::BadArgs(format!("{key} is not a pair"))),
    }
}

/// A press, a motion and a release of the left button, from `x0` to `x1`
/// at height `y`.
fn drag(fig: &Figure, x0: f64, x1: f64, y: f64) {
    let mut source = EventSource::new();
    source.button_press(fig, x0, y, PointerButton::Left, false);
    source.motion(fig, x1, y);
    source.button_release(fig, x1, y, PointerButton::Left);
}

pub(super) fn span_new_axes(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let left = fig
        .add_axes([0.0, 0.0, 0.5, 1.0])
        .map_err(ProbeError::rejected)?;
    left.set_xlim(0.0, 10.0).map_err(ProbeError::rejected)?;
    let span = SpanSelector::new(
        &left,
        Orientation::Horizontal,
        SpanSelectorOptions::default(),
    );
    let selections = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&selections);
    span.on_select(move |vmin, vmax| log.borrow_mut().push(pair((vmin, vmax))));
    let (target_fig, target, (x0, x1)) = match args.str("target")? {
        "same" => {
            let right = fig
                .add_axes([0.5, 0.0, 0.5, 1.0])
                .map_err(ProbeError::rejected)?;
            right.set_xlim(100.0, 200.0).map_err(ProbeError::rejected)?;
            (fig.clone(), right, (240.0, 300.0))
        }
        "other" => {
            let other = Figure::with_size(4.0, 3.0, 100.0);
            let whole = other
                .add_axes([0.0, 0.0, 1.0, 1.0])
                .map_err(ProbeError::rejected)?;
            whole.set_xlim(0.0, 100.0).map_err(ProbeError::rejected)?;
            (other, whole, (80.0, 160.0))
        }
        other => return Err(ProbeError::BadArgs(format!("unknown target {other}"))),
    };
    span.new_axes(&target);
    drag(&fig, 50.0, 150.0, 150.0);
    drag(&target_fig, x0, x1, 150.0);
    Ok(Json::Array(vec![
        Json::Array(selections.take()),
        Json::Bool(span.ax() == target),
        count(target.patches().len()),
        pair(span.extents()),
    ]))
}

pub(super) fn animation_pause(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(2.0, 2.0, 50.0);
    let line = fig
        .subplot()
        .plot([0.0, 1.0], [0.0, 0.0])
        .map_err(ProbeError::rejected)?;
    let frames = Rc::new(RefCell::new(Vec::new()));
    let (seen, moved) = (Rc::clone(&frames), line.clone());
    let options = AnimationOptions {
        blit: args.bool("blit")?,
        interval: Duration::from_millis(10),
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(&fig, Frames::Count(5), options, move |i| {
        seen.borrow_mut().push(i);
        let y = i as f64;
        moved.set_data([0.0, 1.0], [0.0, y]);
        moved.clone()
    });
    fig.canvas().draw().map_err(ProbeError::rejected)?;
    let (timer, _) = *fig
        .timers()
        .first()
        .ok_or_else(|| ProbeError::Rejected("the animation did not start".into()))?;
    fig.fire_timer(timer);
    let mut states = vec![line.animated()];
    animation.pause();
    states.push(line.animated());
    animation.resume();
    states.push(line.animated());
    let frames = frames.take();
    Ok(Json::Array(vec![
        Json::Array(frames.into_iter().map(count).collect()),
        flags(states),
    ]))
}

pub(super) fn manager_resize(_args: Args<'_>) -> Outcome {
    pyplot::switch_backend(pyplot::Backend::Agg);
    let fig = pyplot::figure();
    pyplot::get_current_fig_manager()
        .resize(300, 200)
        .map_err(ProbeError::rejected)?;
    let size = fig.size_inches();
    pyplot::close_figure(&fig);
    Ok(pair(size))
}

pub(super) fn mouse_buttons(_args: Args<'_>) -> Outcome {
    Ok(Json::Array(
        PointerButton::ALL
            .into_iter()
            .map(|button| {
                Json::Array(vec![
                    Json::String(button.keymap_name().into()),
                    count(button.number().into()),
                ])
            })
            .collect(),
    ))
}

pub(super) fn toggle_tools(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let toggle = |name: &str, group: Option<&str>, start: bool| {
        let spec = ToolSpec {
            name: name.into(),
            kind: ToolKind::Toggle,
            radio_group: group.map(str::to_owned),
            default_toggled: start,
            ..ToolSpec::default()
        };
        let log = Rc::clone(&log);
        fig.add_tool(spec, move |event| {
            log.borrow_mut().push(Json::Array(vec![
                Json::String(event.name.into()),
                Json::Bool(event.toggled),
            ]));
        })
        .map(|_| ())
        .map_err(ProbeError::rejected)
    };
    let tools: &[&str] = match args.str("scenario")? {
        "radio" => {
            toggle("a", Some("g"), false)?;
            toggle("b", Some("g"), false)?;
            toggle("c", None, false)?;
            for name in ["a", "c", "b", "b"] {
                fig.trigger_tool(name);
            }
            &["a", "b", "c"]
        }
        "default_toggled" => {
            toggle("a", Some("g"), false)?;
            fig.trigger_tool("a");
            toggle("d", Some("g"), true)?;
            &["a", "d"]
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scenario {other}"))),
    };
    let toggled = tools
        .iter()
        .map(|name| fig.tool_toggled(name).unwrap_or(false));
    Ok(Json::Array(vec![Json::Array(log.take()), flags(toggled)]))
}

pub(super) fn callbacks_blocked(args: Args<'_>) -> Outcome {
    const SIGNALS: [(EventKind, &str); 3] = [
        (EventKind::ButtonPress, "button_press_event"),
        (EventKind::KeyPress, "key_press_event"),
        (EventKind::Pick, "pick_event"),
    ];
    let signal = match args.opt_str("signal")? {
        None => None,
        Some(name) => Some(
            SIGNALS
                .iter()
                .find(|(_, signal)| *signal == name)
                .map(|(kind, _)| *kind)
                .ok_or_else(|| ProbeError::BadArgs(format!("unknown signal {name}")))?,
        ),
    };
    let (fig, ax) = framed()?;
    ax.set_picker(Some(Picker::Default));
    let log = Rc::new(RefCell::new(Vec::new()));
    for (kind, name) in SIGNALS {
        let log = Rc::clone(&log);
        fig.connect(kind, move |_| log.borrow_mut().push(name));
    }
    let send = || {
        let mut source = EventSource::new();
        source.button_press(&fig, 200.0, 150.0, PointerButton::Left, false);
        source.key_press(&fig, "a", 200.0, 150.0);
    };
    fig.with_callbacks_blocked(signal, send);
    let blocked = log.take();
    send();
    Ok(Json::Array(vec![names(blocked), names(log.take())]))
}
