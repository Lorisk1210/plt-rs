//! The limit settings of a 3-D axes: view limits, bounds, inversion,
//! autoscaling and margins (matplotlib's `set_xlim`, `set_xbound`,
//! `invert_xaxis`, `set_inverted`, `set_xmargin` and `margins` on
//! `Axes3D`).

use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use crate::units::AxisValue;

impl Axes3D {
    /// The view limits of one axis; decreasing when inverted.
    pub fn limits(&self, axis: Axis3) -> (f64, f64) {
        self.with(|axes| axes.limits.view(axis))
    }

    /// The interval the data of one axis spans (matplotlib's
    /// `get_data_interval` of the axes of a 3-D axes: `xy_dataLim.intervalx`,
    /// `xy_dataLim.intervaly` or `zz_dataLim.intervalx`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_data_interval(Axis3::Z, 2.0, 5.0);
    /// assert_eq!(ax.data_interval(Axis3::Z), (2.0, 5.0));
    /// ```
    pub fn data_interval(&self, axis: Axis3) -> (f64, f64) {
        self.with(|axes| axes.limits.data_interval(axis))
    }

    /// Replaces the interval the data of one axis spans (matplotlib's
    /// `set_data_interval(vmin, vmax, ignore=True)`); the view limits stay
    /// as they are until the axes autoscales.
    pub fn set_data_interval(&self, axis: Axis3, low: f64, high: f64) -> &Self {
        self.with_mut(|axes| axes.limits.set_data_interval(axis, (low, high)));
        self
    }

    /// The x view limits.
    pub fn xlim(&self) -> (f64, f64) {
        self.limits(Axis3::X)
    }

    /// The y view limits.
    pub fn ylim(&self) -> (f64, f64) {
        self.limits(Axis3::Y)
    }

    /// The z view limits.
    pub fn zlim(&self) -> (f64, f64) {
        self.limits(Axis3::Z)
    }

    /// Sets the view limits of one axis, in axis numbers, and turns its
    /// autoscaling off; `low > high` inverts the axis, and the axes
    /// sharing it follow (matplotlib's `set_xlim`). With
    /// `axes3d.automargin` set at the time, the limits are widened by the
    /// [view margin](Axes3D::view_margin) on either side.
    ///
    /// # Errors
    ///
    /// When a limit is infinite or NaN.
    pub fn set_limits(&self, axis: Axis3, low: f64, high: f64) -> Result<&Self, Axes3DError> {
        self.set_limits_with_margin(axis, low, high, None)
    }

    /// Sets the view limits of one axis as [`Axes3D::set_limits`] does,
    /// widened by `view_margin` times their span on either side, in the
    /// coordinates of the axis scale (matplotlib's `set_xlim(...,
    /// view_margin=...)`, `set_ylim`, `set_zlim` and their `3d` names);
    /// `None` widens them as `axes3d.automargin` says, as
    /// [`Axes3D::set_limits`] does, and a negative margin narrows them.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_limits_with_margin(Axis3::Z, 0.0, 10.0, Some(0.1))?;
    /// assert_eq!(ax.zlim(), (-1.0, 11.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a limit or the margin is infinite or NaN.
    pub fn set_limits_with_margin(
        &self,
        axis: Axis3,
        low: f64,
        high: f64,
        view_margin: Option<f64>,
    ) -> Result<&Self, Axes3DError> {
        check_view_margin(view_margin)?;
        self.with_mut(|axes| axes.set_lim3d(axis, low, high, Some(false), view_margin))?;
        Ok(self)
    }

