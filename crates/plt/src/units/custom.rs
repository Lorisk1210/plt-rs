//! User values, units and converters with their types erased, so that
//! axes and lines can hold them (matplotlib's duck-typed unit objects).

use std::any::{Any, TypeId, type_name};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::interface::UnitOf;
use super::{AxisInfo, AxisValue, AxisValues, ConversionError, ConversionInterface, Convertible};
use crate::axes::Axis;
use crate::ticker::AxisName;

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
    fn axisinfo(&self, unit: Option<&Unit>, axis: &Axis) -> Option<AxisInfo>;

    /// The type of the converter.
    fn type_name(&self) -> &'static str;
}

impl<C: ConversionInterface> AnyConverter for C {
    fn axisinfo(&self, unit: Option<&Unit>, axis: &Axis) -> Option<AxisInfo> {
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
    pub(crate) fn axisinfo(&self, unit: Option<&Unit>, axis: &Axis) -> Option<AxisInfo> {
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
/// the type, as [`AxisValues::Custom`] holds them.
///
/// ```
/// # use plt::axes::Axis;
/// # use plt::units::{ConversionError, ConversionInterface, Convertible};
/// use plt::units::UnitValues;
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
/// let values = UnitValues::new(vec![Miles(1.0), Miles(2.5)]);
/// assert_eq!(values.len(), 2);
/// assert_eq!(values.downcast_ref::<Miles>(), Some(&[Miles(1.0), Miles(2.5)][..]));
/// ```
#[derive(Clone)]
pub struct UnitValues(Rc<dyn AnyValues>);

/// What [`UnitValues`] needs of the values it holds.
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
}

impl UnitValues {
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
}

impl PartialEq for UnitValues {
    fn eq(&self, other: &Self) -> bool {
        self.0.equals(other.0.as_ref())
    }
}

impl fmt::Debug for UnitValues {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Convertible> From<Vec<T>> for UnitValues {
    fn from(values: Vec<T>) -> Self {
        Self::new(values)
    }
}

/// The plotting values of a line that came from a user converter, kept so
/// that the line converts them again when its axis changes unit
/// (matplotlib's `Line2D._xorig` and `_yorig`).
#[derive(Clone, Debug, Default)]
pub(crate) struct LineSources {
    /// The x values.
    pub(crate) x: Option<UnitValues>,
    /// The y values.
    pub(crate) y: Option<UnitValues>,
}

impl LineSources {
    /// The user values among `x` and `y`.
    pub(crate) fn of(x: &AxisValues, y: &AxisValues) -> Self {
        Self {
            x: x.custom().cloned(),
            y: y.custom().cloned(),
        }
    }

    /// The user values along axis `name`, if any.
    pub(crate) fn along(&self, name: AxisName) -> Option<&UnitValues> {
        match name {
            AxisName::X => self.x.as_ref(),
            AxisName::Y => self.y.as_ref(),
        }
    }

    /// `value` along axis `name`, if it is a user value: the source of
    /// the line of `axhline` or `axvline`.
    pub(crate) fn of_value(name: AxisName, value: &AxisValue) -> Self {
        let source = match value {
            AxisValue::Custom(value) => Some(value.clone()),
            _ => None,
        };
        match name {
            AxisName::X => Self { x: source, y: None },
            AxisName::Y => Self { x: None, y: source },
        }
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
        let values = UnitValues::new(vec![Millimetres(20.0), Millimetres(5.0)]);
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
            UnitValues::from(vec![Millimetres(20.0), Millimetres(5.0)])
        );
        assert!(UnitValues::new(Vec::<Millimetres>::new()).is_empty());
    }

    #[test]
    fn converters_compare_by_type() {
        let converter = CustomConverter::new(MillimetreConverter);
        assert_eq!(
            converter,
            UnitValues::new(vec![Millimetres(1.0)]).converter()
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
        let values = UnitValues::new(vec![Millimetres(3.0)]);
        let other = CustomConverter::new(Lossy);
        assert_eq!(
            values.convert(Some(&other), None, &ax.xaxis()),
            Ok(vec![3.0])
        );
        assert_eq!(values.default_units(&other, &ax.xaxis()), None);
        let dropped = UnitValues::new(vec![Dropped(1.0), Dropped(2.0)]);
        assert_eq!(
            dropped.convert(None, None, &ax.xaxis()),
            Err(ConversionError::Count {
                values: 2,
                numbers: 1
            })
        );
    }
}
