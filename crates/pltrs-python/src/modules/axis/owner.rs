//! The axes an axis of `_native.axis` belongs to: a 2-D axes, or a 3-D
//! axes, whose x, y and z axes are matplotlib's `XAxis` too (an
//! `axis3d.Axis`). The functions serving both take an [`AxisOwner`] and
//! match on the [`AnyAxis`] it gives; the others take an `Axes`, which
//! refuses the projection plane a 3-D axes otherwise converts to (see
//! [`super::axis`]).

use pltrs::axes::{Axes, Axis};
use pltrs::mplot3d::{Axes3D, Axis3D};

use crate::binding::prelude::*;
use crate::modules::mplot3d::axis3;

/// A 2-D axes or a 3-D axes.
#[derive(Clone)]
pub(crate) enum AxisOwner {
    /// A 2-D axes.
    Flat(Axes),
    /// A 3-D axes.
    Deep(Axes3D),
}

impl FromPy<'_> for AxisOwner {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract::<Axes3D>() {
            Ok(axes) => Ok(Self::Deep(axes)),
            Err(_) => object.extract::<Axes>().map(Self::Flat),
        }
    }
}

/// An axis of a 2-D or of a 3-D axes.
pub(crate) enum AnyAxis {
    /// The x or y axis of a 2-D axes.
    Flat(Axis),
    /// The x, y or z axis of a 3-D axes.
    Deep(Axis3D),
}

impl AxisOwner {
    /// The axis called `name`: `"x"` or `"y"`, or `"z"` for a 3-D axes.
    pub(crate) fn axis(&self, name: &str) -> PyResult<AnyAxis> {
        match self {
            Self::Flat(axes) => super::axis(axes, name).map(AnyAxis::Flat),
            Self::Deep(axes) => Ok(AnyAxis::Deep(axes.axis(axis3(name)?))),
        }
    }
}
