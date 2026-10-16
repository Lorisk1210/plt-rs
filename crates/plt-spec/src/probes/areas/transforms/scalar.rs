//! Scalar range helpers: `_nonsingular` and `_interval_contains*`.

use plt::transforms;

use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// `_nonsingular(vmin, vmax, expander=0.001, tiny=1e-15, increasing=True)`.
pub(super) fn nonsingular(args: Args<'_>) -> Outcome {
    let (lo, hi) = transforms::nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        args.opt_f64("expander")?.unwrap_or(0.001),
        args.opt_f64("tiny")?.unwrap_or(1e-15),
        args.opt_bool("increasing")?.unwrap_or(true),
    );
    Ok(Json::from(vec![lo, hi]))
}

/// `_interval_contains(interval, val)`.
pub(super) fn interval_contains(args: Args<'_>) -> Outcome {
    Ok(Json::from(transforms::interval_contains(
        interval_arg(args)?,
        args.f64("val")?,
    )))
}

/// `_interval_contains_open(interval, val)`.
pub(super) fn interval_contains_open(args: Args<'_>) -> Outcome {
    Ok(Json::from(transforms::interval_contains_open(
        interval_arg(args)?,
        args.f64("val")?,
    )))
}

/// `_interval_contains_close(interval, val, rtol=1e-10)`.
pub(super) fn interval_contains_close(args: Args<'_>) -> Outcome {
    let rtol = args.opt_f64("rtol")?.unwrap_or(1e-10);
    Ok(Json::from(transforms::interval_contains_close(
        interval_arg(args)?,
        args.f64("val")?,
        rtol,
    )))
}

/// The `[a, b]` interval argument.
fn interval_arg(args: Args<'_>) -> Result<(f64, f64), ProbeError> {
    match args.f64_vec("interval")?.as_slice() {
        &[a, b] => Ok((a, b)),
        other => Err(ProbeError::BadArgs(format!(
            "interval {other:?} is not [a, b]"
        ))),
    }
}
