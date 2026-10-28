//! Probe operations for the `mpl_backend_tools` area: the shortcuts of the
//! help window, ported from matplotlib's `tests/test_backend_tools.py`
//! (see `spec/reference/probes/test_mpl_backend_tools.py`).

use plt::gui::format_shortcut;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new(
    "backend_tools.ToolHelpBase.format_shortcut",
    shortcut,
)];

fn shortcut(args: Args<'_>) -> Outcome {
    Ok(Json::from(format_shortcut(args.str("key_sequence")?)))
}
