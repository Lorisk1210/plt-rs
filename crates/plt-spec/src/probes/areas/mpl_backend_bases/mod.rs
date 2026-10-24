//! Probe operations for the `mpl_backend_bases` area: matplotlib's own
//! tests of collections' path reuse, canvas helpers, events and the
//! navigation toolbar (see `spec/reference/probes/test_mpl_backend_bases.py`).
//! The toolbar ops are in [`toolbar`], the colorbar one in [`colorbar`].

mod colorbar;
mod toolbar;

use std::cell::RefCell;
use std::rc::Rc;

use plt::artist::{ArtistHandle, Picker};
use plt::axes::AxisName;
use plt::events::{Event, EventKind, EventSource};
use plt::figure::Figure;
use plt::navigation::PointerButton;
use plt::pyplot;
use plt::text::{HAlign, VAlign};
use plt::ticker::{Formatter, FuncFormatter};
use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{GraphicsContext, PathCollection};

use crate::json::Json;
use crate::probes::encode::{count, pair};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("RendererBase._iter_collection", iter_collection),
    Op::new("FigureCanvasBase.get_default_filename", default_filename),
    Op::new("FigureCanvasBase.grab_mouse", grab_mouse),
    Op::new("FigureCanvasBase.get_width_height", width_height),
    Op::new("LocationEvent", location_event),
    Op::new("Axes.format_coord", format_coord),
    Op::new("pick_event", pick_event),
    Op::new("NavigationToolbar2.zoom", toolbar::zoom),
    Op::new("NavigationToolbar2.widgetlock", toolbar::widgetlock),
    Op::new("NavigationToolbar2.home", toolbar::home),
    Op::new("NavigationToolbar2.pan", toolbar::pan),
    Op::new("NavigationToolbar2.events", toolbar::events),
    Op::new("FigureCanvasBase.draw", toolbar::draw),
    Op::new("Colorbar.navigation", colorbar::navigation),
];

/// A count argument.
fn count_arg(args: Args<'_>, key: &str) -> Result<usize, ProbeError> {
    usize::try_from(args.i64(key)?)
        .map_err(|_| ProbeError::BadArgs(format!("{key} is not a count")))
}

/// The collection of `test_uses_per_path`: `paths` regular polygons,
/// `transforms` rotations, `offsets` points of `arange(20)` and two
/// colours, as many as asked, counting how often each raw path is drawn.
fn iter_collection(args: Args<'_>) -> Outcome {
    let paths: Vec<Path> = (3..3 + count_arg(args, "paths")?)
        .map(Path::unit_regular_polygon)
        .collect();
    let mut master = Affine2D::identity();
    let transforms: Vec<Affine2D> = (1..=count_arg(args, "transforms")?)
        .map(|i| {
            master = master.rotate(i as f64);
            master
        })
        .collect();
    let offsets: Vec<Point> = (0..count_arg(args, "offsets")?)
        .map(|i| Point::new((2 * i) as f64, (2 * i + 1) as f64))
        .collect();
    let colors = [Rgba::new(1.0, 0.0, 0.0, 1.0), Rgba::new(0.0, 0.5, 0.0, 1.0)];
    let take = |key| count_arg(args, key).map(|n| &colors[..n.min(colors.len())]);
    let collection = PathCollection {
        master_transform: master,
        paths: &paths,
        path_transforms: &transforms,
        offsets: &offsets,
        facecolors: take("facecolors")?,
        edgecolors: take("edgecolors")?,
        ..PathCollection::default()
    };
    let mut seen = vec![0_usize; collection.raw_paths().count()];
    for item in collection.items(&GraphicsContext::default()) {
        seen[item.path_index] += 1;
    }
    Ok(Json::Array(vec![
        count(collection.uses_per_path()),
        Json::Array(seen.into_iter().map(count).collect()),
    ]))
}

fn default_filename(args: Args<'_>) -> Outcome {
    pyplot::close_all();
    pyplot::figure();
    let manager = pyplot::get_current_fig_manager();
    let before = manager.default_filename();
    manager.set_window_title(args.str("title")?);
    let after = manager.default_filename();
    pyplot::close_all();
    let name = |name: Option<String>| name.map_or(Json::Null, Json::String);
    Ok(Json::Array(vec![name(before), name(after)]))
}

