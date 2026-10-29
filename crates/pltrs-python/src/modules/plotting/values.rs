//! Plotting values as the Python side passes them: numbers, category
//! names or dates (matplotlib's unit-converted data), and the extents
//! added to them: numbers or durations.

use pltrs::dates::TimeDelta;
use pltrs::units::{AxisDelta, AxisDeltas, AxisValue, AxisValues};

use crate::binding::prelude::*;

/// Values along an axis: a float array, a list of category names, or
/// `("dates", numbers)` for dates as matplotlib's date numbers.
pub(super) struct PyAxisValues(pub(super) AxisValues);

impl FromPy<'_> for PyAxisValues {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok((tag, numbers)) = object.extract::<(String, NdArray)>()
            && tag == "dates"
        {
            return Ok(Self(AxisValues::Dates {
                numbers: numbers.into_vec()?,
                tz: None,
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

/// Extents along an axis: a float array, or `("durations",
/// microseconds)` for `timedelta`s.
pub(super) struct PyAxisDeltas(pub(super) AxisDeltas);

impl FromPy<'_> for PyAxisDeltas {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok((tag, micros)) = object.extract::<(String, NdArray<i64>)>()
            && tag == "durations"
        {
            let durations = micros
                .data
                .iter()
                .map(|&micros| {
                    TimeDelta::new(0, 0, micros).map_err(|err| PyErr::value_error(err.to_string()))
                })
                .collect::<PyResult<Vec<_>>>()?;
            return Ok(Self(AxisDeltas::Durations(durations)));
        }
        let numbers: NdArray = object.extract()?;
        Ok(Self(AxisDeltas::Numbers(numbers.into_vec()?)))
    }
}

impl PyAxisValues {
    /// The values one by one.
    pub(super) fn items(self) -> Vec<AxisValue> {
        match self.0 {
            AxisValues::Numbers(numbers) => numbers.into_iter().map(AxisValue::Number).collect(),
            AxisValues::Categories(names) => names.into_iter().map(AxisValue::Category).collect(),
            AxisValues::Dates { numbers, tz } => numbers
                .into_iter()
                .map(|number| AxisValue::Date {
                    number,
                    tz: tz.clone(),
                })
                .collect(),
            AxisValues::Custom(values) => vec![AxisValue::Custom(values)],
        }
    }
}

impl PyAxisDeltas {
    /// The extents one by one.
    pub(super) fn items(self) -> Vec<AxisDelta> {
        match self.0 {
            AxisDeltas::Numbers(numbers) => numbers.into_iter().map(AxisDelta::Number).collect(),
            AxisDeltas::Durations(durations) => {
                durations.into_iter().map(AxisDelta::Duration).collect()
            }
            AxisDeltas::Custom(values) => vec![AxisDelta::Custom(values)],
        }
    }
}
