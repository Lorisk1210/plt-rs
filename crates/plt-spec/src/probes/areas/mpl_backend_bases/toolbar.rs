//! The toolbar ops of the `mpl_backend_bases` area: zooming, panning,
//! going home and the widget lock (matplotlib's `NavigationToolbar2`),
//! and layout on every canvas. Each draws its figure once before any
//! input, as a window does.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, AxisName};
use plt::events::LockOwner;
use plt::figure::Figure;
use plt::layout::LayoutEngine;
use plt::navigation::{DragKey, NavMode, Navigation, PointerButton};
use plt::rc::{self, Aspect};
use plt_core::{Bbox, Point};

use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// A pixel argument, `[x, y]`.
pub(super) fn pixel(args: Args<'_>, key: &str) -> Result<Point, ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        &[x, y] => Ok(Point::new(x, y)),
        _ => Err(ProbeError::BadArgs(format!("{key} is not a pixel"))),
    }
}

/// `[low, high]` as JSON.
fn interval((low, high): (f64, f64)) -> Json {
    Json::Array(vec![Json::from(low), Json::from(high)])
}

/// `[xlim, ylim]` of `ax` as JSON.
fn limits(ax: &Axes) -> Json {
    Json::Array(vec![interval(ax.xlim()), interval(ax.ylim())])
}

/// The navigate mode matplotlib gives every axes for `mode`.
fn mode_name(mode: NavMode) -> Json {
    match mode {
        NavMode::None => Json::Null,
        NavMode::Pan => Json::from("PAN"),
        NavMode::Zoom => Json::from("ZOOM"),
    }
}

/// A toolbar in `mode` on `fig`.
pub(super) fn toolbar(fig: &Figure, mode: NavMode) -> Result<Navigation, ProbeError> {
    let mut navigation = Navigation::new();
    navigation.toggle(fig, mode).map_err(ProbeError::rejected)?;
    Ok(navigation)
}

/// The toolbar mode of argument `tool`, `"zoom"` or `"pan"`.
pub(super) fn tool_mode(args: Args<'_>) -> Result<NavMode, ProbeError> {
    match args.str("tool")? {
        "zoom" => Ok(NavMode::Zoom),
        "pan" => Ok(NavMode::Pan),
        other => Err(ProbeError::BadArgs(format!("unknown tool {other}"))),
    }
}

/// A mouse button by matplotlib's number.
pub(super) fn button(number: i64) -> Result<PointerButton, ProbeError> {
    match number {
        1 => Ok(PointerButton::Left),
        2 => Ok(PointerButton::Middle),
        3 => Ok(PointerButton::Right),
        other => Err(ProbeError::BadArgs(format!("unknown button {other}"))),
    }
}

/// The key held, in matplotlib's naming.
fn drag_key(name: Option<&str>) -> Result<DragKey, ProbeError> {
    Ok(match name {
        None => DragKey::None,
        Some("x") => DragKey::X,
        Some("y") => DragKey::Y,
        Some("control") => DragKey::Control,
        Some("shift") => DragKey::Shift,
        Some(other) => return Err(ProbeError::BadArgs(format!("unknown key {other:?}"))),
    })
}

/// A press at `start`, a motion to `stop` with `key` held unless
/// `motion` is false, and a release there.
pub(super) fn drag(
    navigation: &mut Navigation,
    fig: &Figure,
    (start, stop): (Point, Point),
    button: PointerButton,
    key: DragKey,
    motion: bool,
) {
    navigation.press(fig, start, button);
    if motion {
        navigation.drag(stop, key);
    }
    navigation.release(fig, stop, key);
}

/// The data coordinates of pixel `at` in `ax`.
fn data_at(fig: &Figure, ax: &Axes, at: Point) -> Result<Point, ProbeError> {
    let to_data = fig
        .resolve_transform(&ax.trans_data().inverted())
        .map_err(ProbeError::rejected)?;
    Ok(to_data.transform_point(at))
}