fn grab_mouse(_: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    fig.grab_mouse(&ax);
    let grabbed = fig.mouse_grabber() == Some(ax);
    fig.clear();
    Ok(Json::Array(vec![
        Json::from(grabbed),
        Json::from(fig.mouse_grabber().is_none()),
    ]))
}

fn width_height(args: Args<'_>) -> Outcome {
    let &[width, height] = args.f64_vec("figsize")?.as_slice() else {
        return Err(ProbeError::BadArgs("figsize is not a pair".into()));
    };
    let fig = Figure::with_size(width, height, args.f64("dpi")?);
    let (columns, rows) = fig.width_height();
    Ok(Json::Array(vec![
        Json::from(fig.size_pixels().1),
        pair((f64::from(columns), f64::from(rows))),
    ]))
}

/// The location of a pointer motion to `(x, y)`, as figure callbacks
/// receive it.
fn location_event(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    fig.subplot();
    let seen = Rc::new(RefCell::new(None));
    let sink = Rc::clone(&seen);
    fig.connect(EventKind::Motion, move |event| {
        if let Some(location) = event.location() {
            *sink.borrow_mut() = Some((location.x, location.y));
        }
    });
    EventSource::new().motion(&fig, args.f64("x")?, args.f64("y")?);
    let (x, y) = seen
        .borrow()
        .ok_or_else(|| ProbeError::rejected("no motion event"))?;
    Ok(pair((x, y)))
}

/// `format_coord` of a status bar, on fresh axes or after twins are added.
fn format_coord(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    if let Some(twins) = args.get("twins") {
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 20.0))
            .map_err(ProbeError::rejected)?;
        let mut seen = vec![Json::String(ax.format_coord(5.0, 5.0))];
        for twin in twins
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("twins is not a list".into()))?
        {
            match twin.as_str() {
                Some("x") => ax
                    .twinx()
                    .map_err(ProbeError::rejected)?
                    .set_ylim(0.0, 40.0)
                    .map_err(ProbeError::rejected)?,
                Some("y") => ax
                    .twiny()
                    .map_err(ProbeError::rejected)?
                    .set_xlim(0.0, 5.0)
                    .map_err(ProbeError::rejected)?,
                _ => return Err(ProbeError::BadArgs(format!("unknown twin {twin}"))),
            };
            seen.push(Json::String(ax.format_coord(5.0, 5.0)));
        }
        return Ok(Json::Array(seen));
    }
    if let Some(text) = args.opt_str("fmt")? {
        let text = text.to_owned();
        let formatter: Rc<dyn Formatter> = Rc::new(FuncFormatter::new(move |_, _| text.clone()));
        ax.set_fmt_data(AxisName::X, Some(Rc::clone(&formatter)))
            .set_fmt_data(AxisName::Y, Some(formatter));
    }
    let (x, y) = (args.f64("x")?, args.f64("y")?);
    let data = |value: Option<String>| value.map_or(Json::Null, Json::String);
    Ok(Json::Array(vec![
        Json::String(ax.format_coord(x, y)),
        data(ax.format_data(AxisName::X, x)),
        data(ax.format_data(AxisName::Y, y)),
    ]))
}

/// The pick events of a press over a picker text with `key` held, and the
/// key their mouse event reports.
fn pick_event(args: Args<'_>) -> Outcome {
    let key = args.str("key")?;
    let fig = Figure::new();
    let text = fig.text(0.5, 0.5, "hello");
    text.set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_picker(Some(Picker::Default));
    fig.layout();
    let keys = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&keys);
    fig.connect(EventKind::Pick, move |event| {
        if let Event::Pick(pick) = event {
            sink.borrow_mut().push(pick.mouse.key.clone());
        }
    });
    let (width, height) = fig.size_pixels();
    let mut source = EventSource::new();
    source.key_press(&fig, key, 0.0, 0.0);
    source.button_press(&fig, width / 2.0, height / 2.0, PointerButton::Left, false);
    source.key_release(&fig, key, 0.0, 0.0);
    let keys = keys.borrow();
    let first = keys
        .first()
        .cloned()
        .flatten()
        .map_or(Json::Null, Json::String);
    Ok(Json::Array(vec![count(keys.len()), first]))
}
