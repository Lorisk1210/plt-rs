//! Figure sizes: sizes given in inches, centimetres or pixels, with a
//! side left to `figure.figsize` (matplotlib's `figsize` argument), and
//! the checks matplotlib makes before resizing a figure.

use std::fmt;
use std::str::FromStr;

use super::Figure;

/// The unit of a [`FigSize`] (the third item of matplotlib's `figsize`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeUnit {
    /// Inches (`"in"`).
    #[default]
    Inches,
    /// Centimetres (`"cm"`).
    Centimeters,
    /// Pixels at the figure's resolution (`"px"`).
    Pixels,
}

impl SizeUnit {
    /// `value` of this unit in inches at `dpi`.
    fn to_inches(self, value: f64, dpi: f64) -> f64 {
        match self {
            Self::Inches => value,
            Self::Centimeters => value / 2.54,
            Self::Pixels => value / dpi,
        }
    }
}

impl FromStr for SizeUnit {
    type Err = SizeError;

    /// Parses matplotlib's unit names `"in"`, `"cm"` and `"px"`.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "in" => Ok(Self::Inches),
            "cm" => Ok(Self::Centimeters),
            "px" => Ok(Self::Pixels),
            _ => Err(SizeError::UnknownUnit(name.to_owned())),
        }
    }
}

/// A figure size as matplotlib's `figsize` gives it: a width and a height
/// in a unit, either of which may be left to `figure.figsize`.
///
/// ```
/// use pltrs::figure::{FigSize, SizeUnit};
/// let size = FigSize::new(Some(600.0), None, SizeUnit::Pixels);
/// assert_eq!(size.inches(100.0, (6.4, 4.8)), Ok((6.0, 4.8)));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FigSize {
    /// The width, `None` for `figure.figsize`'s.
    pub width: Option<f64>,
    /// The height, `None` for `figure.figsize`'s.
    pub height: Option<f64>,
    /// The unit of both.
    pub unit: SizeUnit,
}

impl FigSize {
    /// A size of `width × height` in `unit`.
    pub const fn new(width: Option<f64>, height: Option<f64>, unit: SizeUnit) -> Self {
        Self {
            width,
            height,
            unit,
        }
    }

    /// The size in inches at `dpi`, a missing side taken from `default`
    /// (inches; matplotlib's `_parse_figsize`).
    ///
    /// # Errors
    ///
    /// When neither side is given, or a side is negative or not finite.
    pub fn inches(&self, dpi: f64, default: (f64, f64)) -> Result<(f64, f64), SizeError> {
        if self.width.is_none() && self.height.is_none() {
            return Err(SizeError::Unspecified);
        }
        let side = |value: Option<f64>, fallback: f64| {
            value.map_or(fallback, |v| self.unit.to_inches(v, dpi))
        };
        let size = (side(self.width, default.0), side(self.height, default.1));
        check_size(size.0, size.1)?;
        Ok(size)
    }
}

/// Why a figure size is rejected.
#[derive(Clone, Debug, PartialEq)]
pub enum SizeError {
    /// A side is negative or not finite (inches).
    Invalid {
        /// The width asked for.
        width: f64,
        /// The height asked for.
        height: f64,
    },
    /// Neither the width nor the height is given.
    Unspecified,
    /// A unit other than `"in"`, `"cm"` and `"px"`.
    UnknownUnit(String),
}

impl fmt::Display for SizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { width, height } => {
                write!(
                    f,
                    "figure size must be positive finite, not {width} x {height}"
                )
            }
            Self::Unspecified => f.write_str("a figure size needs a width or a height"),
            Self::UnknownUnit(unit) => write!(
                f,
                "invalid figure size unit {unit:?}; supported units are \"in\", \"cm\", \"px\""
            ),
        }
    }
}

impl std::error::Error for SizeError {}

/// Rejects sizes matplotlib refuses: negative or non-finite sides.
fn check_size(width: f64, height: f64) -> Result<(), SizeError> {
    let valid = |side: f64| side.is_finite() && side >= 0.0;
    if valid(width) && valid(height) {
        Ok(())
    } else {
        Err(SizeError::Invalid { width, height })
    }
}

