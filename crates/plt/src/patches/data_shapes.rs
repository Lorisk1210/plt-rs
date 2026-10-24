//! Patch shapes given in plotting values before unit conversion, which an
//! axes converts to its numbers when the patch is added (matplotlib's
//! patches made of values with units, which convert them through their
//! axes: `Rectangle._convert_units`).

use super::{PatchShape, Rectangle, ShapeError};
use crate::axes::Axes;
use crate::ticker::AxisName;
use crate::units::{AxisDelta, DataPoint};

/// What [`Axes::add_patch`] adds: a [`PatchShape`] in axis numbers, which
/// it adds as it is, or a shape given in plotting values, such as a
/// [`DataRectangle`], which the axes converts first.
pub trait IntoPatchShape {
    /// The shape the patch draws.
    type Shape: PatchShape;

    /// The shape in the axis numbers of `axes`.
    fn into_shape(self, axes: &Axes) -> Self::Shape;
}

impl<S: PatchShape> IntoPatchShape for S {
    type Shape = S;

    fn into_shape(self, _: &Axes) -> S {
        self
    }
}

/// A rectangle given in plotting values before unit conversion: a corner
/// of numbers, dates, category names or user values, and a width and a
/// height, extents from it (matplotlib's `Rectangle` made of values with
/// units). [`Axes::add_patch`] adds it as the [`Rectangle`] between its
/// corner and its far corner, each converted as artists convert their
/// values, which gives an axis without a converter one but leaves its
/// unit alone (matplotlib's `Rectangle._convert_units`).
///
/// ```
/// use plt::dates::{DateTime, TimeDelta};
/// use plt::patches::DataRectangle;
/// use plt::prelude::*;
/// let ax = Figure::new().subplot();
/// let start = DateTime::new(1970, 1, 3, 0, 0, 0, 0)?;
/// let week = DataRectangle::new((start, 1.0), TimeDelta::new(7, 0, 0)?, 2.0)?;
/// let rectangle = ax.add_patch(week).shape();
/// assert_eq!((rectangle.xy.x, rectangle.width, rectangle.height), (2.0, 7.0, 2.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DataRectangle {
    /// The corner the sides extend from.
    corner: DataPoint,
    /// The corner the width and the height reach.
    far_corner: DataPoint,
}

impl DataRectangle {
    /// The rectangle from `xy` spanning `width` along x and `height`
    /// along y.
    ///
    /// # Errors
    ///
    /// [`ShapeError::UnaddableSide`] when the width or the height does not
    /// add to its coordinate of the corner (a user value's
    /// [`Convertible::add_delta`](crate::units::Convertible::add_delta)
    /// refusing it, or a duration from a number), where matplotlib fails
    /// with a `TypeError`.
    pub fn new(
        xy: impl Into<DataPoint>,
        width: impl Into<AxisDelta>,
        height: impl Into<AxisDelta>,
    ) -> Result<Self, ShapeError> {
        let corner = xy.into();
        let far_corner = DataPoint {
            x: corner
                .x
                .add_delta(&width.into())
                .ok_or(ShapeError::UnaddableSide(AxisName::X))?,
            y: corner
                .y
                .add_delta(&height.into())
                .ok_or(ShapeError::UnaddableSide(AxisName::Y))?,
        };
        Ok(Self { corner, far_corner })
    }
}

impl IntoPatchShape for DataRectangle {
    type Shape = Rectangle;

    fn into_shape(self, axes: &Axes) -> Rectangle {
        let (xaxis, yaxis) = (axes.xaxis(), axes.yaxis());
        let x0 = xaxis.convert_without_update(&self.corner.x);
        let y0 = yaxis.convert_without_update(&self.corner.y);
        let x1 = xaxis.convert_without_update(&self.far_corner.x);
        let y1 = yaxis.convert_without_update(&self.far_corner.y);
        Rectangle::new((x0, y0), x1 - x0, y1 - y0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::Axis;
    use crate::dates::TimeDelta;
    use crate::figure::Figure;
    use crate::units::{AxisValue, ConversionError, ConversionInterface, Convertible, Unit};

    /// A distance in kilometres, which adds to distances.
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Km(f64);

    /// Kilometres in metres when the axis unit says so.
    struct KmConverter;

    impl ConversionInterface for KmConverter {
        type Value = Km;
        type Unit = &'static str;

        fn convert(
            &self,
            values: &[Km],
            unit: Option<&&'static str>,
            _: &Axis,
        ) -> Result<Vec<f64>, ConversionError> {
            let scale = if unit == Some(&"m") { 1000.0 } else { 1.0 };
            Ok(values.iter().map(|km| km.0 * scale).collect())
        }

        fn default_units(&self, _: &[Km], _: &Axis) -> Option<&'static str> {
            Some("km")
        }
    }

    impl Convertible for Km {
        type Converter = KmConverter;
        fn converter() -> KmConverter {
            KmConverter
        }

        fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
            delta
                .downcast_ref::<Self>()
                .map(|km| Km(self.0 + km.0).into())
        }
    }

    #[test]
    fn corners_convert_without_giving_the_axes_units() {
        let ax = Figure::new().subplot();
        let rectangle = DataRectangle::new((Km(5.0), Km(6.0)), Km(1.0), Km(2.0)).unwrap();
        let patch = ax.add_patch(rectangle);
        assert_eq!(patch.shape(), Rectangle::new((5.0, 6.0), 1.0, 2.0));
        assert!(
            ax.xaxis().converter().is_some(),
            "the converter of kilometres"
        );
        assert_eq!(ax.xaxis().units(), None, "but no unit");
        ax.yaxis().set_units("m");
        let metres =
            ax.add_patch(DataRectangle::new((Km(0.0), Km(1.0)), Km(1.0), Km(1.0)).unwrap());
        assert_eq!(
            (metres.shape().xy.y, metres.shape().height),
            (1000.0, 1000.0)
        );
        assert_eq!(ax.yaxis().units(), Some(Unit::new("m")));
    }

    #[test]
    fn sides_must_add_to_the_corner() {
        let day = TimeDelta::new(1, 0, 0).unwrap();
        assert_eq!(
            DataRectangle::new((0.0, 0.0), day, 1.0),
            Err(ShapeError::UnaddableSide(AxisName::X))
        );
        assert_eq!(
            DataRectangle::new((Km(0.0), Km(0.0)), Km(1.0), 1.0),
            Err(ShapeError::UnaddableSide(AxisName::Y))
        );
        let numbers = DataRectangle::new((1.0, 2.0), 3.0, 4.0).unwrap();
        let ax = Figure::new().subplot();
        assert_eq!(
            ax.add_patch(numbers).shape(),
            Rectangle::new((1.0, 2.0), 3.0, 4.0)
        );
    }
}
