//! Creating polar axes: on a figure (alone, in grid cells, in shared
//! grids, at a free position) and as insets of other axes (matplotlib's
//! `projection="polar"`).

use super::PolarAxes;
use crate::axes::{Axes, share_free};
use crate::figure::{AxesRectError, Figure, Share};
use crate::gridspec::{GridSpec, GridSpecError, SubplotSpec};
use crate::numeric::Grid;

impl Figure {
    /// Adds polar axes filling the default subplot area
    /// (`fig.add_subplot(projection="polar")`).
    pub fn polar_subplot(&self) -> PolarAxes {
        PolarAxes::convert(self.subplot())
    }

    /// A grid of `nrows × ncols` polar axes sharing their θ and radial axes
    /// as requested (`fig.subplots(nrows, ncols, sharex=..., sharey=...,
    /// subplot_kw={"projection": "polar"})`). Unlike rectilinear grids,
    /// every axes keeps its tick labels.
    ///
    /// ```
    /// use plt::figure::{Figure, Share};
    /// let fig = Figure::new();
    /// let axs = fig.polar_subplots(1, 2, Share::None, Share::All)?;
    /// axs[(0, 1)].set_ylim(0.0, 3.0)?;
    /// assert_eq!(axs[(0, 0)].ylim(), (0.0, 3.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the grid is empty.
    pub fn polar_subplots(
        &self,
        nrows: usize,
        ncols: usize,
        sharex: Share,
        sharey: Share,
    ) -> Result<Grid<PolarAxes>, GridSpecError> {
        let grid = GridSpec::new(nrows, ncols)?;
        let axes = Self::subplots_made(
            &grid,
            sharex,
            sharey,
            |spec| {
                self.add_polar_subplot_spec(spec)
                    .map(|polar| polar.axes().clone())
            },
            |axes, name, target| {
                share_free(axes, name, target);
                Ok(())
            },
        )?;
        Ok(axes.map(|ax| ax.as_polar().expect("polar_subplots makes polar axes")))
    }

    /// Adds polar axes in cell `index` (1-based, row-major) of an
    /// `nrows × ncols` grid.
    ///
    /// # Errors
    ///
    /// When the grid is empty or `index` is outside it.
    pub fn add_polar_subplot(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
    ) -> Result<PolarAxes, GridSpecError> {
        self.add_subplot(nrows, ncols, index)
            .map(PolarAxes::convert)
    }

    /// Adds polar axes covering a grid cell or block.
    ///
    /// # Errors
    ///
    /// When the subplot parameters leave no room for the cell.
    pub fn add_polar_subplot_spec(&self, spec: &SubplotSpec) -> Result<PolarAxes, GridSpecError> {
        self.add_subplot_spec(spec).map(PolarAxes::convert)
    }

    /// Adds polar axes occupying `[left, bottom, width, height]` in figure
    /// fractions.
    ///
    /// # Errors
    ///
    /// As [`Figure::add_axes`].
    pub fn add_polar_axes(&self, rect: [f64; 4]) -> Result<PolarAxes, AxesRectError> {
        self.add_axes(rect).map(PolarAxes::convert)
    }
}

impl Axes {
    /// Adds polar inset axes at `[left, bottom, width, height]` in this
    /// axes' fractions (matplotlib's `inset_axes(..., polar=True)`; see
    /// [`Axes::inset_axes`]).
    #[must_use]
    pub fn polar_inset_axes(&self, bounds: [f64; 4]) -> PolarAxes {
        PolarAxes::convert(self.inset_axes(bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::ProjectionKind;

    #[test]
    fn shared_polar_grids_keep_every_tick_label() {
        // matplotlib's test_shared_polar_keeps_ticklabels: grids hide inner
        // labels on rectangular axes only.
        let fig = Figure::new();
        let axs = fig.polar_subplots(2, 2, Share::All, Share::All).unwrap();
        let ax = &axs[(0, 1)];
        assert_eq!(ax.xaxis().ticklabels(false).unwrap().len(), 8);
        assert_eq!(ax.yaxis().ticklabels(false).unwrap().len(), 5);
        let rectilinear = fig.subplots_shared(2, 2, Share::All, Share::All).unwrap();
        assert_eq!(
            rectilinear[(0, 1)].xaxis().ticklabels(false).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn polar_insets_are_child_axes() {
        let fig = Figure::new();
        let host = fig.subplot();
        let inset = host.polar_inset_axes([0.5, 0.1, 0.45, 0.45]);
        assert_eq!(inset.projection(), ProjectionKind::Polar);
        assert_eq!(inset.zorder(), 5.0);
        assert_eq!(host.child_axes(), [inset.axes().clone()]);
        assert_eq!(fig.axes(), [host]);
    }
}
