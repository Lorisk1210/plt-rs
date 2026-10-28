//! The gallery's reference strips: one bare axis per locator or formatter,
//! named in monospace above it (`tick-locators`, `tick-formatters` and
//! `date_formatters_locators`).

use plt::axes::{TickLevel, TickParams};
use plt::dates::rrule::{Frequency, RRuleSpec};
use plt::dates::{
    AutoDateFormatter, AutoDateLocator, ConciseDateFormatter, DateFormatter, MicrosecondLocator,
    RRuleLocator, Weekday, YearLocator,
};
use plt::figure::SubfigureOptions;
use plt::fonts::FontSize;
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::text::HAlign;
use plt::ticker::{
    FixedFormatter, FixedLocator, FormatStrFormatter, FuncFormatter, Locator, MultipleLocator,
    NullFormatter, PercentFormatter, ScalarFormatter, StrMethodFormatter, pyfmt,
};

use crate::figures::cases::common::{
    datetime64, locator_strips, named_tick_strip, strip_title, tick_strip,
};
use crate::figures::{CaseResult, FigureCase};

fn tick_locators(fig: &Figure) -> CaseResult {
    locator_strips(fig, named_tick_strip)
}

/// `setup` of `tick-formatters`: the strip with major ticks at whole
/// numbers and labelled minor ticks at quarters.
fn formatter_strip(ax: &Axes, title: &str) -> CaseResult {
    named_tick_strip(ax, title)?;
    ax.xaxis()
        .set_major_locator(MultipleLocator::new(1.0)?)
        .set_minor_locator(MultipleLocator::new(0.25)?);
    ax.tick_params(TickParams {
        which: TickLevel::Minor,
        labelsize: Some(FontSize::Points(10.0)),
        ..TickParams::default()
    });
    Ok(())
}

/// A subfigure titled at its left edge, as the sections of
/// `tick-formatters` are.
fn section(fig: &Figure, title: &str) {
    fig.suptitle(title)
        .set_fontsize(16.0)
        .set_x(0.0)
        .set_halign(HAlign::Left);
}

fn tick_formatters(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let parts = fig.subfigures(
        3,
        1,
        SubfigureOptions {
            height_ratios: Some(vec![1.5, 1.5, 7.5]),
            ..SubfigureOptions::default()
        },
    )?;
    let (fig0, fig1, fig2) = (&parts[(0, 0)], &parts[(1, 0)], &parts[(2, 0)]);

    section(fig0, "String Formatting");
    let ax0 = fig0.subplot();
    formatter_strip(&ax0, "'{x} km'")?;
    ax0.xaxis()
        .set_major_formatter(StrMethodFormatter::new("{x} km")?);

    section(fig1, "Function Formatting");
    let ax1 = fig1.subplot();
    formatter_strip(&ax1, "def(x, pos): return str(x-5)")?;
    ax1.xaxis()
        .set_major_formatter(FuncFormatter::new(|x, _| pyfmt::repr(x - 5.0)));

    section(fig2, "Formatter Object Formatting");
    let axs2 = fig2.subplots(7, 1)?;
    let ax = |i: usize| &axs2[(i, 0)];
    formatter_strip(ax(0), "NullFormatter()")?;
    ax(0).xaxis().set_major_formatter(NullFormatter);
    formatter_strip(ax(1), "StrMethodFormatter('{x:.3f}')")?;
    ax(1)
        .xaxis()
        .set_major_formatter(StrMethodFormatter::new("{x:.3f}")?);
    formatter_strip(ax(2), "FormatStrFormatter('#%d')")?;
    ax(2)
        .xaxis()
        .set_major_formatter(FormatStrFormatter::new("#%d")?);
    formatter_strip(ax(3), "FuncFormatter(\"[{:.2f}]\".format)")?;
    ax(3)
        .xaxis()
        .set_major_formatter(FuncFormatter::new(|x, _| {
            format!("[{}]", pyfmt::fixed(x, 2))
        }));
    formatter_strip(ax(4), "FixedFormatter(['A', 'B', 'C', 'D', 'E', 'F'])")?;
    ax(4)
        .xaxis()
        .set_major_locator(FixedLocator::new(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]))
        .set_major_formatter(FixedFormatter::new(
            ["A", "B", "C", "D", "E", "F"].map(String::from).to_vec(),
        ));
    formatter_strip(ax(5), "ScalarFormatter()")?;
    ax(5)
        .xaxis()
        .set_major_formatter(ScalarFormatter::new().with_use_mathtext(true));
    formatter_strip(ax(6), "PercentFormatter(xmax=5)")?;
    ax(6)
        .xaxis()
        .set_major_formatter(PercentFormatter::new(5.0));
    Ok(())
}

