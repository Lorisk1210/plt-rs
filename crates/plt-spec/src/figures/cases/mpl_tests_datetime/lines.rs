//! The line, marker, fill and text cases: `test_plot`, `test_step`,
//! `test_scatter`, `test_stackplot`, `test_fill`, `test_fill_between`,
//! `test_fill_betweenx`, `test_errorbar`, `test_stem`, `test_text` and
//! `test_annotate`.

use plt::axes::{
    Axes, ErrorbarOptions, FillBetweenOptions, PlotError, StackplotOptions, StemOptions,
    TickLabelProps, TickLevel,
};
use plt::dates::{DateError, DateTime, TimeDelta};
use plt::fonts::FontWeight;
use plt::lines::{MarkEvery, StepWhere};
use plt::orientation::Orientation;
use plt::patches::PatchProps;
use plt::prelude::*;
use plt::text::{AnnotateOptions, ArrowProps, HAlign, LegacyArrowProps};
use plt::units::AxisValues;

use super::{axes, concise, constrained, daily, day, days, numbers};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    concise("mpl_tests_datetime/plot", plot),
    concise("mpl_tests_datetime/step", step),
    concise("mpl_tests_datetime/scatter", scatter).sized(6.0, 6.0, 100.0),
    concise("mpl_tests_datetime/stackplot", stackplot),
    concise("mpl_tests_datetime/fill", fill),
    concise("mpl_tests_datetime/fill_between", fill_between),
    concise("mpl_tests_datetime/fill_betweenx", fill_betweenx),
    concise("mpl_tests_datetime/errorbar", errorbar),
    concise("mpl_tests_datetime/stem", stem),
    concise("mpl_tests_datetime/text", text),
    concise("mpl_tests_datetime/annotate", annotate),
];

/// `test_plot` and `test_step`: dates against numbers, numbers against
/// dates and dates against dates, drawn by `draw`.
fn lines(
    fig: &Figure,
    draw: fn(&Axes, AxisValues, AxisValues) -> Result<(), PlotError>,
) -> CaseResult {
    constrained(fig)?;
    let ax = axes(fig, 3, 1)?;
    let x = daily(day(2023, 9, 1)?, 0..5)?;
    let n = numbers(1..6);
    draw(&ax[0], x.clone().into(), n.clone().into())?;
    draw(&ax[1], n.into(), x.clone().into())?;
    draw(&ax[2], x.clone().into(), x.into())?;
    Ok(())
}

/// `test_plot`.
fn plot(fig: &Figure) -> CaseResult {
    lines(fig, |ax, x, y| ax.plot(x, y).map(drop))
}

/// `test_step`.
fn step(fig: &Figure) -> CaseResult {
    lines(fig, |ax, x, y| {
        ax.step(x, y, StepWhere::Pre, LineProps::default())
            .map(drop)
    })
}

/// matplotlib's `for label in ax.get_xticklabels(): label.set_rotation(40);
/// label.set_horizontalalignment("right")`.
fn slanted_labels(ax: &Axes) {
    ax.xaxis().set_ticklabel_props(
        TickLevel::Major,
        TickLabelProps {
            rotation: Some(40.0),
            halign: Some(HAlign::Right),
            ..TickLabelProps::default()
        },
    );
}

/// `test_scatter`: twenty hours of dates against a random walk.
fn scatter(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let base = day(2005, 2, 1)?;
    let dates = (0..10)
        .map(|i| base.checked_add(TimeDelta::new(0, 7200 * i, 0)?))
        .collect::<Result<Vec<_>, DateError>>()?;
    let y: Vec<f64> = SpecRng::new(19_680_801)
        .normal(dates.len(), 0.0, 1.0)
        .into_iter()
        .scan(0.0, |sum, step| {
            *sum += step;
            Some(*sum)
        })
        .collect();
    let ax = axes(fig, 3, 1)?;
    ax[0].scatter(dates.clone(), y.clone())?;
    slanted_labels(&ax[0]);
    ax[1].scatter(y, dates.clone())?;
    ax[2].scatter(dates.clone(), dates)?;
    slanted_labels(&ax[2]);
    Ok(())
}

/// `test_stackplot`: four equal layers over nine new years.
fn stackplot(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let dates = (0..9)
        .map(|i| day(2020 + i, 1, 1))
        .collect::<Result<Vec<_>, _>>()?;
    let layer = numbers(1..10);
    let ax = fig.subplot();
    ax.stackplot(dates, &vec![layer; 4], StackplotOptions::default())?;
    Ok(())
}

