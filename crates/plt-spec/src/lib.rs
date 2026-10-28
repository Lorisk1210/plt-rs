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
//!
//! A few of matplotlib's tests check something internal to one crate and
//! are ported as that crate's unit tests rather than as probes, as
//! matplotlib cannot record them here: `test_backend_webagg.py`'s
//! `test_websocket_rejects_cross_origin` (the WebSocket handshake of
//! `plt::web`, `crates/plt/src/web/websocket`) and
//! `test_toolbar_button_dispatch_allowlist` (its canvas,
//! `crates/plt/src/web/canvas/tests.rs`); both need Tornado, which the
//! reference venv does not have.
#![forbid(unsafe_code)]

pub mod compare;
pub mod figures;
pub mod json;
pub mod probes;
pub mod rng;
mod scratch;
mod user_units;

use std::path::PathBuf;

/// Puts the process and the current thread in the configuration the
/// goldens assume (`reference_rc` in `spec/reference/conftest.py`):
/// matplotlib's test setup ([`plt::testing::setup`], which also keeps the
/// developer's own `pltrc` and style library out) with unhinted glyph
/// outlines.
pub fn reference_rc() {
    plt::testing::setup();
    plt::rc::update(|params| params.text_hinting = String::from("none"));
}

/// Root of the `spec/` directory of this repository.
pub fn spec_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// Directory holding the probe golden files.
pub fn probe_goldens_dir() -> PathBuf {
    spec_root().join("goldens/probes")
}
