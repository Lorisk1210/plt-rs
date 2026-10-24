//! Probe operations for the `mpl_marker` area: matplotlib's own non-image
//! tests of marker styles that the `markers` area leaves out, mathtext
//! markers and a few invalid ones (see
//! `spec/reference/probes/test_mpl_marker.py`). Markers are decoded and
//! derived as in the `markers` area.

use super::markers::{apply_step, build};
use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode;
use crate::probes::{Args, Op, Outcome};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("MarkerStyle", marker_style)];

/// `{filled, fillstyle, joinstyle, capstyle, user_transform}` of a marker
/// after its derivation steps.
fn marker_style(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let mut marker = build(args, None)?;
        for step in args.get("steps").and_then(Json::as_array).unwrap_or(&[]) {
            marker = apply_step(&marker, step)?;
        }
        let user = marker
            .user_transform()
            .as_ref()
            .map_or(Json::Null, encode::matrix);
        Ok(Json::Object(vec![
            ("filled".into(), marker.is_filled().into()),
            ("fillstyle".into(), marker.fillstyle().name().into()),
            ("joinstyle".into(), marker.joinstyle().name().into()),
            ("capstyle".into(), marker.capstyle().name().into()),
            ("user_transform".into(), user),
        ]))
    })
}
