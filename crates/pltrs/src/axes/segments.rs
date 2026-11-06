//! Families of straight segments: `vlines` and `hlines`.

use pltrs_core::{Path, Point};

use super::broadcast::broadcast;
use super::children::DataUpdate;
use super::{Axes, AxesState, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist, CollectionKind, EdgeColors, paths_extents};
use crate::orientation::Orientation;
use crate::transforms::Transform;
use crate::units::AxisValues;

impl Axes {
    /// Maps `points` in place through the scales of the axes and back
    /// (see [`AxisHandle::round_trip`](super::AxisHandle)).
    fn round_trip(&self, points: &mut [Point]) {
        let (mut xs, mut ys): (Vec<f64>, Vec<f64>) = points.iter().map(|p| (p.x, p.y)).unzip();
        self.xaxis().round_trip(&mut xs);
        self.yaxis().round_trip(&mut ys);
        for (point, (x, y)) in points.iter_mut().zip(xs.into_iter().zip(ys)) {
            *point = Point::new(x, y);
        }
    }

    /// Vertical segments at `x` from `ymin` to `ymax` (matplotlib's
    /// `vlines`). `ymin` and `ymax` hold one value or one per segment;
    /// every argument may hold numbers, dates or categories.
    pub fn vlines(
        &self,
        x: impl Into<AxisValues>,
        ymin: impl Into<AxisValues>,
        ymax: impl Into<AxisValues>,
    ) -> Result<Collection, PlotError> {
        self.vlines_in(x, ymin, ymax, self.trans_data())
    }

    /// Horizontal segments at `y` from `xmin` to `xmax` (matplotlib's
    /// `hlines`). `xmin` and `xmax` hold one value or one per segment;
    /// every argument may hold numbers, dates or categories.
    pub fn hlines(
        &self,
        y: impl Into<AxisValues>,
        xmin: impl Into<AxisValues>,
        xmax: impl Into<AxisValues>,
    ) -> Result<Collection, PlotError> {
        self.hlines_in(y, xmin, xmax, self.trans_data())
    }

    /// [`Axes::vlines`] in `transform` coordinates (matplotlib's
    /// `vlines(..., transform=...)`): only the directions that are data
    /// values extend the data limits.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.vlines_in([2.0, 3.0], [0.0], [0.5], ax.trans_xaxis())?;
    /// let limits = ax.data_lim().bbox;
    /// assert_eq!((limits.x0(), limits.x1()), (2.0, 3.0));
    /// assert!(limits.y0().is_infinite(), "the heights are axes fractions");
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn vlines_in(
        &self,
        x: impl Into<AxisValues>,
        ymin: impl Into<AxisValues>,
        ymax: impl Into<AxisValues>,
        transform: Transform,
    ) -> Result<Collection, PlotError> {
        let (xaxis, yaxis) = (self.xaxis(), self.yaxis());
        self.segments_in(
            Orientation::Vertical,
            &xaxis.convert_units(x),
            &yaxis.convert_units(ymin),
            &yaxis.convert_units(ymax),
            transform,
        )
    }

    /// [`Axes::hlines`] in `transform` coordinates; see
    /// [`Axes::vlines_in`].
    pub fn hlines_in(
        &self,
        y: impl Into<AxisValues>,
        xmin: impl Into<AxisValues>,
        xmax: impl Into<AxisValues>,
        transform: Transform,
    ) -> Result<Collection, PlotError> {
        let (xaxis, yaxis) = (self.xaxis(), self.yaxis());
        self.segments_in(
            Orientation::Horizontal,
            &yaxis.convert_units(y),
            &xaxis.convert_units(xmin),
            &xaxis.convert_units(xmax),
            transform,
        )
    }

    /// Segments at `positions` spanning `low..high` along the value axis
    /// of `orientation`, drawn in `lines.color`.
    pub(crate) fn segments(
        &self,
        orientation: Orientation,
        positions: &[f64],
        low: &[f64],
        high: &[f64],
    ) -> Result<Collection, PlotError> {
        self.segments_in(orientation, positions, low, high, self.trans_data())
    }

    /// [`Axes::segments`] in `transform` coordinates.
    fn segments_in(
        &self,
        orientation: Orientation,
        positions: &[f64],
        low: &[f64],
        high: &[f64],
        transform: Transform,
    ) -> Result<Collection, PlotError> {
        let n = positions.len();
        let low = broadcast("min", low, n)?;
        let high = broadcast("max", high, n)?;
        let paths: Vec<Path> = positions
            .iter()
            .zip(low.iter().zip(&high))
            .map(|(&at, (&lo, &hi))| {
                // matplotlib combines the masks: a segment with a missing
                // coordinate is missing as a whole, also from the limits.
                let (at, lo, hi) = if [at, lo, hi].iter().all(|v| v.is_finite()) {
                    (at, lo, hi)
                } else {
                    (f64::NAN, f64::NAN, f64::NAN)
                };
                let start = orientation.place(at, lo);
                let end = orientation.place(at, hi);
                Path::polyline(vec![start.into(), end.into()])
            })
            .collect();
        // matplotlib adds the lines with `autolim=False`, so that `relim`
        // leaves them out, and extends the data limits itself, by the
        // extent's corners only, in the directions that are data values;
        // on rectilinear axes, the extent of the lines' vertices through
        // the scales and back (`get_datalim`), which linear scales leave
        // as they are.
        let through_maps = self.raw().with::<AxesState, _>(|axes| {
            axes.projection.is_rectilinear()
                && (axes.xaxis.map().is_some() || axes.yaxis.map().is_some())
        });
        let extents = if through_maps && transform == self.trans_data() {
            let mut through_scales = paths.clone();
            for path in &mut through_scales {
                self.round_trip(path.vertices_mut());
            }
            paths_extents(&through_scales).bbox
        } else {
            paths_extents(&paths).bbox
        };
        let rc = crate::rc::params();
        let mut collection = CollectionArtist::new(CollectionKind::Line, paths, rc.clone());
        collection.edgecolors = EdgeColors::Colors(vec![rc.lines_color.clone()]);
        let update = if n == 0 {
            DataUpdate::default()
        } else {
            DataUpdate {
                update: transform.data_directions(self.id()),
                ..DataUpdate::data(vec![extents.p0, extents.p1])
            }
        };
        collection.base_mut().transform = Some(transform);
        Ok(Collection::from_raw(self.adopt(collection, &update)))
    }
}
