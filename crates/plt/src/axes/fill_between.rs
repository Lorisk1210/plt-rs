//! Areas between two curves (matplotlib's `fill_between` and
//! `fill_betweenx`, drawn as a `FillBetweenPolyCollection`).

use plt_core::{Path, Point};

use super::broadcast::{broadcast, common_length};
use super::children::DataUpdate;
use super::{Axes, DataLimits, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist, CollectionKind, CollectionProps};
use crate::lines::{DrawStyle, StepWhere};
use crate::numeric::argsort;
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
        let curves = Curves::new(
            Orientation::Vertical,
            x.as_ref(),
            y1.as_ref(),
            y2.as_ref(),
            options.mask.as_deref(),
        )?;
        Ok(self.add_fill_between(Orientation::Vertical, &curves, options))
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
        let curves = Curves::new(
            Orientation::Horizontal,
            y.as_ref(),
            x1.as_ref(),
            x2.as_ref(),
            options.mask.as_deref(),
        )?;
        Ok(self.add_fill_between(Orientation::Horizontal, &curves, options))
    }

    fn add_fill_between(
        &self,
        orientation: Orientation,
        curves: &Curves,
        options: FillBetweenOptions,
    ) -> Collection {
        let mut props = options.props;
        if !props.sets_face() {
            props = props.facecolor(self.next_patch_color());
        }
        let geometry = FillGeometry {
            orientation,
            step: options.step,
            interpolate: options.interpolate,
        };
        let mut limits = DataLimits::NULL;
        limits.update_from_points(
            &curves.selected_points(|t, f| geometry.place(t, f)),
            true,
            true,
        );
        let mut collection = CollectionArtist::new(
            CollectionKind::Poly,
            geometry.polygons(curves),
            crate::rc::params(),
        );
        collection.fill_geometry = Some(geometry);
        props.apply(&mut collection);
        let transform = collection
            .base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        // Only the directions that are data values extend the data limits
        // (matplotlib's `_update_collection_limits`).
        let update = DataUpdate {
            update: transform.data_directions(self.id()),
            ..DataUpdate::limits(limits)
        };
        Collection::from_raw(self.adopt(collection, &update))
    }
}

/// How a `fill_between` collection turns its curves into polygons
/// (the state of matplotlib's `FillBetweenPolyCollection`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillGeometry {
    /// `Vertical` fills along x between y values (`fill_between`),
    /// `Horizontal` along y between x values (`fill_betweenx`).
    orientation: Orientation,
    step: Option<StepWhere>,
    interpolate: bool,
}

impl FillGeometry {
    /// The point at parameter `t` with value `f`.
    fn place(self, t: f64, f: f64) -> Point {
        let (x, y) = self.orientation.place(t, f);
        Point::new(x, y)
    }

    /// One polygon per contiguous filled region (matplotlib's
    /// `_make_verts`).
    fn polygons(self, curves: &Curves) -> Vec<Path> {
        contiguous_regions(&curves.fill)
            .into_iter()
            .map(|(start, stop)| {
                let outline = curves.region(start, stop, self.step, self.interpolate);
                Path::polygon(outline.into_iter().map(|(t, f)| self.place(t, f)))
            })
            .collect()
    }
}

