//! The pixel checks: a PNG against matplotlib's, and the frames of two
//! animation pages, frame by frame.

use std::error::Error;
use std::path::Path;

use pltrs::testing::compare_images;

use super::{TOLERANCE, Verdict};

/// Compares the PNG `actual` with matplotlib's `expected`. A difference
/// image is written beside `actual` when they differ.
pub(in super::super) fn compare_pngs(expected: &Path, actual: &Path) -> Verdict {
    match compare_images(expected, actual, TOLERANCE) {
        Ok(comparison) if comparison.matches() => Verdict::Same(comparison.rms),
        Ok(comparison) => {
            Verdict::Different(format!("RMS {:.2}, over {TOLERANCE}", comparison.rms))
        }
        Err(err) => Verdict::Incomparable(err.to_string()),
    }
}

/// Compares the frames embedded in the animation page `actual` with
/// those of matplotlib's page `expected`: as many frames, each within
/// [`TOLERANCE`]. The frames are written as PNG files into `frames`
/// (`<expected|actual>_<n>.png`); the verdict's measure is the largest
/// RMS difference.
pub(in super::super) fn compare_frames(expected: &Path, actual: &Path, frames: &Path) -> Verdict {
    match frame_pairs(expected, actual, frames) {
        Ok(pairs) => pairs
            .iter()
            .enumerate()
            .map(
                |(n, (expected, actual))| match compare_pngs(expected, actual) {
                    Verdict::Same(rms) => Verdict::Same(rms),
                    Verdict::Different(why) => Verdict::Different(format!("frame {n}: {why}")),
                    Verdict::Incomparable(why) => {
                        Verdict::Incomparable(format!("frame {n}: {why}"))
                    }
                },
            )
            .try_fold(Verdict::Same(0.0), |worst, verdict| {
                match (worst, verdict) {
                    (Verdict::Same(a), Verdict::Same(b)) => Ok(Verdict::Same(a.max(b))),
                    (_, failed) => Err(failed),
                }
            })
            .unwrap_or_else(|failed| failed),
        Err(err) => Verdict::Incomparable(err.to_string()),
    }
}

/// The frames of both pages written to `dir`, paired, when the pages hold
/// as many.
fn frame_pairs(
    expected: &Path,
    actual: &Path,
    dir: &Path,
) -> Result<Vec<(std::path::PathBuf, std::path::PathBuf)>, Box<dyn Error>> {
    let expected = embedded_frames(&std::fs::read_to_string(expected)?)?;
    let actual = embedded_frames(&std::fs::read_to_string(actual)?)?;
    if expected.len() != actual.len() || expected.is_empty() {
        return Err(format!(
            "{} frames against matplotlib's {}",
            actual.len(),
            expected.len()
        )
        .into());
    }
    std::fs::create_dir_all(dir)?;
    expected
        .iter()
        .zip(&actual)
        .enumerate()
        .map(|(n, (e, a))| {
            let paths = (
                dir.join(format!("expected_{n}.png")),
                dir.join(format!("actual_{n}.png")),
            );
            std::fs::write(&paths.0, e)?;
            std::fs::write(&paths.1, a)?;
            Ok(paths)
        })
        .collect()
}

/// The PNG frames embedded in an HTML page of matplotlib's JavaScript
/// player: `frames[n] = "data:image/png;base64,..."`, the base64 text
/// broken into lines ending in a backslash.
fn embedded_frames(page: &str) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    const START: &str = "\"data:image/png;base64,";
    page.split(START)
        .skip(1)
        .map(|rest| {
            let end = rest.find('"').ok_or("an unterminated frame")?;
            let text: String = rest[..end].chars().filter(|&c| c != '\\').collect();
            Ok(pltrs_codec::base64::decode(&text)?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_read_from_the_page() {
        let page = "frames[0] = \"data:image/png;base64,cGx0\\\ncw==\"\n\
                    frames[1] = \"data:image/png;base64,cGw=\"\n";
        let frames = embedded_frames(page).expect("two frames");
        assert_eq!(frames, [b"plts".to_vec(), b"pl".to_vec()]);
        assert!(embedded_frames("\"data:image/png;base64,cGx0").is_err());
        assert_eq!(
            embedded_frames("no frames").expect("none"),
            Vec::<Vec<u8>>::new()
        );
    }
}
