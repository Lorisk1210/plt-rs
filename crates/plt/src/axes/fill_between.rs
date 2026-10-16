//! Areas between two curves (matplotlib's `fill_between` and
//! `fill_betweenx`, drawn as a `FillBetweenPolyCollection`).

use plt_core::{Path, Point};

use super::broadcast::{broadcast, common_length};
use super::children::DataUpdate;
use super::{Axes, DataLimits, PlotError};
use crate::artist::Artist;
use crate::collections::{Collection, CollectionArtist, CollectionKind, CollectionProps};
use crate::lines::{DrawStyle, StepWhere};
use crate::orientation::Orientation;

/// Options of [`Axes::fill_between`] and [`Axes::fill_betweenx`].
#[derive(Clone, Debug, Default)]
pub struct FillBetweenOptions {
    /// Fill only where set, one flag per point (matplotlib's `where`;
    /// `None` fills everywhere).
    pub mask: Option<Vec<bool>>,
    /// Extend each filled region to where the curves cross.
    pub interpolate: bool,
    /// Fill under a step function instead of straight segments.
    pub step: Option<StepWhere>,
    /// Style of the collection; without a face colour the next colour of
    /// the property cycle is used.
    pub props: CollectionProps,
}

/// The curves `f1(t)` and `f2(t)` with their validity mask.
struct Curves {
    t: Vec<f64>,
    f1: Vec<f64>,
    f2: Vec<f64>,
    /// Points that are selected and finite in all three coordinates.
    fill: Vec<bool>,
}

impl Axes {
    /// Fills between `y1` and `y2` along `x` (matplotlib's
    /// `fill_between`); `y1` and `y2` hold one value or one per point.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::FillBetweenOptions;
    /// let ax = Figure::new().subplot();
    /// ax.fill_between([0.0, 1.0, 2.0], [1.0, 3.0, 2.0], [0.0], FillBetweenOptions::default())?;
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the lengths of `x`, `y1`, `y2` or the mask disagree.
    pub fn fill_between(
        &self,
        x: impl AsRef<[f64]>,
        y1: impl AsRef<[f64]>,
        y2: impl AsRef<[f64]>,
        options: FillBetweenOptions,
    ) -> Result<Collection, PlotError> {
        let curves = Curves::new(Orientation::Vertical, x.as_ref(), y1.as_ref(), y2.as_ref(), options.mask.as_deref())?;
        self.add_fill_between(Orientation::Vertical, &curves, options)
    }

    /// Fills between `x1` and `x2` along `y` (matplotlib's
    /// `fill_betweenx`).
    ///
    /// # Errors
    ///
    /// As [`Axes::fill_between`].
    pub fn fill_betweenx(
        &self,
        y: impl AsRef<[f64]>,
        x1: impl AsRef<[f64]>,
        x2: impl AsRef<[f64]>,
        options: FillBetweenOptions,
    ) -> Result<Collection, PlotError> {
        let curves = Curves::new(Orientation::Horizontal, y.as_ref(), x1.as_ref(), x2.as_ref(), options.mask.as_deref())?;
        self.add_fill_between(Orientation::Horizontal, &curves, options)
    }

    fn add_fill_between(
        &self,
        orientation: Orientation,
        curves: &Curves,
        options: FillBetweenOptions,
    ) -> Result<Collection, PlotError> {
        let mut props = options.props;
        if !props.sets_face() {
            props = props.facecolor(self.next_patch_color());
        }
        let place = |t: f64, f: f64| {
            let (x, y) = orientation.place(t, f);
            Point::new(x, y)
        };
        let paths = contiguous_regions(&curves.fill)
            .into_iter()
            .map(|(start, stop)| {
                let outline = curves.region(start, stop, options.step, options.interpolate);
                Path::polygon(outline.into_iter().map(|(t, f)| place(t, f)))
            })
            .collect();
        let mut limits = DataLimits::NULL;
        limits.update_from_points(&curves.selected_points(place), true, true);
        let mut collection =
            CollectionArtist::new(CollectionKind::Poly, paths, crate::rc::params());
        props.apply(&mut collection);
        collection.base_mut().transform = Some(self.trans_data());
        Ok(Collection::from_raw(
            self.adopt(collection, &DataUpdate::limits(limits)),
        ))
    }
}

impl Curves {
    /// Validates and broadcasts the curves (matplotlib's `_validate_shapes`
    /// and `_get_data_mask`); `orientation` only names the arguments.
    fn new(
        orientation: Orientation,
        t: &[f64],
        f1: &[f64],
        f2: &[f64],
        mask: Option<&[bool]>,
    ) -> Result<Self, PlotError> {
        let names = match orientation {
            Orientation::Vertical => ("x", "y1", "y2"),
            Orientation::Horizontal => ("y", "x1", "x2"),
        };
        let n = common_length(&[(names.0, t.len()), (names.1, f1.len()), (names.2, f2.len())])?;
        if let Some(mask) = mask
            && mask.len() != t.len()
        {
            return Err(PlotError::Broadcast {
                argument: "where",
                len: mask.len(),
                points: t.len(),
            });
        }
        let t = broadcast(names.0, t, n)?;
        let f1 = broadcast(names.1, f1, n)?;
        let f2 = broadcast(names.2, f2, n)?;
        let selected = |i: usize| mask.is_none_or(|m| m.get(i).copied().unwrap_or(m[0]));
        let fill = (0..n)
            .map(|i| selected(i) && t[i].is_finite() && f1[i].is_finite() && f2[i].is_finite())
            .collect();
        Ok(Self { t, f1, f2, fill })
    }

