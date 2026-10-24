//! Extents along an axis given in plotting values (bar heights and
//! widths, error sizes, the sides of rectangles) converted to axis
//! numbers through the values they extend from (matplotlib's
//! `Axes._convert_dx` and `Rectangle._convert_units`).

use super::axis_handle::Axis;
use crate::units::{AxisDelta, AxisDeltas, AxisValue, AxisValues, ConversionError, first_finite};

impl Axis {
    /// Extents converted to axis numbers by themselves (matplotlib's
    /// `convert(dx)`): numbers as they are, durations in days and user
    /// values by the axis converter, which an axis without one takes from
    /// them.
    pub(crate) fn convert_deltas(&self, deltas: &AxisDeltas) -> Vec<f64> {
        deltas
            .to_numbers()
            .unwrap_or_else(|values| self.convert_custom(values))
    }

    /// Extents from `origin`, whose values convert to `converted`, in axis
    /// numbers (matplotlib's `Axes._convert_dx`): the converted far end of
    /// each extent from the first finite origin less the first finite
    /// converted origin. When there is no origin, or an extent does not
    /// add to it or its far end fails to convert, the extents are
    /// converted by themselves instead.
    ///
    /// Numbers round through their origin as in matplotlib: a bar one
    /// unit high from 1e16 is no height at all.
    pub(crate) fn convert_dx(
        &self,
        deltas: &AxisDeltas,
        origin: &AxisValues,
        converted: &[f64],
    ) -> Vec<f64> {
        let (Some(start), Some(converted_start)) = (origin.first_finite(), first_finite(converted))
        else {
            return self.convert_deltas(deltas);
        };
        let ends: Option<Vec<f64>> = deltas
            .iter()
            .map(|delta| {
                let end = start.add_delta(&delta)?;
                self.try_convert_value(&end).ok()
            })
            .collect();
        match ends {
            Some(ends) => ends.into_iter().map(|end| end - converted_start).collect(),
            None => self.convert_deltas(deltas),
        }
    }

    /// The axis numbers of `start` and of the far end of the extent
    /// `delta` from it, which plotting methods placing spans compute as
    /// matplotlib's `broken_barh` does (`convert((x0, x0 + dx))`). An
    /// extent that does not add to its start, where matplotlib fails, is
    /// converted by itself and added to the converted start.
    pub(crate) fn convert_span(&self, start: AxisValue, delta: &AxisDelta) -> (f64, f64) {
        let end = start.add_delta(delta);
        let start_number = self.convert_value(start);
        let end_number = match end {
            Some(end) => self.convert_without_update(&end),
            None => start_number + one_number(self.convert_deltas(&delta.clone().into())),
        };
        (start_number, end_number)
    }

    /// Converts `value` as an artist converts its own values (matplotlib's
    /// `Axis.convert_units`, which `convert_xunits` calls): a user value
    /// without updating the unit of the axis, which only takes a converter
    /// if it has none. A failed conversion gives NaN and is reported when
    /// the figure next draws.
    pub(crate) fn convert_without_update(&self, value: &AxisValue) -> f64 {
        match value {
            AxisValue::Custom(values) => one_number(self.convert_custom(values)),
            value => self.convert_value(value.clone()),
        }
    }

    /// [`convert_without_update`](Self::convert_without_update), failing
    /// instead of reporting.
    fn try_convert_value(&self, value: &AxisValue) -> Result<f64, ConversionError> {
        match value {
            AxisValue::Custom(values) => self.try_convert_custom(values).map(one_number),
            value => Ok(self.convert_value(value.clone())),
        }
    }
}

/// The number one value converted to.
fn one_number(numbers: Vec<f64>) -> f64 {
    numbers
        .into_iter()
        .next()
        .expect("one value converts to one number")
}

#[cfg(test)]
mod tests {
    use crate::axes::Axis;
    use crate::dates::{DateTime, TimeDelta};
    use crate::figure::Figure;
    use crate::units::{
        AxisDelta, AxisDeltas, AxisValue, AxisValues, ConversionError, ConversionInterface,
        Convertible,
    };

    /// A time on a clock running from noon, in hours.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Clock(f64);

    /// A number of hours, which adds to clock times.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Hours(f64);

    /// Clock times as hours since midnight; a negative time fails.
    struct ClockConverter;

    impl ConversionInterface for ClockConverter {
        type Value = Clock;
        type Unit = ();

