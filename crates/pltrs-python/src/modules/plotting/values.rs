//! Values in the units pltrs (Rust) converts, as the Python side passes
//! them to have an axis take their units or convert them (matplotlib's
//! `update_units` and `convert_units` with its date and category
//! converters): numbers, category names or dates.

use pltrs::units::AxisValues;

use crate::binding::prelude::*;
use crate::modules::axis::{AnyAxis, AxisOwner};
use crate::modules::dates::Zone;

/// Values along an axis: a float array, a list of category names, or
/// `("dates", numbers, zone)` for dates as matplotlib's date numbers, with
/// the zone of the first (`None` when it is naive).
pub(crate) struct PyAxisValues(pub(crate) AxisValues);

impl FromPy<'_> for PyAxisValues {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok((tag, numbers, zone)) = object.extract::<(String, NdArray, Option<Zone>)>()
            && tag == "dates"
        {
            return Ok(Self(AxisValues::Dates {
                numbers: numbers.into_vec()?,
                tz: zone.map(|Zone(tz)| tz),
            }));
        }
        if object.type_name() == "list"
            && let Ok(names) = object.extract::<Vec<String>>()
            && !names.is_empty()
        {
            return Ok(Self(AxisValues::Categories(names)));
        }
        let numbers: NdArray = object.extract()?;
        Ok(Self(AxisValues::Numbers(numbers.into_vec()?)))
    }
}

/// `values` as numbers of the axis `name` of `owner` (`"x"` or `"y"`, or
/// `"z"` of a 3-D axes), which takes the units they bring, the axes
/// sharing it their category names (matplotlib's `_process_unit_info`
/// and `convert_xunits`).
pub(super) fn convert_units(
    owner: AxisOwner,
    name: String,
    values: PyAxisValues,
) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.convert_units(values.0),
        AnyAxis::Deep(axis) => axis.convert_units(values.0),
    }))
}
