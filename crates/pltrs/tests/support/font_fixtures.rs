//! The test fixtures of `pltrs-font` that some of pltrs' tests read,
//! shared by the unit tests and the fuzz tests. In the repository they
//! are files of the crate beside this one. A package of pltrs, as
//! crates.io holds it, carries no other crate's files: there the tests
//! that need them say so and skip.

use std::path::Path;

/// The bytes of the fixture `name` of `pltrs-font`'s tests
/// (`tests/fixtures/{name}`), or `None` when the tests run from pltrs'
/// package.
///
/// # Panics
///
/// In the repository, when the fixture cannot be read.
#[allow(
    clippy::print_stderr,
    reason = "the notice of a skip (`allow`: only the fuzz tests flag it)"
)]
pub(crate) fn font_fixture(name: &str) -> Option<Vec<u8>> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // `cargo package` keeps the manifest it was given as `Cargo.toml.orig`.
    if crate_dir.join("Cargo.toml.orig").is_file() {
        eprintln!("pltrs-font's fixture {name} is not in pltrs' package: skipped");
        return None;
    }
    let path = crate_dir.join("../pltrs-font/tests/fixtures").join(name);
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}
