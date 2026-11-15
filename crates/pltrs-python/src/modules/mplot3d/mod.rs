//! `_native.mplot3d`: 3-D axes (matplotlib's `mpl_toolkits.mplot3d`),
//! over pltrs (Rust)'s [`pltrs::mplot3d`].
//!
//! A Python `Axes3D` holds the native object of its [`Axes3D`]; where a
//! native function takes an `Axes`, that object stands for the axes'
//! projection plane, the 2-D axes drawing in projected coordinates (see
//! `modules/axes.rs`), so the 2-D methods `Axes3D` inherits from `Axes`
//! (titles, 2-D texts, legends, the position) work on it.
//!
//! * [`create`]: making 3-D axes in a figure and listing them with the
//!   figure's other axes;
//! * [`limits`]: limits, bounds, autoscaling, margins, scales, inversion;
//! * [`view`]: the camera, the box, the decorations and the readout;
//! * [`plotting`]: the 3-D plotting methods of lines, points, surfaces and
//!   texts, [`solids`] those of boxes, arrows and triangulations,
//!   [`charts`] those of error bars, stems and areas, and [`voxels`] that
//!   of voxels;
//! * [`contours`]: contours of grids and triangulations in 3-D;
//! * [`art3d`]: the 3-D geometry of artists already made, and the
//!   projection math of matplotlib's `proj3d`;
//! * [`drawn`]: what the axes keeps of its last draw (`M`, `invM`);
//! * [`failures`]: the problems drawing its artists meets, recorded in
//!   advance;
//! * [`axis3d`]: the formatters, ticks and edges of the three axes.

mod art3d;
mod axis3d;
mod charts;
mod contours;
mod create;
mod drawn;
mod edits;
mod failures;
mod limits;
mod plotting;
mod solids;
mod view;
mod voxels;

use pltrs::mplot3d::{Axes3D, Axes3DError, Axis3};

use crate::binding::prelude::*;

artist_handle!(Axes3D);

impl From<Axes3DError> for PyErr {
    /// matplotlib's `ValueError` for invalid values, numpy's `IndexError`
    /// for a juggled index out of range, `TypeError` for error
    /// sizes with units and for collections of a kind that is not lifted
    /// to 3-D (the package adds those as they are, as matplotlib does),
    /// and the exception recorded for an artist the axes fails to draw.
    fn from(err: Axes3DError) -> Self {
        let kind = match &err {
            Axes3DError::UnaddableErrors(_) | Axes3DError::UnsupportedCollection => {
                ExceptionKind::TypeError
            }
            Axes3DError::JuggledIndex(_) => ExceptionKind::IndexError,
            Axes3DError::Draw(failure) => {
                return super::figure::draw_error(failure.kind, &failure.message)
                    .unwrap_or_else(|| PyErr::runtime_error(failure.message.clone()));
            }
            _ => ExceptionKind::ValueError,
        };
        PyErr::new(kind, err.to_string())
    }
}

/// The axis called `name` (`"x"`, `"y"` or `"z"`).
pub(in crate::modules) fn axis3(name: &str) -> PyResult<Axis3> {
    match name {
        "x" => Ok(Axis3::X),
        "y" => Ok(Axis3::Y),
        "z" => Ok(Axis3::Z),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for zdir; supported values are 'x', 'y', 'z'"
        ))),
    }
}

/// The axis `zdir` names for the plane 2-D data is lifted into: `"x"`
/// and `"y"`, any other name `"z"`, as matplotlib's `juggle_axes` and
/// `rotate_axes` treat it (they read the first character of a name not
/// `"x"` or `"y"`, so an empty one is an `IndexError`).
pub(in crate::modules) fn zdir_axis(name: &str) -> PyResult<Axis3> {
    match name {
        "x" => Ok(Axis3::X),
        "y" => Ok(Axis3::Y),
        "" => Err(PyErr::new(
            ExceptionKind::IndexError,
            "string index out of range",
        )),
        _ => Ok(Axis3::Z),
    }
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    create::register(module)?;
    limits::register(module)?;
    view::register(module)?;
    plotting::register(module)?;
    solids::register(module)?;
    art3d::register(module)?;
    voxels::register(module)?;
    contours::register(module)?;
    charts::register(module)?;
    edits::register(module)?;
    drawn::register(module)?;
    failures::register(module)?;
    axis3d::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_are_named_x_y_z() {
        assert_eq!(axis3("y").unwrap(), Axis3::Y);
        assert!(axis3("w").is_err());
    }

    #[test]
    fn a_zdir_other_than_x_or_y_is_z() {
        assert_eq!(zdir_axis("x").unwrap(), Axis3::X);
        assert_eq!(zdir_axis("q").unwrap(), Axis3::Z);
        assert_eq!(zdir_axis("-z").unwrap(), Axis3::Z);
        assert!(zdir_axis("").is_err());
    }
}
