//! A colormap and norm from levels and colours (matplotlib's
//! `from_levels_and_colors`).

use std::fmt;

use super::{BoundaryNorm, Color, Colormap, ColormapError, Extend, NormError};

/// Why [`from_levels_and_colors`] failed.
#[derive(Clone, Debug, PartialEq)]
pub enum LevelsError {
    /// The number of colours does not fit the levels and extensions.
    ColorCount {
        /// Colours needed: one per bin plus one per extension.
        expected: usize,
        /// Colours given.
        actual: usize,
    },
    /// The colours do not make a colormap.
    Colormap(ColormapError),
    /// The levels do not make a norm.
    Norm(NormError),
}

impl fmt::Display for LevelsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColorCount { expected, actual } => {
                write!(f, "expected {expected} colors, but got {actual}")
            }
            Self::Colormap(err) => err.fmt(f),
            Self::Norm(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for LevelsError {}

/// A listed colormap and a [`BoundaryNorm`] that colour values the way
/// `contourf` colours bands between `levels` (matplotlib's
/// `from_levels_and_colors`): one colour per bin, plus the under colour
/// first and/or the over colour last as `extend` asks. The colormap
/// remembers `extend` for colorbars.
///
/// ```
/// use plt::colors::{Color, Extend, Normalized, from_levels_and_colors};
/// let colors = ["blue", "green", "red"].map(Color::from);
/// let (cmap, norm) = from_levels_and_colors(&[0.0, 1.0, 2.0], &colors, Extend::Max)?;
/// assert_eq!(cmap.n(), 2);
/// assert_eq!(cmap.colorbar_extend(), Some(Extend::Max));
/// assert_eq!(cmap.over(), Color::from("red").resolve()?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the number of colours is wrong, a colour is invalid or there are
/// fewer than two levels.
pub fn from_levels_and_colors(
    levels: &[f64],
    colors: &[Color],
    extend: Extend,
) -> Result<(Colormap, BoundaryNorm), LevelsError> {
    let (under, over) = match extend {
        Extend::Neither => (false, false),
        Extend::Min => (true, false),
        Extend::Max => (false, true),
        Extend::Both => (true, true),
    };
    let bins = levels.len().saturating_sub(1);
    let expected = bins + usize::from(under) + usize::from(over);
    if levels.is_empty() || colors.len() != expected {
        return Err(LevelsError::ColorCount {
            expected,
            actual: colors.len(),
        });
    }
    let under_color = if under {
        colors[0].clone()
    } else {
        Color::none()
    };
    let over_color = if over {
        colors[colors.len() - 1].clone()
    } else {
        Color::none()
    };
    let data = colors[usize::from(under)..colors.len() - usize::from(over)].to_vec();
    let cmap = Colormap::listed("from_list", data)
        .and_then(|cmap| cmap.with_under(under_color))
        .and_then(|cmap| cmap.with_over(over_color))
        .map_err(LevelsError::Colormap)?
        .with_colorbar_extend(Some(extend));
    let norm = BoundaryNorm::new(levels, bins, Extend::Neither).map_err(LevelsError::Norm)?;
    Ok((cmap, norm))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors(names: &[&str]) -> Vec<Color> {
        names.iter().map(|&name| Color::from(name)).collect()
    }

    #[test]
    fn extensions_take_the_end_colours() {
        let (cmap, _) = from_levels_and_colors(
            &[0.0, 1.0, 2.0],
            &colors(&["k", "r", "g", "b"]),
            Extend::Both,
        )
        .unwrap();
        assert_eq!(cmap.n(), 2);
        assert_eq!(cmap.under(), Color::from("k").resolve().unwrap());
        assert_eq!(cmap.over(), Color::from("b").resolve().unwrap());
    }

    #[test]
    fn the_colour_count_must_fit() {
        assert_eq!(
            from_levels_and_colors(&[0.0, 1.0, 2.0], &colors(&["r"]), Extend::Neither),
            Err(LevelsError::ColorCount {
                expected: 2,
                actual: 1
            })
        );
        assert!(from_levels_and_colors(&[], &[], Extend::Neither).is_err());
        assert!(matches!(
            from_levels_and_colors(&[1.0], &[], Extend::Neither),
            Err(LevelsError::Colormap(_) | LevelsError::Norm(_))
        ));
    }
}
