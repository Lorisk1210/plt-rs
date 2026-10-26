//! Fixtures shared by the Type 1 unit tests.

use std::fmt::Write;

use super::crypt::{Type1Cipher, encrypt};

/// The bytes of the Type 1 fixture `name` (`cmr10.pfb` or
/// `Courier10PitchBT-Bold.pfb`, from matplotlib's test data).
pub(crate) fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/type1")
        .join(name);
    std::fs::read(path).expect("a Type 1 fixture")
}

/// A minimal PFA font whose encrypted part is a font name, a bounding box
/// and `private`, written as matplotlib's tests write it.
pub(crate) fn pfa(private: &[u8]) -> Vec<u8> {
    let mut plain = b"/FontName /X def\n/FontBBox [0 0 1 1] def\n".to_vec();
    plain.extend_from_slice(private);
    let hex = encrypt(&plain, Type1Cipher::Eexec, 4)
        .iter()
        .fold(String::new(), |mut hex, b| {
            write!(hex, "{b:02x}").expect("writing to a String cannot fail");
            hex
        });
    format!(
        "%!PS-AdobeFont-1.0: X 001.000\neexec\n{hex}\n{}\ncleartomark\n",
        "0".repeat(512)
    )
    .into_bytes()
}
