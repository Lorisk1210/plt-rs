//! The correctness guard: before a workload is timed, every
//! implementation saves its figure, and each rendering is compared with
//! matplotlib's, so that a timing of different work is caught.
//!
//! * The pixels: the figure as PNG (cropped as the workload crops it), or
//!   every frame of an animation, against matplotlib's within an RMS
//!   difference of [`TOLERANCE`] (`pixels`).
//! * The structure: an SVG, PDF, PostScript or EPS document, reduced to
//!   its drawing operations with everything they refer to resolved, must
//!   hold the same operations as matplotlib's, their numbers within
//!   [`DEVIATION`] (`structure`, through the conformance spec's
//!   `pltrs_spec::figures::vector`).

mod pixels;
mod structure;

use std::fmt;

pub(super) use pixels::{compare_frames, compare_pngs};
pub(super) use structure::compare_documents;

/// The largest RMS difference (of 0–255 channel values) accepted between
/// two renderings of the same figure. Loose: the conformance spec holds
/// pltrs to 1.0 against matplotlib under its reference configuration,
/// and a figure missing a title or drawing a line in another colour
/// differs by several units.
pub(super) const TOLERANCE: f64 = 2.0;

/// The largest difference accepted between corresponding numbers of two
/// vector documents (in points, or pixels in SVG): a hundredth, a few
/// times the precision the writers print.
pub(super) const DEVIATION: f64 = 0.01;

/// How a rendering compares with matplotlib's.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Verdict {
    /// The same, within tolerance: the measure is the RMS difference of
    /// the pixels, or the largest difference between numbers of the
    /// structure.
    Same(f64),
    /// A different figure, as described.
    Different(String),
    /// Not comparable (sizes differ, a document cannot be read).
    Incomparable(String),
}

impl Verdict {
    /// Whether the rendering diverges from matplotlib's.
    pub(super) fn diverges(&self) -> bool {
        !matches!(self, Self::Same(_))
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Same(measure) => write!(f, "{measure:.2}"),
            Self::Different(why) | Self::Incomparable(why) => f.write_str(why),
        }
    }
}

/// The guard's verdicts on one rendering.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Guarded {
    /// The pixels of the PNG or of the frames.
    pub(super) pixels: Verdict,
    /// The structure of the vector document, when the workload saves one.
    pub(super) structure: Option<Verdict>,
}

impl Guarded {
    /// Whether some check failed.
    pub(super) fn diverges(&self) -> bool {
        self.pixels.diverges() || self.structure.as_ref().is_some_and(Verdict::diverges)
    }

    /// The failed checks, described.
    pub(super) fn failures(&self) -> Vec<String> {
        let mut failures = Vec::new();
        if self.pixels.diverges() {
            failures.push(format!("pixels: {}", self.pixels));
        }
        if let Some(structure) = self.structure.as_ref().filter(|v| v.diverges()) {
            failures.push(format!("structure: {structure}"));
        }
        failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_same_figure_passes() {
        let same = Guarded {
            pixels: Verdict::Same(0.5),
            structure: Some(Verdict::Same(0.0)),
        };
        assert!(!same.diverges());
        assert_eq!(same.failures(), Vec::<String>::new());
        let different = Guarded {
            structure: Some(Verdict::Different("3 items missing".to_owned())),
            ..same
        };
        assert!(different.diverges());
        assert_eq!(different.failures(), ["structure: 3 items missing"]);
        assert!(Verdict::Incomparable("sizes".to_owned()).diverges());
        assert_eq!(Verdict::Same(0.004).to_string(), "0.00");
    }
}
