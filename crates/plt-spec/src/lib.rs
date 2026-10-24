//! Conformance harness comparing plt-rs against matplotlib-generated goldens.
//!
//! The specification lives in `spec/` at the repository root: a pytest
//! suite executed against real matplotlib writes *goldens* that this crate
//! replays against plt-rs.
//!
//! * [`probes`] — `op(args) -> value` observations per functional area.
//! * [`figures`] — Rust translations of the figure cases.
//! * [`json`] / [`compare`] — golden parsing and tolerant comparison.
//! * [`rng`] — the deterministic data generator shared with the Python side.
//! * `user_units` — unit classes of matplotlib's tests and gallery written
//!   against plt's unit API.
#![forbid(unsafe_code)]

pub mod compare;
pub mod figures;
pub mod json;
pub mod probes;
pub mod rng;
mod scratch;
mod user_units;

use std::path::PathBuf;

/// Root of the `spec/` directory of this repository.
pub fn spec_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// Directory holding the probe golden files.
pub fn probe_goldens_dir() -> PathBuf {
    spec_root().join("goldens/probes")
}
