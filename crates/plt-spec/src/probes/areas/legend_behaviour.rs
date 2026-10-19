//! Probe operations for the `legend_behaviour` area: scenarios ported from
//! matplotlib's legend tests (see
//! `spec/reference/probes/test_legend_behaviour.py` for the upstream test
//! each case mirrors), replayed by the [`scenario`](super::scenario)
//! interpreter.

use super::scenario;
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("legend_behaviour.run", scenario::replay)];
