//! Probe operations for the `axes_behaviour` area: scenarios ported from
//! matplotlib's axes, subplot and figure tests (see
//! `spec/reference/probes/test_axes_behaviour.py` for the upstream test
//! each case mirrors), replayed by the [`scenario`](super::scenario)
//! interpreter.

use super::scenario;
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("axes_behaviour.run", scenario::replay)];