/// `plot_axis` of `date_formatters_locators` before its locator or
/// formatter is set: the bare strip from 2000-02-01 to `xmax`, named
/// `title`.
fn date_strip(ax: &Axes, title: &str, xmax: &str) -> CaseResult {
    tick_strip(ax);
    let limits = ax
        .xaxis()
        .convert_units(vec![datetime64("2000-02-01")?, datetime64(xmax)?]);
    ax.set_xlim(limits[0], limits[1])?;
    strip_title(ax, title);
    Ok(())
}

/// A strip of the locators figure: `locator` placing the ticks, labelled
/// by `format`.
fn locator_strip(
    ax: &Axes,
    title: &str,
    xmax: &str,
    format: &str,
    locator: impl Locator,
) -> CaseResult {
    date_strip(ax, title, xmax)?;
    ax.xaxis()
        .set_major_locator(locator)
        .set_major_formatter(DateFormatter::new(format));
    Ok(())
}

fn date_formatters_locators_1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(10, 1)?;
    let ax = |i: usize| &axs[(i, 0)];
    fig.suptitle("Date Locators");
    let (year_end, weeks) = ("2003-02-01", "2000-07-01");
    locator_strip(
        ax(0),
        "AutoDateLocator(maxticks=8)",
        year_end,
        "%Y-%m",
        AutoDateLocator::new().with_maxticks(8),
    )?;
    locator_strip(
        ax(1),
        "YearLocator(month=4)",
        year_end,
        "%Y-%m",
        YearLocator::new(1, 4, 1)?,
    )?;
    locator_strip(
        ax(2),
        "MonthLocator(bymonth=[4, 8, 12])",
        year_end,
        "%Y-%m",
        RRuleLocator::months(&[4, 8, 12], &[1], 1),
    )?;
    locator_strip(
        ax(3),
        "DayLocator(interval=180)",
        year_end,
        "%Y-%m-%d",
        RRuleLocator::days(&[], 180)?,
    )?;
    locator_strip(
        ax(4),
        "WeekdayLocator(byweekday=SU, interval=4)",
        weeks,
        "%a %Y-%m-%d",
        RRuleLocator::weekdays(&[Weekday::Sunday], 4),
    )?;
    locator_strip(
        ax(5),
        "HourLocator(byhour=range(0, 24, 6))",
        "2000-02-04",
        "%H h",
        RRuleLocator::hours(&[0, 6, 12, 18], 1),
    )?;
    locator_strip(
        ax(6),
        "MinuteLocator(interval=15)",
        "2000-02-01 02:00",
        "%H:%M",
        RRuleLocator::minutes(&[], 15),
    )?;
    locator_strip(
        ax(7),
        "SecondLocator(bysecond=(0, 30))",
        "2000-02-01 00:02",
        "%H:%M:%S",
        RRuleLocator::seconds(&[0, 30], 1),
    )?;
    locator_strip(
        ax(8),
        "MicrosecondLocator(interval=1000)",
        "2000-02-01 00:00:00.005",
        "%S.%f",
        MicrosecondLocator::new(1000)?,
    )?;
    let workdays = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
    ];
    locator_strip(
        ax(9),
        "RRuleLocator(rrulewrapper(freq=MONTHLY, \nbyweekday=(MO, TU, WE, TH, FR), bysetpos=-1))",
        weeks,
        "%Y-%m-%d",
        RRuleLocator::new(RRuleSpec {
            byweekday: workdays.map(Into::into).to_vec(),
            bysetpos: vec![-1],
            ..RRuleSpec::new(Frequency::Monthly)
        }),
    )?;
    Ok(())
}

fn date_formatters_locators_2(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(3, 1)?;
    fig.suptitle("Date Formatters");
    let xmax = "2002-02-01";
    // The axis' own locator, which the first two formatters are given:
    // the date converter's.
    let locator = || AutoDateLocator::new().with_interval_multiples(true);
    date_strip(
        &axs[(0, 0)],
        "AutoDateFormatter(ax.xaxis.get_major_locator())",
        xmax,
    )?;
    axs[(0, 0)]
        .xaxis()
        .set_major_formatter(AutoDateFormatter::new(locator()));
    date_strip(
        &axs[(1, 0)],
        "ConciseDateFormatter(ax.xaxis.get_major_locator())",
        xmax,
    )?;
    axs[(1, 0)]
        .xaxis()
        .set_major_formatter(ConciseDateFormatter::new());
    date_strip(&axs[(2, 0)], "DateFormatter(\"%b %Y\")", xmax)?;
    axs[(2, 0)]
        .xaxis()
        .set_major_formatter(DateFormatter::new("%b %Y"));
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/tick_locators", tick_locators).sized(8.0, 6.0, 100.0),
    FigureCase::new("gallery_ticks/tick_formatters", tick_formatters).sized(8.0, 8.0, 100.0),
    FigureCase::new(
        "gallery_ticks/date_formatters_locators_1",
        date_formatters_locators_1,
    )
    .sized(8.0, 8.0, 100.0),
    FigureCase::new(
        "gallery_ticks/date_formatters_locators_2",
        date_formatters_locators_2,
    )
    .sized(8.0, 3.0 * 0.8, 100.0),
];
