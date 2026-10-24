//! Non-linear scales on the axes of a 3-D axes (matplotlib 3.11's
//! `Axes3D.set_xscale`, `set_yscale`, `set_zscale` and `_set_axis_scale`):
//! data are transformed by the scales before the projection, ticks are
//! placed by the scales' locators, and autoscaling, panning and zooming
//! work in transformed coordinates.
//!
//! matplotlib 3.11 marks `semilogx`, `semilogy`, `semilogz` and `loglog`
//! unsupported on 3-D axes (pointing to `set_xscale`, `set_yscale`,
//! `set_zscale` and `plot` instead), so they have no counterpart here.

use std::rc::Rc;

use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::scaling::AxisScale;
use super::share::ShareKey;
use super::state::Axes3DState;
use crate::scale::{Scale, ScaleName, ScaleOptions, scale_factory};

impl Axes3DState {
    /// The default limits of the major locator, which tell whether a new
    /// scale needs autoscaling (matplotlib's
    /// `get_major_locator().nonsingular(-inf, inf)`).
    fn default_limits(&self, axis: Axis3) -> (f64, f64) {
        let span = self.limits.span(axis);
        self.axis(axis)
            .nonsingular(&span, f64::NEG_INFINITY, f64::INFINITY)
    }
}

impl Axes3D {
    /// Sets the scale of one axis, for every axes sharing it, with the
    /// scale's default tick helpers; the axis autoscales when the new
    /// locator's default limits differ (matplotlib's `set_xscale`). A
    /// non-linear z scale on an axes without z data first takes the
    /// initial data limits and margin of x, so that its limits are
    /// positive.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Axis3;
    /// use plt::scale::{LogScale, ScaleName};
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[1.0, 1000.0])?;
    /// ax.set_scale(Axis3::Z, Box::new(LogScale::new(10.0)?))?;
    /// assert_eq!(ax.scale(Axis3::Z), ScaleName::Log);
    /// let (low, high) = ax.zlim();
    /// assert!(low > 0.5 && low < 1.0 && high > 1000.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When autoscaling fails.
    pub fn set_scale(&self, axis: Axis3, scale: Box<dyn Scale>) -> Result<&Self, Axes3DError> {
        let scale = AxisScale::new(Rc::from(scale));
        let old = self.with_mut(|axes| {
            if axis == Axis3::Z
                && scale.name() != ScaleName::Linear
                && axes.limits.z_data_is_initial()
            {
                axes.limits.use_xy_initial_z();
            }
            axes.default_limits(axis)
        });
        self.update_group(ShareKey::Axis(axis), |axes| {
            axes.axis_mut(axis).set_scale(scale.clone());
        });
        self.with_mut(|axes| {
            // matplotlib compares the default limits exactly.
            if axes.default_limits(axis) == old {
                return Ok(());
            }
            let mut selected = [false; 3];
            selected[axis.index()] = true;
            axes.autoscale_axes(selected)
        })?;
        Ok(self)
    }

    /// Sets the scale of one axis by name with options, like
    /// `set_xscale("log", base=2)`.
    ///
    /// # Errors
    ///
    /// When no scale is registered under `name`, an option is invalid, or
    /// autoscaling fails.
    pub fn set_scale_with(
        &self,
        axis: Axis3,
        name: impl Into<ScaleName>,
        options: ScaleOptions,
    ) -> Result<&Self, Axes3DError> {
        self.set_scale(axis, scale_factory(name, options)?)
    }

    /// Sets the x scale by name (matplotlib's `set_xscale`).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_scale_with`].
    pub fn set_xscale(&self, name: impl Into<ScaleName>) -> Result<&Self, Axes3DError> {
        self.set_scale_with(Axis3::X, name, ScaleOptions::default())
    }

    /// Sets the y scale by name (matplotlib's `set_yscale`).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_scale_with`].
    pub fn set_yscale(&self, name: impl Into<ScaleName>) -> Result<&Self, Axes3DError> {
        self.set_scale_with(Axis3::Y, name, ScaleOptions::default())
    }

    /// Sets the z scale by name (matplotlib's `set_zscale`).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_scale_with`].
    pub fn set_zscale(&self, name: impl Into<ScaleName>) -> Result<&Self, Axes3DError> {
        self.set_scale_with(Axis3::Z, name, ScaleOptions::default())
    }

    /// The name of the scale of one axis (matplotlib's `get_xscale`).
    pub fn scale(&self, axis: Axis3) -> ScaleName {
        self.with(|axes| axes.axis(axis).scale.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn a_log_z_scale_of_an_empty_axes_matches_matplotlib() {
        let ax = Figure::new().subplot_3d();
        ax.set_zscale("log").unwrap();
        let (low, high) = ax.zlim();
        // matplotlib: (0.03640537388223389, 1.1918138759519783).
        assert!((low - 0.036_405_373_882_233_89).abs() < 1e-12, "{low}");
        assert!((high - 1.191_813_875_951_978_3).abs() < 1e-12, "{high}");
        assert_eq!(ax.margin(Axis3::Z), ax.margin(Axis3::X));
    }

    #[test]
    fn logged_data_autoscales_in_transformed_coordinates() {
        let ax = Figure::new().subplot_3d();
        ax.set_zscale("log").unwrap();
        ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[1.0, 1000.0]).unwrap();
        let (low, high) = ax.zlim();
        // matplotlib: (0.6042963902381328, 1654.817099943182).
        assert!((low - 0.604_296_390_238_132_8).abs() < 1e-9, "{low}");
        assert!((high - 1_654.817_099_943_182).abs() < 1e-6, "{high}");
    }

    #[test]
    fn scales_follow_sharing_and_reject_unknown_names() {
        let fig = Figure::new();
        let a = fig.subplot_3d();
        let b = fig.subplot_3d();
        b.share_x(&a).unwrap();
        a.set_xscale("symlog").unwrap();
        assert_eq!(b.scale(Axis3::X), ScaleName::Symlog);
        assert!(a.set_yscale("nonsense").is_err());
        a.set_xscale("linear").unwrap();
        assert_eq!(b.scale(Axis3::X), ScaleName::Linear);
    }
}
