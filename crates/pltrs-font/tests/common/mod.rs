//! Shared helpers for the integration tests: fixture files (read with
//! `pltrs_core::json`) and font loading.

#![allow(
    dead_code,
    reason = "each test binary uses a different subset of the helpers"
)]

pub(crate) mod json;

use std::path::PathBuf;

use json::Json;

/// The path of the fixture file `name`.
pub(crate) fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Reads and parses `tests/fixtures/<name>`.
pub(crate) fn fixture(name: &str) -> Json {
    let path = fixture_path(name);
    json::parse(
        &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
}

/// Bytes of a bundled font file.
pub(crate) fn font_bytes(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fonts")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Converts recorded outline commands to matplotlib path codes with
/// coordinates multiplied by `scale` and rounded: `[code, x, y, ...]`.
pub(crate) fn path_codes(commands: &[pltrs_font::PathCommand], scale: f64) -> Vec<i64> {
    use pltrs_font::PathCommand;
    let s = |v: f32| (f64::from(v) * scale).round() as i64;
    let mut out = Vec::new();
    for command in commands {
        match *command {
            PathCommand::MoveTo(x, y) => out.extend([1, s(x), s(y)]),
            PathCommand::LineTo(x, y) => out.extend([2, s(x), s(y)]),
            PathCommand::QuadTo(x1, y1, x, y) => out.extend([3, s(x1), s(y1), 3, s(x), s(y)]),
            PathCommand::CurveTo(x1, y1, x2, y2, x, y) => {
                out.extend([4, s(x1), s(y1), 4, s(x2), s(y2), 4, s(x), s(y)]);
            }
            PathCommand::Close => out.extend([79, 0, 0]),
        }
    }
    out
}