        fn convert(
            &self,
            values: &[Clock],
            _: Option<&()>,
            _: &Axis,
        ) -> Result<Vec<f64>, ConversionError> {
            values
                .iter()
                .map(|clock| {
                    if clock.0 < 0.0 {
                        Err(ConversionError::Failed("before midnight".into()))
                    } else {
                        Ok(clock.0 + 12.0)
                    }
                })
                .collect()
        }
    }

    impl Convertible for Clock {
        type Converter = ClockConverter;
        fn converter() -> ClockConverter {
            ClockConverter
        }

        fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
            delta
                .downcast_ref::<Hours>()
                .map(|hours| Clock(self.0 + hours.0).into())
        }
    }

    /// Hours as they are.
    struct HoursConverter;

    impl ConversionInterface for HoursConverter {
        type Value = Hours;
        type Unit = ();

        fn convert(
            &self,
            values: &[Hours],
            _: Option<&()>,
            _: &Axis,
        ) -> Result<Vec<f64>, ConversionError> {
            Ok(values.iter().map(|hours| hours.0).collect())
        }
    }

    impl Convertible for Hours {
        type Converter = HoursConverter;
        fn converter() -> HoursConverter {
            HoursConverter
        }
    }

    #[test]
    fn numbers_round_through_their_origin() {
        let ax = Figure::new().subplot();
        let axis = ax.xaxis();
        let origin = AxisValues::Numbers(vec![f64::NAN, 1e16]);
        let deltas = AxisDeltas::from([1.0]);
        assert_eq!(axis.convert_dx(&deltas, &origin, &[f64::NAN, 1e16]), [0.0]);
        let none = AxisValues::Numbers(Vec::new());
        let tenth = AxisDeltas::from(0.1);
        assert_eq!(axis.convert_dx(&tenth, &none, &[]), [0.1]);
    }

    #[test]
    fn extents_that_add_convert_as_far_end_less_start() {
        let ax = Figure::new().subplot();
        let axis = ax.xaxis();
        let origin = AxisValues::from(vec![Clock(1.0), Clock(5.0)]);
        let converted = axis.convert_units(origin.clone());
        let deltas = AxisDeltas::from(vec![Hours(2.0), Hours(0.5)]);
        assert_eq!(axis.convert_dx(&deltas, &origin, &converted), [2.0, 0.5]);
        // A far end that fails to convert falls back to the extents alone.
        let back = AxisDeltas::from(vec![Hours(-3.0)]);
        assert_eq!(axis.convert_dx(&back, &origin, &converted), [-3.0]);
        // So do numbers, which do not add to clock times.
        assert_eq!(
            axis.convert_dx(&AxisDeltas::from(0.8), &origin, &converted),
            [0.8]
        );
    }

    #[test]
    fn durations_add_to_dates_and_fall_back_elsewhere() {
        let ax = Figure::new().subplot();
        let axis = ax.xaxis();
        let start = DateTime::new(1970, 1, 3, 0, 0, 0, 0).expect("a date");
        let origin = AxisValues::from(vec![start]);
        let converted = axis.convert_units(origin.clone());
        let day = TimeDelta::new(1, 43_200, 0).expect("a duration");
        let deltas = AxisDeltas::from(day);
        assert_eq!(axis.convert_dx(&deltas, &origin, &converted), [1.5]);
        let numbers = AxisValues::Numbers(vec![4.0]);
        assert_eq!(axis.convert_dx(&deltas, &numbers, &[4.0]), [1.5]);
        assert_eq!(
            axis.convert_span(AxisValue::Number(4.0), &day.into()),
            (4.0, 5.5)
        );
    }

    #[test]
    fn user_extents_by_themselves_give_the_axis_their_converter() {
        let ax = Figure::new().subplot();
        let yaxis = ax.yaxis();
        let deltas = AxisDeltas::from(vec![Hours(1.5)]);
        assert_eq!(yaxis.convert_deltas(&deltas), [1.5]);
        assert!(yaxis.converter().is_some() && yaxis.units().is_none());
    }

    #[test]
    fn spans_convert_both_ends() {
        let ax = Figure::new().subplot();
        let axis = ax.xaxis();
        assert_eq!(
            axis.convert_span(Clock(1.0).into(), &Hours(2.0).into()),
            (13.0, 15.0)
        );
        assert_eq!(
            axis.convert_span(Clock(1.0).into(), &AxisDelta::Number(0.5)),
            (13.0, 13.5),
            "an extent that does not add converts by itself"
        );
        let fig = Figure::new();
        let ax = fig.subplot();
        let (_, end) = ax
            .xaxis()
            .convert_span(Clock(1.0).into(), &Hours(-4.0).into());
        assert!(end.is_nan(), "a failed far end is reported");
        assert!(fig.to_png().is_err());
    }
}
