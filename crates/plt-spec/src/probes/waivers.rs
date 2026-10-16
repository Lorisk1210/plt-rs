//! Probes where plt-rs deliberately differs from matplotlib.
//!
//! Each waiver names the probe exactly (operation and canonical arguments)
//! and records *why* reproducing matplotlib would be wrong — typically an
//! accidental behaviour or a matplotlib bug. Waived probes are reported
//! separately and never count as passing.

/// One documented, intentional deviation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waiver {
    /// Probe operation.
    pub op: &'static str,
    /// The probe's arguments, as printed by the JSON serialiser.
    pub args: &'static str,
    /// Why plt-rs differs.
    pub reason: &'static str,
}

const NON_INTEGER_SIDES: &str = "matplotlib accidentally accepts negative and fractional \
     polygon side counts (numpy arange semantics); plt-rs requires a positive integer";
const COMPOSITE_USER_TRANSFORM: &str = "matplotlib raises AttributeError when rotating or \
     scaling a marker whose user transform became composite (a bug); plt-rs composes normally";

/// Every waiver, grouped by area.
pub const WAIVERS: &[Waiver] = &[
    Waiver {
        op: "markers.MarkerStyle",
        args: r#"{"marker": [-3.0, 0.0], "fillstyle": "full"}"#,
        reason: NON_INTEGER_SIDES,
    },
    Waiver {
        op: "markers.MarkerStyle",
        args: r#"{"marker": [4.5, 0.0], "fillstyle": "full"}"#,
        reason: NON_INTEGER_SIDES,
    },
    Waiver {
        op: "markers.MarkerStyle.derived",
        args: r#"{"marker": "s", "fillstyle": "full", "transform": [["rotate_deg", 10.0]], "steps": [["transformed", [["scale", 2.0]]], ["rotated", {"deg": 5.0}]]}"#,
        reason: COMPOSITE_USER_TRANSFORM,
    },
    Waiver {
        op: "markers.MarkerStyle.derived",
        args: r#"{"marker": "s", "fillstyle": "full", "transform": [["rotate_deg", 10.0]], "steps": [["transformed", [["scale", 2.0]]], ["scaled", 2.0]]}"#,
        reason: COMPOSITE_USER_TRANSFORM,
    },
];

/// The waiver covering a probe, if any.
pub fn find(op: &str, args: &str) -> Option<&'static Waiver> {
    WAIVERS.iter().find(|w| w.op == op && w.args == args)
}
