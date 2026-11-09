//! An axes given its figure again (matplotlib's `_AxesBase.set_figure`
//! called on an axes already in it), which makes its boxes and transforms
//! anew.

use pltrs_core::Bbox;

use super::{Axes, AxesState, DataLimits};

impl Axes {
    /// What matplotlib's `_AxesBase.set_figure` leaves of an axes it makes
    /// the boxes and transforms of anew: the view limits the unit box (as
    /// they are: no autoscaling, the scales not consulted), the data limits
    /// null, and the transform of each axis' scale the identity (a new
    /// `transScale`) until the axis' scale is set again. The scales
    /// themselves, which place the ticks, stay.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot([1.0, 100.0], [1.0, 10.0])?;
    /// ax.set_yscale("log")?;
    /// ax.reset_limits_and_scale_transforms();
    /// assert_eq!(ax.ylim(), (0.0, 1.0));
    /// assert!(ax.data_lim().bbox.is_null());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn reset_limits_and_scale_transforms(&self) {
        self.raw.with_mut::<AxesState, _>(|axes| {
            axes.view_lim = Bbox::UNIT;
            axes.view_mutated = (false, false);
            axes.data_lim = DataLimits::NULL;
            axes.xaxis.unmap_scale();
            axes.yaxis.unmap_scale();
        });
        self.raw.figure_state().borrow_mut().stale = true;
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;

    #[test]
    fn the_data_maps_linearly_until_the_scale_is_set_again() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]).unwrap();
        ax.set_autoscale_on(false);
        ax.set_xscale("log").unwrap();
        ax.reset_limits_and_scale_transforms();
        let middle = ax.data_to_display((0.5, 0.5)).unwrap();
        assert!((middle.x - 200.0).abs() < 1e-9, "linear: {middle:?}");
        ax.set_xscale("log").unwrap();
        ax.set_xlim(1.0, 100.0).unwrap();
        let middle = ax.data_to_display((10.0, 0.5)).unwrap();
        assert!(
            (middle.x - 200.0).abs() < 1e-9,
            "logarithmic again: {middle:?}"
        );
    }
}
