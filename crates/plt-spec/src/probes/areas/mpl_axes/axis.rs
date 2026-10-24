//! Formatters of an axis, twinned axes, arrows and annotations.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, Axis, AxisName};
use plt::patches::{FancyArrow, PatchShape};
use plt::text::{AnnotateOptions, ArrowProps, LegacyArrowProps};
use plt::ticker::{FuncFormatter, StrMethodFormatter, TickContext};
use plt_core::Point;

use super::{axis_arg, made, numbers, on_axes, pair};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::install_formatter;
use crate::probes::encode::points;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `[x major, x minor, y major, y minor]`: whether each formatter is
/// still the default one.
fn formatter_flags(ax: &Axes) -> Json {
    let flags = [ax.xaxis(), ax.yaxis()].into_iter().flat_map(|axis| {
        [
            axis.is_default_major_formatter(),
            axis.is_default_minor_formatter(),
        ]
    });
    Json::Array(flags.map(Json::from).collect())
}

/// Gives `axis` the major (or minor) formatter argument `formatter`
/// names: a function returning `""` or the template `"{x}_{pos}"`.
fn install_named_formatter(args: Args<'_>, axis: &Axis, minor: bool) -> Result<(), ProbeError> {
    match args.str("formatter")? {
        "func" => install_formatter(axis, minor, FuncFormatter::new(|_, _| String::new())),
        "str" => install_formatter(axis, minor, made(StrMethodFormatter::new("{x}_{pos}"))?),
        other => return Err(ProbeError::BadArgs(format!("unknown formatter {other}"))),
    }
    Ok(())
}

/// The default-formatter flags before and after setting a formatter, and
/// the installed formatter's label for 0.5 at position 1.
fn set_formatter(args: Args<'_>, minor: bool) -> Outcome {
    on_axes(args, |ax| {
        let before = formatter_flags(ax);
        let axis = axis_arg(ax, args)?;
        install_named_formatter(args, &axis, minor)?;
        let installed = if minor {
            axis.minor_formatter()
        } else {
            axis.major_formatter()
        };
        let labels = made(installed.format_ticks(&[0.0, 0.5], &TickContext::default()))?;
        Ok(Json::Object(vec![
            ("before".into(), before),
            ("after".into(), formatter_flags(ax)),
            ("label".into(), Json::from(labels.labels[1].as_str())),
        ]))
    })
}

pub(super) fn set_major_formatter(args: Args<'_>) -> Outcome {
    set_formatter(args, false)
}

pub(super) fn set_minor_formatter(args: Args<'_>) -> Outcome {
    set_formatter(args, true)
}

/// `(axis name, twin)` of argument `twin`.
fn twin_arg(ax: &Axes, args: Args<'_>) -> Result<(AxisName, Axes), ProbeError> {
    match args.str("twin")? {
        "x" => Ok((AxisName::X, ax.twinx())),
        "y" => Ok((AxisName::Y, ax.twiny())),
        other => Err(ProbeError::BadArgs(format!("unknown twin {other}"))),
    }
}

pub(super) fn twin_units(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        made(ax.plot(["0", "1"], ["a", "b"]))?;
        let (name, twin) = twin_arg(ax, args)?;
        let categories = match name {
            AxisName::X => ["1", "0"],
            AxisName::Y => ["b", "a"],
        };
        let shared = twin.axis_of(name);
        Ok(Json::Array(vec![
            Json::from(ax.axis_of(name).have_units()),
            Json::from(shared.have_units()),
            numbers(shared.convert_units(categories)),
        ]))
    })
}

pub(super) fn twin_in_layout(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let (_, twin) = twin_arg(ax, args)?;
        Ok(Json::Array(vec![
            Json::from(ax.in_layout()),
            Json::from(twin.in_layout()),
        ]))
    })
}

pub(super) fn arrow(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let arrow = FancyArrow {
            head_length: Some(args.f64("head_length")?),
            ..FancyArrow::new(
                args.f64("x")?,
                args.f64("y")?,
                args.f64("dx")?,
                args.f64("dy")?,
            )
        };
        let patch = ax.arrow(arrow);
        let vertices: Vec<Point> = patch.shape().path().vertices().to_vec();
        Ok(encode::object([
            ("xy", points(&vertices)),
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
        ]))
    })
}

pub(super) fn annotation_arrow(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = AnnotateOptions {
            xytext: Some(Point::new(2.0, 3.0)),
            arrowprops: args
                .contains("arrowprops")
                .then(|| ArrowProps::Legacy(LegacyArrowProps::default())),
            ..AnnotateOptions::default()
        };
        let note = made(ax.annotate("foo", (0.0, 1.0), options))?;
        Ok(Json::from(note.has_arrow()))
    })
}