/// `start` followed by `steps` dates, each `rng.integers(1, high)` whole
/// days after the previous one (`timedelta(days=np.random.randint(...))`).
fn date_walk(
    rng: &mut SpecRng,
    start: DateTime,
    steps: usize,
    high: i64,
) -> Result<Vec<DateTime>, DateError> {
    let mut dates = vec![start];
    for step in rng.integers(steps, 1, high) {
        let last = dates[dates.len() - 1];
        dates.push(last.checked_add(days(step)?)?);
    }
    Ok(dates)
}

/// `test_fill`: polygons through dates and random numbers.
fn fill(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let ax = axes(fig, 4, 1)?;
    let mut rng = SpecRng::new(19_680_801);
    let x_dates = date_walk(&mut rng, day(2023, 1, 1)?, 4, 5)?;
    let y_dates = date_walk(&mut rng, day(2023, 1, 1)?, 4, 5)?;
    let x_values: Vec<f64> = rng.uniform(5, 0.0, 1.0).iter().map(|u| u * 5.0).collect();
    let y_values: Vec<f64> = rng
        .uniform(5, 0.0, 1.0)
        .iter()
        .map(|u| u * 5.0 - 2.0)
        .collect();
    let props = PatchProps::default;
    ax[0].fill(x_dates.clone(), y_values.clone(), props())?;
    ax[1].fill(x_values.clone(), y_dates.clone(), props())?;
    ax[2].fill(x_values, y_values, props())?;
    ax[3].fill(x_dates, y_dates, props())?;
    Ok(())
}

/// The data of `test_fill_between` and `test_fill_betweenx`.
struct Between {
    /// A walk of dates.
    dates1: Vec<DateTime>,
    /// A walk of dates from the end of the first.
    dates2: Vec<DateTime>,
    /// Sorted values along the other axis.
    values: Vec<f64>,
    /// Sorted lower values.
    values1: Vec<f64>,
    /// Sorted upper values.
    values2: Vec<f64>,
    /// A slower walk of dates.
    slow_dates: Vec<DateTime>,
}

/// `values` sorted in increasing order.
fn sorted(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    values
}

impl Between {
    /// The data in the order the tests draw it.
    fn new() -> Result<Self, DateError> {
        let mut rng = SpecRng::new(19_680_801);
        let dates1 = date_walk(&mut rng, day(2023, 1, 1)?, 9, 5)?;
        let dates2 = date_walk(&mut rng, dates1[dates1.len() - 1], 9, 5)?;
        let tenfold = |rng: &mut SpecRng| -> Vec<f64> {
            rng.uniform(10, 0.0, 1.0).iter().map(|u| u * 10.0).collect()
        };
        let values = sorted(tenfold(&mut rng));
        let values1 = tenfold(&mut rng);
        let values2 = values1
            .iter()
            .zip(tenfold(&mut rng))
            .map(|(low, extra)| low + extra)
            .collect();
        let slow_dates = date_walk(&mut rng, day(2023, 1, 1)?, 9, 10)?;
        Ok(Self {
            dates1,
            dates2,
            values,
            values1: sorted(values1),
            values2: sorted(values2),
            slow_dates,
        })
    }
}

/// `test_fill_between`.
fn fill_between(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let data = Between::new()?;
    let ax = axes(fig, 3, 1)?;
    let options = FillBetweenOptions::default;
    ax[0].fill_between(
        data.values,
        data.dates1.clone(),
        data.dates2.clone(),
        options(),
    )?;
    ax[1].fill_between(
        data.slow_dates.clone(),
        data.values1,
        data.values2,
        options(),
    )?;
    ax[2].fill_between(data.slow_dates, data.dates1, data.dates2, options())?;
    Ok(())
}

/// `test_fill_betweenx`.
fn fill_betweenx(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let data = Between::new()?;
    let ax = axes(fig, 1, 3)?;
    let options = FillBetweenOptions::default;
    ax[0].fill_betweenx(
        data.values,
        data.dates1.clone(),
        data.dates2.clone(),
        options(),
    )?;
    ax[1].fill_betweenx(
        data.slow_dates.clone(),
        data.values1,
        data.values2,
        options(),
    )?;
    ax[2].fill_betweenx(data.slow_dates, data.dates1, data.dates2, options())?;
    Ok(())
}

