//! Probe operations for the `mpl_scale` area: matplotlib's own tests of
//! axis scales (see `spec/reference/probes/test_mpl_scale.py`). The scale
//! operations are the `scales` area's; the axes are drawn under the
//! reference configuration.

use plt::axes::{Axes, AxisName, Limits};
use plt::figure::Figure;
use plt::scale::{LogScale, ScaleOptions};

use super::scales::{asinh, factory, options, scale};
use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("scale.SymmetricalLogScale.round_trip", |args| {
        round_trip("symlog", args)
    }),
    Op::new("scale.AsinhScale.round_trip", |args| {
        round_trip("asinh", args)
    }),
    Op::new("scale.AsinhScale.transform", asinh::transform),
    Op::new(
        "scale.AsinhScale.default_locators_and_formatters",
        asinh::defaults,
    ),
    Op::new("scale.scale_factory", factory),
    Op::new("axes.Axes.set_yscale.ticks", |args| {
        with_rc(args, || yscale_ticks(args))
    }),
    Op::new("axes.Axes.set_lim", set_lim),
    Op::new("axes.Axes.set_xscale.object", scale_objects),
    Op::new("axes.Axes.get_tightbbox", |args| {
        with_rc(args, || tight_bbox(args))
    }),
];

/// The inverse transform of the forward transform of `values`.
fn round_trip(name: &str, args: Args<'_>) -> Outcome {
    let scale = scale(name, args)?;
    Ok(Json::from(
        scale.inverse(&scale.forward(&args.f64_vec("values")?)),
    ))
}

/// A view interval as recorded.
fn interval((low, high): (f64, f64)) -> Json {
    Json::from(vec![low, high])
}

fn yscale_ticks(args: Args<'_>) -> Outcome {
    let figure = Figure::new();
    let ax = figure.subplot();
    ax.set_scale_with(AxisName::Y, args.str("scale")?, options(args)?)
        .map_err(ProbeError::rejected)?;
    figure.render_raster().map_err(ProbeError::rejected)?;
    let ticks = |minor| {
        ax.yaxis()
            .ticklocs(minor)
            .map(Json::from)
            .map_err(ProbeError::rejected)
    };
    Ok(encode::object([
        ("ylim", interval(ax.ylim())),
        ("major", ticks(false)?),
        ("minor", ticks(true)?),
    ]))
}

/// The view limits of one axis.
fn limits(ax: &Axes, axis: AxisName) -> Json {
    interval(match axis {
        AxisName::X => ax.xlim(),
        AxisName::Y => ax.ylim(),
    })
}

fn set_lim(args: Args<'_>) -> Outcome {
    let axis = match args.str("axis")? {
        "x" => AxisName::X,
        "y" => AxisName::Y,
        other => return Err(ProbeError::BadArgs(format!("unknown axis {other:?}"))),
    };
    let ax = Figure::new().subplot();
    let values = [0.0, 1.0, 2.0, 3.0];
    ax.scatter(values, values).map_err(ProbeError::rejected)?;
    ax.set_scale_with(axis, "log", ScaleOptions::default())
        .map_err(ProbeError::rejected)?;
    let before = limits(&ax, axis);
    let change = Limits {
        low: args.opt_f64("low")?,
        high: args.opt_f64("high")?,
        ..Limits::default()
    };
    ax.set_limits_with(axis, change)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![before, limits(&ax, axis)]))
}

fn scale_objects(args: Args<'_>) -> Outcome {
    if args.str("scale")? != "LogScale" {
        return Err(ProbeError::BadArgs(
            "only LogScale objects are probed".into(),
        ));
    }
    let ax = Figure::new().subplot();
    for axis in [AxisName::X, AxisName::Y] {
        ax.set_scale(
            axis,
            Box::new(LogScale::new(10.0).map_err(ProbeError::rejected)?),
        );
    }
    Ok(Json::from(vec![ax.xscale(), ax.yscale()]))
}

fn tight_bbox(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    ax.plot(args.f64_vec("x")?, args.f64_vec("y")?)
        .map_err(ProbeError::rejected)?;
    ax.set_xscale(args.str("xscale")?)
        .map_err(ProbeError::rejected)?;
    ax.grid(args.bool("grid")?);
    let tight = ax
        .tight_bbox(false)
        .ok_or_else(|| ProbeError::rejected("the axes is invisible"))?;
    Ok(encode::bbox(&tight))
}
