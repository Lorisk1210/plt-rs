//! The rectangle that new axes occupy in a figure, checked as
//! matplotlib's `Figure.add_axes` checks it.

use std::fmt;

use pltrs_core::Bbox;

/// Why `[left, bottom, width, height]` cannot place new axes (the
/// `ValueError`s of matplotlib's `Figure.add_axes`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxesRectError {
    /// An entry is NaN or infinite.
    NonFinite([f64; 4]),
    /// The width or the height is negative.
    NegativeSize([f64; 4]),
}

impl fmt::Display for AxesRectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite(rect) => {
                write!(f, "all entries in rect must be finite not {rect:?}")
            }
            Self::NegativeSize(rect) => {
                write!(f, "width and height must be non-negative, not {rect:?}")
            }
        }
    }
}

impl std::error::Error for AxesRectError {}

/// The box `rect` (`[left, bottom, width, height]`) covers, in the
/// fractions it is given in.
///
/// # Errors
///
/// When an entry is NaN or infinite, or the width or the height is
/// negative.
pub(crate) fn axes_bounds(rect: [f64; 4]) -> Result<Bbox, AxesRectError> {
    let [left, bottom, width, height] = rect;
    if !rect.iter().all(|entry| entry.is_finite()) {
        return Err(AxesRectError::NonFinite(rect));
    }
    if width < 0.0 || height < 0.0 {
        return Err(AxesRectError::NegativeSize(rect));
    }
    Ok(Bbox::from_bounds(left, bottom, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_rectangles_give_their_box() {
        let bounds = axes_bounds([0.25, 0.5, 0.5, 0.0]).expect("a valid rectangle");
        assert_eq!(bounds, Bbox::from_extents(0.25, 0.5, 0.75, 0.5));
    }

    #[test]
    fn non_finite_entries_are_refused() {
        for rect in [
            [f64::NAN, 0.0, 1.0, 1.0],
            [0.0, f64::INFINITY, 1.0, 1.0],
            [0.0, 0.0, f64::NEG_INFINITY, 1.0],
        ] {
            let Err(AxesRectError::NonFinite(refused)) = axes_bounds(rect) else {
                panic!("{rect:?} is refused as not finite");
            };
            assert_eq!(refused.map(f64::to_bits), rect.map(f64::to_bits));
        }
        assert_eq!(
            AxesRectError::NonFinite([f64::NAN, 0.0, 1.0, 1.0]).to_string(),
            "all entries in rect must be finite not [NaN, 0.0, 1.0, 1.0]"
        );
    }

    #[test]
    fn negative_sizes_are_refused() {
        for rect in [[0.0, 0.0, -0.5, 1.0], [0.0, 0.0, 1.0, -1e-9]] {
            assert_eq!(axes_bounds(rect), Err(AxesRectError::NegativeSize(rect)));
        }
        assert!(
            axes_bounds([0.5, 0.5, -0.0, 0.0]).is_ok(),
            "-0 is not negative"
        );
    }
}
