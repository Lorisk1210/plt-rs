//! Families of straight segments: `vlines` and `hlines`.

use plt_core::Path;

use super::broadcast::broadcast;
use super::children::DataUpdate;
use super::{Axes, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist, CollectionKind, EdgeColors};
use crate::orientation::Orientation;
use crate::transforms::Transform;

impl Axes {
    /// Vertical segments at `x` from `ymin` to `ymax` (matplotlib's
    /// `vlines`). `ymin` and `ymax` hold one value or one per segment.
    pub fn vlines(
        &self,
        x: impl AsRef<[f64]>,
        ymin: impl AsRef<[f64]>,
        ymax: impl AsRef<[f64]>,
    ) -> Result<Collection, PlotError> {
        self.vlines_in(x, ymin, ymax, self.trans_data())
    }

    /// Horizontal segments at `y` from `xmin` to `xmax` (matplotlib's
    /// `hlines`). `xmin` and `xmax` hold one value or one per segment.
    pub fn hlines(
        &self,
        y: impl AsRef<[f64]>,
        xmin: impl AsRef<[f64]>,
        xmax: impl AsRef<[f64]>,
    ) -> Result<Collection, PlotError> {
        self.hlines_in(y, xmin, xmax, self.trans_data())
    }

    /// [`Axes::vlines`] in `transform` coordinates (matplotlib's
    /// `vlines(..., transform=...)`): only the directions that are data
    /// values extend the data limits.
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.vlines_in([2.0, 3.0], [0.0], [0.5], ax.trans_xaxis())?;
    /// let limits = ax.data_lim().bbox;
    /// assert_eq!((limits.x0(), limits.x1()), (2.0, 3.0));
    /// assert!(limits.y0().is_infinite(), "the heights are axes fractions");
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn vlines_in(
        &self,
        x: impl AsRef<[f64]>,
        ymin: impl AsRef<[f64]>,
        ymax: impl AsRef<[f64]>,
        transform: Transform,
    ) -> Result<Collection, PlotError> {
        self.segments_in(
            Orientation::Vertical,
            x.as_ref(),
            ymin.as_ref(),
            ymax.as_ref(),
            transform,
        )
    }

    /// [`Axes::hlines`] in `transform` coordinates; see
    /// [`Axes::vlines_in`].
    pub fn hlines_in(
        &self,
        y: impl AsRef<[f64]>,
        xmin: impl AsRef<[f64]>,
        xmax: impl AsRef<[f64]>,
        transform: Transform,
    ) -> Result<Collection, PlotError> {
        self.segments_in(
            Orientation::Horizontal,
            y.as_ref(),
            xmin.as_ref(),
            xmax.as_ref(),
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
        let paths = positions
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
        let rc = crate::rc::params();
        let mut collection = CollectionArtist::new(CollectionKind::Line, paths, rc.clone());
        collection.edgecolors = EdgeColors::Colors(vec![rc.lines_color.clone()]);
        // matplotlib extends the data limits by the extent's corners only,
        // in the directions that are data values.
        let extents = collection.shape_extents().bbox;
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
