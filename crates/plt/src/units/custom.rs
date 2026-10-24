//! User values, units and converters with their types erased, so that
//! axes and artists can hold them (matplotlib's duck-typed unit objects).

use std::any::{Any, TypeId, type_name};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::interface::UnitOf;
use super::{
    AxisDelta, AxisValue, ConversionError, ConversionInterface, Convertible, UnitAxisInfo,
};
use crate::axes::Axis;

/// The unit of an axis (matplotlib's `Axis.units`): a value of the unit
/// type of a [`ConversionInterface`], whatever its type. Units compare
/// equal when they are of one type and equal as such.
///
/// ```
/// use plt::units::Unit;
/// assert_eq!(Unit::new("hours"), Unit::new("hours"));
/// assert_ne!(Unit::new("hours"), Unit::new(String::from("hours")));
/// assert_eq!(Unit::new(2.0).downcast_ref::<f64>(), Some(&2.0));
/// ```
#[derive(Clone)]
pub struct Unit(Rc<dyn AnyUnit>);

/// What a [`Unit`] needs of the value it holds.
trait AnyUnit: Any + fmt::Debug {
    /// Whether `other` is of the same type and equal.
    fn equals(&self, other: &dyn AnyUnit) -> bool;
}

impl<U: PartialEq + fmt::Debug + 'static> AnyUnit for U {
    fn equals(&self, other: &dyn AnyUnit) -> bool {
        (other as &dyn Any).downcast_ref::<U>() == Some(self)
    }
}

impl Unit {
    /// `unit` as an axis unit; a `Unit` stays itself.
    pub fn new<U: PartialEq + fmt::Debug + 'static>(unit: U) -> Self {
        match (&unit as &dyn Any).downcast_ref::<Self>() {
            Some(unit) => unit.clone(),
            None => Self(Rc::new(unit)),
        }
    }

    /// The unit as a `U`, if it is one.
    pub fn downcast_ref<U: 'static>(&self) -> Option<&U> {
        (self.0.as_ref() as &dyn Any).downcast_ref()
    }

    /// Whether the unit is a `U`.
    pub fn is<U: 'static>(&self) -> bool {
        self.downcast_ref::<U>().is_some()
    }
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.0.equals(other.0.as_ref())
    }
}

impl fmt::Debug for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A [`ConversionInterface`] whatever its type, as an axis holds it
/// (matplotlib's converter instance on an axis). Converters compare
/// equal when they are of one type.
///
/// ```
/// use plt::units::{CustomConverter, UnitConverter};
/// # use plt::axes::Axis;
/// # use plt::units::{ConversionError, ConversionInterface};
/// # struct Scaled;
/// # impl ConversionInterface for Scaled {
/// #     type Value = f32;
/// #     type Unit = f64;
/// #     fn convert(&self, values: &[f32], _: Option<&f64>, _: &Axis)
/// #         -> Result<Vec<f64>, ConversionError> {
/// #         Ok(values.iter().map(|&v| f64::from(v)).collect())
/// #     }
/// # }
/// let converter = CustomConverter::new(Scaled);
/// assert!(converter.is::<Scaled>());
/// assert_eq!(UnitConverter::from(Scaled), UnitConverter::Custom(converter));
/// ```
#[derive(Clone)]
pub struct CustomConverter(Rc<dyn AnyConverter>);

/// What a [`CustomConverter`] needs of the converter it holds.
trait AnyConverter: Any {
    /// [`ConversionInterface::axisinfo`] in an axis unit, nothing when the
    /// unit is not one of the converter's.
    fn axisinfo(&self, unit: Option<&Unit>, axis: &Axis) -> Option<UnitAxisInfo>;

    /// The type of the converter.
    fn type_name(&self) -> &'static str;
}

impl<C: ConversionInterface> AnyConverter for C {
    fn axisinfo(&self, unit: Option<&Unit>, axis: &Axis) -> Option<UnitAxisInfo> {
        let unit = match unit {
            Some(unit) => Some(unit.downcast_ref::<C::Unit>()?),
            None => None,
        };
        ConversionInterface::axisinfo(self, unit, axis)
    }

    fn type_name(&self) -> &'static str {
        type_name::<C>()
    }
}

impl CustomConverter {
    /// `converter`, with its type erased.
    pub fn new<C: ConversionInterface>(converter: C) -> Self {
        Self(Rc::new(converter))
    }

    /// The converter as a `C`, if it is one.
    pub fn downcast_ref<C: ConversionInterface>(&self) -> Option<&C> {
        (self.0.as_ref() as &dyn Any).downcast_ref()
    }

