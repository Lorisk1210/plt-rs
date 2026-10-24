//! Probe operations for the `widgets` area: sliders, text boxes and
//! selectors driven by replayed mouse and key events (see
//! `spec/reference/probes/test_widgets.py`).

use std::cell::RefCell;
use std::rc::Rc;

use plt::events::EventSource;
use plt::figure::Figure;
use plt::navigation::PointerButton;
use plt::widgets::{
    Orientation, RectangleSelector, RectangleSelectorOptions, Slider, SliderOptions, SpanSelector,
    SpanSelectorOptions, TextBox, TextBoxOptions, ValStep,
};

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("widgets.Slider", slider),
    Op::new("widgets.TextBox", textbox),
    Op::new("widgets.SpanSelector", span),
    Op::new("widgets.RectangleSelector", rectangle),
];

fn bad(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("{what} is malformed"))
}

/// A value recorder shared with a widget callback.
type Log<T> = Rc<RefCell<Vec<T>>>;

fn numbers(values: &[f64]) -> Json {
    Json::Array(values.iter().map(|&v| Json::Number(v)).collect())
}

/// Replays `events` on `figure`; the pointer stays where the last mouse
/// event left it for key events.
fn replay(figure: &Figure, events: &Json) -> Result<(), ProbeError> {
    let mut source = EventSource::new();
    let mut at = (0.0, 0.0);
    for event in events.as_array().ok_or_else(|| bad("events"))? {
        let field = |name: &str| event.get(name).ok_or_else(|| bad(name));
        let kind = field("type")?.as_str().ok_or_else(|| bad("type"))?;
        if matches!(kind, "press" | "release" | "motion") {
            let coordinate = |name: &str| field(name)?.as_f64().ok_or_else(|| bad(name));
            at = (coordinate("x")?, coordinate("y")?);
        }
        let button = || match field("button")?.as_i64() {
            Some(1) => Ok(PointerButton::Left),
            Some(2) => Ok(PointerButton::Middle),
            Some(3) => Ok(PointerButton::Right),
            _ => Err(bad("button")),
        };
        let key = || field("key")?.as_str().ok_or_else(|| bad("key"));
        match kind {
            "press" => source.button_press(figure, at.0, at.1, button()?, false),
            "release" => source.button_release(figure, at.0, at.1, button()?),
            "motion" => source.motion(figure, at.0, at.1),
            "key_press" => source.key_press(figure, key()?, at.0, at.1),
            "key_release" => source.key_release(figure, key()?, at.0, at.1),
            other => return Err(ProbeError::BadArgs(format!("unknown event {other:?}"))),
        }
    }
    Ok(())
}

fn orientation(name: &str) -> Result<Orientation, ProbeError> {
    match name {
        "horizontal" => Ok(Orientation::Horizontal),
        "vertical" => Ok(Orientation::Vertical),
        other => Err(ProbeError::BadArgs(format!(
            "unknown orientation {other:?}"
        ))),
    }
}

/// A slider from 0 to 10; its final value and every value it reported.
fn slider(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let direction = orientation(args.str("orientation")?)?;
    let rect = match direction {
        Orientation::Horizontal => [0.2, 0.4, 0.6, 0.1],
        Orientation::Vertical => [0.45, 0.1, 0.1, 0.8],
    };
    let valstep = match args.get("valstep") {
        None => None,
        Some(Json::Array(_)) => Some(ValStep::Values(args.f64_vec("valstep")?)),
        Some(_) => Some(ValStep::Step(args.f64("valstep")?)),
    };
    let options = SliderOptions {
        valinit: args.f64("valinit")?,
        valstep,
        closedmin: args.bool("closedmin")?,
        closedmax: args.bool("closedmax")?,
        dragging: args.bool("dragging")?,
        orientation: direction,
        ..SliderOptions::default()
    };
    let slider = Slider::new(
        &fig.add_axes(rect).map_err(ProbeError::rejected)?,
        "s",
        0.0,
        10.0,
        options,
    )
    .map_err(ProbeError::rejected)?;
    let changes: Log<f64> = Rc::default();
    let sink = Rc::clone(&changes);
    slider.on_changed(move |value| sink.borrow_mut().push(value));
    fig.layout();
    replay(&fig, args.json("events")?)?;
    let changes = numbers(&changes.borrow());
    Ok(Json::Object(vec![
        ("val".to_owned(), Json::Number(slider.val())),
        ("changes".to_owned(), changes),
    ]))
}

