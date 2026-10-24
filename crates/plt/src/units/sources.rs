//! The plotting values of user types that artists keep after converting
//! them, so that they convert them again when an axis changes unit
//! (matplotlib's artists holding their values with units: `Line2D._xorig`,
//! `Text._x`, `Annotation.xy`, `Rectangle._x0`, converted when drawn or on
//! the axis' `units` callback).

use plt_core::Point;

use super::{AxisValue, AxisValues, DataPoint, UnitValues};
use crate::ticker::AxisName;

/// The user values along x and along y that an artist's numbers came
/// from: the data of a line, or the coordinates of a point (one value
/// each).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct UnitSources {
    /// The x values.
    pub(crate) x: Option<UnitValues>,
    /// The y values.
    pub(crate) y: Option<UnitValues>,
}

impl UnitSources {
    /// The user values among `x` and `y`.
    pub(crate) fn of(x: &AxisValues, y: &AxisValues) -> Self {
        Self {
            x: x.custom().cloned(),
            y: y.custom().cloned(),
        }
    }

    /// The coordinates of `point` that are user values.
    pub(crate) fn of_point(point: &DataPoint) -> Self {
        Self {
            x: value_source(&point.x),
            y: value_source(&point.y),
        }
    }

    /// `value` along axis `name`, if it is a user value: the source of
    /// the line of `axhline` or `axvline`.
    pub(crate) fn of_value(name: AxisName, value: &AxisValue) -> Self {
        let mut sources = Self::default();
        *sources.along_mut(name) = value_source(value);
        sources
    }

    /// The user values along axis `name`, if any.
    pub(crate) fn along(&self, name: AxisName) -> Option<&UnitValues> {
        match name {
            AxisName::X => self.x.as_ref(),
            AxisName::Y => self.y.as_ref(),
        }
    }

    /// The sources of item `index` alone: those of one point of a line
    /// split into a line per point (the caps of polar error bars).
    pub(crate) fn item(&self, index: usize) -> Self {
        Self {
            x: self.x.as_ref().and_then(|values| values.get(index)),
            y: self.y.as_ref().and_then(|values| values.get(index)),
        }
    }

    /// Forgets the values along axis `name`, whose numbers were replaced.
    pub(crate) fn forget(&mut self, name: AxisName) {
        *self.along_mut(name) = None;
    }

    /// Moves the coordinate along axis `name` of `point`, whose
    /// coordinates these sources are, to the next of `numbers` when it
    /// came from a user value: `numbers` holds the values of every
    /// [`along`](Self::along) asked for, converted again, in order.
    pub(crate) fn reconvert_point(
        &self,
        name: AxisName,
        point: &mut Point,
        numbers: &mut impl Iterator<Item = Vec<f64>>,
    ) {
        if self.along(name).is_none() {
            return;
        }
        if let Some(&number) = numbers.next().as_deref().and_then(<[f64]>::first) {
            match name {
                AxisName::X => point.x = number,
                AxisName::Y => point.y = number,
            }
        }
    }

    /// The values along axis `name`, to be set.
    fn along_mut(&mut self, name: AxisName) -> &mut Option<UnitValues> {
        match name {
            AxisName::X => &mut self.x,
            AxisName::Y => &mut self.y,
        }
    }
}

/// `value` as the source of a coordinate, if it is a user value.
fn value_source(value: &AxisValue) -> Option<UnitValues> {
    match value {
        AxisValue::Custom(value) => Some(value.clone()),
        AxisValue::Number(_) | AxisValue::Category(_) | AxisValue::Date(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::Axis;
    use crate::units::{ConversionError, ConversionInterface, Convertible};

    /// A number of steps.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Steps(f64);

    struct StepConverter;

    impl ConversionInterface for StepConverter {
        type Value = Steps;
        type Unit = ();

        fn convert(
            &self,
            values: &[Steps],
            _: Option<&()>,
            _: &Axis,
        ) -> Result<Vec<f64>, ConversionError> {
            Ok(values.iter().map(|steps| steps.0).collect())
        }
    }

    impl Convertible for Steps {
        type Converter = StepConverter;
        fn converter() -> StepConverter {
            StepConverter
        }
    }

    #[test]
    fn only_user_values_are_kept() {
        let point = DataPoint::from((Steps(1.0), 2.0));
        let sources = UnitSources::of_point(&point);
        assert_eq!(
            sources.along(AxisName::X),
            Some(&UnitValues::new(vec![Steps(1.0)]))
        );
        assert_eq!(sources.along(AxisName::Y), None);
        let line = UnitSources::of(&vec![Steps(1.0)].into(), &AxisValues::from(["a"]));
        assert!(line.x.is_some() && line.y.is_none());
        let mark = UnitSources::of_value(AxisName::Y, &Steps(3.0).into());
        assert!(mark.x.is_none() && mark.y.is_some());
        assert_eq!(
            UnitSources::of_value(AxisName::X, &AxisValue::Date(1.0)),
            UnitSources::default()
        );
    }

    #[test]
    fn points_take_the_numbers_of_their_sources_in_order() {
        let mut sources = UnitSources::of_point(&DataPoint::from((Steps(1.0), Steps(2.0))));
        let mut point = Point::new(1.0, 2.0);
        let mut numbers = vec![vec![10.0], vec![20.0]].into_iter();
        sources.reconvert_point(AxisName::Y, &mut point, &mut numbers);
        sources.reconvert_point(AxisName::X, &mut point, &mut numbers);
        assert_eq!(point, Point::new(20.0, 10.0), "each takes the next number");
        sources.forget(AxisName::Y);
        let mut none = std::iter::empty();
        sources.reconvert_point(AxisName::Y, &mut point, &mut none);
        assert_eq!(point.y, 10.0, "a forgotten source leaves the point alone");
    }
}
