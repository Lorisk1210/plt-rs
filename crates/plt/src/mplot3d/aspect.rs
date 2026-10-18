//! Equal data scales on the axes of a 3-D axes, by changing the box
//! aspect or the limits (matplotlib's `Axes3D.set_aspect` and
//! `_equal_aspect_axis_indices`), and zooming that keeps them equal
//! (`_zoom_data_limits`).

use std::str::FromStr;

use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::{Vec3, norm};
use super::state::Axes3DState;
use crate::axes::Adjustable;

/// Which axes of a 3-D axes have equal data scales (matplotlib's
/// `set_aspect` values of `Axes3D`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Aspect3D {
    /// The data fill the box.
    #[default]
    Auto,
    /// All three axes have the same scale.
    Equal,
    /// x and y have the same scale.
    EqualXY,
    /// x and z have the same scale.
    EqualXZ,
    /// y and z have the same scale.
    EqualYZ,
}

impl Aspect3D {
    /// The axes constrained to equal scales (matplotlib's
    /// `_equal_aspect_axis_indices`).
    pub fn equal_axes(self) -> &'static [Axis3] {
        match self {
            Self::Auto => &[],
            Self::Equal => &Axis3::ALL,
            Self::EqualXY => &[Axis3::X, Axis3::Y],
            Self::EqualXZ => &[Axis3::X, Axis3::Z],
            Self::EqualYZ => &[Axis3::Y, Axis3::Z],
        }
    }
}

impl FromStr for Aspect3D {
    type Err = Axes3DError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "auto" => Ok(Self::Auto),
            "equal" => Ok(Self::Equal),
            "equalxy" => Ok(Self::EqualXY),
            "equalxz" => Ok(Self::EqualXZ),
            "equalyz" => Ok(Self::EqualYZ),
            other => Err(Axes3DError::UnknownAspect(other.to_owned())),
        }
    }
}

/// The aspect settings of a 3-D axes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AspectSettings {
    pub(crate) aspect: Aspect3D,
    pub(crate) adjustable: Adjustable,
    /// The limits to go back to when an equal aspect set with
    /// [`Adjustable::Datalim`] returns to auto; an axis whose limits were
    /// set since is `None` (matplotlib's `_auto_aspect_restore_limits`).
    restore: Option<[Option<(f64, f64)>; 3]>,
}

impl AspectSettings {
    /// Forgets the limits to restore along `axis`, which were replaced.
    pub(crate) fn forget_restore_limits(&mut self, axis: Axis3) {
        if let Some(restore) = &mut self.restore {
            restore[axis.index()] = None;
        }
    }
}

impl Axes3DState {
    /// matplotlib's `Axes3D.set_aspect(aspect, adjustable)`.
    fn set_aspect(&mut self, aspect: Aspect3D, adjustable: Adjustable) -> Result<(), Axes3DError> {
        self.aspect.adjustable = adjustable;
        if self.aspect.aspect == Aspect3D::Auto && aspect != Aspect3D::Auto {
            self.aspect.restore = Some(Axis3::ALL.map(|axis| Some(self.limits.view(axis))));
        }
        self.aspect.aspect = aspect;
        let equal = aspect.equal_axes();
        if equal.is_empty() {
            return match adjustable {
                Adjustable::Box => self.camera.set_box_aspect(None, 1.0),
                Adjustable::Datalim => self.restore_auto_limits(),
            };
        }
        let ptp = Axis3::ALL.map(|axis| {
            let (lo, hi) = self.limits.bound(axis);
            hi - lo
        });
        // matplotlib indexes the box aspect as stored (rolled to the
        // vertical axis) with data axis indices.
        let stored = self.camera.stored_box_aspect();
        match adjustable {
            Adjustable::Datalim => {
                let scale = equal
                    .iter()
                    .map(|axis| ptp[axis.index()] / stored[axis.index()])
                    .fold(f64::NEG_INFINITY, f64::max);
                for &axis in equal {
                    let (lo, hi) = self.limits.view(axis);
                    let (mean, delta) = (f64::midpoint(lo, hi), scale * stored[axis.index()]);
                    self.set_lim3d(
                        axis,
                        mean - delta / 2.0,
                        mean + delta / 2.0,
                        Some(true),
                        None,
                    )?;
                }
                Ok(())
            }
            Adjustable::Box => {
                let mut aspect = stored;
                for axis in equal {
                    aspect[axis.index()] = ptp[axis.index()];
                }
                if let Some(remaining) = Axis3::ALL.iter().find(|axis| !equal.contains(axis)) {
                    let factor = diagonal(equal, aspect) / diagonal(equal, stored);
                    aspect[remaining.index()] *= factor;
                }
                self.camera.set_box_aspect(Some(aspect), 1.0)
            }
        }
    }

    /// Restores the limits an equal aspect replaced, keeping autoscaling
    /// on (matplotlib's `_auto_aspect_restore_limits`).
    fn restore_auto_limits(&mut self) -> Result<(), Axes3DError> {
        let Some(restore) = self.aspect.restore.take() else {
            return Ok(());
        };
        for (axis, limits) in Axis3::ALL.into_iter().zip(restore) {
            if let Some((lo, hi)) = limits {
                self.set_lim3d(axis, lo, hi, Some(true), None)?;
            }
        }
        Ok(())
    }
}

