//! The correctness guard: before a workload is timed, its figure is saved
//! as PNG by every implementation and compared with matplotlib's, so that
//! a timing of different work is caught.

use std::fmt;
use std::path::Path;

use plt::testing::compare_images;

/// The largest RMS difference (of 0–255 channel values) accepted between
/// two renderings of the same figure. Loose: the conformance spec holds
/// plt-rs to 1.0 against matplotlib under its reference configuration,
/// and a figure missing a title or drawing a line in another colour
/// differs by several units.
pub(super) const TOLERANCE: f64 = 2.0;

/// How a PNG compares with matplotlib's.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Verdict {
    /// The same figure: the RMS difference is within [`TOLERANCE`].
    Same {
        /// The RMS difference.
        rms: f64,
    },
    /// A different figure: the RMS difference exceeds [`TOLERANCE`].
    Different {
        /// The RMS difference.
        rms: f64,
    },
    /// The images could not be compared (different sizes, unreadable).
    Incomparable(String),
}

impl Verdict {
    /// Whether the figure diverges from matplotlib's.
    pub(super) fn diverges(&self) -> bool {
        !matches!(self, Self::Same { .. })
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Same { rms } => write!(f, "RMS {rms:.2}"),
            Self::Different { rms } => write!(f, "RMS {rms:.2}, over {TOLERANCE}"),
            Self::Incomparable(why) => f.write_str(why),
        }
    }
}

/// Compares the PNG `actual` with matplotlib's `expected`. A difference
/// image is written beside `actual` when they differ.
pub(super) fn compare(expected: &Path, actual: &Path) -> Verdict {
    match compare_images(expected, actual, TOLERANCE) {
        Ok(comparison) if comparison.matches() => Verdict::Same {
            rms: comparison.rms,
        },
        Ok(comparison) => Verdict::Different {
            rms: comparison.rms,
        },
        Err(err) => Verdict::Incomparable(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_same_figure_passes() {
        assert!(!Verdict::Same { rms: 0.5 }.diverges());
        assert!(Verdict::Different { rms: 9.0 }.diverges());
        assert!(Verdict::Incomparable("sizes".to_owned()).diverges());
        assert_eq!(
            Verdict::Different { rms: 9.0 }.to_string(),
            "RMS 9.00, over 2"
        );
    }
}