pub(super) fn zoom(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.set_xscale(args.str("scale")?)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let (start, stop) = (pixel(args, "start")?, pixel(args, "stop")?);
    let mut seen = vec![mode_name(NavMode::None)];
    let mut navigation = toolbar(&fig, NavMode::Zoom)?;
    seen.push(mode_name(navigation.mode()));
    let (from, to) = (data_at(&fig, &ax, start)?, data_at(&fig, &ax, stop)?);
    let rectangle = (start, stop);
    drag(
        &mut navigation,
        &fig,
        rectangle,
        PointerButton::Left,
        DragKey::None,
        false,
    );
    seen.push(limits(&ax));
    seen.push(Json::Array(vec![
        interval((from.x, to.x)),
        interval((from.y, to.y)),
    ]));
    let back = (stop, start);
    drag(
        &mut navigation,
        &fig,
        back,
        PointerButton::Right,
        DragKey::None,
        false,
    );
    seen.push(limits(&ax));
    navigation
        .toggle(&fig, NavMode::Zoom)
        .map_err(ProbeError::rejected)?;
    seen.extend([
        mode_name(navigation.mode()),
        Json::from(ax.autoscale_on(Some(AxisName::X))),
        Json::from(ax.autoscale_on(Some(AxisName::Y))),
    ]);
    Ok(Json::Array(seen))
}

pub(super) fn widgetlock(_: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.plot([0.0, 1.0], [0.0, 1.0])
        .map_err(ProbeError::rejected)?;
    fig.lock_widgets(LockOwner::new())
        .map_err(ProbeError::rejected)?;
    let mut navigation = Navigation::new();
    let mut seen = Vec::new();
    for mode in [NavMode::Zoom, NavMode::Pan] {
        // A refusal is what the upstream test checks: the mode stays off.
        let _ = navigation.toggle(&fig, mode);
        seen.push(mode_name(navigation.mode()));
    }
    Ok(Json::Array(seen))
}

/// Both axes of `ax` on scale `name`, as the `k` and `l` keys toggle them.
fn set_scales(ax: &Axes, name: &str) -> Result<(), ProbeError> {
    ax.set_xscale(name).map_err(ProbeError::rejected)?;
    ax.set_yscale(name).map_err(ProbeError::rejected)?;
    Ok(())
}

pub(super) fn home(_: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let values: Vec<f64> = (0..11).map(f64::from).collect();
    ax.plot(values.clone(), values)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let mut navigation = toolbar(&fig, NavMode::Zoom)?;
    set_scales(&ax, "log")?;
    let mut seen = vec![limits(&ax)];
    set_scales(&ax, "linear")?;
    seen.push(limits(&ax));
    let to_pixels = fig
        .resolve_transform(&ax.trans_data())
        .map_err(ProbeError::rejected)?;
    let whole = |p: Point| Point::new(p.x.trunc(), p.y.trunc());
    let corners = (
        whole(to_pixels.transform_point(Point::new(2.0, 2.0))),
        whole(to_pixels.transform_point(Point::new(5.0, 5.0))),
    );
    drag(
        &mut navigation,
        &fig,
        corners,
        PointerButton::Left,
        DragKey::None,
        false,
    );
    navigation.home(&fig);
    seen.push(limits(&ax));
    set_scales(&ax, "log")?;
    seen.push(limits(&ax));
    Ok(Json::Array(seen))
}

pub(super) fn pan(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let y: Vec<f64> = (0..10).map(f64::from).collect();
    ax.plot_y(y);
    ax.set_aspect(Aspect::Equal).map_err(ProbeError::rejected)?;
    fig.layout();
    let key = drag_key(args.opt_str("key")?)?;
    let mut navigation = toolbar(&fig, NavMode::Pan)?;
    let path = (pixel(args, "start")?, pixel(args, "end")?);
    drag(&mut navigation, &fig, path, PointerButton::Left, key, true);
    Ok(limits(&ax))
}

