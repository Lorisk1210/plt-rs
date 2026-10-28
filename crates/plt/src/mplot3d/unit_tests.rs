//! Units on 3-D axes: dates, category names and user values plotted by
//! the 3-D plotting methods, converted on the axis each lies along, the
//! tick helpers and labels they bring, shared with the axes sharing an
//! axis, and the lines and texts converting their user values again when
//! an axis changes unit.

use std::rc::Rc;

use crate::dates::{DateTime, TimeZone};
use crate::figure::Figure;
use crate::mplot3d::{Axes3DError, Axis3, Scatter3DOptions};
use crate::ticker::MultipleLocator;
use crate::units::{
    ConversionError, ConversionInterface, Convertible, Unit, UnitAxis, UnitAxisInfo, UnitConverter,
};

/// A length, which keeps the unit it was given in.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Length {
    metres: f64,
    unit: LengthUnit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum LengthUnit {
    Metres,
    Kilometres,
}

impl LengthUnit {
    fn metres(self) -> f64 {
        match self {
            Self::Metres => 1.0,
            Self::Kilometres => 1000.0,
        }
    }
}

fn km(value: f64) -> Length {
    Length {
        metres: value * 1000.0,
        unit: LengthUnit::Kilometres,
    }
}

/// Converts lengths on 3-D axes only.
struct LengthConverter;

impl ConversionInterface for LengthConverter {
    type Value = Length;
    type Unit = LengthUnit;

    fn convert(
        &self,
        values: &[Length],
        unit: Option<&LengthUnit>,
        axis: &UnitAxis,
    ) -> Result<Vec<f64>, ConversionError> {
        if !matches!(axis, UnitAxis::Axes3D(_)) {
            return Err(ConversionError::Failed("3-D axes only".into()));
        }
        let unit = unit.copied().unwrap_or(LengthUnit::Metres);
        Ok(values.iter().map(|l| l.metres / unit.metres()).collect())
    }

    fn axisinfo(&self, unit: Option<&LengthUnit>, _: &UnitAxis) -> Option<UnitAxisInfo> {
        let locator = MultipleLocator::new(0.5).expect("a positive base");
        Some(UnitAxisInfo {
            majloc: Some(Rc::new(locator)),
            label: Some(format!("{:?}", unit?)),
            ..UnitAxisInfo::default()
        })
    }

    fn default_units(&self, values: &[Length], _: &UnitAxis) -> Option<LengthUnit> {
        values.first().map(|l| l.unit)
    }
}

impl Convertible for Length {
    type Converter = LengthConverter;
    fn converter() -> LengthConverter {
        LengthConverter
    }
}

fn day(d: u32) -> DateTime {
    DateTime::new(2020, 1, d, 0, 0, 0, 0).expect("a date")
}

