//! Probe operations for the `colorbar_behaviour` area: scenarios ported
//! from matplotlib's colorbar tests (see
//! `spec/reference/probes/test_colorbar_behaviour.py` for the upstream test
//! each case mirrors), replayed by the [`scenario`](super::scenario)
//! interpreter.

use super::scenario;
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("colorbar_behaviour.run", scenario::replay)];
