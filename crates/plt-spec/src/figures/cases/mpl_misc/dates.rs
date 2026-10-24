//! The cases from `test_dates.py` and `test_units.py`.

use plt::axes::{Axes, Axis, TickLevel};
use plt::dates::{
    AutoDateFormatter, DateError, DateTime, Frequency, RRuleLocator, RRuleSpec, TimeDelta,
};
use plt::gridspec::SubplotOverrides;
use plt::numeric::linspace;
use plt::prelude::*;
use plt::text::HAlign;

use super::{classic, remove_text};
use crate::figures::{CaseResult, FigureCase};

/// `datetime(year, month, day)`.
fn day(year: i32, month: u32, day: u32) -> Result<DateTime, DateError> {
    DateTime::new(year, month, day, 0, 0, 0, 0)
}

/// `t0 - timedelta(days=days)` and `tf + timedelta(days=days)`, as axis
/// numbers of `axis` (which becomes a date axis).
fn padded_dates(
    axis: &Axis,
    (t0, tf): (DateTime, DateTime),
    days: i64,
) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let pad = TimeDelta::new(days, 0, 0)?;
    match axis.convert_units(vec![t0.checked_sub(pad)?, tf.checked_add(pad)?])[..] {
        [low, high] => Ok((low, high)),
        _ => Err("two dates convert to two numbers".into()),
    }
}

/// `fig.subplots_adjust(left=0.25)`, making room for date labels on the
/// y axis.
fn room_on_the_left(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.25),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// `fig.autofmt_xdate()` with matplotlib's defaults.
fn autofmt_xdate(fig: &Figure) -> CaseResult {
    fig.autofmt_xdate(0.2, 30.0, HAlign::Right, TickLevel::Major)?;
    Ok(())
}

/// `test_date_axhspan`: a band between two dates on the y axis.
fn date_axhspan(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let span = (day(2009, 1, 20)?, day(2009, 1, 21)?);
    let (y0, y1) = padded_dates(&ax.yaxis(), span, 0)?;
    ax.axhspan(y0, y1)
        .set_facecolor("blue")
        .set_alpha(Some(0.25));
    let (low, high) = padded_dates(&ax.yaxis(), span, 5)?;
    ax.set_ylim(low, high);
    room_on_the_left(fig)
}

/// `test_date_axvspan`: a band between two dates on the x axis.
fn date_axvspan(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let span = (day(2000, 1, 20)?, day(2010, 1, 21)?);
    let (x0, x1) = padded_dates(&ax.xaxis(), span, 0)?;
    ax.axvspan(x0, x1)
        .set_facecolor("blue")
        .set_alpha(Some(0.25));
    let (low, high) = padded_dates(&ax.xaxis(), span, 720)?;
    ax.set_xlim(low, high);
    autofmt_xdate(fig)
}

/// `test_date_axhline`: a thick line at a date on the y axis.
fn date_axhline(fig: &Figure) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    let span = (day(2009, 1, 20)?, day(2009, 1, 31)?);
    let (y0, _) = padded_dates(&ax.yaxis(), span, 0)?;
    ax.axhline(y0, LineProps::default().color("blue").linewidth(3.0));
    let (low, high) = padded_dates(&ax.yaxis(), span, 5)?;
    ax.set_ylim(low, high);
    room_on_the_left(fig)?;
    Ok(ax)
}

/// `test_date_axvline`: a thick line at a date on the x axis.
fn date_axvline(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let span = (day(2000, 1, 20)?, day(2000, 1, 21)?);
    let (x0, _) = padded_dates(&ax.xaxis(), span, 0)?;
    ax.axvline(x0, LineProps::default().color("red").linewidth(3.0));
    let (low, high) = padded_dates(&ax.xaxis(), span, 5)?;
    ax.set_xlim(low, high);
    autofmt_xdate(fig)
}

/// `test_RRuleLocator` and `test_DateFormatter`: a line between two dates
/// with markers, autoscaled.
fn dated_line(
    fig: &Figure,
    t0: DateTime,
    tf: DateTime,
) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.add_subplot(1, 1, 1)?;
    ax.set_autoscale_on(true);
    let props = LineProps::default().marker("o".parse::<plt::markers::Marker>()?);
    ax.plot_with(vec![t0, tf], [0.0, 1.0], props)?;
    Ok(ax)
}

/// `test_RRuleLocator`: ticks every 500 years from year 1000 to 6000,
/// where the padded view would leave the range of dates.
fn rrule_locator(fig: &Figure) -> CaseResult {
    let ax = dated_line(fig, day(1000, 1, 1)?, day(6000, 1, 1)?)?;
    let every_500_years = || {
        RRuleLocator::new(RRuleSpec {
            interval: 500,
            ..RRuleSpec::new(Frequency::Yearly)
        })
    };
    ax.xaxis()
        .set_major_locator(every_500_years())
        .set_major_formatter(AutoDateFormatter::new(every_500_years()));
    ax.autoscale_view(None);
    autofmt_xdate(fig)
}

/// `test_DateFormatter`: a line one second long.
fn date_formatter(fig: &Figure) -> CaseResult {
    let t0 = day(2001, 1, 1)?;
    let ax = dated_line(fig, t0, t0.checked_add(TimeDelta::new(0, 1, 0)?)?)?;
    ax.autoscale_view(None);
    autofmt_xdate(fig)
}

/// `test_plot_masked_units`: a masked array wrapped in an object without
/// a unit converter plots as the bare masked array.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn plot_masked_units(fig: &Figure) -> CaseResult {
    let masked: Vec<f64> = linspace(-5.0, 5.0, 50)
        .into_iter()
        .map(|x| if -2.0 < x && x < 2.0 { f64::NAN } else { x })
        .collect();
    fig.subplot().plot_y(masked);
    remove_text(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_misc/date_axhspan", date_axhspan),
    FigureCase::new("mpl_misc/date_axvspan", date_axvspan),
    FigureCase::new("mpl_misc/date_axhline", |fig| {
        date_axhline(fig)?;
        Ok(())
    }),
    FigureCase::new("mpl_misc/date_axvline", date_axvline),
    FigureCase::new("mpl_misc/RRuleLocator", rrule_locator),
    FigureCase::new("mpl_misc/DateFormatter", date_formatter),
    FigureCase::new("mpl_misc/date_inverted_limit", |fig| {
        date_axhline(fig)?.yaxis().set_inverted(true);
        Ok(())
    }),
    FigureCase::new("mpl_misc/plot_masked_units", plot_masked_units),
    classic("mpl_misc/subclass", |fig| {
        fig.subplot()
            .plot_with(vec![day(2000, 1, 1)?], [0.0], LineProps::fmt("o")?)?;
        Ok(())
    }),
];