    /// Whether the converter is a `C`.
    pub fn is<C: ConversionInterface>(&self) -> bool {
        self.downcast_ref::<C>().is_some()
    }

    /// The type of the converter.
    pub fn type_name(&self) -> &'static str {
        self.0.type_name()
    }

    /// The tick helpers, label and default limits the converter gives
    /// `axis` in `unit`.
    pub(crate) fn axisinfo(&self, unit: Option<&Unit>, axis: &Axis) -> Option<UnitAxisInfo> {
        self.0.axisinfo(unit, axis)
    }

    /// The type of the converter, which identifies it.
    fn converter_type(&self) -> TypeId {
        (self.0.as_ref() as &dyn Any).type_id()
    }
}

impl PartialEq for CustomConverter {
    fn eq(&self, other: &Self) -> bool {
        self.converter_type() == other.converter_type()
    }
}

impl Eq for CustomConverter {}

impl Hash for CustomConverter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.converter_type().hash(state);
    }
}

impl fmt::Debug for CustomConverter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CustomConverter")
            .field(&self.type_name())
            .finish()
    }
}

/// Plotting values of a [`Convertible`] type before conversion, whatever
/// the type, as [`AxisValues::Custom`](super::AxisValues::Custom) holds
/// them.
///
/// ```
/// # use plt::axes::Axis;
/// # use plt::units::{ConversionError, ConversionInterface, Convertible};
/// use plt::units::CustomValues;
/// # #[derive(Clone, Debug, PartialEq)]
/// # struct Miles(f64);
/// # struct MilesConverter;
/// # impl ConversionInterface for MilesConverter {
/// #     type Value = Miles;
/// #     type Unit = ();
/// #     fn convert(&self, values: &[Miles], _: Option<&()>, _: &Axis)
/// #         -> Result<Vec<f64>, ConversionError> {
/// #         Ok(values.iter().map(|m| m.0).collect())
/// #     }
/// # }
/// # impl Convertible for Miles {
/// #     type Converter = MilesConverter;
/// #     fn converter() -> MilesConverter { MilesConverter }
/// # }
/// let values = CustomValues::new(vec![Miles(1.0), Miles(2.5)]);
/// assert_eq!(values.len(), 2);
/// assert_eq!(values.downcast_ref::<Miles>(), Some(&[Miles(1.0), Miles(2.5)][..]));
/// ```
#[derive(Clone)]
pub struct CustomValues(Rc<dyn AnyValues>);

/// What [`CustomValues`] needs of the values it holds.
trait AnyValues: Any + fmt::Debug {
    /// The number of values.
    fn len(&self) -> usize;

    /// The converter of the values' type.
    fn converter(&self) -> CustomConverter;

    /// The unit `converter` picks for the values on `axis`, if it is the
    /// converter of their type and picks one.
    fn default_units(&self, converter: &CustomConverter, axis: &Axis) -> Option<Unit>;

    /// The values in `unit` on `axis`, converted by `converter` when it is
    /// of the converter type of the values, else by their own converter.
    fn convert(
        &self,
        converter: Option<&CustomConverter>,
        unit: Option<&Unit>,
        axis: &Axis,
    ) -> Result<Vec<f64>, ConversionError>;

    /// Whether `other` holds equal values of the same type.
    fn equals(&self, other: &dyn AnyValues) -> bool;

    /// Value `index` alone.
    fn get(&self, index: usize) -> Option<CustomValues>;

    /// The first value moved by `delta`, if it adds
    /// ([`Convertible::add_delta`]).
    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue>;

    /// The first value `factor` times, if it scales
    /// ([`Convertible::scale`]).
    fn scale(&self, factor: i64) -> Option<CustomValues>;

    /// These values followed by those of `rest`, if all are of this type.
    fn concat(&self, rest: &[&CustomValues]) -> Option<CustomValues>;
}

/// Values of one [`Convertible`] type.
#[derive(Debug, PartialEq)]
struct Values<T>(Vec<T>);

impl<T: Convertible> Values<T> {
    /// The values in `unit` on `axis`, converted by `converter`.
    fn convert_with(
        &self,
        converter: &T::Converter,
        unit: Option<&Unit>,
        axis: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        let unit = unit
            .map(|unit| {
                unit.downcast_ref::<UnitOf<T>>()
                    .ok_or_else(|| ConversionError::ForeignUnit {
                        unit: format!("{unit:?}"),
                        converter: type_name::<T::Converter>(),
                    })
            })
            .transpose()?;
        let numbers = converter.convert(&self.0, unit, axis)?;
        if numbers.len() == self.0.len() {
            Ok(numbers)
        } else {
            Err(ConversionError::Count {
                values: self.0.len(),
                numbers: numbers.len(),
            })
        }
    }
}

