//! `figures/cases_units.py`: artists with values in units when the units
//! change (rectangles, texts and annotations convert theirs again; spans,
//! bars, fills and scatters keep the numbers they converted to), error
//! bars computed in the units of the data, date rectangles and texts that
//! leave the date ticks alone, and units set on shared axes. The unit
//! classes are in `crate::user_units`.

use pltrs::axes::{BarOptions, ErrorbarOptions, FillBetweenOptions};
use pltrs::collections::CollectionProps;
use pltrs::dates::{DateTime, TimeDelta};
use pltrs::figure::{Figure, Share};
use pltrs::patches::DataRectangle;
use pltrs::prelude::*;
use pltrs::text::{AnnotateOptions, ArrowProps};
use pltrs::units::AxisDeltas;

use crate::figures::{CaseResult, FigureCase};
use crate::user_units::jpl::{UnitDbl, UnitName};
use crate::user_units::temperature::{Temperature, TemperatureUnit};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        FigureCase::new("units/rectangle_set_units", rectangle_set_units),
        FigureCase::new("units/texts_set_units", texts_set_units),
        FigureCase::new("units/date_rectangle_ticks", date_rectangle_ticks),
        FigureCase::new("units/errorbar_temperatures", errorbar_temperatures),
        FigureCase::new("units/shared_set_units", shared_set_units),
    ]
}

/// One `jpl_units` kilometre.
fn km() -> UnitDbl {
    UnitDbl::new(1.0, UnitName::Kilometres)
}

/// The arrow `{"arrowstyle": "->"}`.
fn arrow() -> Result<Option<ArrowProps>, Box<dyn std::error::Error>> {
    Ok(Some("->".parse()?))
}

