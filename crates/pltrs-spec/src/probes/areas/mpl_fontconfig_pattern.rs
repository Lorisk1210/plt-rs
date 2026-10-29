//! Probe operations for the `mpl_fontconfig_pattern` area: fontconfig
//! pattern strings ported from matplotlib's
//! `tests/test_fontconfig_pattern.py` (see
//! `spec/reference/probes/test_mpl_fontconfig_pattern.py`).

use pltrs::fonts::FontProperties;
use pltrs::rc;

use super::text_metrics::{
    font_properties, font_properties_op, parse_fontconfig_pattern, properties_json, with_rc,
};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("font_manager.FontProperties", font_properties_op),
    Op::new("font_manager.FontProperties.round_trip", round_trip),
    Op::new(
        "_fontconfig_pattern.parse_fontconfig_pattern",
        parse_fontconfig_pattern,
    ),
];

/// `FontProperties(str(FontProperties(**kw)))`.
fn round_trip(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let pattern = font_properties(args.json("kw")?)?.fontconfig_pattern();
        let parsed =
            FontProperties::from_pattern(&pattern, &rc::params()).map_err(ProbeError::rejected)?;
        Ok(properties_json(&parsed))
    })
}