    /// Sets the x view limits: numbers, dates, category names or user
    /// values, converted on the x axis (see [`Axes3D::set_limits`]).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_xlim(
        &self,
        left: impl Into<AxisValue>,
        right: impl Into<AxisValue>,
    ) -> Result<&Self, Axes3DError> {
        self.set_converted_limits(Axis3::X, left.into(), right.into())
    }

    /// Sets the y view limits, converted on the y axis.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_ylim(
        &self,
        bottom: impl Into<AxisValue>,
        top: impl Into<AxisValue>,
    ) -> Result<&Self, Axes3DError> {
        self.set_converted_limits(Axis3::Y, bottom.into(), top.into())
    }

    /// Sets the z view limits, converted on the z axis.
    ///
    /// ```
    /// use pltrs::dates::DateTime;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let day = |d| DateTime::new(2020, 1, d, 0, 0, 0, 0);
    /// ax.set_zlim(day(1)?, day(3)?)?;
    /// assert_eq!(ax.zlim(), (18_262.0, 18_264.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_zlim(
        &self,
        bottom: impl Into<AxisValue>,
        top: impl Into<AxisValue>,
    ) -> Result<&Self, Axes3DError> {
        self.set_converted_limits(Axis3::Z, bottom.into(), top.into())
    }

    /// Sets the view limits of one axis from plotting values converted
    /// on it (matplotlib's `_validate_converted_limits`).
    fn set_converted_limits(
        &self,
        axis: Axis3,
        low: AxisValue,
        high: AxisValue,
    ) -> Result<&Self, Axes3DError> {
        let along = self.axis(axis);
        let (low, high) = (along.convert_value(low), along.convert_value(high));
        self.set_limits(axis, low, high)
    }

    /// The view limits of one axis in increasing order (matplotlib's
    /// `get_xbound`).
    pub fn bound(&self, axis: Axis3) -> (f64, f64) {
        self.with(|axes| axes.limits.bound(axis))
    }

    /// Sets the lower and upper bounds of one axis whatever their order,
    /// keeping its orientation and autoscaling; `None` keeps a bound
    /// (matplotlib's `set_xbound`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlim(5.0, 0.0)?;
    /// ax.set_bound(Axis3::Z, Some(-1.0), Some(2.0))?;
    /// assert_eq!(ax.zlim(), (2.0, -1.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a bound is infinite or NaN.
    pub fn set_bound(
        &self,
        axis: Axis3,
        lower: Option<f64>,
        upper: Option<f64>,
    ) -> Result<&Self, Axes3DError> {
        self.set_bound_with_margin(axis, lower, upper, None)
    }

    /// Sets the bounds of one axis as [`Axes3D::set_bound`] does, widened
    /// by `view_margin` as [`Axes3D::set_limits_with_margin`] widens limits
    /// (matplotlib's `set_xbound(..., view_margin=...)`, `set_ybound` and
    /// `set_zbound`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_bound_with_margin(Axis3::X, Some(4.0), Some(-4.0), Some(0.25))?;
    /// assert_eq!(ax.xlim(), (-6.0, 6.0));
    /// assert!(ax.autoscale_on(Axis3::X), "bounds leave autoscaling on");
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a bound or the margin is infinite or NaN.
    pub fn set_bound_with_margin(
        &self,
        axis: Axis3,
        lower: Option<f64>,
        upper: Option<f64>,
        view_margin: Option<f64>,
    ) -> Result<&Self, Axes3DError> {
        check_view_margin(view_margin)?;
        self.with_mut(|axes| {
            let (old_lower, old_upper) = axes.limits.bound(axis);
            axes.set_bound(
                axis,
                lower.unwrap_or(old_lower),
                upper.unwrap_or(old_upper),
                view_margin,
            )
        })?;
        Ok(self)
    }

    /// Sets the x bounds.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_bound`].
    pub fn set_xbound(&self, lower: Option<f64>, upper: Option<f64>) -> Result<&Self, Axes3DError> {
        self.set_bound(Axis3::X, lower, upper)
    }

    /// Sets the y bounds.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_bound`].
    pub fn set_ybound(&self, lower: Option<f64>, upper: Option<f64>) -> Result<&Self, Axes3DError> {
        self.set_bound(Axis3::Y, lower, upper)
    }

    /// Sets the z bounds.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_bound`].
    pub fn set_zbound(&self, lower: Option<f64>, upper: Option<f64>) -> Result<&Self, Axes3DError> {
        self.set_bound(Axis3::Z, lower, upper)
    }

    /// Flips the direction of one axis, keeping its autoscaling
    /// (matplotlib's `invert_xaxis`, `invert_yaxis` and `invert_zaxis`).
    ///
    /// # Errors
    ///
    /// When `axes3d.automargin` widens the z limits beyond the finite
    /// range.
    pub fn invert(&self, axis: Axis3) -> Result<&Self, Axes3DError> {
        // matplotlib inverts x and y through `Axis.set_inverted`, z through
        // `set_zlim` (which honours `axes3d.automargin`).
        self.with_mut(|axes| match axis {
            Axis3::X | Axis3::Y => {
                axes.set_inverted(axis, !axes.limits.inverted(axis));
                Ok(())
            }
            Axis3::Z => {
                let (low, high) = axes.limits.view(axis);
                axes.set_lim3d(axis, high, low, None, None)
            }
        })?;
        Ok(self)
    }

    /// Whether one axis runs from high to low values (matplotlib's
    /// `get_xinverted`).
    pub fn inverted(&self, axis: Axis3) -> bool {
        self.with(|axes| axes.limits.inverted(axis))
    }

    /// Makes one axis run from high to low values or back, keeping its
    /// range and autoscaling (matplotlib's `set_xinverted`).
    pub fn set_inverted(&self, axis: Axis3, inverted: bool) -> &Self {
        self.with_mut(|axes| axes.set_inverted(axis, inverted));
        self
    }

    /// Whether one axis follows the data (matplotlib's
    /// `get_autoscalex_on`), as [`Axes::autoscale_on`] tells on 2-D axes.
    ///
    /// [`Axes::autoscale_on`]: crate::axes::Axes::autoscale_on
    pub fn autoscale_on(&self, axis: Axis3) -> bool {
        self.with(|axes| axes.limits.autoscale[axis.index()])
    }

    /// Turns autoscaling of one axis on or off without autoscaling now
    /// (matplotlib's `set_autoscalex_on`), as
    /// [`Axes::set_axis_autoscale_on`] does on 2-D axes.
    ///
    /// [`Axes::set_axis_autoscale_on`]: crate::axes::Axes::set_axis_autoscale_on
    pub fn set_axis_autoscale_on(&self, axis: Axis3, on: bool) -> &Self {
        self.with_mut(|axes| axes.limits.autoscale[axis.index()] = on);
        self
    }

    /// The padding added on both sides of autoscaled limits, and of
    /// explicit limits under `axes3d.automargin`, as a fraction of their
    /// span: 1/48 unless changed (matplotlib's `_view_margin`).
    pub fn view_margin(&self) -> f64 {
        self.with(|axes| axes.limits.view_margin)
    }

    /// Sets the [view margin](Axes3D::view_margin); it applies from the
    /// next autoscaling or setting of limits on.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_view_margin(0.0)?;
    /// ax.set_margins([Some(0.0); 3], None)?;
    /// ax.plot(&[0.0, 1.0], &[0.0, 2.0], &[0.0, 3.0])?;
    /// assert_eq!(ax.view_margin(), 0.0);
    /// assert_eq!(ax.zlim(), (0.0, 3.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the margin is infinite or NaN.
    pub fn set_view_margin(&self, margin: f64) -> Result<&Self, Axes3DError> {
        check_view_margin(Some(margin))?;
        self.with_mut(|axes| axes.limits.view_margin = margin);
        Ok(self)
    }

    /// Sets the autoscaling margin of one axis as a fraction of the data
    /// span and autoscales that axis (matplotlib's `set_xmargin`).
    ///
    /// # Errors
    ///
    /// When the margin is not above -0.5 or a locator fails.
    pub fn set_margin(&self, axis: Axis3, margin: f64) -> Result<&Self, Axes3DError> {
        let mut margins = [None; 3];
        margins[axis.index()] = Some(margin);
        self.set_margins(margins, None)
    }

    /// matplotlib's `autoscale(enable, axis, tight)`: `enable` turns
    /// autoscaling of the axes `selected` (indexed x, y, z) on or off and
    /// those that autoscale fit their data; `None` leaves the switches
    /// alone and fits every axis that autoscales. `tight` replaces the
    /// tight setting, which skips rounding the view out to ticks.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.autoscale(Some(false), [false, false, true], None)?;
    /// assert!(!ax.autoscale_on(Axis3::Z) && ax.autoscale_on(Axis3::X));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator fails while autoscaling.
    pub fn autoscale(
        &self,
        enable: Option<bool>,
        selected: [bool; 3],
        tight: Option<bool>,
    ) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| {
            let mut fit = [true; 3];
            if let Some(on) = enable {
                for (i, chosen) in selected.into_iter().enumerate() {
                    if chosen {
                        axes.limits.autoscale[i] = on;
                    }
                    fit[i] = chosen && axes.limits.autoscale[i];
                }
            }
            if let Some(tight) = tight {
                axes.limits.tight = tight;
            }
            axes.autoscale_axes(fit)
        })?;
        Ok(self)
    }

    /// Fits the view limits of the axes `selected` (indexed x, y, z) that
    /// autoscale to their data now (matplotlib's `autoscale_view(tight,
    /// scalex, scaley, scalez)`); `tight` replaces the tight setting, which
    /// skips rounding the view out to ticks.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 10.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_xlim(2.0, 3.0)?.set_axis_autoscale_on(Axis3::X, true);
    /// ax.autoscale_view(Some(true), [false, true, true])?;
    /// assert_eq!(ax.xlim(), (2.0, 3.0), "x is left alone");
    /// ax.autoscale_view(None, [true, true, true])?;
    /// assert_ne!(ax.xlim(), (2.0, 3.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator fails while autoscaling.
    pub fn autoscale_view(
        &self,
        tight: Option<bool>,
        selected: [bool; 3],
    ) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| {
            if let Some(tight) = tight {
                axes.limits.tight = tight;
            }
            axes.autoscale_axes(selected)
        })?;
        Ok(self)
    }

    /// The autoscaling margin of one axis.
    pub fn margin(&self, axis: Axis3) -> f64 {
        self.with(|axes| axes.limits.margins[axis.index()])
    }

    /// Sets the autoscaling margins of the axes given and autoscales them;
    /// `tight` (matplotlib's default is `Some(true)`) also decides from
    /// now on whether autoscaling skips the locators' rounding
    /// (matplotlib's `margins(x=..., y=..., z=..., tight=...)`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 10.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.set_margins([Some(0.1), None, None], Some(true))?;
    /// let (lo, hi) = ax.xlim();
    /// assert!((lo + 1.25).abs() < 1e-12 && (hi - 11.25).abs() < 1e-12);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a margin is not above -0.5 or a locator fails.
    pub fn set_margins(
        &self,
        margins: [Option<f64>; 3],
        tight: Option<bool>,
    ) -> Result<&Self, Axes3DError> {
        if let Some(bad) = margins.iter().flatten().find(|m| m.is_nan() || **m <= -0.5) {
            return Err(Axes3DError::Margin(*bad));
        }
        self.with_mut(|axes| {
            for (slot, margin) in axes.limits.margins.iter_mut().zip(margins) {
                if let Some(margin) = margin {
                    *slot = margin;
                }
            }
            if let Some(tight) = tight {
                axes.limits.tight = tight;
            }
            axes.autoscale_axes(margins.map(|m| m.is_some()))
        })?;
        Ok(self)
    }
}