/// The bottom and top axes of `test_interactive_pan_zoom_events` and their
/// twins, in that order.
fn events_scene(fig: &Figure, args: Args<'_>) -> Result<[Axes; 4], ProbeError> {
    let bottom = fig.subplot();
    let top = fig.add_subplot(2, 2, 1).map_err(ProbeError::rejected)?;
    top.set_zorder(99.0);
    let forward = match args.json("forward")? {
        Json::String(auto) if auto == "auto" => None,
        _ => Some(args.bool("forward")?),
    };
    top.set_forward_navigation_events(forward)
        .set_patch_visible(args.bool("patch_visible")?);
    let (top_twin, bottom_twin) = match args.str("twin_or_share")? {
        "share" => {
            let shared = |index, with: &Axes| -> Result<Axes, ProbeError> {
                let ax = fig.add_subplot(2, 2, index).map_err(ProbeError::rejected)?;
                ax.share_x(with).map_err(ProbeError::rejected)?;
                ax.share_y(with).map_err(ProbeError::rejected)?;
                Ok(ax)
            };
            (shared(2, &top)?, shared(3, &bottom)?)
        }
        "twin" => (top.twinx(), bottom.twinx()),
        other => return Err(ProbeError::BadArgs(format!("unknown pairing {other}"))),
    };
    for ax in [&top, &bottom] {
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    }
    Ok([bottom, top, bottom_twin, top_twin])
}

pub(super) fn events(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let [bottom, top, bottom_twin, top_twin] = events_scene(&fig, args)?;
    fig.layout();
    let mut navigation = toolbar(&fig, tool_mode(args)?)?;
    let path = (pixel(args, "start")?, pixel(args, "stop")?);
    drag(
        &mut navigation,
        &fig,
        path,
        button(args.i64("button")?)?,
        DragKey::None,
        true,
    );
    Ok(Json::Array(
        [&top, &bottom, &top_twin, &bottom_twin]
            .into_iter()
            .map(limits)
            .collect(),
    ))
}

/// The positions `place` gives `axes`, in figure fractions, as JSON.
fn positions(axes: &[Axes], place: fn(&Axes) -> Bbox) -> Json {
    Json::Array(
        axes.iter()
            .map(|ax| {
                let (x, y, width, height) = place(ax).bounds();
                Json::Array([x, y, width, height].into_iter().map(Json::from).collect())
            })
            .collect(),
    )
}

/// The positions before and after a draw, under the spec's reference
/// configuration (`matplotlib.testing.setup`'s font and no hinting), on
/// which the text extents the layout makes room for depend. Before the
/// draw, they are the axes' original positions: plt-rs's `position()`
/// runs a pending layout first, where matplotlib's `get_position` waits
/// for the draw.
pub(super) fn draw(args: Args<'_>) -> Outcome {
    let backend = args.str("backend")?;
    rc::context(|| {
        rc::reset();
        rc::set("font.family", "DejaVu Sans").map_err(ProbeError::rejected)?;
        rc::set("text.hinting", "none").map_err(ProbeError::rejected)?;
        let fig = Figure::new();
        fig.set_layout_engine(LayoutEngine::constrained());
        let axes = fig.subplots(2, 2).map_err(ProbeError::rejected)?;
        let axes = axes.as_slice();
        let before = positions(axes, Axes::original_position);
        match backend {
            "agg" => fig.to_png().map(drop),
            "svg" => fig.to_svg().map(drop),
            "pdf" => fig.to_pdf().map(drop),
            "ps" => fig.to_ps().map(drop),
            other => return Err(ProbeError::BadArgs(format!("unknown backend {other}"))),
        }
        .map_err(ProbeError::rejected)?;
        Ok(Json::Array(vec![before, positions(axes, Axes::position)]))
    })
}