impl Collection {
    /// Replaces the curves of a collection made by [`Axes::fill_between`]
    /// (`t` along x) or [`Axes::fill_betweenx`] (`t` along y), keeping its
    /// steps and interpolation; `mask` selects where to fill as the
    /// `fill_between` option does. The data limits stay as they were
    /// (matplotlib's `FillBetweenPolyCollection.set_data`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::FillBetweenOptions;
    /// let ax = Figure::new().subplot();
    /// let fill = ax.fill_between([0.0, 1.0], [0.0], [1.0], FillBetweenOptions::default())?;
    /// fill.set_fill_between_data(&[0.0, 1.0, 2.0], &[0.0], &[2.0, 1.0, 2.0], None)?;
    /// // Along y1 and back along y2, closed: 2 · 3 + 2 + 1 vertices.
    /// assert_eq!(fill.paths()[0].vertices().len(), 9);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the collection was not made by `fill_between` or
    /// `fill_betweenx`, or as [`Axes::fill_between`].
    pub fn set_fill_between_data(
        &self,
        t: &[f64],
        f1: &[f64],
        f2: &[f64],
        mask: Option<&[bool]>,
    ) -> Result<&Self, PlotError> {
        let geometry = self
            .raw()
            .with::<CollectionArtist, _>(|c| c.fill_geometry)
            .ok_or(PlotError::NotFillBetween)?;
        let curves = Curves::new(geometry.orientation, t, f1, f2, mask)?;
        let paths = geometry.polygons(&curves);
        self.raw()
            .with_mut::<CollectionArtist, _>(|c| c.paths = paths);
        Ok(self)
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
        let (t, f1, f2) = (
            &self.t[start..stop],
            &self.f1[start..stop],
            &self.f2[start..stop],
        );
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
        let valid =
            |i: usize| self.t[i].is_finite() && self.f1[i].is_finite() && self.f2[i].is_finite();
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
    let (xs, fs): (Vec<f64>, Vec<f64>) = argsort(xp).into_iter().map(|i| (xp[i], fp[i])).unzip();
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
    fn new_curves_keep_the_steps_and_the_data_limits() {
        use crate::figure::Figure;

        let ax = Figure::new().subplot();
        let options = FillBetweenOptions {
            step: Some(StepWhere::Pre),
            ..FillBetweenOptions::default()
        };
        let fill = ax
            .fill_between([0.0, 1.0, 2.0], [0.0], [1.0], options)
            .unwrap();
        let limits = ax.data_lim();
        fill.set_fill_between_data(&[0.0, 1.0, 2.0], &[0.0], &[5.0, 6.0, 7.0], None)
            .unwrap();
        assert_eq!(ax.data_lim(), limits, "the data limits stay");
        let outline = fill.paths()[0].vertices().to_vec();
        // Two stepped curves of 2 · 3 - 1 points, their ends and the closure.
        assert_eq!(outline.len(), 2 * 5 + 2 + 1);
        assert!(outline.iter().any(|p| (p.y - 7.0).abs() < 1e-12));
        let lines = ax.vlines([1.0], [0.0], [1.0]).unwrap();
        assert_eq!(
            lines
                .set_fill_between_data(&[0.0], &[0.0], &[1.0], None)
                .err(),
            Some(PlotError::NotFillBetween)
        );
    }

    #[test]
    fn only_data_directions_extend_the_data_limits() {
        use crate::figure::Figure;

        let ax = Figure::new().subplot();
        let options = FillBetweenOptions {
            mask: Some(vec![false, true, true, false]),
            props: CollectionProps::default().transform(ax.trans_xaxis()),
            ..FillBetweenOptions::default()
        };
        ax.fill_between([0.0, 1.0, 2.0, 3.0], [0.0], [1.0], options)
            .unwrap();
        let limits = ax.data_lim().bbox;
        assert_eq!((limits.x0(), limits.x1()), (1.0, 2.0), "the filled x");
        assert!(limits.y0().is_infinite(), "the heights are axes fractions");
    }

    #[test]
    fn regions_follow_the_mask() {
        assert_eq!(
            contiguous_regions(&[true, true, false, true, false, false, true]),
            [(0, 2), (3, 4), (6, 7)]
        );
        assert_eq!(contiguous_regions(&[]), Vec::new());
        assert_eq!(contiguous_regions(&[false]), Vec::new());
    }

    #[test]
    fn interpolation_matches_numpy() {
        assert_eq!(interp_sorted(0.0, &[1.0, -1.0], &[2.0, 3.0]), 2.5);
        assert_eq!(interp_sorted(5.0, &[1.0, 2.0], &[10.0, 20.0]), 20.0);
        assert_eq!(interp_sorted(0.0, &[4.0], &[7.0]), 7.0);
    }
}
