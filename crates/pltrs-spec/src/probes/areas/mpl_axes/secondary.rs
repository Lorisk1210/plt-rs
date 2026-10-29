//! Secondary axes: where they may go, the host's position after a
//! resize, the locators and formatters they keep, and their colour.

use std::any::Any;

use pltrs::axes::{Axes, SecondaryLocation};
use pltrs::figure::Figure;
use pltrs::scale::FuncTransform;
use pltrs::spines::SpineSide;
use pltrs::ticker::{
    AutoMinorLocator, FixedLocator, Formatter, Locator, LogFormatter, LogLocator, NullLocator,
    ScalarFormatter,
};

use super::{made, numbers, on_axes};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::encode::resolved_rgba;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `ax.plot(np.arange(2, 11), np.arange(2, 11))`.
fn plot_range(ax: &Axes) -> Result<(), ProbeError> {
    let x: Vec<f64> = (2..11).map(f64::from).collect();
    made(ax.plot(x.clone(), x))?;
    Ok(())
}

/// `1 / x` both ways.
fn invert() -> FuncTransform {
    FuncTransform::new(|x| 1.0 / x, |x| 1.0 / x)
}

/// A side named as matplotlib names it.
fn side(name: &str) -> Result<SpineSide, ProbeError> {
    match name {
        "left" => Ok(SpineSide::Left),
        "right" => Ok(SpineSide::Right),
        "bottom" => Ok(SpineSide::Bottom),
        "top" => Ok(SpineSide::Top),
        other => Err(ProbeError::BadArgs(format!("bad side {other}"))),
    }
}

pub(super) fn secondary_axis(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        plot_range(ax)?;
        let location = SecondaryLocation::Side(side(args.str("location")?)?);
        match args.str("orientation")? {
            "x" => made(ax.secondary_xaxis(location, None))?,
            _ => made(ax.secondary_yaxis(location, None))?,
        };
        Ok(Json::Bool(true))
    })
}

pub(super) fn secondary_resize(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        fig.set_size_inches(10.0, 5.0)
            .map_err(ProbeError::rejected)?;
        let ax = fig.subplot();
        plot_range(&ax)?;
        made(ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(invert())))?;
        fig.layout();
        fig.set_size_inches(7.0, 4.0)
            .map_err(ProbeError::rejected)?;
        let position = ax.position();
        Ok(numbers([
            position.x0(),
            position.y0(),
            position.x1(),
            position.y1(),
        ]))
    })
}

/// The class name of a locator among those the probes meet.
fn locator_kind(locator: &dyn Locator) -> &'static str {
    let any: &dyn Any = locator;
    if any.is::<NullLocator>() {
        "NullLocator"
    } else if any.is::<AutoMinorLocator>() {
        "AutoMinorLocator"
    } else if any.is::<LogLocator>() {
        "LogLocator"
    } else if any.is::<FixedLocator>() {
        "FixedLocator"
    } else {
        "other"
    }
}

/// The class name of a formatter among those the probes meet.
fn formatter_kind(formatter: &dyn Formatter) -> &'static str {
    let any: &dyn Any = formatter;
    if any.is::<ScalarFormatter>() {
        "ScalarFormatter"
    } else if any.is::<LogFormatter>() {
        "LogFormatter"
    } else {
        "other"
    }
}

pub(super) fn secondary_minor_locator(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        fig.set_size_inches(10.0, 5.0)
            .map_err(ProbeError::rejected)?;
        let ax = fig.subplot();
        plot_range(&ax)?;
        let secax =
            made(ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(invert())))?;
        let kind = || Json::from(locator_kind(secax.xaxis().minor_locator().as_ref()));
        let mut kinds = vec![kind()];
        secax.minorticks_on();
        kinds.push(kind());
        made(ax.set_xscale("log"))?;
        fig.layout();
        kinds.push(kind());
        made(ax.set_xscale("linear"))?;
        fig.layout();
        kinds.push(kind());
        Ok(Json::Array(kinds))
    })
}

pub(super) fn secondary_formatter(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        made(ax.set_xscale("log"))?;
        let secax = made(ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), None))?;
        secax
            .xaxis()
            .set_major_formatter(ScalarFormatter::default());
        fig.layout();
        Ok(Json::from(formatter_kind(
            secax.xaxis().major_formatter().as_ref(),
        )))
    })
}

pub(super) fn secondary_ticks(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let location = SecondaryLocation::Fraction(1.0);
        let axis = if args.str("orientation")? == "x" {
            let secax = made(ax.secondary_xaxis(location, None))?;
            secax.set_xticks(vec![0.0, 1.0]);
            secax.xaxis()
        } else {
            let secax = made(ax.secondary_yaxis(location, None))?;
            secax.set_yticks(vec![0.0, 1.0]);
            secax.yaxis()
        };
        Ok(Json::from(locator_kind(axis.major_locator().as_ref())))
    })
}

pub(super) fn secondary_color(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let location = SecondaryLocation::Fraction(1.0);
        let (secax, axis) = if args.str("orientation")? == "x" {
            let secax = made(ax.secondary_xaxis(location, None))?;
            let axis = secax.xaxis();
            (secax, axis)
        } else {
            let secax = made(ax.secondary_yaxis(location, None))?;
            let axis = secax.yaxis();
            (secax, axis)
        };
        // matplotlib's `color` keyword calls `set_color` once the axes is
        // made, as here for both spellings.
        axis.set_color("red");
        let spines = secax
            .spines()
            .iter()
            .map(|spine| {
                Ok((
                    spine.side().name().to_owned(),
                    resolved_rgba(&spine.edgecolor())?,
                ))
            })
            .collect::<Result<Vec<_>, ProbeError>>()?;
        let major = axis.tick_style(false);
        Ok(encode::object([
            ("spines", Json::Object(spines)),
            ("color", resolved_rgba(&major.color)?),
            ("labelcolor", resolved_rgba(&major.label_color)?),
            ("label", resolved_rgba(&axis.label_color())?),
            ("minor", resolved_rgba(&axis.tick_style(true).color)?),
        ]))
    })
}