impl Figure {
    /// A figure of `size` at `dpi` (matplotlib's `Figure(figsize=...,
    /// dpi=...)`), a missing side taken from `figure.figsize`.
    ///
    /// ```
    /// use pltrs::figure::{FigSize, Figure, SizeUnit};
    /// let size = FigSize::new(Some(5.08), Some(2.54), SizeUnit::Centimeters);
    /// let fig = Figure::with_figsize(&size, 100.0)?;
    /// assert_eq!(fig.size_inches(), (2.0, 1.0));
    /// # Ok::<(), pltrs::figure::SizeError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`FigSize::inches`].
    pub fn with_figsize(size: &FigSize, dpi: f64) -> Result<Self, SizeError> {
        let [width, height] = crate::rc::params().figure_figsize;
        let (width, height) = size.inches(dpi, (width, height))?;
        Ok(Self::with_size(width, height, dpi))
    }

    /// Resizes the canvas (in inches; matplotlib's `set_size_inches`).
    ///
    /// # Errors
    ///
    /// When a side is negative or not finite; the size is unchanged.
    pub fn set_size_inches(&self, width: f64, height: f64) -> Result<&Self, SizeError> {
        check_size(width, height)?;
        let mut state = self.state_mut();
        state.size_inches = (width, height);
        state.stale = true;
        drop(state);
        Ok(self)
    }

    /// Changes the width of the canvas in inches (matplotlib's
    /// `set_figwidth`).
    ///
    /// # Errors
    ///
    /// As [`Figure::set_size_inches`].
    pub fn set_figwidth(&self, width: f64) -> Result<&Self, SizeError> {
        self.set_size_inches(width, self.size_inches().1)
    }

    /// Changes the height of the canvas in inches (matplotlib's
    /// `set_figheight`).
    ///
    /// # Errors
    ///
    /// As [`Figure::set_size_inches`].
    pub fn set_figheight(&self, height: f64) -> Result<&Self, SizeError> {
        self.set_size_inches(self.size_inches().0, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_convert_from_their_unit() {
        let inches = |w, h, unit: &str| {
            FigSize::new(w, h, unit.parse().expect("known unit")).inches(100.0, (8.0, 6.0))
        };
        assert_eq!(inches(Some(6.0), Some(4.0), "in"), Ok((6.0, 4.0)));
        assert_eq!(inches(Some(5.08), Some(2.54), "cm"), Ok((2.0, 1.0)));
        assert_eq!(inches(Some(600.0), Some(400.0), "px"), Ok((6.0, 4.0)));
        assert_eq!(inches(None, Some(4.0), "in"), Ok((8.0, 4.0)));
        assert_eq!(inches(Some(6.0), None, "px"), Ok((0.06, 6.0)));
        assert_eq!(inches(None, None, "in"), Err(SizeError::Unspecified));
        assert_eq!(
            "um".parse::<SizeUnit>(),
            Err(SizeError::UnknownUnit("um".into()))
        );
    }

    #[test]
    fn negative_and_non_finite_sizes_are_rejected() {
        for (width, height) in [(1.0, f64::NAN), (-1.0, 1.0), (f64::INFINITY, 1.0)] {
            let size = FigSize::new(Some(width), Some(height), SizeUnit::Inches);
            assert!(Figure::with_figsize(&size, 100.0).is_err());
            let fig = Figure::with_size(2.0, 1.0, 100.0);
            assert!(fig.set_size_inches(width, height).is_err());
            assert_eq!(fig.size_inches(), (2.0, 1.0), "the size stays");
        }
        let fig = Figure::with_size(2.0, 1.0, 100.0);
        assert_eq!(
            fig.set_size_inches(0.0, 3.0).map(Figure::size_inches),
            Ok((0.0, 3.0))
        );
    }

    #[test]
    fn width_and_height_change_alone() {
        // matplotlib's test_set_fig_size.
        let fig = Figure::with_size(8.0, 6.0, 100.0);
        fig.set_figwidth(5.0).expect("valid width");
        assert_eq!(fig.size_inches(), (5.0, 6.0));
        fig.set_figheight(1.0).expect("valid height");
        assert_eq!(fig.size_inches(), (5.0, 1.0));
        assert!(fig.set_figheight(-2.0).is_err());
    }
}