/// The length of the components of `v` along `axes`.
fn diagonal(axes: &[Axis3], v: Vec3) -> f64 {
    axes.iter()
        .map(|axis| v[axis.index()] * v[axis.index()])
        .sum::<f64>()
        .sqrt()
}

impl Axes3D {
    /// Gives the axes equal data scales as `aspect` asks, by changing the
    /// box aspect ([`Adjustable::Box`], which also resets the zoom) or the
    /// limits of the constrained axes ([`Adjustable::Datalim`], which
    /// leaves them autoscaled); [`Aspect3D::Auto`] restores the default
    /// box or the limits replaced (matplotlib's `set_aspect`).
    ///
    /// ```
    /// use plt::axes::Adjustable;
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Aspect3D;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_xlim(0.0, 4.0)?.set_ylim(0.0, 1.0)?.set_zlim(0.0, 3.0)?;
    /// ax.set_aspect(Aspect3D::EqualXY, Adjustable::Datalim)?;
    /// assert_eq!(ax.ylim(), (-1.5, 2.5));
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a limit or the box aspect becomes invalid.
    pub fn set_aspect(
        &self,
        aspect: Aspect3D,
        adjustable: Adjustable,
    ) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| axes.set_aspect(aspect, adjustable))?;
        Ok(self)
    }

    /// Which axes have equal data scales.
    pub fn aspect(&self) -> Aspect3D {
        self.with(|axes| axes.aspect.aspect)
    }

    /// What [`Axes3D::set_aspect`] last changed to equalise the scales.
    pub fn adjustable(&self) -> Adjustable {
        self.with(|axes| axes.aspect.adjustable)
    }

    /// Scales the limits by `scale_u`, `scale_v` and `scale_w` along the
    /// screen's horizontal, vertical and depth directions, by more than 1
    /// to zoom out: unequal factors are converted to the data axes for the
    /// current view, the axes of an equal aspect taking the factor closest
    /// to 1 (matplotlib's `_zoom_data_limits`).
    ///
    /// # Errors
    ///
    /// When a limit becomes infinite or NaN.
    pub fn zoom_data_limits(
        &self,
        scale_u: f64,
        scale_v: f64,
        scale_w: f64,
    ) -> Result<&Self, Axes3DError> {
        let mut scale = [scale_u, scale_v, scale_w];
        let close = |a: f64| (a - scale_u).abs() <= 1e-8 + 1e-5 * scale_u.abs();
        if !scale.iter().all(|&s| close(s)) {
            let (frame, aspect) = self.with(|axes| (axes.camera.view_frame(), axes.aspect.aspect));
            scale = [0, 1, 2].map(|i| norm([0, 1, 2].map(|j| frame[j][i] * scale[j])));
            let equal = aspect.equal_axes();
            if let Some(&closest) = equal.iter().min_by(|a, b| {
                (scale[a.index()] - 1.0)
                    .abs()
                    .total_cmp(&(scale[b.index()] - 1.0).abs())
            }) {
                let value = scale[closest.index()];
                for axis in equal {
                    scale[axis.index()] = value;
                }
            }
        }
        self.scale_limits(scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn boxed() -> Axes3D {
        let ax = Figure::new().subplot_3d();
        ax.set_xlim(0.0, 4.0).unwrap();
        ax.set_ylim(0.0, 1.0).unwrap();
        ax.set_zlim(0.0, 3.0).unwrap();
        ax
    }

    #[test]
    fn names_parse_like_matplotlib() {
        assert_eq!("equalyz".parse(), Ok(Aspect3D::EqualYZ));
        assert!("square".parse::<Aspect3D>().is_err());
        assert_eq!(Aspect3D::Auto.equal_axes(), &[]);
    }

    #[test]
    fn datalim_widens_the_constrained_axes_and_auto_restores_them() {
        let ax = boxed();
        ax.set_aspect(Aspect3D::Equal, Adjustable::Datalim).unwrap();
        // The x span 4 over the box width 4 (of 4:4:3) sets the scale.
        assert_eq!(ax.ylim(), (-1.5, 2.5));
        assert_eq!(ax.zlim(), (0.0, 3.0));
        assert!(ax.autoscale(Axis3::Y));
        ax.set_aspect(Aspect3D::Auto, Adjustable::Datalim).unwrap();
        assert_eq!(ax.ylim(), (0.0, 1.0));
    }

    #[test]
    fn box_changes_the_box_aspect() {
        let ax = boxed();
        ax.set_aspect(Aspect3D::Equal, Adjustable::Box).unwrap();
        let [x, y, z] = ax.box_aspect();
        assert!((x / y - 4.0).abs() < 1e-12 && (z / y - 3.0).abs() < 1e-12);
        ax.set_aspect(Aspect3D::Auto, Adjustable::Box).unwrap();
        let [x, y, z] = ax.box_aspect();
        assert!((x / y - 1.0).abs() < 1e-12 && (z / x - 0.75).abs() < 1e-12);
    }

    #[test]
    fn equal_zooms_keep_the_constrained_scales_together() {
        let ax = boxed();
        ax.set_aspect(Aspect3D::EqualXY, Adjustable::Box).unwrap();
        ax.zoom_data_limits(0.5, 1.0, 1.0).unwrap();
        let (x0, x1) = ax.xlim();
        let (y0, y1) = ax.ylim();
        assert!(((x1 - x0) / 4.0 - (y1 - y0)).abs() < 1e-9);
    }
}
