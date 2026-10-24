//! The field, image and statistics cases: `test_barbs`, `test_contour`,
//! `test_contourf`, `test_clabel`, `test_imshow`, `test_matshow`,
//! `test_bxp` and `test_violin` (vertical and horizontal).

use plt::axes::{
    Axes, Axis, BxpOptions, ImshowOptions, TickLabelProps, TickLevel, TickParams, ViolinDrawOptions,
};
use plt::contour::{ClabelOptions, ContourOptions, QuadCoords, QuadField};
use plt::dates::{DateError, DateFormatter, DateTime, TimeDelta};
use plt::image::Extent;
use plt::numeric::{Grid, linspace};
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::quiver::{BarbsOptions, VectorField};
use plt::stats::{BoxStats, ViolinStats};

use super::{axes, concise, constrained, daily, datetime_case, day, hour, numbers};
use crate::figures::{CaseResult, FigureCase};

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    concise("mpl_tests_datetime/barbs", barbs).sized(12.0, 6.0, 100.0),
    concise("mpl_tests_datetime/contour", contour),
    concise("mpl_tests_datetime/contourf", contourf),
    datetime_case("mpl_tests_datetime/clabel", clabel),
    datetime_case("mpl_tests_datetime/imshow", imshow),
    datetime_case("mpl_tests_datetime/matshow", matshow),
    concise("mpl_tests_datetime/bxp", bxp),
    datetime_case("mpl_tests_datetime/violin_vertical", violin_vertical),
    datetime_case("mpl_tests_datetime/violin_horizontal", violin_horizontal),
];

/// `test_barbs`: barbs at hourly dates against a sine, and the other way
/// round.
fn barbs(fig: &Figure) -> CaseResult {
    let start = hour(2022, 2, 8, 22)?;
    let dates = (0..12)
        .map(|i| start.checked_add(TimeDelta::new(0, 3600 * i, 0)?))
        .collect::<Result<Vec<_>, DateError>>()?;
    let sine: Vec<f64> = linspace(0.0, 2.0 * std::f64::consts::PI, 12)
        .into_iter()
        .map(f64::sin)
        .collect();
    let (u, v) = (vec![10.0; 12], numbers((0..12).map(|i| 10 * i)));
    let ax = axes(fig, 1, 2)?;
    let options = || BarbsOptions {
        length: 7.0,
        ..BarbsOptions::default()
    };
    let x = ax[0].xaxis().convert_units(dates.clone());
    ax[0].barbs(
        VectorField::new(x, sine.clone(), u.clone(), v.clone()),
        options(),
    )?;
    ax[0].set_title("Datetime vs. Numeric Data");
    ax[0].set_xlabel("Datetime");
    ax[0].set_ylabel("Numeric Data");
    let y = ax[1].yaxis().convert_units(dates);
    ax[1].barbs(VectorField::new(sine, y, u, v), options())?;
    ax[1].set_title("Numeric vs. Datetime Data");
    ax[1].set_xlabel("Numeric Data");
    ax[1].set_ylabel("Datetime");
    Ok(())
}

/// Grid coordinates along `axis`: nine October days, or the numbers 1 to
/// 9.
fn coordinates(axis: &Axis, dated: bool) -> Result<Vec<f64>, DateError> {
    Ok(if dated {
        axis.convert_units(daily(day(2023, 10, 1)?, 0..9)?)
    } else {
        numbers(1..10)
    })
}

/// `test_contour` and `test_contourf`: `cos(x / 4) + sin(y / 4)` over
/// dates against dates, dates against numbers and numbers against dates.
fn contours(fig: &Figure, filled: bool) -> CaseResult {
    constrained(fig)?;
    let z = Grid::from_fn(9, 9, |row, column| {
        ((column + 1) as f64 / 4.0).cos() + ((row + 1) as f64 / 4.0).sin()
    });
    for (ax, (x_dated, y_dated)) in
        axes(fig, 3, 1)?
            .iter()
            .zip([(true, true), (true, false), (false, true)])
    {
        let coords = QuadCoords::Axes {
            x: coordinates(&ax.xaxis(), x_dated)?,
            y: coordinates(&ax.yaxis(), y_dated)?,
        };
        let field = QuadField::new(z.clone()).with_coords(coords);
        if filled {
            ax.contourf(&field, ContourOptions::default())?;
        } else {
            ax.contour(&field, ContourOptions::default())?;
        }
    }
    Ok(())
}

/// `test_contour`.
fn contour(fig: &Figure) -> CaseResult {
    contours(fig, false)
}