impl<T: Convertible> AnyValues for Values<T> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn converter(&self) -> CustomConverter {
        CustomConverter::new(T::converter())
    }

    fn default_units(&self, converter: &CustomConverter, axis: &Axis) -> Option<Unit> {
        let converter = converter.downcast_ref::<T::Converter>()?;
        converter.default_units(&self.0, axis).map(Unit::new)
    }

    fn convert(
        &self,
        converter: Option<&CustomConverter>,
        unit: Option<&Unit>,
        axis: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        match converter.and_then(CustomConverter::downcast_ref::<T::Converter>) {
            Some(converter) => self.convert_with(converter, unit, axis),
            None => self.convert_with(&T::converter(), unit, axis),
        }
    }

    fn equals(&self, other: &dyn AnyValues) -> bool {
        (other as &dyn Any).downcast_ref::<Self>() == Some(self)
    }

    fn get(&self, index: usize) -> Option<CustomValues> {
        self.0
            .get(index)
            .map(|value| CustomValues::new(vec![value.clone()]))
    }

    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        self.0.first()?.add_delta(delta)
    }

    fn scale(&self, factor: i64) -> Option<CustomValues> {
        let scaled = self.0.first()?.scale(factor)?;
        Some(CustomValues::new(vec![scaled]))
    }

    fn concat(&self, rest: &[&CustomValues]) -> Option<CustomValues> {
        let mut values = self.0.clone();
        for part in rest {
            values.extend_from_slice(part.downcast_ref::<T>()?);
        }
        Some(CustomValues::new(values))
    }
}

impl CustomValues {
    /// `values`, with their type erased.
    pub fn new<T: Convertible>(values: Vec<T>) -> Self {
        Self(Rc::new(Values(values)))
    }

    /// The number of values.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The values as `T`s, if they are.
    pub fn downcast_ref<T: Convertible>(&self) -> Option<&[T]> {
        (self.0.as_ref() as &dyn Any)
            .downcast_ref::<Values<T>>()
            .map(|values| values.0.as_slice())
    }

    /// The converter of the values' type (matplotlib's
    /// `registry.get_converter`).
    pub fn converter(&self) -> CustomConverter {
        self.0.converter()
    }

    /// The unit `converter` picks for the values on `axis` (matplotlib's
    /// `default_units`).
    pub(crate) fn default_units(&self, converter: &CustomConverter, axis: &Axis) -> Option<Unit> {
        self.0.default_units(converter, axis)
    }

    /// The values in `unit` on `axis` (matplotlib's `convert`).
    pub(crate) fn convert(
        &self,
        converter: Option<&CustomConverter>,
        unit: Option<&Unit>,
        axis: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        self.0.convert(converter, unit, axis)
    }

    /// Value `index` alone, if there is one.
    pub(crate) fn get(&self, index: usize) -> Option<Self> {
        self.0.get(index)
    }