    /// Every filled point on both curves, for the data limits.
    fn selected_points(&self, place: impl Fn(f64, f64) -> Point) -> Vec<Point> {
        let on = |f: &[f64]| -> Vec<Point> {
            (0..self.t.len())
                .filter(|&i| self.fill[i])
                .map(|i| place(self.t[i], f[i]))
                .collect()
        };
        let mut points = on(&self.f1);
        points.extend(on(&self.f2));
        points
    }

    /// The outline of the region `start..stop` as `(t, f)` pairs: along
    /// `f1`, then back along `f2` (matplotlib's `_make_verts_for_region`).
    fn region(
        &self,
        start: usize,
        stop: usize,
        step: Option<StepWhere>,
        interpolate: bool,
    ) -> Vec<(f64, f64)> {
        let (t, f1, f2) = (&self.t[start..stop], &self.f1[start..stop], &self.f2[start..stop]);
        let style = step.map_or(DrawStyle::Default, DrawStyle::from);
        let mut rows = style
            .apply(t, &[f1, f2])
            .expect("the curves have equal lengths")
            .into_iter();
        let (t, f1, f2) = (
            rows.next().unwrap_or_default(),
            rows.next().unwrap_or_default(),
            rows.next().unwrap_or_default(),
        );
        let (first, last) = if interpolate {
            (self.crossing(start), self.crossing(stop))
        } else {
            ((t[0], f2[0]), (t[t.len() - 1], f2[f2.len() - 1]))
        };
        let mut outline = Vec::with_capacity(2 * t.len() + 2);
        outline.push(first);
        outline.extend(t.iter().copied().zip(f1));
        outline.push(last);
        outline.extend(t.iter().copied().zip(f2).rev());
        outline
    }

    /// Where the curves cross next to index `idx`, or the nearest valid
    /// point (matplotlib's `_get_interpolating_points`).
    fn crossing(&self, idx: usize) -> (f64, f64) {
        let before = idx.saturating_sub(1);
        let end = (idx + 1).min(self.t.len());
        let valid = |i: usize| self.t[i].is_finite() && self.f1[i].is_finite() && self.f2[i].is_finite();
        if end - before == 2 {
            if !valid(idx) {
                return (self.t[before], self.f1[before]);
            }
            if !valid(before) {
                return (self.t[idx], self.f1[idx]);
            }
        }
        let t = &self.t[before..end];
        let diff: Vec<f64> = (before..end).map(|i| self.f1[i] - self.f2[i]).collect();
        let root = interp_sorted(0.0, &diff, t);
        (root, interp_sorted(root, t, &self.f1[before..end]))
    }
}

/// numpy's `interp(x, xp[order], fp[order])` with `order = xp.argsort()`
/// for the one or two points of a crossing.
#[expect(
    clippy::float_cmp,
    reason = "numpy returns the sample itself when x hits it exactly"
)]
fn interp_sorted(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    let mut order: Vec<usize> = (0..xp.len()).collect();
    order.sort_by(|&a, &b| xp[a].total_cmp(&xp[b]));
    let (xs, fs): (Vec<f64>, Vec<f64>) = order.iter().map(|&i| (xp[i], fp[i])).unzip();
    let (Some(&first), Some(&last)) = (xs.first(), xs.last()) else {
        return f64::NAN;
    };
    if x.is_nan() {
        return f64::NAN;
    }
    if x <= first {
        return fs[0];
    }
    if x >= last {
        return fs[fs.len() - 1];
    }
    let j = xs.partition_point(|&v| v <= x) - 1;
    if xs[j] == x {
        return fs[j];
    }
    let slope = (fs[j + 1] - fs[j]) / (xs[j + 1] - xs[j]);
    let value = slope * (x - xs[j]) + fs[j];
    if !value.is_nan() {
        return value;
    }
    let other = slope * (x - xs[j + 1]) + fs[j + 1];
    if other.is_nan() && fs[j] == fs[j + 1] {
        fs[j]
    } else {
        other
    }
}

/// The `(start, stop)` index ranges where `mask` is set (matplotlib's
/// `cbook.contiguous_regions`).
fn contiguous_regions(mask: &[bool]) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut start = None;
    for (i, &on) in mask.iter().enumerate() {
        match (on, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                regions.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        regions.push((s, mask.len()));
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_follow_the_mask() {
        assert_eq!(
            contiguous_regions(&[true, true, false, true, false, false, true]),
            [(0, 2), (3, 4), (6, 7)]
        );
        assert!(contiguous_regions(&[]).is_empty());
        assert!(contiguous_regions(&[false]).is_empty());
    }

    #[test]
    fn interpolation_matches_numpy() {
        assert_eq!(interp_sorted(0.0, &[1.0, -1.0], &[2.0, 3.0]), 2.5);
        assert_eq!(interp_sorted(5.0, &[1.0, 2.0], &[10.0, 20.0]), 20.0);
        assert_eq!(interp_sorted(0.0, &[4.0], &[7.0]), 7.0);
    }
}