/// `test_contourf`.
fn contourf(fig: &Figure) -> CaseResult {
    contours(fig, true)
}

/// `test_clabel`: contours over numbers against dates, labelled by hand
/// at the first point.
fn clabel(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x: Vec<f64> = (0..30).map(|i| -10.0 + 0.5 * f64::from(i)).collect();
    let dates = ax.yaxis().convert_units(daily(day(2023, 10, 1)?, 0..10)?);
    let z = Grid::from_fn(10, 30, |row, column| (row * 30 + column) as f64);
    let first = (x[0], dates[0]);
    let field = QuadField::new(z).with_coords(QuadCoords::Axes { x, y: dates });
    let contours = ax.contour(&field, ContourOptions::default())?;
    contours.clabel(ClabelOptions {
        manual: Some(vec![first]),
        ..ClabelOptions::default()
    })?;
    Ok(())
}

/// `np.diag(range(5))`.
fn diagonal() -> Grid<f64> {
    Grid::from_fn(
        5,
        5,
        |row, column| {
            if row == column { row as f64 } else { 0.0 }
        },
    )
}

/// The extent `(start, end, start, end)` in axis numbers.
fn date_extent(ax: &Axes, start: DateTime, end: DateTime) -> Extent {
    let x = ax.xaxis().convert_units(vec![start, end]);
    let y = ax.yaxis().convert_units(vec![start, end]);
    Extent::new(x[0], x[1], y[0], y[1])
}

/// `test_imshow`: a diagonal over ten days, with upright x labels.
fn imshow(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let extent = date_extent(&ax, day(2010, 11, 1)?, day(2010, 11, 11)?);
    ax.imshow(
        diagonal(),
        ImshowOptions {
            extent: Some(extent),
            ..ImshowOptions::default()
        },
    )?;
    ax.tick_params(&TickParams {
        axis: Some(plt::axes::AxisName::X),
        labelrotation: Some(90.0),
        ..TickParams::default()
    });
    Ok(())
}

/// `test_matshow`: a diagonal over forty years, with upright x labels.
fn matshow(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let extent = date_extent(&ax, day(1980, 4, 15)?, day(2020, 11, 11)?);
    ax.matshow(
        diagonal(),
        ImshowOptions {
            extent: Some(extent),
            ..ImshowOptions::default()
        },
    )?;
    ax.xaxis().set_ticklabel_props(
        TickLevel::Major,
        TickLabelProps {
            rotation: Some(90.0),
            ..TickLabelProps::default()
        },
    );
    Ok(())
}

/// `test_bxp`: a horizontal box of January dates.
fn bxp(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let january = |d| -> Result<f64, DateError> { Ok(ax.xaxis().convert_value(day(2020, 1, d)?)) };
    let (q1, med, q3) = (january(10)?, january(15)?, january(20)?);
    let stats = BoxStats {
        label: None,
        mean: f64::NAN,
        iqr: q3 - q1,
        cilo: med,
        cihi: med,
        whishi: january(25)?,
        whislo: january(5)?,
        fliers: vec![january(3)?, january(27)?],
        q1,
        med,
        q3,
    };
    ax.bxp(
        &[stats],
        BxpOptions {
            orientation: Orientation::Horizontal,
            ..BxpOptions::default()
        },
    )?;
    ax.xaxis()
        .set_major_formatter(DateFormatter::new("%Y-%m-%d"));
    ax.set_title("Box plot with datetime data");
    Ok(())
}

/// `test_violin`: a violin of three dates in `orientation`.
fn violin(fig: &Figure, orientation: Orientation) -> CaseResult {
    let ax = fig.subplot();
    let values = match orientation {
        Orientation::Vertical => ax.yaxis(),
        Orientation::Horizontal => ax.xaxis(),
    };
    let dates = values.convert_units(vec![day(2023, 2, 10)?, day(2023, 5, 18)?, day(2023, 6, 6)?]);
    let stats = ViolinStats {
        coords: dates.clone(),
        vals: vec![0.1, 0.5, 0.2],
        mean: dates[1],
        median: dates[1],
        min: dates[0],
        max: dates[2],
        quantiles: dates,
    };
    ax.violin(
        &[stats],
        ViolinDrawOptions {
            orientation,
            ..ViolinDrawOptions::default()
        },
    )?;
    Ok(())
}

/// `test_violin[vertical]`.
fn violin_vertical(fig: &Figure) -> CaseResult {
    violin(fig, Orientation::Vertical)
}

/// `test_violin[horizontal]`.
fn violin_horizontal(fig: &Figure) -> CaseResult {
    violin(fig, Orientation::Horizontal)
}
