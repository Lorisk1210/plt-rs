//! Probes where plt-rs deliberately differs from matplotlib.
//!
//! Each waiver names the probe (operation and the arguments that single it
//! out) and records *why* reproducing matplotlib would be wrong — typically an
//! accidental behaviour or a matplotlib bug. Waived probes are reported
//! separately and never count as passing.

use crate::compare::{Tolerance, compare};
use crate::json::Json;

/// One documented, intentional deviation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waiver {
    /// Probe operation.
    pub op: &'static str,
    /// A JSON object of probe arguments, as in the golden file; arguments
    /// not listed are not compared.
    pub args: &'static str,
    /// Why plt-rs differs.
    pub reason: &'static str,
}

const NON_INTEGER_SIDES: &str = "matplotlib accidentally accepts negative and fractional \
     polygon side counts (numpy arange semantics); plt-rs requires a positive integer";
const NUMPY_SIMD_SINH: &str = "numpy's AVX-512 sinh differs from correctly rounded libm in the \
     last bit, which flips a floor(log) decision in AsinhLocator on this machine; plt-rs \
     matches glibc/musl and numpy without AVX-512";
const COMPOSITE_USER_TRANSFORM: &str = "matplotlib raises AttributeError when rotating or \
     scaling a marker whose user transform became composite (a bug); plt-rs composes normally";
const OPENBLAS_FMA: &str = "the result depends on whether OpenBLAS uses fused multiply-add on \
     this machine (confirmed); plt-rs evaluates without FMA";
const UNVALIDATED_CODES: &str = "matplotlib does not validate path codes; plt-rs rejects \
     undefined codes";
const NAN_EXTENTS: &str = "numpy's min lets NaN leak into the extents; plt-rs's extents \
     skip non-finite vertices as documented";
const ZERO_STEPS: &str = "interpolating with zero steps is an accident of numpy's \
     arithmetic; plt-rs requires a positive step count";
const READONLY_FLAG: &str = "Python marks shared cached paths read-only; plt-rs paths are \
     plain values, so the flag does not exist";
const RGBA_ARRAY_FAST_PATH: &str = "matplotlib's to_rgba_array fast path for homogeneous \
     numeric lists skips the 0-1 range check every other input applies; plt-rs validates all colours";
const ROUNDING_NOISE_RATIO: &str = "after linear detrending every segment's zero-frequency \
     term is rounding noise (~1e-15), so the coherence there is a ratio of noise that depends \
     on numpy's FFT summation order; every other bin matches";