/// `test_errorbar`: dates and numbers with errors of half a unit or a
/// day, limits and every other bar.
fn errorbar(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let ax = axes(fig, 4, 1)?;
    let dates = daily(day(2023, 10, 1)?, 0..6)?;
    let values = numbers(1..7);
    let one_day = days(1)?;
    let labelled = |options: ErrorbarOptions| ErrorbarOptions {
        line: LineProps::default().label("Data"),
        ..options
    };
    ax[0].errorbar(
        dates.clone(),
        values.clone(),
        labelled(ErrorbarOptions {
            yerr: Some(0.5.into()),
            capsize: Some(10.0),
            barsabove: true,
            ..ErrorbarOptions::default()
        }),
    )?;
    ax[1].errorbar(
        values.clone(),
        dates.clone(),
        labelled(ErrorbarOptions {
            xerr: Some(0.5.into()),
            yerr: Some(one_day.into()),
            errorevery: Some(MarkEvery::Every { start: 1, step: 2 }),
            fmt: "-o".parse()?,
            ..ErrorbarOptions::default()
        }),
    )?;
    ax[2].errorbar(
        dates.clone(),
        dates.clone(),
        labelled(ErrorbarOptions {
            xerr: Some(one_day.into()),
            yerr: Some(one_day.into()),
            lolims: vec![true],
            xlolims: vec![true],
            ..ErrorbarOptions::default()
        }),
    )?;
    ax[3].errorbar(
        dates,
        values,
        labelled(ErrorbarOptions {
            xerr: Some(one_day.into()),
            yerr: Some(0.5.into()),
            uplims: vec![true],
            xuplims: vec![true],
            ..ErrorbarOptions::default()
        }),
    )?;
    Ok(())
}

/// `test_stem`: stems along dates or numbers from a date or a number,
/// vertical then horizontal.
fn stem(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let ax = axes(fig, 6, 1)?;
    let (above, below) = (day(2023, 9, 18)?, day(2023, 11, 18)?);
    let ranges = AxisValues::from(numbers(1..10));
    let dates = AxisValues::from(daily(day(2023, 10, 1)?, 0..9)?);
    for (ax, orientation, locs, heads, bottom) in [
        (&ax[0], Orientation::Vertical, &dates, &dates, Some(above)),
        (&ax[1], Orientation::Vertical, &dates, &ranges, None),
        (&ax[2], Orientation::Vertical, &ranges, &dates, Some(below)),
        (
            &ax[3],
            Orientation::Horizontal,
            &ranges,
            &dates,
            Some(above),
        ),
        (&ax[4], Orientation::Horizontal, &dates, &ranges, None),
        (
            &ax[5],
            Orientation::Horizontal,
            &ranges,
            &dates,
            Some(below),
        ),
    ] {
        // The bottom is a date, or `bottom=5`, on the heads' axis.
        let head_axis = match orientation {
            Orientation::Vertical => ax.yaxis(),
            Orientation::Horizontal => ax.xaxis(),
        };
        let options = StemOptions {
            bottom: bottom.map_or(5.0, |date| head_axis.convert_value(date)),
            orientation,
            ..StemOptions::default()
        };
        ax.stem(locs.clone(), heads.clone(), options)?;
    }
    Ok(())
}

/// `test_text`: bold serif text at dates.
fn text(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let ax = axes(fig, 3, 1)?;
    let test_date = day(2023, 10, 1)?;
    let values = numbers(1..10);
    let dates = daily(test_date, 0..9)?;
    let styled = |note: plt::text::Text| {
        note.set_family(["serif"])
            .set_fontsize(12.0)
            .set_weight(FontWeight::BOLD);
    };
    ax[0].plot(dates.clone(), values.clone())?;
    styled(ax[0].text(test_date, 5.0, "Inserted Text"));
    ax[1].plot(values, dates.clone())?;
    styled(ax[1].text(7.0, test_date, "Inserted Text"));
    ax[2].plot(dates.clone(), dates)?;
    styled(ax[2].text(test_date, test_date, "Inserted Text"));
    Ok(())
}

/// `test_annotate`: annotations at dates, the last with a text placed at
/// dates and a red arrow.
fn annotate(fig: &Figure) -> CaseResult {
    constrained(fig)?;
    let ax = axes(fig, 4, 1)?;
    let dates = daily(day(2023, 10, 1)?, 0..31)?;
    let data = numbers(1..32);
    let note = "Test Text";
    let plain = AnnotateOptions::default;
    ax[0].plot(dates.clone(), data.clone())?;
    ax[0].annotate(note, (dates[15], data[15]), plain())?;
    ax[1].plot(data.clone(), dates.clone())?;
    ax[1].annotate(note, (data[5], dates[26]), plain())?;
    ax[2].plot(dates.clone(), dates.clone())?;
    ax[2].annotate(note, (dates[15], dates[3]), plain())?;
    ax[3].plot(dates.clone(), dates.clone())?;
    let arrow = LegacyArrowProps {
        patch: PatchProps::default().facecolor("red"),
        ..LegacyArrowProps::default()
    };
    ax[3].annotate(
        note,
        (dates[5], dates[30]),
        AnnotateOptions {
            xytext: Some(ax[3].convert_point((dates[1], dates[7]))),
            arrowprops: Some(ArrowProps::Legacy(arrow)),
            ..plain()
        },
    )?;
    Ok(())
}
