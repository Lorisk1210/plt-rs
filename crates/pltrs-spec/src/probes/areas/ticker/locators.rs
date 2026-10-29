//! Locator adapters of the `ticker` area.

use pltrs::ticker::{
    AsinhLocator, AutoLocator, AutoMinorLocator, AxisInfo, AxisName, FixedLocator, IndexLocator,
    LinearLocator, Locator, LogLocator, LogSubs, LogitLocator, MaxNLocator, MultipleLocator, Nbins,
    NullLocator, NumTicks, Prune, SymmetricalLogLocator, TickContext, TickerSettings,
};

use super::{
    Params, floats, minor_divisions, pair, pair_arg, settings, tick_values, usize_arg, view_axis,
    view_limits,
};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

pub(super) fn locator_nonsingular(args: Args<'_>) -> Outcome {
    let (lo, hi) =
        NullLocator.nonsingular(args.f64("v0")?, args.f64("v1")?, &TickContext::default());
    Ok(Json::from(vec![lo, hi]))
}

fn nbins(value: Option<&Json>) -> Result<Option<Nbins>, ProbeError> {
    value
        .filter(|v| !v.is_null())
        .map(|v| match (v.as_str(), v.as_i64()) {
            (Some("auto"), _) => Ok(Nbins::Auto),
            (_, Some(n)) => usize::try_from(n)
                .map(Nbins::Fixed)
                .map_err(|_| ProbeError::Rejected("negative nbins".into())),
            _ => Err(ProbeError::Rejected(format!("invalid nbins {v}"))),
        })
        .transpose()
}

fn max_n(args: Args<'_>) -> Result<MaxNLocator, ProbeError> {
    let params = Params::of(
        args,
        &[
            "nbins",
            "steps",
            "integer",
            "symmetric",
            "prune",
            "min_n_ticks",
        ],
    )?;
    let p = params.args();
    let mut locator = MaxNLocator::new(nbins(params.raw("nbins"))?.unwrap_or(Nbins::Fixed(10)));
    if let Some(steps) = p.get("steps") {
        let steps = steps
            .as_f64_vec()
            .ok_or_else(|| ProbeError::Rejected("steps must be a sequence".into()))?;
        locator = locator.with_steps(&steps).map_err(ProbeError::rejected)?;
    }
    let prune = match p.opt_str("prune")? {
        None => None,
        Some("lower") => Some(Prune::Lower),
        Some("upper") => Some(Prune::Upper),
        Some("both") => Some(Prune::Both),
        Some(other) => return Err(ProbeError::Rejected(format!("invalid prune {other:?}"))),
    };
    let mut locator = locator
        .with_integer(p.opt_bool("integer")?.unwrap_or(false))
        .with_symmetric(p.opt_bool("symmetric")?.unwrap_or(false))
        .with_prune(prune);
    if let Some(n) = p.opt_i64("min_n_ticks")? {
        locator = locator.with_min_n_ticks(usize::try_from(n).unwrap_or(0));
    }
    Ok(locator)
}

pub(crate) fn maxn_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&max_n(args)?, args)
}

pub(crate) fn maxn_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&max_n(args)?, args, "dmin", "dmax")
}

pub(super) fn auto_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&AutoLocator::from_settings(&settings(args)?), args)
}

pub(super) fn auto_view_limits(args: Args<'_>) -> Outcome {
    view_limits(
        &AutoLocator::from_settings(&settings(args)?),
        args,
        "dmin",
        "dmax",
    )
}

fn multiple(args: Args<'_>) -> Result<MultipleLocator, ProbeError> {
    let p = Params::of(args, &["base", "offset"])?.args();
    Ok(MultipleLocator::new(p.opt_f64("base")?.unwrap_or(1.0))
        .map_err(ProbeError::rejected)?
        .with_offset(p.opt_f64("offset")?.unwrap_or(0.0)))
}

pub(crate) fn multiple_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&multiple(args)?, args)
}

pub(crate) fn multiple_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&multiple(args)?, args, "dmin", "dmax")
}

pub(super) fn fixed_tick_values(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["locs", "nbins"])?.args();
    let locs = p
        .json("locs")?
        .as_f64_vec()
        .ok_or_else(|| ProbeError::Rejected("locs must be one-dimensional".into()))?;
    let mut locator = FixedLocator::new(locs);
    if let Some(n) = usize_arg(p.opt_i64("nbins")?, "nbins")? {
        locator = locator.with_nbins(n);
    }
    floats(locator.locate(&TickContext::default()))
}

fn linear(args: Args<'_>) -> Result<LinearLocator, ProbeError> {
    let p = Params::of(args, &["numticks"])?.args();
    Ok(LinearLocator::new(usize_arg(
        p.opt_i64("numticks")?,
        "numticks",
    )?))
}

pub(super) fn linear_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&linear(args)?, args)
}

pub(super) fn linear_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&linear(args)?, args, "vmin", "vmax")
}

pub(crate) fn index_tick_values(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["base", "offset"])?.args();
    tick_values(&IndexLocator::new(p.f64("base")?, p.f64("offset")?), args)
}

pub(super) fn null_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&NullLocator, args)
}

