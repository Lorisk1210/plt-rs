//! Date ticks: month names centred between ticks, the default, concise
//! and manual date formatters, date numbers from `drange`, ticks from a
//! recurrence rule, and daily data plotted at its dates or at its indices
//! (`centered_ticklabels`, `date`, `date_demo_convert`, `date_demo_rrule`
//! and `date_index_formatter`).

use plt::axes::{AxisName, TickLevel, TickParams, TitleLocation};
use plt::dates::calendar::{self, Weekday};
use plt::dates::{
    ConciseDateFormatter, DateError, DateFormatter, DateTime, Datetime64, Frequency, RRuleLocator,
    RRuleSpec, TimeDelta, TimeZone, ZonedDateTime, drange,
};
use plt::fonts::RelativeSize;
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::legend::{LegendHandle, LegendOptions};
use plt::prelude::*;
use plt::text::{HAlign, TextRotationMode as RotationMode};
use plt::ticker::{FuncFormatter, NullFormatter};

use crate::figures::cases::common::{cumsum, map};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The seed of the examples' random data.
const SEED: u64 = 19_680_801;

/// Seconds in a day.
const DAY: i64 = 86_400;

/// A synthetic stand-in for `goog.npz`'s price data: trading days
/// (weekdays) from 2004-08-19 and a random walk of adjusted closing
/// prices.
struct Prices {
    dates: Vec<Datetime64>,
    adj_close: Vec<f64>,
}

impl Prices {
    /// The 1047 days of the sample file.
    fn goog() -> Self {
        let n = 1047;
        let first = calendar::days_from_civil(2004, 8, 19);
        let dates = (first..)
            .filter(|&day| {
                !matches!(
                    Weekday::from_days_since_epoch(day),
                    Weekday::Saturday | Weekday::Sunday
                )
            })
            .take(n)
            .map(|day| Datetime64::new(day * DAY, 0))
            .collect();
        let steps = SpecRng::new(2004).normal(n, 0.002, 0.02);
        Self {
            dates,
            adj_close: map(&cumsum(&steps), |c| 100.0 * c.exp()),
        }
    }

    /// The days in `range`.
    fn slice(self, range: std::ops::Range<usize>) -> Self {
        Self {
            dates: self.dates[range.clone()].to_vec(),
            adj_close: self.adj_close[range].to_vec(),
        }
    }
}

/// The UTC date-time of a `datetime64` value.
fn utc(instant: Datetime64) -> Result<DateTime, DateError> {
    DateTime::from_timestamp_micros(instant.timestamp_micros())
}

fn centered_ticklabels(fig: &Figure) -> CaseResult {
    let prices = Prices::goog();
    let n = prices.dates.len();
    let r = prices.slice(n - 250..n);
    let ax = fig.subplot();
    ax.plot(r.dates.clone(), r.adj_close)?;
    ax.xaxis()
        .set_major_locator(RRuleLocator::months(&[], &[1], 1))
        .set_minor_locator(RRuleLocator::months(&[], &[16], 1))
        .set_major_formatter(NullFormatter)
        .set_minor_formatter(DateFormatter::new("%b"));
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        which: TickLevel::Minor,
        bottom: Some(false),
        top: Some(false),
        ..TickParams::default()
    });
    let middle = utc(r.dates[r.dates.len() / 2])?;
    ax.set_xlabel(middle.year().to_string());
    Ok(())
}

/// A title at the top left inside the axes, as `date` sets them.
fn inner_title(ax: &Axes, text: &str) {
    ax.set_title_at(TitleLocation::Left, text)
        .set_x(0.02)
        .set_fontsize(RelativeSize::Medium);
    ax.set_title_y(TitleLocation::Left, 0.85);
}

fn date(fig: &Figure) -> CaseResult {
    let data = Prices::goog();
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(3, 1)?;
    for ax in axs.as_slice() {
        ax.plot_with(
            data.dates.clone(),
            data.adj_close.clone(),
            LineProps::default().label("adj_close"),
        )?;
        ax.xaxis()
            .set_major_locator(RRuleLocator::months(&[1, 7], &[1], 1))
            .set_minor_locator(RRuleLocator::months(&[], &[1], 1));
        ax.grid(true);
        ax.set_ylabel(r"Price [\$]");
    }
    inner_title(&axs[(0, 0)], "DefaultFormatter");
    inner_title(&axs[(1, 0)], "ConciseFormatter");
    axs[(1, 0)]
        .xaxis()
        .set_major_formatter(ConciseDateFormatter::new());
    inner_title(&axs[(2, 0)], "Manual DateFormatter");
    axs[(2, 0)]
        .xaxis()
        .set_major_formatter(DateFormatter::new("%Y-%b"))
        .set_tick_params(&TickParams {
            labelrotation: Some(30.0),
            labelrotation_mode: Some(RotationMode::XTick),
            ..TickParams::default()
        });
    Ok(())
}

