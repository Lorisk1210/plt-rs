//! Extents along a 2-D axis given in plotting values (bar heights and
//! widths, error sizes, the sides of rectangles) converted to axis
//! numbers through the values they extend from, and the positions of
//! texts and annotations (matplotlib's `Axes._convert_dx` and
//! `Rectangle._convert_units`; the conversions are
//! [`UnitHandle`]'s).

use crate::dates::{DateTime, TimeDelta};
use crate::figure::Figure;
use crate::units::{
    AxisDelta, AxisDeltas, AxisValue, AxisValues, ConversionError, ConversionInterface,
    Convertible, UnitAxis, UnitHandle,
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
        _: &UnitAxis,
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
        _: &UnitAxis,
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

#[test]
fn category_positions_need_an_axis_with_categories() {
    use crate::text::AnnotateOptions;

    let fig = Figure::new();
    let ax = fig.subplot();
    ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a valid line");
    let text = ax.text("a", 0.5, "hello");
    let note = ax
        .annotate("note", ("b", 0.5), AnnotateOptions::default())
        .expect("data coordinates");
    assert!(text.position().x.is_nan() && note.xy().x.is_nan());
    assert_eq!(ax.xaxis().categories(), None, "nothing registers");
    let problems = fig.to_png().expect_err("the names cannot be placed");
    let message = problems.to_string();
    assert!(message.contains("\"a\" is a category name") && message.contains("\"b\""));
    assert!(fig.to_png().is_ok(), "reported once, as other conversions");

    let ax = Figure::new().subplot();
    ax.plot(vec!["x", "y"], [0.0, 1.0]).expect("a valid line");
    let text = ax.text(0.5, 0.25, "numbers");
    let named = ax.text("z", 0.5, "joins");
    assert_eq!((text.position().x, named.position().x), (0.5, 2.0));
    let names = ax.xaxis().categories().expect("categories");
    assert_eq!(names.names(), ["x", "y", "z"], "matplotlib's unit.update");
}
