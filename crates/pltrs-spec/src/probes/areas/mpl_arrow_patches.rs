//! Probe operations for the `mpl_arrow_patches` area: matplotlib's own
//! non-image tests of arrow patches (see
//! `spec/reference/probes/test_mpl_arrow_patches.py`).

use pltrs::figure::Figure;
use pltrs::patches::{ConnectionStyle, FancyArrowPatch};

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("FancyArrowPatch", fancy_arrow)];

/// Whether a figure with an arrow connected by `Angle3(angleA, angleB)`
/// draws (`test_invalid_intersection`).
fn fancy_arrow(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let style = ConnectionStyle::Angle3 {
            angle_a: args.f64("angleA")?,
            angle_b: args.f64("angleB")?,
        };
        fig.subplot().add_patch(
            FancyArrowPatch::between((0.2, 0.2), (0.5, 0.5)).with_connectionstyle(style),
        );
        fig.canvas().draw().map_err(ProbeError::rejected)?;
        Ok(Json::Bool(true))
    })
}
