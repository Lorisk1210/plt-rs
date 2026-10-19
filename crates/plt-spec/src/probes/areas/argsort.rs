//! Probe operations for the `argsort` area: numpy's unstable `argsort` of
//! `float64` arrays, equal values included.

use crate::json::Json;
use crate::probes::{Args, Op, Outcome};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("numpy.argsort", argsort)];

/// The indices that sort `values`.
fn argsort(args: Args<'_>) -> Outcome {
    let values = args.f64_vec("values")?;
    Ok(Json::Array(
        plt::numeric::argsort(&values)
            .into_iter()
            .map(|i| Json::Number(i as f64))
            .collect(),
    ))
}
