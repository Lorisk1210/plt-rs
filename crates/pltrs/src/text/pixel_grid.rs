//! Where matplotlib's Agg renderer puts the glyphs of a text: it rounds
//! each glyph's pen position to whole 64ths of a pixel as Python rounds
//! numbers (`round(0x40 * x)` in `_draw_text_glyphs_and_boxes`), and a
//! line of TeX to whole pixels (`round(x + xd)` in `draw_tex`). Python's
//! `round` refuses what is not finite, so a text whose lines are spaced,
//! turned or sized by NaN or infinity fails the draw there, with a
//! `ValueError` (NaN) or an `OverflowError` (infinity), after numpy warned
//! about casting the rotation matrix of a turn that is no number.

use std::fmt;

use pltrs_core::Point;
use pltrs_text::TextRun;

/// A position of a glyph (or of a line of TeX) that a renderer rounding
/// positions to its pixel grid cannot round: NaN or an infinity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnroundablePosition(f64);

impl UnroundablePosition {
    /// Whether the position is NaN (Python's `ValueError`), not an
    /// infinity (its `OverflowError`).
    pub fn is_nan(self) -> bool {
        self.0.is_nan()
    }
}

impl fmt::Display for UnroundablePosition {
    /// Python's message for rounding the number.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = if self.is_nan() { "NaN" } else { "infinity" };
        write!(f, "cannot convert float {what} to integer")
    }
}

impl std::error::Error for UnroundablePosition {}

/// `value` rounded as Python's `round` would take it.
fn rounded(value: f64) -> Result<(), UnroundablePosition> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(UnroundablePosition(value))
    }
}

/// Checks the positions matplotlib's Agg renderer rounds for the glyphs
/// of `run` drawn at `origin` and turned by `angle` degrees, in its order:
/// for each glyph its pen's x, then its y (the first glyph of a turn that
/// is no number warns numpy's cast warning first).
///
/// # Errors
///
/// The first position that is not finite.
pub(crate) fn check_glyphs(
    run: &TextRun,
    origin: Point,
    angle: f64,
) -> Result<(), UnroundablePosition> {
    let (sin, cos) = angle.to_radians().sin_cos();
    for glyph in &run.glyphs {
        if !(sin.is_finite() && cos.is_finite()) {
            // `np.array([[cos, -sin], [sin, cos]]).round().astype(int)`.
            crate::warnings::warn_runtime("invalid value encountered in cast");
        }
        rounded(64.0 * (origin.x + glyph.x * cos - glyph.y * sin))?;
        rounded(64.0 * (origin.y + glyph.x * sin + glyph.y * cos))?;
    }
    Ok(())
}

/// Checks the position matplotlib's Agg renderer rounds for the line of
/// TeX `run` drawn at `origin` and turned by `angle` degrees: its origin
/// moved by its descent along the turn, x then y.
///
/// # Errors
///
/// The first coordinate that is not finite.
pub(crate) fn check_tex(
    run: &TextRun,
    origin: Point,
    angle: f64,
) -> Result<(), UnroundablePosition> {
    let (sin, cos) = angle.to_radians().sin_cos();
    rounded(origin.x + run.descent * sin)?;
    rounded(origin.y + run.descent * cos)
}

#[cfg(test)]
mod tests {
    use pltrs_text::{FontQuery, FontRequest, TextEngine};

    use super::*;
    use crate::warnings;

    fn run(text: &str) -> TextRun {
        TextEngine::global()
            .layout_line(
                text,
                &FontRequest::new(FontQuery {
                    size: 10.0,
                    ..FontQuery::families(["DejaVu Sans"])
                }),
                72.0,
            )
            .expect("DejaVu Sans lays text out")
    }

    #[test]
    fn finite_positions_round() {
        let (checked, warned) =
            warnings::capturing_runtime(|| check_glyphs(&run("ab"), Point::new(3.5, 7.25), 30.0));
        assert_eq!(checked, Ok(()));
        assert_eq!(warned, Vec::<String>::new());
    }

    #[test]
    fn positions_that_are_no_number_fail_as_python_rounds() {
        let err = check_glyphs(&run("ab"), Point::new(f64::NAN, 1.0), 0.0)
            .expect_err("NaN is not rounded");
        assert!(err.is_nan());
        assert_eq!(err.to_string(), "cannot convert float NaN to integer");
        let err = check_glyphs(&run("ab"), Point::new(1.0, f64::NEG_INFINITY), 0.0)
            .expect_err("infinity is not rounded");
        assert!(!err.is_nan());
        assert_eq!(err.to_string(), "cannot convert float infinity to integer");
        // Positions too large for 64ths of a pixel overflow.
        assert!(check_glyphs(&run("a"), Point::new(f64::MAX / 2.0, 0.0), 0.0).is_err());
    }

    #[test]
    fn turns_that_are_no_number_warn_once_then_fail() {
        let (checked, warned) = warnings::capturing_runtime(|| {
            check_glyphs(&run("ab"), Point::new(1.0, 1.0), f64::NAN)
        });
        assert!(checked.is_err_and(UnroundablePosition::is_nan));
        assert_eq!(warned, ["invalid value encountered in cast"]);
    }

    #[test]
    fn runs_without_glyphs_round_nothing() {
        let empty = TextRun {
            glyphs: Vec::new(),
            ..run("a")
        };
        assert_eq!(
            check_glyphs(&empty, Point::new(f64::NAN, 0.0), f64::NAN),
            Ok(())
        );
    }

    #[test]
    fn tex_lines_round_their_origin_moved_by_their_descent() {
        let line = TextRun {
            descent: 2.0,
            ..run("a")
        };
        assert_eq!(check_tex(&line, Point::new(1.0, 2.0), 90.0), Ok(()));
        assert!(check_tex(&line, Point::new(1.0, 2.0), f64::NAN).is_err());
        let err = check_tex(&line, Point::new(1.0, f64::INFINITY), 0.0).expect_err("infinite");
        assert!(!err.is_nan());
    }
}
