//! Inset axes anchored in a box of their parent (matplotlib's
//! `AnchoredSizeLocator` and `AnchoredZoomLocator` of
//! `axes_grid1.inset_locator`).

use pltrs::axes::Axes;
use pltrs::axes_grid1::{InsetOptions, SizeSpec, place_sized_inset, place_zoomed_inset};
use pltrs::offsetbox::{AnchorLoc, BboxToAnchor};
use pltrs::transforms::Transform;

use crate::binding::prelude::*;

/// A size as Python passes it: `(value, is_percent)`.
type PySize = (f64, bool);

/// The size `value`, a percentage of the anchor box when `percent`.
fn size_spec((value, percent): PySize) -> SizeSpec {
    if percent {
        SizeSpec::Percent(value)
    } else {
        SizeSpec::Inches(value)
    }
}

/// Where an inset goes: its location in the anchor box (a name or a
/// code), the anchor box (a point or `(x, y, width, height)`) and its
/// transform, and the padding to the box in font sizes.
type Placement = (PyAnchorLoc, Option<Vec<f64>>, Option<Transform>, (f64, f64));

/// A location of an anchored box as Python gives it: a name such as
/// `"upper right"` or a legend code.
pub(super) struct PyAnchorLoc(pub(super) AnchorLoc);

impl FromPy<'_> for PyAnchorLoc {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return name
                .parse::<AnchorLoc>()
                .map(Self)
                .map_err(|_| PyErr::value_error(format!("{name:?} is not a valid value for loc")));
        }
        let code: i64 = object.extract()?;
        AnchorLoc::from_code(code)
            .map(Self)
            .ok_or_else(|| PyErr::value_error(format!("{code} is not a valid value for loc")))
    }
}

/// The inset options `placement` describes.
fn options((loc, bbox_to_anchor, bbox_transform, borderpad): Placement) -> PyResult<InsetOptions> {
    let loc = loc.0;
    let bbox_to_anchor = match bbox_to_anchor.as_deref() {
        None => None,
        Some(&[x, y]) => Some(BboxToAnchor::Point(x, y)),
        Some(&[x, y, width, height]) => Some(BboxToAnchor::Bounds(x, y, width, height)),
        Some(_) => {
            return Err(PyErr::value_error(
                "bbox_to_anchor must be a point or a box (x, y, width, height)",
            ));
        }
    };
    Ok(InsetOptions {
        loc,
        bbox_to_anchor,
        bbox_transform,
        borderpad,
    })
}

/// Places `inset` `width` × `height` anchored in `parent` at every layout
/// pass (`AnchoredSizeLocator`).
fn place_sized(
    parent: Axes,
    inset: Axes,
    (width, height): (PySize, PySize),
    placement: Placement,
) -> PyResult<()> {
    place_sized_inset(
        &parent,
        &inset,
        size_spec(width),
        size_spec(height),
        options(placement)?,
    );
    Ok(())
}

/// Places `inset` showing its view `zoom` times as large as `parent`
/// shows it at every layout pass (`AnchoredZoomLocator`).
fn place_zoomed(parent: Axes, inset: Axes, zoom: f64, placement: Placement) -> PyResult<()> {
    place_zoomed_inset(&parent, &inset, zoom, options(placement)?);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("place_sized_inset", place_sized)?;
    module.add("place_zoomed_inset", place_zoomed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_inches_or_percentages() {
        assert_eq!(size_spec((30.0, true)), SizeSpec::Percent(30.0));
        assert_eq!(size_spec((1.5, false)), SizeSpec::Inches(1.5));
    }
}
