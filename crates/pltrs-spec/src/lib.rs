//! Conformance harness comparing pltrs against matplotlib-generated goldens.
//!
//! The specification lives in `spec/` at the repository root: a pytest
//! suite executed against real matplotlib writes *goldens* that this crate
//! replays against pltrs.
//!
//! * [`probes`] — `op(args) -> value` observations per functional area.
//! * [`golden_path`], [`PLATFORM`], [`level_platform`] — the goldens are
//!   matplotlib's on Linux x86-64 with AVX-512; a platform or a processor
//!   level where matplotlib computes something else has its own for what
//!   differs, which this crate prefers there.
//! * [`figures`] — Rust translations of the figure cases.
//! * [`json`] / [`compare`] — golden parsing and tolerant comparison.
//! * [`rng`] — the deterministic data generator shared with the Python side.
//! * `user_units` — unit classes of matplotlib's tests and gallery written
//!   against pltrs's unit API.
//!
//! A few of matplotlib's tests check something internal to one crate and
//! are ported as that crate's unit tests rather than as probes, as
//! matplotlib cannot record them here: `test_backend_webagg.py`'s
//! `test_websocket_rejects_cross_origin` (the WebSocket handshake of
//! `pltrs::web`, `crates/pltrs/src/web/websocket`) and
//! `test_toolbar_button_dispatch_allowlist` (its canvas,
//! `crates/pltrs/src/web/canvas/tests.rs`); both need Tornado, which the
//! reference venv does not have. `test_text.py`'s
//! `test_single_artist_usenotex` only checks that saving succeeds, which
//! `crates/pltrs/src/text/tex.rs` tests without TeX, and `test_axes.py`'s
//! `test_latex_pie_percent` that two autopct templates give the same
//! labels under TeX, which `crates/pltrs/src/axes/pie.rs` tests.
#![forbid(unsafe_code)]

pub mod compare;
pub mod figures;
mod fonts;
pub mod json;
pub mod probes;
pub mod rng;
mod scratch;
mod user_units;

use std::path::{Path, PathBuf};

use pltrs::numeric::CpuLevel;

/// Puts the process and the current thread in the configuration the
/// goldens assume (`reference_rc` in `spec/reference/conftest.py`):
/// matplotlib's test setup ([`pltrs::testing::setup`], which also keeps the
/// developer's own `pltrsrc` and style library out) with unhinted glyph
/// outlines.
pub fn reference_rc() {
    pltrs::testing::setup();
    pltrs::rc::update(|params| params.text_hinting = pltrs::text::TextHinting::None);
}

/// Root of the `spec/` directory of this repository.
pub fn spec_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// Directory holding the probe golden files.
pub fn probe_goldens_dir() -> PathBuf {
    spec_root().join("goldens/probes")
}

/// The directory, below `spec/goldens/platforms`, of the goldens of the
/// target this is built for, or `None` where pltrs is held to the common
/// goldens alone: Linux x86-64, where they are recorded, and every target
/// without goldens of its own so far.
///
/// matplotlib does not compute the same everywhere (its arm64 macOS wheel
/// fuses products into sums, each platform has its own maths library), so
/// where its output on a platform differs from a common golden, the
/// reference runner records it there under the same relative path (only
/// what differs; see `spec/reference/spec_support/golden.py`, whose
/// `_platform` this mirrors).
pub const PLATFORM: Option<&str> = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    Some("aarch64-apple-darwin")
} else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
    Some("x86_64-apple-darwin")
} else {
    None
};

/// The directory, below `spec/goldens/platforms`, of the goldens of the
/// processor level pltrs computes at, or `None` at the level the common
/// goldens are recorded at (`X86_V4`) and off x86-64.
///
/// numpy runs another copy of its loops on a processor with AVX2 alone or
/// without it than with AVX-512 (see [`CpuLevel`]), and a target triple
/// does not say which a machine has, so what matplotlib computes
/// otherwise there is recorded by level, for every x86-64 system
/// (`_level` of `spec/reference/spec_support/golden.py`, which this
/// mirrors). `PLTRS_CPU_LEVEL`, or `cargo xtask spec --level`, holds pltrs
/// to another level's goldens on an AVX-512 machine.
pub fn level_platform() -> Option<&'static str> {
    if !cfg!(target_arch = "x86_64") {
        return None;
    }
    match CpuLevel::current() {
        CpuLevel::Baseline => Some("x86_64-v2"),
        CpuLevel::V3 => Some("x86_64-v3"),
        CpuLevel::V4 => None,
    }
}

/// The goldens recorded for `relative` (a path below `spec/goldens`)
/// that this build is held to before the common one: its target's
/// ([`PLATFORM`]), then its processor level's ([`level_platform`]).
pub fn platform_goldens(relative: impl AsRef<Path>) -> impl Iterator<Item = PathBuf> {
    let platforms = spec_root().join("goldens/platforms");
    [PLATFORM, level_platform()]
        .into_iter()
        .flatten()
        .map(move |platform| platforms.join(platform).join(relative.as_ref()))
        .filter(|own| own.exists())
}

/// The golden `relative` (a path below `spec/goldens`) this build is held
/// to: its target's where one is recorded, else its processor level's,
/// else the common one.
pub fn golden_path(relative: impl AsRef<Path>) -> PathBuf {
    let relative = relative.as_ref();
    platform_goldens(relative)
        .next()
        .unwrap_or_else(|| spec_root().join("goldens").join(relative))
}