/// A line, a rectangle, a span, a bar and a fill in kilometres, then the
/// axes in miles and metres: only the line and the rectangle convert
/// again.
fn rectangle_set_units(fig: &Figure) -> CaseResult {
    let km = km();
    let ax = fig.subplot();
    let black = LineProps::default().color("k");
    ax.plot_with(vec![0.0 * km, 10.0 * km], vec![0.0 * km, 6.0 * km], black)?;
    let rectangle = DataRectangle::new((2.0 * km, 1.0 * km), 3.0 * km, 2.0 * km)?;
    ax.add_patch_with(rectangle, PatchProps::default().alpha(0.5));
    ax.axvspan(6.0 * km, 7.0 * km)
        .set_color("C1")
        .set_alpha(Some(0.3));
    let bar = BarOptions {
        thickness: vec![1.0 * km].into(),
        color: vec!["C2".into()],
        ..BarOptions::default()
    };
    ax.bar(vec![8.0 * km], vec![2.0 * km], bar)?;
    let fill = FillBetweenOptions {
        props: CollectionProps::default().color("C3"),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(
        vec![0.0 * km, 2.0 * km],
        vec![4.0 * km, 5.0 * km],
        vec![5.0 * km, 6.0 * km],
        fill,
    )?;
    ax.xaxis().set_units(UnitName::Miles);
    ax.yaxis().set_units(UnitName::Metres);
    Ok(())
}

/// Texts, annotations and a scatter in kilometres, then the axes in miles
/// and metres: the texts and annotations convert again, the scatter does
/// not.
fn texts_set_units(fig: &Figure) -> CaseResult {
    let km = km();
    let ax = fig.subplot();
    ax.text(1.0 * km, 5.0 * km, "before the data");
    let black = LineProps::default().color("k");
    ax.plot_with(vec![0.0 * km, 10.0 * km], vec![0.0 * km, 6.0 * km], black)?;
    ax.text(2.0 * km, 4.0 * km, "text");
    let in_data = AnnotateOptions {
        xytext: Some((7.0 * km, 1.0 * km).into()),
        arrowprops: arrow()?,
        ..AnnotateOptions::default()
    };
    ax.annotate("data", (5.0 * km, 3.0 * km), in_data)?;
    let offset = AnnotateOptions {
        xytext: Some((10.0, -20.0).into()),
        textcoords: Some("offset points".parse()?),
        arrowprops: arrow()?,
        ..AnnotateOptions::default()
    };
    ax.annotate("offset", (3.0 * km, 2.0 * km), offset)?;
    ax.scatter(vec![1.0 * km, 4.0 * km], vec![1.0 * km, 5.0 * km])?;
    ax.xaxis().set_units(UnitName::Miles);
    ax.yaxis().set_units(UnitName::Metres);
    Ok(())
}

/// Rectangles from dates with durations as widths, and a text and an
/// annotation at dates, on axes with numeric limits, which keep their
/// numeric ticks.
fn date_rectangle_ticks(fig: &Figure) -> CaseResult {
    let date = |day| DateTime::new(2020, 1, day, 0, 0, 0, 0);
    let axs = fig.subplots(2, 1)?;
    let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
    top.add_patch(DataRectangle::new(
        (date(1)?, 0.0),
        TimeDelta::new(2, 0, 0)?,
        1.0,
    )?);
    let half_day = TimeDelta::new(0, 43_200, 0)?;
    let orange = PatchProps::default().color("C1");
    top.add_patch_with(DataRectangle::new((date(4)?, 0.5), half_day, 0.25)?, orange);
    top.set_xlim(18_261.0, 18_266.0)?.set_ylim(0.0, 1.5)?;
    bottom.set_xlim(18_261.0, 18_266.0)?;
    bottom.text(date(2)?, 0.5, "a date");
    let note = AnnotateOptions {
        xytext: Some((date(3)?, 0.75).into()),
        arrowprops: arrow()?,
        ..AnnotateOptions::default()
    };
    bottom.annotate("a point", (date(4)?, 0.25), note)?;
    Ok(())
}

/// Error bars of temperatures on an axis in degrees Fahrenheit, and on
/// one in degrees Celsius that then turns to kelvin.
fn errorbar_temperatures(fig: &Figure) -> CaseResult {
    let x = [0.0, 1.0, 2.0, 3.0];
    let y = [20.0, 25.0, 30.0, 35.0].map(Temperature).to_vec();
    let axs = fig.subplots(1, 2)?;
    let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
    left.yaxis().set_units(TemperatureUnit::Fahrenheit);
    let five = ErrorbarOptions {
        yerr: Some(AxisDeltas::from(vec![Temperature(5.0)]).into()),
        capsize: Some(4.0),
        fmt: "o".parse()?,
        ..ErrorbarOptions::default()
    };
    left.errorbar(x, y.clone(), five)?;
    let growing = ErrorbarOptions {
        yerr: Some(AxisDeltas::from([1.0, 2.0, 3.0, 4.0].map(Temperature).to_vec()).into()),
        xerr: Some(0.2.into()),
        capsize: Some(4.0),
        fmt: "s".parse()?,
        ..ErrorbarOptions::default()
    };
    right.errorbar(x, y, growing)?;
    right.yaxis().set_units(TemperatureUnit::Kelvin);
    Ok(())
}

/// A line in kilometres on the upper of two axes sharing x, a rectangle,
/// a text and a twin's line on the lower one, then the upper x axis in
/// miles, which all of them follow.
fn shared_set_units(fig: &Figure) -> CaseResult {
    let km = km();
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
    top.plot(vec![0.0 * km, 10.0 * km], [0.0, 1.0])?;
    bottom.add_patch(DataRectangle::new((2.0 * km, 0.2), 3.0 * km, 0.5)?);
    bottom.text(6.0 * km, 0.5, "shared");
    let twin = bottom.twinx()?;
    let orange = LineProps::default().color("C1");
    twin.plot_with(vec![1.0 * km, 9.0 * km], [5.0, 3.0], orange)?;
    bottom.set_ylim(0.0, 1.0)?;
    top.xaxis().set_units(UnitName::Miles);
    Ok(())
}
