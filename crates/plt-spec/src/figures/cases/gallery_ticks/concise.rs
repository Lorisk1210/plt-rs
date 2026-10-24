//! `date_concise_formatter`: the same two-hourly random walk over two
//! months, twelve days and a day and a bit, under the default date
//! formatter, the concise one, the concise converter, and the concise
//! formatter with custom formats (set on the axis or as the converter's).

use plt::axes::{AxisName, TickParams};
use plt::dates::{AutoDateLocator, ConciseDateFormatter, DateTime, Datetime64, TimeDelta};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::text::TextRotationMode as RotationMode;

use crate::figures::cases::common::cumsum;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The view of each axes, as numpy `datetime64` strings.
const LIMITS: [(&str, &str); 3] = [
    ("2005-02", "2005-04"),
    ("2005-02-03", "2005-02-15"),
    ("2005-02-03 11:00", "2005-02-04 13:20"),
];

/// The tick formats of the customised formatter, by tick level (years
/// to seconds).
const FORMATS: [&str; 6] = ["%y", "%b", "%d", "%H:%M", "%H:%M", "%S.%f"];

/// Their formats for ticks at the start of the next level up: those of
/// the level above, but day and month for hours.
const ZERO_FORMATS: [&str; 6] = ["", "%y", "%b", "%d-%b", "%H:%M", "%H:%M"];

/// The offset text formats of the customised formatter.
const OFFSET_FORMATS: [&str; 6] = ["", "%Y", "%b %Y", "%d %b %Y", "%d %b %Y", "%d %b %Y %H:%M"];

/// The three axes of the example, each prepared by `setup` before the
/// random walk is plotted and its view set, the first titled `title`.
fn concise_axes(
    fig: &Figure,
    title: &str,
    setup: impl Fn(&Axes) -> CaseResult,
) -> Result<Vec<Axes>, Box<dyn std::error::Error>> {
    let base = DateTime::new(2005, 2, 1, 0, 0, 0, 0)?;
    let dates = (0..732)
        .map(|i| base.checked_add(TimeDelta::new(0, 2 * 3600 * i, 0)?))
        .collect::<Result<Vec<_>, _>>()?;
    let y = cumsum(&SpecRng::new(19_680_801).normal(dates.len(), 0.0, 1.0));
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(3, 1)?.as_slice().to_vec();
    for (ax, (low, high)) in axs.iter().zip(LIMITS) {
        setup(ax)?;
        ax.plot(dates.clone(), y.clone())?;
        let instants = [low, high]
            .map(|text| Datetime64::parse(text).ok().flatten().ok_or("bad limit"))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let limits = ax.xaxis().convert_units(instants);
        ax.set_xlim(limits[0], limits[1]);
    }
    axs[0].set_title(title);
    Ok(axs)
}

/// Leaves an axes as it is.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a setup step shares the fallible signature"
)]
fn unchanged(_: &Axes) -> CaseResult {
    Ok(())
}

fn default_formatter(fig: &Figure) -> CaseResult {
    for ax in concise_axes(fig, "Default Date Formatter", unchanged)? {
        ax.tick_params(&TickParams {
            axis: Some(AxisName::X),
            labelrotation: Some(40.0),
            labelrotation_mode: Some(RotationMode::XTick),
            ..TickParams::default()
        });
    }
    Ok(())
}

fn concise_formatter(fig: &Figure) -> CaseResult {
    concise_axes(fig, "Concise Date Formatter", |ax| {
        ax.xaxis()
            .set_major_locator(AutoDateLocator::new().with_minticks(3).with_maxticks(7))
            .set_major_formatter(ConciseDateFormatter::new());
        Ok(())
    })?;
    Ok(())
}

/// The converter's choice under `date.converter: concise`.
fn concise_converter(fig: &Figure) -> CaseResult {
    concise_axes(fig, "Concise Date Formatter", unchanged)?;
    Ok(())
}

/// The concise formatter with the custom formats, on the automatic date
/// locator.
fn custom_formats(ax: &Axes) -> CaseResult {
    ax.xaxis()
        .set_major_locator(AutoDateLocator::new())
        .set_major_formatter(
            ConciseDateFormatter::new()
                .with_formats(&FORMATS)?
                .with_zero_formats(&ZERO_FORMATS)?
                .with_offset_formats(&OFFSET_FORMATS)?,
        );
    Ok(())
}

fn custom_formatter(fig: &Figure) -> CaseResult {
    concise_axes(fig, "Concise Date Formatter", custom_formats)?;
    Ok(())
}

/// matplotlib registers a `ConciseDateConverter` with the custom formats;
/// its locator and formatter are those of [`custom_formats`].
fn custom_converter(fig: &Figure) -> CaseResult {
    concise_axes(
        fig,
        "Concise Date Formatter registered non-default",
        custom_formats,
    )?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/date_concise_formatter_1", default_formatter)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_ticks/date_concise_formatter_2", concise_formatter)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_ticks/date_concise_formatter_3", concise_converter)
        .sized(6.0, 6.0, 100.0)
        .with_rc(&[("date.converter", "concise")]),
    FigureCase::new("gallery_ticks/date_concise_formatter_4", custom_formatter)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_ticks/date_concise_formatter_5", custom_converter)
        .sized(6.0, 6.0, 100.0),
];
