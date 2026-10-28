//! The axis a unit converter converts values for: an axis of 2-D axes or
//! one of 3-D axes (matplotlib's `Axis`, which every axis class of 2-D
//! and 3-D axes derives from).

use super::{Unit, UnitConverter};
use crate::axes::Axis;
use crate::mplot3d::Axis3D;

/// The axis a [`ConversionInterface`](super::ConversionInterface)
/// converts values for, picks a unit on or describes (matplotlib passes
/// the axis object itself: an `XAxis` or `YAxis`, or an axis of 3-D
/// axes). Match on it to reach the axis, its axes and figure.
///
/// ```
/// use plt::prelude::*;
/// use plt::units::UnitAxis;
/// let ax = Figure::new().subplot_3d();
/// let axis = UnitAxis::from(ax.zaxis());
/// assert!(matches!(&axis, UnitAxis::Axes3D(z) if z.dir() == plt::mplot3d::Axis3::Z));
/// assert_eq!(axis.units(), None);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitAxis {
    /// The x or y axis of 2-D axes.
    Axes(Axis),
    /// The x, y or z axis of 3-D axes.
    Axes3D(Axis3D),
}

impl UnitAxis {
    /// The unit of the axis, once one was set or picked for user values
    /// (matplotlib's `axis.units`, which converters often consult).
    pub fn units(&self) -> Option<Unit> {
        match self {
            Self::Axes(axis) => axis.units(),
            Self::Axes3D(axis) => axis.units(),
        }
    }

    /// The unit converter of the axis, if it has one (matplotlib's
    /// `get_converter`).
    pub fn converter(&self) -> Option<UnitConverter> {
        match self {
            Self::Axes(axis) => axis.converter(),
            Self::Axes3D(axis) => axis.converter(),
        }
    }
}

impl From<Axis> for UnitAxis {
    fn from(axis: Axis) -> Self {
        Self::Axes(axis)
    }
}

impl From<Axis3D> for UnitAxis {
    fn from(axis: Axis3D) -> Self {
        Self::Axes3D(axis)
    }
}