/// Every waiver, grouped by area.
pub const WAIVERS: &[Waiver] = &[
    Waiver {
        op: "ticker.AsinhLocator.__call__",
        args: r#"{"params": {"linear_width": 10.0, "numticks": 7}, "view": [10.0, 100.0]}"#,
        reason: NUMPY_SIMD_SINH,
    },
    Waiver {
        op: "ticker.AsinhLocator.tick_values",
        args: r#"{"params": {"linear_width": 1.0, "base": 2, "subs": [2, 4, 8]}, "vmin": -3.0, "vmax": 2.0}"#,
        reason: NUMPY_SIMD_SINH,
    },
    Waiver {
        op: "ticker.AsinhLocator.tick_values",
        args: r#"{"params": {"linear_width": 1.0, "base": 2}, "vmin": -3.0, "vmax": 2.0}"#,
        reason: NUMPY_SIMD_SINH,
    },
    Waiver {
        op: "markers.MarkerStyle",
        args: r#"{"marker": [-3, 0], "fillstyle": "full"}"#,
        reason: NON_INTEGER_SIDES,
    },
    Waiver {
        op: "markers.MarkerStyle",
        args: r#"{"marker": [4.5, 0], "fillstyle": "full"}"#,
        reason: NON_INTEGER_SIDES,
    },
    Waiver {
        op: "markers.MarkerStyle.derived",
        args: r#"{"marker": "s", "fillstyle": "full", "transform": [["rotate_deg", 10]], "steps": [["transformed", [["scale", 2]]], ["rotated", {"deg": 5}]]}"#,
        reason: COMPOSITE_USER_TRANSFORM,
    },
    Waiver {
        op: "markers.MarkerStyle.derived",
        args: r#"{"marker": "s", "fillstyle": "full", "transform": [["rotate_deg", 10]], "steps": [["transformed", [["scale", 2]]], ["scaled", 2]]}"#,
        reason: COMPOSITE_USER_TRANSFORM,
    },
    Waiver {
        op: "bezier.find_bezier_t_intersecting_with_closedpath",
        args: r#"{"control_points": [[1e17, 0.0], [1.00000000000001e17, 0.0]], "circle": [0, 0, 1.0000000000000051e17], "t0": 0.0, "t1": 1.0, "tolerance": 1e-30}"#,
        reason: OPENBLAS_FMA,
    },
    Waiver {
        op: "path.Path",
        args: r#"{"vertices": [[0, 0], [1, 2], [2, 0]], "codes": [1, 5, 7], "closed": false}"#,
        reason: UNVALIDATED_CODES,
    },
    Waiver {
        op: "path.Path.get_extents",
        args: r#"{"path": {"vertices": [[0.0, 0.0], [1.0, 1.0], [NaN, 2.0], [3.0, 3.0], [4.0, 4.0], [5.0, 5.0]], "codes": [1, 2, 2, 2, 2, 2]}, "transform": null}"#,
        reason: NAN_EXTENTS,
    },
    Waiver {
        op: "path.Path.get_extents",
        args: r#"{"path": {"vertices": [[0.0, 0.0], [1.0, 1.0], [NaN, 2.0], [3.0, 3.0], [4.0, 4.0], [5.0, NaN], [6.0, 6.0]], "codes": null}, "transform": null}"#,
        reason: NAN_EXTENTS,
    },
    Waiver {
        op: "path.Path.interpolated",
        args: r#"{"path": {"vertices": [[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]], "codes": null}, "steps": 0}"#,
        reason: ZERO_STEPS,
    },
    Waiver {
        op: "path.Path.readonly",
        args: r#"{"constructor": "arc"}"#,
        reason: READONLY_FLAG,
    },
    Waiver {
        op: "path.Path.readonly",
        args: r#"{"constructor": "unit_circle"}"#,
        reason: READONLY_FLAG,
    },
    Waiver {
        op: "path.Path.readonly",
        args: r#"{"constructor": "unit_circle_righthalf"}"#,
        reason: READONLY_FLAG,
    },
    Waiver {
        op: "path.Path.readonly",
        args: r#"{"constructor": "unit_rectangle"}"#,
        reason: READONLY_FLAG,
    },
    Waiver {
        op: "path.Path.readonly",
        args: r#"{"constructor": "unit_regular_polygon"}"#,
        reason: READONLY_FLAG,
    },
    Waiver {
        op: "path.Path.unit_regular_polygon",
        args: r#"{"numVertices": -3}"#,
        reason: NON_INTEGER_SIDES,
    },
    Waiver {
        op: "colors.to_rgba_array",
        args: r#"{"c": [[2, 0, 0, 1], [0, 1, 0, 1]]}"#,
        reason: RGBA_ARRAY_FAST_PATH,
    },
    Waiver {
        op: "colors.to_rgba_array",
        args: r#"{"c": [[2, 0, 0], [0, 1, 0]]}"#,
        reason: RGBA_ARRAY_FAST_PATH,
    },
    Waiver {
        op: "mlab.cohere",
        args: r#"{"NFFT": 32, "detrend": "linear", "window": "none"}"#,
        reason: ROUNDING_NOISE_RATIO,
    },
];

/// The waiver covering a probe, if any: every argument the waiver lists
/// must equal the probe's exactly, with NaN equal to NaN.
pub fn find(op: &str, args: &Json) -> Option<&'static Waiver> {
    WAIVERS
        .iter()
        .find(|w| w.op == op && Json::parse(w.args).is_ok_and(|waived| covers(&waived, args)))
}

/// Whether every entry of the `waived` object equals the same entry of
/// `args`.
fn covers(waived: &Json, args: &Json) -> bool {
    let exact = Tolerance { rel: 0.0, abs: 0.0 };
    waived.as_object().is_some_and(|entries| {
        entries.iter().all(|(key, value)| {
            args.get(key)
                .is_some_and(|actual| compare(value, actual, exact).is_ok())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_arguments_must_match_and_others_are_ignored() {
        let waived = Json::parse(r#"{"a": 1, "b": [NaN]}"#).unwrap();
        let probe = Json::parse(r#"{"a": 1, "b": [NaN], "c": "x"}"#).unwrap();
        assert!(covers(&waived, &probe));
        let other = Json::parse(r#"{"a": 2, "b": [NaN], "c": "x"}"#).unwrap();
        assert!(!covers(&waived, &other));
        let missing = Json::parse(r#"{"a": 1}"#).unwrap();
        assert!(!covers(&waived, &missing));
    }

    #[test]
    fn every_waiver_is_valid_json() {
        for waiver in WAIVERS {
            assert!(
                Json::parse(waiver.args).is_ok(),
                "{}: {}",
                waiver.op,
                waiver.args
            );
        }
    }
}
