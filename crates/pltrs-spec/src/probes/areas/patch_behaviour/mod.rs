//! Probe operations for the `patch_behaviour` area: patch geometry
//! (corners, axis ends, outlines, hit tests, setters) and the resolution
//! of patch colours, ported from matplotlib's `test_patches.py` (see
//! `spec/reference/probes/test_patch_behaviour.py`). Every op runs under
//! the reference configuration.

mod geometry;
mod style;

use super::text_metrics::with_rc;
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("patch.geometry", |args| {
        with_rc(args, || geometry::geometry(args))
    }),
    Op::new("patch.style", |args| with_rc(args, || style::style(args))),
];