/// Midnight UTC of a day, as an aware date-time.
fn midnight(year: i32, month: u32, day: u32) -> Result<ZonedDateTime, DateError> {
    ZonedDateTime::from_utc(DateTime::new(year, month, day, 0, 0, 0, 0)?, TimeZone::Utc)
}

fn date_demo_convert(fig: &Figure) -> CaseResult {
    let dates = drange(
        &midnight(2000, 3, 2)?,
        &midnight(2000, 3, 6)?,
        TimeDelta::new(0, 6 * 3600, 0)?,
    )?;
    let squares: Vec<f64> = (0..dates.len()).map(|i| (i * i) as f64).collect();
    let ax = fig.subplot();
    ax.plot_fmt(dates.clone(), squares, "o")?;
    ax.set_xlim(dates[0], dates[dates.len() - 1]);
    ax.xaxis()
        .set_major_locator(RRuleLocator::days(&[], 1)?)
        .set_minor_locator(RRuleLocator::hours(&[0, 6, 12, 18, 24], 1))
        .set_major_formatter(DateFormatter::new("%Y-%m-%d"));
    fig.autofmt_xdate(0.2, 30.0, HAlign::Right)?;
    Ok(())
}

fn date_demo_rrule(fig: &Figure) -> CaseResult {
    let easter_monday = RRuleSpec {
        byeaster: vec![1],
        interval: 5,
        ..RRuleSpec::new(Frequency::Yearly)
    };
    let dates = drange(
        &midnight(1952, 1, 1)?,
        &midnight(2004, 4, 12)?,
        TimeDelta::new(100, 0, 0)?,
    )?;
    let s = SpecRng::new(SEED).uniform(dates.len(), 0.0, 1.0);
    let ax = fig.subplot();
    ax.plot_fmt(dates, s, "o")?;
    ax.xaxis()
        .set_major_locator(RRuleLocator::new(easter_monday))
        .set_major_formatter(DateFormatter::new("%m/%d/%y"))
        .set_tick_params(&TickParams {
            labelrotation: Some(30.0),
            labelsize: Some(plt::fonts::FontSize::Points(10.0)),
            ..TickParams::default()
        });
    Ok(())
}

fn date_index_formatter(fig: &Figure) -> CaseResult {
    let r = Prices::goog().slice(0..9);
    fig.set_layout_engine(LayoutEngine::Constrained(ConstrainedLayout {
        hspace: 0.15,
        ..ConstrainedLayout::default()
    }));
    let axs = fig.subplots(2, 1)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    ax1.plot_fmt(r.dates.clone(), r.adj_close.clone(), "o-")?;
    for gap in 0..r.dates.len() - 1 {
        if r.dates[gap + 1].seconds() - r.dates[gap].seconds() > DAY {
            ax1.plot_with(
                r.dates[gap..gap + 2].to_vec(),
                r.adj_close[gap..gap + 2].to_vec(),
                LineProps::fmt("w--")?.linewidth(2.0),
            )?;
        }
    }
    ax1.legend(LegendOptions::default().handles([LegendHandle::line(
        LineProps::fmt("--")?.label("Gaps in daily data"),
    )]))?;
    ax1.set_title("Plot y at x Coordinates");
    ax1.xaxis()
        .set_major_locator(RRuleLocator::days(&[], 1)?)
        .set_major_formatter(DateFormatter::new("%a"));

    let indices: Vec<f64> = (0..r.dates.len()).map(|i| i as f64).collect();
    ax2.plot_fmt(indices, r.adj_close, "o-")?;
    ax2.set_title("Plot y at Index Coordinates Using Custom Formatter");
    // `MyFormatter`: the weekday of the day at the (rounded) index, with
    // numpy's negative indices counting from the end.
    let weekdays = r
        .dates
        .iter()
        .map(|&day| utc(day).map(|d| d.weekday().short_name().to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    ax2.xaxis()
        .set_major_formatter(FuncFormatter::new(move |x, _| {
            let len = weekdays.len() as i64;
            let index = x.round_ties_even() as i64;
            if (-len..len).contains(&index) {
                weekdays[index.rem_euclid(len) as usize].clone()
            } else {
                String::new()
            }
        }));
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/centered_ticklabels", centered_ticklabels),
    FigureCase::new("gallery_ticks/date", date).sized(6.4, 7.0, 100.0),
    FigureCase::new("gallery_ticks/date_demo_convert", date_demo_convert),
    FigureCase::new("gallery_ticks/date_demo_rrule", date_demo_rrule),
    FigureCase::new("gallery_ticks/date_index_formatter", date_index_formatter)
        .sized(6.0, 6.0, 100.0),
];