/// A text box: its final text, and what it reported on every change and
/// submission.
fn textbox(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig
        .add_axes([0.2, 0.4, 0.6, 0.15])
        .map_err(ProbeError::rejected)?;
    let box_ = TextBox::new(&ax, "t", args.str("initial")?, TextBoxOptions::default());
    let changes: Log<String> = Rc::default();
    let submits: Log<String> = Rc::default();
    let (change_sink, submit_sink) = (Rc::clone(&changes), Rc::clone(&submits));
    box_.on_text_change(move |text| change_sink.borrow_mut().push(text.to_owned()));
    box_.on_submit(move |text| submit_sink.borrow_mut().push(text.to_owned()));
    fig.layout();
    replay(&fig, args.json("events")?)?;
    let texts =
        |log: &Log<String>| Json::Array(log.borrow().iter().cloned().map(Json::String).collect());
    Ok(Json::Object(vec![
        ("text".to_owned(), Json::String(box_.text())),
        ("changes".to_owned(), texts(&changes)),
        ("submits".to_owned(), texts(&submits)),
    ]))
}

/// Axes from `add_subplot()` with both limits 0 to 10.
fn selector_axes() -> Result<(Figure, plt::axes::Axes), ProbeError> {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .map_err(ProbeError::rejected)?;
    Ok((fig, ax))
}

fn option_f64(options: &Json, name: &str) -> Option<f64> {
    options.get(name).and_then(Json::as_f64)
}

fn option_bool(options: &Json, name: &str) -> Option<bool> {
    match options.get(name) {
        Some(Json::Bool(value)) => Some(*value),
        _ => None,
    }
}

/// A span selector: its extents and every selection it reported.
fn span(args: Args<'_>) -> Outcome {
    let (fig, ax) = selector_axes()?;
    let given = args.json("options")?;
    let defaults = SpanSelectorOptions::default();
    let snap_values = given
        .get("snap_values")
        .and_then(Json::as_array)
        .map(|values| values.iter().filter_map(Json::as_f64).collect());
    let options = SpanSelectorOptions {
        minspan: option_f64(given, "minspan").unwrap_or(defaults.minspan),
        interactive: option_bool(given, "interactive").unwrap_or(defaults.interactive),
        drag_from_anywhere: option_bool(given, "drag_from_anywhere")
            .unwrap_or(defaults.drag_from_anywhere),
        ignore_event_outside: option_bool(given, "ignore_event_outside")
            .unwrap_or(defaults.ignore_event_outside),
        snap_values,
        ..defaults
    };
    let selector = SpanSelector::new(&ax, orientation(args.str("direction")?)?, options);
    let selects: Log<[f64; 2]> = Rc::default();
    let sink = Rc::clone(&selects);
    selector.on_select(move |low, high| sink.borrow_mut().push([low, high]));
    fig.layout();
    replay(&fig, args.json("events")?)?;
    let (low, high) = selector.extents();
    let selects = Json::Array(selects.borrow().iter().map(|pair| numbers(pair)).collect());
    Ok(Json::Object(vec![
        ("extents".to_owned(), numbers(&[low, high])),
        ("selects".to_owned(), selects),
    ]))
}

/// A rectangle selector: its extents and the press and release points of
/// every selection.
fn rectangle(args: Args<'_>) -> Outcome {
    let (fig, ax) = selector_axes()?;
    let given = args.json("options")?;
    let defaults = RectangleSelectorOptions::default();
    let options = RectangleSelectorOptions {
        minspanx: option_f64(given, "minspanx").unwrap_or(defaults.minspanx),
        minspany: option_f64(given, "minspany").unwrap_or(defaults.minspany),
        interactive: option_bool(given, "interactive").unwrap_or(defaults.interactive),
        drag_from_anywhere: option_bool(given, "drag_from_anywhere")
            .unwrap_or(defaults.drag_from_anywhere),
        ignore_event_outside: option_bool(given, "ignore_event_outside")
            .unwrap_or(defaults.ignore_event_outside),
        ..defaults
    };
    let selector = RectangleSelector::new(&ax, options);
    let selects: Log<[f64; 4]> = Rc::default();
    let sink = Rc::clone(&selects);
    selector.on_select(move |press, release| {
        sink.borrow_mut()
            .push([press.xdata, press.ydata, release.xdata, release.ydata]);
    });
    fig.layout();
    replay(&fig, args.json("events")?)?;
    let (x0, x1, y0, y1) = selector.extents();
    let selects = Json::Array(selects.borrow().iter().map(|four| numbers(four)).collect());
    Ok(Json::Object(vec![
        ("extents".to_owned(), numbers(&[x0, x1, y0, y1])),
        ("selects".to_owned(), selects),
    ]))
}