    /// The first value moved by the extent `delta`, if the extent adds to
    /// it ([`Convertible::add_delta`]).
    pub(crate) fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        self.0.add_delta(delta)
    }

    /// The first value `factor` times, alone, if it scales
    /// ([`Convertible::scale`]).
    pub(crate) fn scale(&self, factor: i64) -> Option<Self> {
        self.0.scale(factor)
    }

    /// The user values among `values` in one list, when every value is a
    /// user value and all are of one type; `None` for no values.
    pub(crate) fn join<'a>(values: impl IntoIterator<Item = &'a AxisValue>) -> Option<Self> {
        let parts = values
            .into_iter()
            .map(|value| match value {
                AxisValue::Custom(values) => Some(values),
                AxisValue::Number(_) | AxisValue::Category(_) | AxisValue::Date(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        let (first, rest) = parts.split_first()?;
        first.0.concat(rest)
    }
}

impl PartialEq for CustomValues {
    fn eq(&self, other: &Self) -> bool {
        self.0.equals(other.0.as_ref())
    }
}

impl fmt::Debug for CustomValues {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Convertible> From<Vec<T>> for CustomValues {
    fn from(values: Vec<T>) -> Self {
        Self::new(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    /// Lengths in a unit of whole millimetres.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Millimetres(f64);

    struct MillimetreConverter;

    impl ConversionInterface for MillimetreConverter {
        type Value = Millimetres;
        type Unit = u32;

        fn convert(
            &self,
            values: &[Millimetres],
            unit: Option<&u32>,
            _: &Axis,
        ) -> Result<Vec<f64>, ConversionError> {
            let per = f64::from(*unit.unwrap_or(&1));
            Ok(values.iter().map(|v| v.0 / per).collect())
        }

        fn default_units(&self, _: &[Millimetres], _: &Axis) -> Option<u32> {
            Some(10)
        }
    }

    impl Convertible for Millimetres {
        type Converter = MillimetreConverter;
        fn converter() -> MillimetreConverter {
            MillimetreConverter
        }
    }

    #[test]
    fn units_compare_by_type_and_value() {
        assert_eq!(Unit::new(3_u32), Unit::new(3_u32));
        assert_ne!(Unit::new(3_u32), Unit::new(3_u64));
        assert_ne!(Unit::new(3_u32), Unit::new(4_u32));
        assert_eq!(Unit::new(Unit::new("cm")), Unit::new("cm"), "no nesting");
        assert!(Unit::new("cm").is::<&str>());
        assert_eq!(format!("{:?}", Unit::new("cm")), "\"cm\"");
    }

    #[test]
    fn values_convert_in_the_axis_unit() {
        let ax = Figure::new().subplot();
        let axis = ax.xaxis();
        let values = CustomValues::new(vec![Millimetres(20.0), Millimetres(5.0)]);
        let converter = values.converter();
        assert!(converter.is::<MillimetreConverter>());
        assert_eq!(
            values.default_units(&converter, &axis),
            Some(Unit::new(10_u32))
        );
        let unit = Unit::new(10_u32);
        assert_eq!(values.convert(None, Some(&unit), &axis), Ok(vec![2.0, 0.5]));
        assert_eq!(
            values.convert(Some(&converter), None, &axis),
            Ok(vec![20.0, 5.0])
        );
        let foreign = values.convert(None, Some(&Unit::new("cm")), &axis);
        assert!(matches!(foreign, Err(ConversionError::ForeignUnit { .. })));
        assert!(values.downcast_ref::<Millimetres>().is_some());
        assert_eq!(
            values,
            CustomValues::from(vec![Millimetres(20.0), Millimetres(5.0)])
        );
        assert!(CustomValues::new(Vec::<Millimetres>::new()).is_empty());
    }

    #[test]
    fn values_come_apart_and_add_nothing_by_default() {
        let values = CustomValues::new(vec![Millimetres(1.0), Millimetres(2.0)]);
        assert_eq!(
            values.get(1),
            Some(CustomValues::new(vec![Millimetres(2.0)]))
        );
        assert_eq!(values.get(2), None);
        assert_eq!(values.add_delta(&AxisDelta::Number(1.0)), None);
        assert_eq!(values.scale(-1), None);
    }

    #[test]
    fn values_of_one_type_join() {
        let one = AxisValue::from(Millimetres(1.0));
        let two = AxisValue::from(Millimetres(2.0));
        assert_eq!(
            CustomValues::join([&one, &two]),
            Some(CustomValues::new(vec![Millimetres(1.0), Millimetres(2.0)]))
        );
        assert_eq!(CustomValues::join([&one, &AxisValue::Number(2.0)]), None);
        assert_eq!(CustomValues::join([&one, &Dropped(2.0).into()]), None);
        assert_eq!(CustomValues::join([]), None);
    }

    #[test]
    fn converters_compare_by_type() {
        let converter = CustomConverter::new(MillimetreConverter);
        assert_eq!(
            converter,
            CustomValues::new(vec![Millimetres(1.0)]).converter()
        );
        assert!(converter.type_name().ends_with("MillimetreConverter"));
        assert!(format!("{converter:?}").contains("MillimetreConverter"));
    }

    /// Numbers whose converter loses the first one.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Dropped(f64);

    struct Lossy;

    impl ConversionInterface for Lossy {
        type Value = Dropped;
        type Unit = ();

        fn convert(
            &self,
            values: &[Dropped],
            _: Option<&()>,
            _: &Axis,
        ) -> Result<Vec<f64>, ConversionError> {
            Ok(values.iter().skip(1).map(|v| v.0).collect())
        }
    }

    impl Convertible for Dropped {
        type Converter = Lossy;
        fn converter() -> Lossy {
            Lossy
        }
    }

    #[test]
    fn values_keep_to_their_own_converter_and_count() {
        let ax = Figure::new().subplot();
        let values = CustomValues::new(vec![Millimetres(3.0)]);
        let other = CustomConverter::new(Lossy);
        assert_eq!(
            values.convert(Some(&other), None, &ax.xaxis()),
            Ok(vec![3.0])
        );
        assert_eq!(values.default_units(&other, &ax.xaxis()), None);
        let dropped = CustomValues::new(vec![Dropped(1.0), Dropped(2.0)]);
        assert_eq!(
            dropped.convert(None, None, &ax.xaxis()),
            Err(ConversionError::Count {
                values: 2,
                numbers: 1
            })
        );
    }
}
