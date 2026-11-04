//! The part of a long line that its axes shows (matplotlib's `_subslice`
//! optimisation): a line of more than 1000 points with sorted x, in the
//! data coordinates of a rectilinear axes with a linear x scale, without
//! `markevery` and clipped to the axes, draws only the points in the x
//! view plus one on each side. Besides saving work, this keeps short
//! visible parts of long lines eligible for pixel snapping and spared from
//! path simplification, as in matplotlib.

use std::borrow::Cow;
use std::ops::Range;

use crate::artist::{ArtistBase, LayoutContext};
use crate::axes::AxesState;
use crate::scale::ScaleName;
use crate::ticker::AxisName;
use crate::transforms::Transform;

/// Lines with more points than this may draw only their visible part
/// (`Line2D._subslice_optim_min_size`).
const MIN_SIZE: usize = 1000;

/// The range of points a line through `x` draws in the current view of
/// its axes, or `None` when it draws them all.
pub(super) fn visible_range(
    ctx: LayoutContext<'_>,
    base: &ArtistBase,
    x: &[f64],
    has_markevery: bool,
) -> Option<Range<usize>> {
    let axes = base.parent?;
    if x.len() <= MIN_SIZE
        || has_markevery
        || !base.clip_on
        || base.transform != Some(Transform::Data(axes))
    {
        return None;
    }
    let state = ctx.store().get_as::<AxesState>(axes)?;
    let linear = state.axis(AxisName::X).scale().name() == ScaleName::Linear;
    if !state.projection.is_rectilinear() || !linear || !is_sorted_and_has_non_nan(x) {
        return None;
    }
    let filled = with_nans_interpolated(x);
    let (v0, v1) = state.view_interval(AxisName::X);
    let (low, high) = (v0.min(v1), v0.max(v1));
    let first = filled.partition_point(|&v| v < low);
    let last = filled.partition_point(|&v| v <= high);
    Some(first.saturating_sub(1)..(last + 1).min(x.len()))
}

/// matplotlib's `_path.is_sorted_and_has_non_nan`: the values other than
/// NaN never decrease, and there is at least one.
fn is_sorted_and_has_non_nan(values: &[f64]) -> bool {
    let mut last = f64::NEG_INFINITY;
    let mut found = false;
    for &v in values.iter().filter(|v| !v.is_nan()) {
        if v < last {
            return false;
        }
        last = v;
        found = true;
    }
    found
}

/// The values with each NaN replaced by linear interpolation over the
/// indices of its neighbours, held constant beyond the ends (numpy's
/// `interp`, as `Line2D.recache` fills `_x_filled`).
fn with_nans_interpolated(values: &[f64]) -> Cow<'_, [f64]> {
    if !values.iter().any(|v| v.is_nan()) {
        return Cow::Borrowed(values);
    }
    let known: Vec<(usize, f64)> = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, v)| !v.is_nan())
        .collect();
    Cow::Owned(
        values
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                if !v.is_nan() {
                    return v;
                }
                let after = known.partition_point(|&(k, _)| k < i);
                match (after.checked_sub(1).map(|b| known[b]), known.get(after)) {
                    (Some((i0, v0)), Some(&(i1, v1))) => {
                        v0 + (v1 - v0) * (i - i0) as f64 / (i1 - i0) as f64
                    }
                    (Some((_, v0)), None) => v0,
                    (None, Some(&(_, v1))) => v1,
                    (None, None) => v,
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::lines::LineProps;
    use crate::testing::{Call, RecordingRenderer};

    /// The vertex count of every line drawn with a width of 1.5 points.
    fn drawn_lines(x: &[f64], props: LineProps) -> Vec<usize> {
        let fig = Figure::new();
        let ax = fig.subplot();
        let y: Vec<f64> = (0..x.len()).map(|i| i as f64).collect();
        ax.plot_with(x, y, props.linewidth(1.5)).unwrap();
        ax.set_xlim(0.0, 2.0)
            .and_then(|ax| ax.set_ylim(0.0, 2.0))
            .expect("finite limits");
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).unwrap();
        renderer
            .calls
            .into_iter()
            .filter_map(|call| match call {
                Call::Path {
                    points, linewidth, ..
                } if (linewidth - 1.5).abs() < 1e-9 => Some(points.len()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn long_sorted_lines_draw_the_visible_part() {
        let sorted: Vec<f64> = (0..2000).map(f64::from).collect();
        // Points 0, 1 and 2 plus one past the view.
        assert_eq!(drawn_lines(&sorted, LineProps::default()), [4]);
        let mut shuffled = sorted.clone();
        shuffled.swap(0, 1999);
        assert_eq!(drawn_lines(&shuffled, LineProps::default()), [2000]);
        let marked = LineProps::default()
            .markevery(Some(crate::lines::MarkEvery::Every { start: 0, step: 1 }));
        assert_eq!(drawn_lines(&sorted, marked), [2000]);
        assert_eq!(drawn_lines(&sorted[..1000], LineProps::default()), [1000]);
    }

    #[test]
    fn sortedness_ignores_nans() {
        assert!(is_sorted_and_has_non_nan(&[1.0, f64::NAN, 3.0]));
        assert!(!is_sorted_and_has_non_nan(&[3.0, 5.0, f64::NAN, 0.0, 2.0]));
        assert!(!is_sorted_and_has_non_nan(&[f64::NAN, f64::NAN]));
        assert!(!is_sorted_and_has_non_nan(&[]));
    }

    #[test]
    fn nans_take_the_interpolated_index_value() {
        let filled = with_nans_interpolated(&[f64::NAN, 1.0, f64::NAN, 3.0, f64::NAN]);
        assert_eq!(*filled, [1.0, 1.0, 2.0, 3.0, 3.0]);
    }
}
