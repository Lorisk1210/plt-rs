//! The limit settings of a 3-D axes: view limits, bounds, inversion,
//! autoscaling and margins (matplotlib's `set_xlim`, `set_xbound`,
//! `invert_xaxis`, `set_inverted`, `set_xmargin` and `margins` on
//! `Axes3D`).

use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;

impl Axes3D {
    /// The view limits of one axis; decreasing when inverted.
    pub fn limits(&self, axis: Axis3) -> (f64, f64) {
        self.with(|axes| axes.limits.view(axis))
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

    /// Sets the view limits of one axis and turns its autoscaling off;
    /// `low > high` inverts the axis, and the axes sharing it follow
    /// (matplotlib's `set_xlim`).
    ///
    /// # Errors
    ///
    /// When a limit is infinite or NaN.
    pub fn set_limits(&self, axis: Axis3, low: f64, high: f64) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| axes.set_lim3d(axis, low, high, Some(false), None))?;
        Ok(self)
    }

    /// Sets the x view limits.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_xlim(&self, left: f64, right: f64) -> Result<&Self, Axes3DError> {
        self.set_limits(Axis3::X, left, right)
    }

    /// Sets the y view limits.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_ylim(&self, bottom: f64, top: f64) -> Result<&Self, Axes3DError> {
        self.set_limits(Axis3::Y, bottom, top)
    }

    /// Sets the z view limits.
    ///
    /// # Errors
    ///
    /// As [`Axes3D::set_limits`].
    pub fn set_zlim(&self, bottom: f64, top: f64) -> Result<&Self, Axes3DError> {
        self.set_limits(Axis3::Z, bottom, top)
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
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlim(5.0, 0.0)?;
    /// ax.set_bound(Axis3::Z, Some(-1.0), Some(2.0))?;
    /// assert_eq!(ax.zlim(), (2.0, -1.0));
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
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
        self.with_mut(|axes| {
            let (old_lower, old_upper) = axes.limits.bound(axis);
            axes.set_bound(
                axis,
                lower.unwrap_or(old_lower),
                upper.unwrap_or(old_upper),
                None,
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
    /// `get_autoscalex_on`).
    pub fn autoscale(&self, axis: Axis3) -> bool {
        self.with(|axes| axes.limits.autoscale[axis.index()])
    }

    /// Turns autoscaling of one axis on or off.
    pub fn set_autoscale(&self, axis: Axis3, on: bool) -> &Self {
        self.with_mut(|axes| axes.limits.autoscale[axis.index()] = on);
        self
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
        self.margins(margins, None)
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
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 10.0], &[0.0, 1.0], &[0.0, 1.0])?;
    /// ax.margins([Some(0.1), None, None], Some(true))?;
    /// let (lo, hi) = ax.xlim();
    /// assert!((lo + 1.25).abs() < 1e-12 && (hi - 11.25).abs() < 1e-12);
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a margin is not above -0.5 or a locator fails.
    pub fn margins(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn bounds_keep_orientation_and_autoscaling() {
        let ax = Figure::new().subplot_3d();
        ax.set_bound(Axis3::X, Some(4.0), Some(-4.0)).unwrap();
        assert_eq!(ax.xlim(), (-4.0, 4.0));
        assert!(ax.autoscale(Axis3::X), "bounds leave autoscaling on");
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