/// Checks that a view margin, if given, is finite.
fn check_view_margin(view_margin: Option<f64>) -> Result<(), Axes3DError> {
    match view_margin {
        Some(margin) if !margin.is_finite() => Err(Axes3DError::ViewMargin(margin)),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn bounds_keep_orientation_and_autoscaling() {
        let ax = Figure::new().subplot_3d();
        ax.set_bound(Axis3::X, Some(4.0), Some(-4.0)).unwrap();
        assert_eq!(ax.xlim(), (-4.0, 4.0));
        assert!(ax.autoscale_on(Axis3::X), "bounds leave autoscaling on");
        ax.set_inverted(Axis3::X, true);
        ax.set_xbound(None, Some(6.0)).unwrap();
        assert_eq!(ax.xlim(), (6.0, -4.0));
        assert_eq!(ax.bound(Axis3::X), (-4.0, 6.0));
        assert!(ax.set_zbound(Some(f64::NAN), None).is_err());
    }

    #[test]
    fn inverting_twice_restores_the_axis() {
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(0.0, 3.0).unwrap();
        ax.invert(Axis3::Z).unwrap().invert(Axis3::Y).unwrap();
        assert!(ax.inverted(Axis3::Z) && ax.inverted(Axis3::Y));
        assert_eq!(ax.zlim(), (3.0, 0.0));
        ax.invert(Axis3::Z).unwrap().set_inverted(Axis3::Y, false);
        assert_eq!(ax.zlim(), (0.0, 3.0));
        assert!(!ax.inverted(Axis3::Y));
    }

    #[test]
    fn view_margins_widen_limits_in_the_scale() {
        let ax = Figure::new().subplot_3d();
        ax.set_limits_with_margin(Axis3::Y, 2.0, 0.0, Some(0.5))
            .unwrap();
        assert_eq!(ax.ylim(), (3.0, -1.0), "inverted limits stay inverted");
        ax.set_limits_with_margin(Axis3::Y, 0.0, 4.0, Some(-0.25))
            .unwrap();
        assert_eq!(ax.ylim(), (1.0, 3.0), "negative margins narrow");
        ax.set_zscale("log").unwrap();
        ax.set_limits_with_margin(Axis3::Z, 1.0, 100.0, Some(0.5))
            .unwrap();
        let (low, high) = ax.zlim();
        assert!((low - 0.1).abs() < 1e-12 && (high - 1000.0).abs() < 1e-9);
        assert!(matches!(
            ax.set_limits_with_margin(Axis3::X, 0.0, 1.0, Some(f64::NAN)),
            Err(Axes3DError::ViewMargin(margin)) if margin.is_nan()
        ));
        assert!(matches!(
            ax.set_bound_with_margin(Axis3::X, None, None, Some(f64::INFINITY)),
            Err(Axes3DError::ViewMargin(_))
        ));
        crate::rc::context(|| {
            crate::rc::set("axes3d.automargin", "True").unwrap();
            let ax = Figure::new().subplot_3d();
            ax.set_limits(Axis3::X, 0.0, 48.0).unwrap();
            assert_eq!(ax.xlim(), (-1.0, 49.0), "automargin widens by 1/48");
            ax.set_limits_with_margin(Axis3::X, 0.0, 48.0, Some(0.0))
                .unwrap();
            assert_eq!(ax.xlim(), (0.0, 48.0), "a zero margin overrides it");
        });
    }

    #[test]
    fn margins_are_validated_and_only_rescale_their_axes() {
        let ax = Figure::new().subplot_3d();
        assert_eq!(
            ax.set_margin(Axis3::Z, -0.5).unwrap_err(),
            Axes3DError::Margin(-0.5)
        );
        ax.set_margin(Axis3::Z, 0.2).unwrap();
        assert_eq!(ax.margin(Axis3::Z), 0.2);
        assert_eq!(ax.margin(Axis3::X), 0.05);
    }
}
