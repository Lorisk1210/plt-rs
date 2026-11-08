//! Tight layout of a single grid (matplotlib's `GridSpec.tight_layout`):
//! the grid's own margins and spacing are fitted to its axes, so several
//! grids can share a figure, each in a part of it.

use super::tight::{
    Measured, TightLayout, TightLayoutError, adjusted_params, warn_about_left_out_axes,
};
use crate::artist::{ArtistHandle, ArtistId};
use crate::figure::Figure;
use crate::gridspec::GridSpec;

impl GridSpec {
    /// Adjusts the margins and spacing of this grid once so that the
    /// decorations of the axes on it fit `params.rect` (the whole figure
    /// by default), and moves those axes (matplotlib's
    /// `GridSpec.tight_layout`). The axes of other grids stay where they
    /// are.
    ///
    /// # Errors
    ///
    /// When the decorations cannot fit; the grid is then left unchanged.
    /// Axes of other grids, or off grids, make it warn (through
    /// [`crate::warnings`]).
    ///
    /// ```
    /// use pltrs::gridspec::GridSpec;
    /// use pltrs::layout::TightLayout;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let grid = GridSpec::new(2, 1)?;
    /// let ax = fig.add_subplot_spec(&grid.cell(0, 0)?)?;
    /// let left_half = TightLayout {
    ///     rect: Some([None, None, Some(0.5), Some(1.0)]),
    ///     ..TightLayout::default()
    /// };
    /// grid.tight_layout(&fig, left_half)?;
    /// let (x, _, w, _) = ax.position().bounds();
    /// assert!(x + w < 0.5);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn tight_layout(
        &self,
        figure: &Figure,
        params: TightLayout,
    ) -> Result<&Self, TightLayoutError> {
        let axes: Vec<ArtistId> = figure.axes().iter().map(ArtistHandle::id).collect();
        let frame = figure.raw().id();
        // The axes of classes of the program's measuring themselves.
        figure.root_handle().measure_painters();
        figure
            .raw()
            .figure_state()
            .work_on(|state| -> Result<(), TightLayoutError> {
                let Some(subplot_params) = state.subplot_params_of(frame) else {
                    return Ok(());
                };
                warn_about_left_out_axes(state, &axes, Some(self));
                let measured = crate::text::measuring(|| {
                    Measured::new(state, &axes, &subplot_params, Some(self))
                })??;
                if let Some(overrides) = adjusted_params(state, &measured, &params)? {
                    self.merge_overrides(&overrides);
                    state.set_subplot_params(subplot_params)?;
                }
                Ok(())
            })?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_tight_layouts_leave_the_other_grids_alone() {
        // matplotlib's test_tight_layout6: two grids side by side.
        let fig = Figure::new();
        let (left, right) = (GridSpec::new(2, 1).unwrap(), GridSpec::new(3, 1).unwrap());
        let ax = fig.add_subplot_spec(&left.cell(0, 0).unwrap()).unwrap();
        let other = fig.add_subplot_spec(&right.cell(2, 0).unwrap()).unwrap();
        ax.set_title("title");
        let before = other.position();
        let half = TightLayout {
            rect: Some([None, None, Some(0.5), Some(1.0)]),
            ..TightLayout::default()
        };
        left.tight_layout(&fig, half).unwrap();
        assert_eq!(other.position(), before);
        let overrides = left.overrides();
        assert!(
            overrides.right.is_some_and(|right| right < 0.5),
            "{overrides:?}"
        );
        assert_eq!(overrides.wspace, None, "a single column");
        let (x, _, w, _) = ax.position().bounds();
        assert!(x + w < 0.5 && overrides.left == Some(x));
    }
}