#[test]
fn dates_and_names_tick_the_axes_they_lie_along() {
    let ax = Figure::new().subplot_3d();
    ax.plot(
        ["a", "b", "c"],
        [0.0, 1.0, 2.0],
        vec![day(1), day(2), day(5)],
    )
    .unwrap();
    assert_eq!(ax.xaxis().converter(), Some(UnitConverter::Categories));
    assert_eq!(ax.zaxis().converter(), Some(UnitConverter::Dates));
    assert!(!ax.yaxis().have_units(), "numbers leave an axis alone");
    assert_eq!(ax.xaxis().ticklabels(false).unwrap(), ["a", "b", "c"]);
    let (z0, z1) = ax.zlim();
    assert!(z0 < 18_262.0 && z1 > 18_266.0, "{z0} {z1}");
    // Placed in the plane normal to x, the 2-D x values lie along y.
    let options = Scatter3DOptions {
        zdir: Axis3::X,
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(["d"], [0.5], [1.0], options).unwrap();
    assert_eq!(ax.yaxis().categories().unwrap().names(), ["d"]);
    assert_eq!(ax.xaxis().categories().unwrap().names(), ["a", "b", "c"]);
}

#[test]
fn user_values_bring_their_unit_and_convert_again() {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    let line = ax
        .plot([0.0, 1.0], [0.0, 1.0], vec![km(1.0), km(2.0)])
        .unwrap();
    let mark = ax.text(0.0, 0.0, km(1.5), "mid", None);
    assert_eq!(ax.zaxis().units(), Some(Unit::new(LengthUnit::Kilometres)));
    assert_eq!(ax.label(Axis3::Z), "Kilometres");
    assert_eq!(ax.line_data(&line).unwrap()[2], [1.0, 2.0]);
    ax.zaxis().set_units(LengthUnit::Metres);
    assert_eq!(ax.line_data(&line).unwrap()[2], [1000.0, 2000.0]);
    assert_eq!(ax.text_position(&mark).unwrap()[2], 1500.0);
    let (z0, z1) = ax.zlim();
    assert!(z0 < 1000.0 && z0 > 900.0 && z1 > 2000.0, "{z0} {z1}");
    assert_eq!(ax.label(Axis3::Z), "Metres");
    ax.set_zlabel("height");
    ax.zaxis().set_units(LengthUnit::Kilometres);
    assert_eq!(ax.label(Axis3::Z), "height", "a label set by hand stays");
    ax.set_line_data(&line, [0.0, 1.0], [0.0, 1.0], [5.0, 6.0])
        .unwrap();
    ax.zaxis().set_units(LengthUnit::Metres);
    assert_eq!(ax.line_data(&line).unwrap()[2], [5.0, 6.0], "numbers stay");
    assert!(fig.to_png().is_ok());
}

#[test]
fn axes_sharing_an_axis_share_its_names_and_unit() {
    let fig = Figure::new();
    let a = fig.subplot_3d();
    let b = fig.subplot_3d();
    b.share_x(&a).unwrap();
    a.scatter(["p", "q"], [0.0, 1.0], [0.0]).unwrap();
    let line = b.plot(["q", "r"], [0.0, 1.0], [0.0]).unwrap();
    assert_eq!(b.line_data(&line).unwrap()[0], [1.0, 2.0]);
    assert_eq!(a.xaxis().categories().unwrap().names(), ["p", "q", "r"]);
    let tall = b.plot([0.0], [0.0], vec![km(3.0)]).unwrap();
    b.share_z(&a).unwrap();
    a.zaxis().set_units(LengthUnit::Metres);
    assert_eq!(b.line_data(&tall).unwrap()[2], [3000.0]);
}

#[test]
fn explicit_converters_stay() {
    let ax = Figure::new().subplot_3d();
    ax.zaxis().set_converter(UnitConverter::Categories).unwrap();
    assert_eq!(
        ax.zaxis().set_converter(UnitConverter::Dates).err(),
        Some(Axes3DError::ExplicitConverter(Axis3::Z))
    );
    ax.zaxis().axis_date(&TimeZone::Utc);
    assert_eq!(ax.zaxis().converter(), Some(UnitConverter::Categories));
    let numbers = ax.zaxis().convert_units(vec![day(1)]);
    assert_eq!(numbers, [18_262.0], "values convert as their own kind");
    ax.yaxis().axis_date(&TimeZone::Utc);
    assert_eq!(ax.yaxis().converter(), Some(UnitConverter::Dates));
    assert_eq!(
        ax.ylim(),
        Figure::new().subplot_3d().ylim(),
        "the view stays"
    );
}

#[test]
fn texts_convert_without_changing_the_axes() {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    let label = ax.text("a", 0.0, 0.0, "nowhere", None);
    assert!(ax.text_position(&label).unwrap()[0].is_nan());
    assert!(ax.xaxis().categories().is_none(), "nothing registers");
    let problems = fig.to_png().expect_err("the name cannot be placed");
    assert!(problems.to_string().contains("\"a\" is a category name"));
    ax.plot(["a", "b"], [0.0, 1.0], [0.0]).unwrap();
    let placed = ax.text("b", 0.0, day(1), "there", None);
    assert_eq!(ax.text_position(&placed).unwrap(), [1.0, 0.0, 18_262.0]);
    assert!(!ax.zaxis().have_units(), "texts leave the axes alone");
}

#[test]
fn limits_take_plotting_values() {
    let ax = Figure::new().subplot_3d();
    ax.set_xlim("a", "c").unwrap();
    assert_eq!(ax.xlim(), (0.0, 1.0), "two names, at 0 and 1");
    assert_eq!(ax.xaxis().categories().unwrap().names(), ["a", "c"]);
    ax.set_ylim(day(1), day(2)).unwrap();
    assert_eq!(ax.yaxis().converter(), Some(UnitConverter::Dates));
}
