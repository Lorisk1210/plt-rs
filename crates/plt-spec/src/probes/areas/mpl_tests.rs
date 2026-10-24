//! What the probe areas porting matplotlib's own tests (`mpl_*`) share:
//! the conditions matplotlib's test fixture runs a test under (see
//! `spec/reference/probes/_mpl_tests.py`).

use plt::rc::{self, RcParams};
use plt::style;

use super::text_metrics::rc_text;
use crate::json::Json;
use crate::probes::{Args, ProbeError};

/// The test suite's default style: `classic` plus `_classic_test_patch`.
const CLASSIC: [&str; 2] = ["classic", "_classic_test_patch"];

/// Runs `f` in the test style of the probe's arguments: the reference
/// configuration with `classic` and `_classic_test_patch` on top unless
/// `style` is `"mpl20"`, then the `rc` overrides; pyplot's figures are
/// closed afterwards.
pub(crate) fn in_test_style<T>(
    args: Args<'_>,
    f: impl FnOnce() -> Result<T, ProbeError>,
) -> Result<T, ProbeError> {
    rc::context(|| {
        let mut params = RcParams::matplotlib();
        // `matplotlib.testing.setup` names the font family, salts the SVG
        // ids and turns hinting off.
        params
            .set("font.family", "DejaVu Sans")
            .map_err(ProbeError::rejected)?;
        params
            .set("svg.hashsalt", "matplotlib")
            .map_err(ProbeError::rejected)?;
        params
            .set("text.hinting", "none")
            .map_err(ProbeError::rejected)?;
        match args.opt_str("style")? {
            None => {
                for name in CLASSIC {
                    style::sheet(name)
                        .ok_or_else(|| ProbeError::BadArgs(format!("no style {name}")))?
                        .apply(&mut params)
                        .map_err(ProbeError::rejected)?;
                }
            }
            Some("mpl20") => {}
            Some(other) => return Err(ProbeError::BadArgs(format!("unknown style {other}"))),
        }
        rc::replace(params);
        for (key, value) in args.get("rc").and_then(Json::as_object).unwrap_or(&[]) {
            rc::set(key, &rc_text(value)?).map_err(ProbeError::rejected)?;
        }
        plt::pyplot::close_all();
        let result = f();
        plt::pyplot::close_all();
        result
    })
}

/// A JSON list of numbers.
pub(crate) fn numbers(values: impl IntoIterator<Item = f64>) -> Json {
    Json::Array(values.into_iter().map(Json::from).collect())
}
