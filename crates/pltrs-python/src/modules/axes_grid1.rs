//! `_native.axes_grid1`: axes placed by pltrs (Rust)'s
//! [`pltrs::axes_grid1`] (matplotlib's `mpl_toolkits.axes_grid1`): inset
//! axes anchored in a box of their parent (`inset_locator`).

use pltrs::axes::Axes;
use pltrs::axes_grid1::{InsetOptions, SizeSpec, inset_axes, zoomed_inset_axes};
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
type Placement = (Owned, Option<Vec<f64>>, Option<Transform>, (f64, f64));

/// The inset options `placement` describes.
fn options((loc, bbox_to_anchor, bbox_transform, borderpad): Placement) -> PyResult<InsetOptions> {
    let loc = Python::with_gil(|py| {
        let loc = loc.bind(py);
        if let Ok(name) = loc.extract::<String>() {
            return name
                .parse::<AnchorLoc>()
                .map_err(|_| PyErr::value_error(format!("{name:?} is not a valid value for loc")));
        }
        let code: i64 = loc.extract()?;
        AnchorLoc::from_code(code)
            .ok_or_else(|| PyErr::value_error(format!("{code} is not a valid value for loc")))
    })?;
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

/// An inset axes `width` × `height` anchored in `parent`
/// (`inset_locator.inset_axes`).
fn inset(parent: Axes, (width, height): (PySize, PySize), placement: Placement) -> PyResult<Axes> {
    Ok(inset_axes(
        &parent,
        size_spec(width),
        size_spec(height),
        options(placement)?,
    ))
}

/// An inset axes showing its view `zoom` times as large as `parent`
/// shows it (`inset_locator.zoomed_inset_axes`).
fn zoomed_inset(parent: Axes, zoom: f64, placement: Placement) -> PyResult<Axes> {
    Ok(zoomed_inset_axes(&parent, zoom, options(placement)?))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("inset_axes", inset)?;
    module.add("zoomed_inset_axes", zoomed_inset)
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
