//! Which axes is under the pointer and the data coordinates there, as a
//! status bar shows them (matplotlib's `inaxes` and `format_coord`).

use std::any::Any;
use std::f64::consts::{PI, TAU};

use plt_core::Point;

use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState, AxisName};
use crate::figure::{Figure, FigureState};
use crate::ticker::{ScalarFormatter, TickContext, g_sig_digits, pyfmt};
use crate::transforms::{ResolvedTransform, TransformContext};

/// The axes under display point `at`: the visible axes whose background
/// contains it, the highest z-order first and, among equals, the first
/// created (matplotlib's `canvas.inaxes`).
pub fn axes_at(figure: &Figure, at: Point) -> Option<Axes> {
    figure
        .axes()
        .into_iter()
        .filter(|ax| ax.visible() && contains(ax, at))
        .fold(None, |best: Option<Axes>, ax| match best {
            Some(best) if best.zorder() >= ax.zorder() => Some(best),
            _ => Some(ax),
        })
}

/// Whether the background of `ax` contains display point `at`
/// (matplotlib's `in_axes`): its box, or the wedge of polar axes.
pub(super) fn contains(ax: &Axes, at: Point) -> bool {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    if let Some(outline) = state.axes_outline(ax.id()) {
        return outline
            .path
            .contains_point(at, Some(&outline.transform), 0.0);
    }
    state
        .axes_bbox(ax.id())
        .is_some_and(|bbox| bbox.contains(at))
}

/// The data → display transform of `ax`, brought up to date.
fn data_transform(ax: &Axes) -> Option<ResolvedTransform> {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    let state: &FigureState = &state;
    state.data_transform(ax.id())
}

/// The data coordinates of `ax` (and of its twins) at display point `at`
/// (matplotlib's `format_coord`); empty when they cannot be formatted.
pub(super) fn format_coord(ax: &Axes, at: Point) -> String {
    if ax.as_polar().is_some() {
        return polar_coord(ax, at).unwrap_or_default();
    }
    let mut twins = ax.twins();
    twins.sort_by(|a, b| a.zorder().total_cmp(&b.zorder()));
    let pairs: Option<Vec<String>> = twins
        .iter()
        .map(|twin| {
            let (x, y) = (
                format_value(twin, AxisName::X, at)?,
                format_value(twin, AxisName::Y, at)?,
            );
            Some(format!("({x}, {y})"))
        })
        .collect();
    pairs.map_or_else(String::new, |pairs| {
        format!("(x, y) = {}", pairs.join(" | "))
    })
}

/// One coordinate of display point `at` in the data of `ax`, formatted by
/// the axis' major formatter (`format_xdata`/`format_ydata`).
fn format_value(ax: &Axes, name: AxisName, at: Point) -> Option<String> {
    let trans = data_transform(ax)?;
    let inverse = trans.inverted()?;
    let pick = |p: Point| match name {
        AxisName::X => p.x,
        AxisName::Y => p.y,
    };
    let value = pick(inverse.transform_point(at));
    // ScalarFormatter resolves to the data size of a screen pixel.
    let step = match name {
        AxisName::X => Point::new(1.0, 0.0),
        AxisName::Y => Point::new(0.0, 1.0),
    };
    let resolution = [at - step, at + step]
        .into_iter()
        .map(|p| (pick(inverse.transform_point(p)) - value).abs())
        .fold(0.0, f64::max);
    ax.raw().with::<AxesState, _>(|state| {
        let axis = state.axis(name);
        let formatter: &dyn Any = axis.major_formatter.as_ref();
        if formatter.is::<ScalarFormatter>() {
            return ScalarFormatter::format_data_resolved(value, resolution, &axis.settings).ok();
        }
        let ((low, high), minpos) = state.data_interval(name);
        let info = axis.info(&crate::axes::axis::AxisGeometry {
            view: state.view_interval(name),
            data: (low, high),
            minpos,
            length_points: 0.0,
            tick_floor: None,
        });
        axis.major_formatter
            .format_data_short(value, &TickContext::new(&info, &axis.settings))
            .ok()
    })
}

/// matplotlib's `PolarAxes.format_coord`: θ in half turns and degrees and
/// r, each to the precision a screen pixel resolves.
fn polar_coord(ax: &Axes, at: Point) -> Option<String> {
    let trans = data_transform(ax)?;
    let inverse = trans.inverted()?;
    let data = inverse.transform_point(at);
    let (theta, r) = (data.x, data.y);
    let screen = trans.transform_point(data);
    let (mut delta_t, mut delta_r) = (0.0_f64, 0.0_f64);
    for dx in [-1.0, 0.0, 1.0] {
        for dy in [-1.0, 0.0, 1.0] {
            let near = inverse.transform_point(screen + Point::new(dx, dy));
            delta_t = delta_t.max(((near.x - theta + PI).rem_euclid(TAU) - PI).abs());
            delta_r = delta_r.max((near.y - r).abs());
        }
    }
    let theta = if theta < 0.0 { theta + TAU } else { theta };
    let (turns, delta_turns) = (theta / PI, delta_t / PI);
    let fixed = |value: f64, delta: f64| {
        let digits = (-delta.log10().floor()).max(0.0);
        digits
            .is_finite()
            .then(|| pyfmt::fixed(value, digits as usize))
    };
    let r_label = pyfmt::general_alternate(r, g_sig_digits(r, delta_r).ok()?);
    Some(format!(
        "\u{3b8}={}\u{3c0} ({}\u{b0}), r={r_label}",
        fixed(turns, delta_turns)?,
        fixed(turns * 180.0, delta_turns * 180.0)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(ax: &Axes, x: f64, y: f64) -> Point {
        data_transform(ax)
            .unwrap()
            .transform_point(Point::new(x, y))
    }

    #[test]
    fn finds_the_axes_under_the_pointer() {
        let fig = Figure::new();
        let grid = fig.subplots(1, 2).unwrap();
        let right = grid[(0, 1)].clone();
        assert_eq!(axes_at(&fig, display(&right, 0.5, 0.5)), Some(right));
        assert_eq!(axes_at(&fig, Point::new(1.0, 1.0)), None);
    }

    #[test]
    fn coordinates_use_the_screen_resolution() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 1.0);
        let text = format_coord(&ax, display(&ax, 2.5, 0.25));
        assert_eq!(text, "(x, y) = (2.50, 0.250)");
        let twin = ax.twinx();
        twin.set_ylim(0.0, 100.0);
        let text = format_coord(&ax, display(&ax, 2.5, 0.25));
        assert_eq!(text, "(x, y) = (2.50, 0.250) | (2.50, 25.0)");
    }

    #[test]
    fn polar_coordinates_in_turns_and_degrees() {
        let fig = Figure::new();
        let polar = fig.polar_subplot();
        polar.set_rlim(0.0, 2.0);
        let text = format_coord(&polar, display(&polar, PI / 2.0, 1.0));
        assert!(text.starts_with("\u{3b8}=0.50"), "{text}");
        assert!(text.contains("(90"), "{text}");
        assert!(text.contains("r=1.00"), "{text}");
    }
}
