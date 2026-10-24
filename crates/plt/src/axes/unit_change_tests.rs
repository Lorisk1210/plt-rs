//! Which artists follow a change of axis unit, as matplotlib's do: lines,
//! rectangles made of plotting values, texts and annotations convert their
//! user values again; spans, bars, fills, scatters and the bars of error
//! bars keep the numbers they converted to. Also the units of shared axes,
//! and the axes that texts and date rectangles leave alone.

use std::any::Any;

use super::{Axis, BarOptions, ErrorbarOptions, FillBetweenOptions};
use crate::dates::{AutoDateFormatter, DateTime, TimeDelta};
use crate::figure::{Figure, Share};
use crate::lines::Line2D;
use crate::patches::{DataRectangle, Rectangle};
use crate::text::AnnotateOptions;
use crate::units::{
    AxisDelta, AxisDeltas, AxisValue, ConversionError, ConversionInterface, Convertible,
    UnitConverter,
};

/// A length, which keeps the unit it was given in.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Length {
    metres: f64,
    unit: LengthUnit,
}

/// The units of lengths and of axes of lengths.
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

/// Lengths in the axis unit, in metres without one; the unit of the first
/// value is the default.
struct LengthConverter;

impl ConversionInterface for LengthConverter {
    type Value = Length;
    type Unit = LengthUnit;

    fn convert(
        &self,
        values: &[Length],
        unit: Option<&LengthUnit>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        let per = unit.map_or(1.0, |unit| unit.metres());
        Ok(values.iter().map(|length| length.metres / per).collect())
    }

    fn default_units(&self, values: &[Length], _: &Axis) -> Option<LengthUnit> {
        values.first().map(|length| length.unit)
    }
}

impl Convertible for Length {
    type Converter = LengthConverter;

    fn converter() -> LengthConverter {
        LengthConverter
    }

    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        let delta = delta.downcast_ref::<Self>()?;
        Some(
            Self {
                metres: self.metres + delta.metres,
                unit: self.unit,
            }
            .into(),
        )
    }

    fn scale(&self, factor: i64) -> Option<Self> {
        Some(Self {
            metres: self.metres * factor as f64,
            unit: self.unit,
        })
    }
}

#[test]
fn rectangles_texts_and_annotations_follow_a_new_unit() {
    let ax = Figure::new().subplot();
    let line = ax
        .plot(vec![km(0.0), km(4.0)], vec![km(0.0), km(2.0)])
        .unwrap();
    let rectangle = ax.add_patch(DataRectangle::new((km(1.0), km(0.5)), km(2.0), km(1.0)).unwrap());
    let text = ax.text(km(3.0), km(1.5), "text");
    let options = AnnotateOptions {
        xytext: Some((km(2.0), km(0.25)).into()),
        ..AnnotateOptions::default()
    };
    let note = ax.annotate("note", (km(1.5), km(1.0)), options).unwrap();
    ax.xaxis().set_units(LengthUnit::Metres);
    assert_eq!(line.xdata(), [0.0, 4000.0]);
    assert_eq!(
        rectangle.shape(),
        Rectangle::new((1000.0, 0.5), 2000.0, 1.0),
        "only the x of the rectangle converts again"
    );
    assert_eq!((text.position().x, text.position().y), (3000.0, 1.5));
    assert_eq!((note.xy().x, note.position().x), (1500.0, 2000.0));
    assert_eq!(ax.data_lim().bbox.x1(), 4000.0, "the limits follow");
    ax.yaxis().set_units(LengthUnit::Metres);
    assert_eq!(rectangle.shape().height, 1000.0);
    assert_eq!((note.xy().y, note.position().y), (1000.0, 250.0));
}

#[test]
fn spans_bars_fills_and_scatters_keep_their_numbers() {
    let ax = Figure::new().subplot();
    ax.plot(vec![km(0.0), km(4.0)], [0.0, 1.0]).unwrap();
    let span = ax.axvspan(km(1.0), km(2.0));
    let bars = ax.bar(vec![km(3.0)], [1.0], BarOptions::default()).unwrap();
    let fill = ax
        .fill_between(
            vec![km(0.0), km(1.0)],
            [0.0],
            [1.0],
            FillBetweenOptions::default(),
        )
        .unwrap();
    let points = ax.scatter(vec![km(2.0)], [0.5]).unwrap();
    let before = (span.shape(), fill.paths(), points.offsets());
    ax.xaxis().set_units(LengthUnit::Metres);
    assert_eq!((span.shape(), fill.paths(), points.offsets()), before);
    assert_eq!(bars.position_centers(), [3.0], "matplotlib converts once");
}

