//! Probe operations for the `colormap_luts` area: the complete lookup
//! table of built-in colormaps, verifying the embedded data.

use super::colormaps::{lut_rows, registered};
use crate::probes::{Args, Op, Outcome};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("colormaps.lut", lut)];

fn lut(args: Args<'_>) -> Outcome {
    Ok(lut_rows(&registered(args, "name")?))
}
