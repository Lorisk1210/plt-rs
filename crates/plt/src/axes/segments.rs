//! Families of straight segments: `vlines` and `hlines`.

use plt_core::Path;

use super::broadcast::broadcast;
use super::children::DataUpdate;
use super::{Axes, PlotError};
use crate::artist::Artist;
use crate::collections::{Collection, CollectionArtist, CollectionKind, EdgeColors};
use crate::orientation::Orientation;

impl Axes {
    /// Vertical segments at `x` from `ymin` to `ymax` (matplotlib's
    /// `vlines`). `ymin` and `ymax` hold one value or one per segment.
    pub fn vlines(
        &self,
        x: impl AsRef<[f64]>,
        ymin: impl AsRef<[f64]>,
        ymax: impl AsRef<[f64]>,
    ) -> Result<Collection, PlotError> {
        self.segments(
            Orientation::Vertical,
            x.as_ref(),
            ymin.as_ref(),
            ymax.as_ref(),
        )
    }

    /// Horizontal segments at `y` from `xmin` to `xmax` (matplotlib's
    /// `hlines`). `xmin` and `xmax` hold one value or one per segment.
    pub fn hlines(
        &self,
        y: impl AsRef<[f64]>,
        xmin: impl AsRef<[f64]>,
        xmax: impl AsRef<[f64]>,
    ) -> Result<Collection, PlotError> {
        self.segments(
            Orientation::Horizontal,
            y.as_ref(),
            xmin.as_ref(),
            xmax.as_ref(),
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
        let n = positions.len();
        let low = broadcast("min", low, n)?;
        let high = broadcast("max", high, n)?;
        let paths = positions
            .iter()
            .zip(low.iter().zip(&high))
            .map(|(&at, (&lo, &hi))| {
                let start = orientation.place(at, lo);
                let end = orientation.place(at, hi);
                Path::polyline(vec![start.into(), end.into()])
            })
            .collect();
        let rc = crate::rc::params();
        let mut collection = CollectionArtist::new(CollectionKind::Line, paths, rc.clone());
        collection.edgecolors = EdgeColors::Colors(vec![rc.lines_color.clone()]);
        collection.base_mut().transform = Some(self.trans_data());
        // matplotlib extends the data limits by the extent's corners only.
        let extents = collection.shape_extents().bbox;
        let update = if n == 0 {
            DataUpdate::default()
        } else {
            DataUpdate::data(vec![extents.p0, extents.p1])
        };
        Ok(Collection::from_raw(self.adopt(collection, &update)))
    }
}
