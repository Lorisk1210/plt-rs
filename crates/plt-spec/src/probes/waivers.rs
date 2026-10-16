//! Probes where plt-rs deliberately differs from matplotlib.
//!
//! Each waiver names the probe exactly (operation and canonical arguments)
//! and records *why* reproducing matplotlib would be wrong — typically an
//! accidental behaviour or a matplotlib bug. Waived probes are reported
//! separately and never count as passing.

use crate::json::Json;

/// One documented, intentional deviation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waiver {
    /// Probe operation.
    pub op: &'static str,
    /// The probe's arguments as JSON, exactly as in the golden file.
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
];

/// The waiver covering a probe, if any.
pub fn find(op: &str, args: &Json) -> Option<&'static Waiver> {
    WAIVERS
        .iter()
        .find(|w| w.op == op && Json::parse(w.args).is_ok_and(|waived| waived == *args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_waiver_is_valid_json() {
        for waiver in WAIVERS {
            assert!(Json::parse(waiver.args).is_ok(), "{}: {}", waiver.op, waiver.args);
        }
    }
}
