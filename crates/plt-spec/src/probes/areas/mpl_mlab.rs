//! Probe operations for the `mpl_mlab` area: the module-level tests of
//! matplotlib's `tests/test_mlab.py` (see
//! `spec/reference/probes/test_mpl_mlab.py`), evaluated by the `stats`
//! area's operations.

use super::stats::{cohere, psd, window_hanning, window_none};
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("mlab.window_hanning", window_hanning),
    Op::new("mlab.window_none", window_none),
    Op::new("mlab.psd", psd),
    Op::new("mlab.cohere", cohere),
];