#[test]
fn values_placed_before_a_unit_follow_it() {
    let ax = Figure::new().subplot();
    let text = ax.text(km(2.0), 0.5, "early");
    assert_eq!(text.position().x, 2000.0, "no unit yet: metres");
    assert!(!ax.xaxis().have_units(), "texts leave the axis alone");
    ax.plot(vec![km(0.0), km(4.0)], [0.0, 1.0]).unwrap();
    assert_eq!(text.position().x, 2.0, "the unit of the data, kilometres");
}

#[test]
fn replaced_numbers_no_longer_follow() {
    let ax = Figure::new().subplot();
    ax.plot(vec![km(0.0), km(4.0)], vec![km(0.0), km(4.0)])
        .unwrap();
    let text = ax.text(km(1.0), km(1.0), "moved");
    text.set_x(0.5);
    let note = ax
        .annotate("moved", (km(2.0), km(2.0)), AnnotateOptions::default())
        .unwrap();
    note.set_xy((0.25, 0.25));
    let rectangle = ax.add_patch(DataRectangle::new((km(1.0), km(1.0)), km(1.0), km(1.0)).unwrap());
    rectangle.update_shape(|shape| shape.angle = 10.0);
    ax.xaxis().set_units(LengthUnit::Metres);
    ax.yaxis().set_units(LengthUnit::Metres);
    assert_eq!((text.position().x, text.position().y), (0.5, 1000.0));
    assert_eq!((note.xy().x, note.position().x), (0.25, 2000.0));
    assert_eq!(rectangle.shape().xy.x, 1.0);
}

#[test]
fn error_bar_lines_and_caps_follow_and_the_bars_do_not() {
    let ax = Figure::new().subplot();
    let errors = ErrorbarOptions {
        yerr: Some(AxisDeltas::from(vec![km(0.5)]).into()),
        capsize: Some(3.0),
        ..ErrorbarOptions::default()
    };
    let container = ax.errorbar([1.0], vec![km(2.0)], errors).unwrap();
    let bar = || container.barlinecols[0].paths()[0].vertices().to_vec();
    assert_eq!((bar()[0].y, bar()[1].y), (1.5, 2.5));
    ax.yaxis().set_units(LengthUnit::Metres);
    let line = container.data_line.as_ref().unwrap();
    assert_eq!(line.ydata(), [2000.0]);
    let caps: Vec<Vec<f64>> = container.caplines.iter().map(Line2D::ydata).collect();
    assert_eq!(caps, [[1500.0], [2500.0]]);
    assert_eq!((bar()[0].y, bar()[1].y), (1.5, 2.5), "converted once");
}

#[test]
fn axes_sharing_an_axis_follow_its_unit() {
    let fig = Figure::new();
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None).unwrap();
    let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
    top.plot(vec![km(0.0), km(4.0)], [0.0, 1.0]).unwrap();
    let rectangle = bottom.add_patch(DataRectangle::new((km(1.0), 0.0), km(1.0), 1.0).unwrap());
    let text = bottom.text(km(3.0), 0.5, "shared");
    let twin = bottom.twinx();
    let line = twin.plot(vec![km(2.0)], [1.0]).unwrap();
    top.xaxis().set_units(LengthUnit::Metres);
    assert_eq!(rectangle.shape().xy.x, 1000.0);
    assert_eq!(text.position().x, 3000.0);
    assert_eq!(line.xdata(), [2000.0], "twins share x too");
}

#[test]
fn date_rectangles_and_texts_leave_the_date_ticks_alone() {
    let ax = Figure::new().subplot();
    let start = DateTime::new(2020, 1, 1, 0, 0, 0, 0).unwrap();
    let day = TimeDelta::new(1, 0, 0).unwrap();
    ax.add_patch(DataRectangle::new((start, 0.0), day, 1.0).unwrap());
    let xaxis = ax.xaxis();
    assert_eq!(xaxis.converter(), Some(UnitConverter::Dates));
    let formatter = xaxis.major_formatter();
    assert!(
        !(formatter.as_ref() as &dyn Any).is::<AutoDateFormatter>(),
        "matplotlib's Axis.convert_units sets the converter alone"
    );
    ax.text(0.5, start, "a date");
    assert!(!ax.yaxis().have_units(), "texts convert when drawn");
}
