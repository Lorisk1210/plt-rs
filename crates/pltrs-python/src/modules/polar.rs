//! `_native.polar`: the settings of polar axes (matplotlib's
//! `projections.polar.PolarAxes`): the orientation and limits of θ, the
//! radial limits and origin, the radial label position and the grids.
//!
//! Angles cross in degrees where matplotlib's setters take degrees
//! (`set_thetamin`, `set_rlabel_position`, ...) and in radians where it
//! takes radians (`set_theta_offset`).

use pltrs::axes::Axes;
use pltrs::polar::{Compass, PolarAxes, PolarError, ThetaDirection};
use pltrs::units::AxisValues;

use crate::binding::prelude::*;

impl From<PolarError> for PyErr {
    /// matplotlib's `ValueError` for θ limits it refuses.
    fn from(err: PolarError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// `axes` as polar axes; `TypeError` for other axes.
fn polar(axes: &Axes) -> PyResult<PolarAxes> {
    axes.as_polar()
        .ok_or_else(|| PyErr::type_error("expected polar axes"))
}

/// The angle θ = 0 is turned by, in radians.
fn get_theta_offset(axes: Axes) -> PyResult<f64> {
    Ok(polar(&axes)?.theta_offset())
}

/// Turns θ = 0 by `offset` radians.
fn set_theta_offset(axes: Axes, offset: f64) -> PyResult<()> {
    polar(&axes)?.set_theta_offset(offset);
    Ok(())
}

/// Puts θ = 0 at the compass point `loc` (`"N"`, `"NW"`, ..., `"E"`),
/// turned by `offset` degrees.
fn set_theta_zero_location(axes: Axes, loc: String, offset: f64) -> PyResult<()> {
    let location = match loc.as_str() {
        "N" => Compass::N,
        "NW" => Compass::NW,
        "W" => Compass::W,
        "SW" => Compass::SW,
        "S" => Compass::S,
        "SE" => Compass::SE,
        "E" => Compass::E,
        "NE" => Compass::NE,
        _ => {
            return Err(PyErr::key_error(format!("{loc:?}")));
        }
    };
    polar(&axes)?.set_theta_zero_location(location, offset);
    Ok(())
}

/// The direction θ increases in: 1 counterclockwise, -1 clockwise.
fn get_theta_direction(axes: Axes) -> PyResult<f64> {
    Ok(polar(&axes)?.theta_direction().sign())
}

/// Makes θ increase counterclockwise (`clockwise` false) or clockwise.
fn set_theta_direction(axes: Axes, clockwise: bool) -> PyResult<()> {
    polar(&axes)?.set_theta_direction(if clockwise {
        ThetaDirection::Clockwise
    } else {
        ThetaDirection::Counterclockwise
    });
    Ok(())
}

/// `(thetamin, thetamax)` in degrees.
fn get_thetalim(axes: Axes) -> PyResult<(f64, f64)> {
    let polar = polar(&axes)?;
    Ok((polar.thetamin(), polar.thetamax()))
}

/// Sets the smallest θ shown, in degrees.
fn set_thetamin(axes: Axes, degrees: f64) -> PyResult<()> {
    polar(&axes)?.set_thetamin(degrees);
    Ok(())
}

/// Sets the largest θ shown, in degrees.
fn set_thetamax(axes: Axes, degrees: f64) -> PyResult<()> {
    polar(&axes)?.set_thetamax(degrees);
    Ok(())
}

/// Sets the θ range shown, in degrees.
fn set_thetalim(axes: Axes, min: f64, max: f64) -> PyResult<()> {
    polar(&axes)?.set_thetalim(min, max)?;
    Ok(())
}

/// `(rmin, rmax)`.
fn get_rlim(axes: Axes) -> PyResult<(f64, f64)> {
    let polar = polar(&axes)?;
    Ok((polar.rmin(), polar.rmax()))
}

/// Sets the radius at the centre (or inner edge).
fn set_rmin(axes: Axes, rmin: f64) -> PyResult<()> {
    polar(&axes)?.set_rmin(rmin);
    Ok(())
}

/// Sets the radius at the outer edge.
fn set_rmax(axes: Axes, rmax: f64) -> PyResult<()> {
    polar(&axes)?.set_rmax(rmax);
    Ok(())
}

/// The radius drawn at the centre.
fn get_rorigin(axes: Axes) -> PyResult<f64> {
    Ok(polar(&axes)?.rorigin())
}

/// The radius set to be drawn at the centre, `None` when it follows
/// `rmin`.
fn get_rorigin_set(axes: Axes) -> PyResult<Option<f64>> {
    Ok(polar(&axes)?.rorigin_set())
}

/// Sets the radius drawn at the centre (`None`: `rmin`).
fn set_rorigin(axes: Axes, rorigin: Option<f64>) -> PyResult<()> {
    polar(&axes)?.set_rorigin(rorigin);
    Ok(())
}

/// The angle of the radial tick labels, in degrees.
fn get_rlabel_position(axes: Axes) -> PyResult<f64> {
    Ok(polar(&axes)?.rlabel_position())
}

/// Puts the radial tick labels at `degrees`.
fn set_rlabel_position(axes: Axes, degrees: f64) -> PyResult<()> {
    polar(&axes)?.set_rlabel_position(degrees);
    Ok(())
}

/// Puts the radial ticks at `radii`.
fn set_rticks(axes: Axes, radii: Vec<f64>) -> PyResult<()> {
    polar(&axes)?.set_rticks(AxisValues::Numbers(radii));
    Ok(())
}

/// Puts the angular gridlines at `degrees`.
fn set_thetagrids(axes: Axes, degrees: Vec<f64>) -> PyResult<()> {
    polar(&axes)?.set_thetagrids(degrees);
    Ok(())
}

/// Puts the radial gridlines at `radii`, their labels at `angle` degrees
/// (`None`: where they are).
fn set_rgrids(axes: Axes, radii: Vec<f64>, angle: Option<f64>) -> PyResult<()> {
    polar(&axes)?.set_rgrids(AxisValues::Numbers(radii), angle);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_theta_offset", get_theta_offset)?;
    module.add("set_theta_offset", set_theta_offset)?;
    module.add("set_theta_zero_location", set_theta_zero_location)?;
    module.add("get_theta_direction", get_theta_direction)?;
    module.add("set_theta_direction", set_theta_direction)?;
    module.add("get_thetalim", get_thetalim)?;
    module.add("set_thetamin", set_thetamin)?;
    module.add("set_thetamax", set_thetamax)?;
    module.add("set_thetalim", set_thetalim)?;
    module.add("get_rlim", get_rlim)?;
    module.add("set_rmin", set_rmin)?;
    module.add("set_rmax", set_rmax)?;
    module.add("get_rorigin", get_rorigin)?;
    module.add("set_rorigin", set_rorigin)?;
    module.add("get_rorigin_set", get_rorigin_set)?;
    module.add("get_rlabel_position", get_rlabel_position)?;
    module.add("set_rlabel_position", set_rlabel_position)?;
    module.add("set_rticks", set_rticks)?;
    module.add("set_thetagrids", set_thetagrids)?;
    module.add("set_rgrids", set_rgrids)
}
