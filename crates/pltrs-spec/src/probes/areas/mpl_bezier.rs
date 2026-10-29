//! Probe operations for the `mpl_bezier` area: Bézier utilities ported
//! from matplotlib's `tests/test_bezier.py` (see
//! `spec/reference/probes/test_mpl_bezier.py`).

use pltrs_core::bezier::real_roots_in_01;

use super::path::split_bezier;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("bezier._real_roots_in_01", roots),
    Op::new(
        "bezier.split_bezier_intersecting_with_closedpath",
        split_bezier,
    ),
];

fn roots(args: Args<'_>) -> Outcome {
    Ok(Json::from(real_roots_in_01(&args.f64_vec("coeffs")?)))
}