pub(crate) fn log_locator(args: Args<'_>) -> Result<LogLocator, ProbeError> {
    let params = Params::of(args, &["base", "subs", "numticks"])?;
    let p = params.args();
    let mut locator =
        LogLocator::new(p.opt_f64("base")?.unwrap_or(10.0)).map_err(ProbeError::rejected)?;
    if let Some(subs) = params.raw("subs") {
        let subs = match (subs, subs.as_str()) {
            (Json::Null, _) | (_, Some("auto")) => LogSubs::Auto,
            (_, Some("all")) => LogSubs::All,
            _ => LogSubs::Explicit(
                subs.as_f64_vec()
                    .ok_or_else(|| ProbeError::Rejected(format!("invalid subs {subs}")))?,
            ),
        };
        locator = locator.with_subs(subs);
    }
    let numticks = match nbins(params.raw("numticks"))? {
        None | Some(Nbins::Auto) => NumTicks::Auto,
        Some(Nbins::Fixed(n)) => NumTicks::Fixed(n),
    };
    Ok(locator.with_numticks(numticks))
}

/// A dummy axis with the probe's `minpos`.
fn minpos_axis(args: Args<'_>) -> Result<AxisInfo, ProbeError> {
    Ok(AxisInfo {
        minpos: args.f64("minpos")?,
        ..AxisInfo::DUMMY
    })
}

pub(crate) fn log_tick_values(args: Args<'_>) -> Outcome {
    let locator = log_locator(args)?;
    if args.get("minpos").is_none() {
        return tick_values(&locator, args);
    }
    let axis = minpos_axis(args)?;
    let settings = settings(args)?;
    let ctx = TickContext::new(&axis, &settings);
    floats(locator.tick_values(args.f64("vmin")?, args.f64("vmax")?, &ctx))
}

pub(super) fn log_view_limits(args: Args<'_>) -> Outcome {
    let locator = log_locator(args)?;
    let axis = minpos_axis(args)?;
    let settings = settings(args)?;
    let ctx = TickContext::new(&axis, &settings);
    pair(locator.view_limits(args.f64("vmin")?, args.f64("vmax")?, &ctx))
}

pub(super) fn log_nonsingular(args: Args<'_>) -> Outcome {
    let locator = log_locator(args)?;
    let axis = minpos_axis(args)?;
    let settings = TickerSettings::default();
    let ctx = TickContext::new(&axis, &settings);
    pair(Ok(locator.nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &ctx,
    )))
}

pub(crate) fn symlog(args: Args<'_>) -> Result<SymmetricalLogLocator, ProbeError> {
    let p = Params::of(args, &["base", "linthresh", "subs", "numticks"])?.args();
    let (Some(base), Some(linthresh)) = (p.opt_f64("base")?, p.opt_f64("linthresh")?) else {
        return Err(ProbeError::Rejected(
            "either transform, or both linthresh and base, must be provided".into(),
        ));
    };
    let mut locator = SymmetricalLogLocator::new(base, linthresh);
    if let Some(subs) = p.get("subs") {
        locator = locator.with_subs(
            subs.as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs("subs".into()))?,
        );
    }
    if let Some(n) = usize_arg(p.opt_i64("numticks")?, "numticks")? {
        locator = locator.with_numticks(n);
    }
    Ok(locator)
}

pub(crate) fn symlog_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&symlog(args)?, args)
}

pub(crate) fn symlog_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&symlog(args)?, args, "vmin", "vmax")
}

fn asinh(args: Args<'_>) -> Result<AsinhLocator, ProbeError> {
    let p = Params::of(
        args,
        &["linear_width", "numticks", "symthresh", "base", "subs"],
    )?
    .args();
    let mut locator = AsinhLocator::new(p.f64("linear_width")?);
    if let Some(n) = usize_arg(p.opt_i64("numticks")?, "numticks")? {
        locator = locator.with_numticks(n);
    }
    if let Some(symthresh) = p.opt_f64("symthresh")? {
        locator = locator.with_symthresh(symthresh);
    }
    if let Some(base) = p.opt_f64("base")? {
        locator = locator.with_base(base);
    }
    if let Some(subs) = p.get("subs") {
        let subs = subs
            .as_f64_vec()
            .ok_or_else(|| ProbeError::BadArgs("subs".into()))?;
        locator = locator.with_subs(subs);
    }
    Ok(locator)
}

pub(crate) fn asinh_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&asinh(args)?, args)
}

pub(crate) fn asinh_call(args: Args<'_>) -> Outcome {
    let axis = view_axis(args)?;
    let settings = TickerSettings::default();
    floats(asinh(args)?.locate(&TickContext::new(&axis, &settings)))
}

fn logit_locator(args: Args<'_>) -> Result<LogitLocator, ProbeError> {
    let params = Params::of(args, &["minor", "nbins"])?;
    let mut locator = LogitLocator::new(params.args().opt_bool("minor")?.unwrap_or(false));
    if let Some(n) = nbins(params.raw("nbins"))? {
        locator = locator.with_nbins(n);
    }
    Ok(locator)
}

pub(crate) fn logit_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&logit_locator(args)?, args)
}

pub(crate) fn logit_nonsingular(args: Args<'_>) -> Outcome {
    let locator = logit_locator(args)?;
    let ctx = TickContext::default();
    pair(Ok(locator.nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &ctx,
    )))
}

pub(super) fn auto_minor_call(args: Args<'_>) -> Outcome {
    let params = Params::of(args, &["n"])?;
    let ndivs = params
        .raw("n")
        .filter(|n| !n.is_null())
        .map(minor_divisions)
        .transpose()?;
    let (vmin, vmax) = pair_arg(args, "view")?;
    let axis = AxisInfo {
        name: match args.str("axis_name")? {
            "y" => AxisName::Y,
            _ => AxisName::X,
        },
        view_interval: (vmin, vmax),
        major_ticks: args.f64_vec("major_locs")?,
        ..AxisInfo::DUMMY
    };
    let settings = settings(args)?;
    floats(AutoMinorLocator::new(ndivs).locate(&TickContext::new(&axis, &settings)))
}
