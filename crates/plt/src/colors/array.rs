//! Colour lists: matplotlib's `to_rgba_array` and `same_color`.

use plt_core::Rgba;

use super::{Color, ColorError};

/// One colour or a list of colours (matplotlib's "color or list of
/// colors" arguments, e.g. a collection's face colours).
///
/// The distinction matters: a single `"none"` means *no colours at all*,
/// while `"none"` inside a list is one transparent entry.
#[derive(Clone, Debug, PartialEq)]
pub enum Colors {
    /// A single colour.
    One(Color),
    /// A sequence of colours.
    Many(Vec<Color>),
}

impl<T: Into<Color>> From<T> for Colors {
    fn from(color: T) -> Self {
        Self::One(color.into())
    }
}

impl<T: Into<Color>> FromIterator<T> for Colors {
    fn from_iter<I: IntoIterator<Item = T>>(colors: I) -> Self {
        Self::Many(colors.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Color> + Clone> From<&[T]> for Colors {
    fn from(colors: &[T]) -> Self {
        colors.iter().cloned().collect()
    }
}

impl<T: Into<Color>> From<Vec<T>> for Colors {
    fn from(colors: Vec<T>) -> Self {
        colors.into_iter().collect()
    }
}

/// An alpha override applied to colours or colormap samples.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Alpha {
    /// Keep each colour's own alpha.
    #[default]
    Keep,
    /// The same alpha for every entry.
    Uniform(f64),
    /// One alpha per entry.
    Each(Vec<f64>),
}

impl From<f64> for Alpha {
    fn from(alpha: f64) -> Self {
        Self::Uniform(alpha)
    }
}

impl From<Option<f64>> for Alpha {
    fn from(alpha: Option<f64>) -> Self {
        alpha.map_or(Self::Keep, Self::Uniform)
    }
}

impl From<Vec<f64>> for Alpha {
    fn from(alphas: Vec<f64>) -> Self {
        Self::Each(alphas)
    }
}

/// Converts colours to RGBA rows (matplotlib's `to_rgba_array`).
///
/// * A single colour gives one row — or one row per entry of an
///   [`Alpha::Each`] — and a single `"none"` gives no rows.
/// * In a list, `"none"` entries stay fully transparent whatever the alpha.
/// * A list with one colour is repeated to match [`Alpha::Each`].
///
/// ```
/// use plt::colors::{Alpha, to_rgba_array};
/// let rows = to_rgba_array(vec!["red", "none"], Alpha::Uniform(0.5))?;
/// assert_eq!((rows[0].a, rows[1].a), (0.5, 0.0));
/// assert!(to_rgba_array("none", Alpha::Keep)?.is_empty());
/// # Ok::<(), plt::colors::ColorError>(())
/// ```
pub fn to_rgba_array(
    colors: impl Into<Colors>,
    alpha: impl Into<Alpha>,
) -> Result<Vec<Rgba>, ColorError> {
    match (colors.into(), alpha.into()) {
        (Colors::One(color), _) if color.is_none() => Ok(Vec::new()),
        (Colors::One(color), Alpha::Keep) => Ok(vec![color.resolve()?]),
        (Colors::One(color), Alpha::Uniform(a)) => Ok(vec![color.resolve_alpha(Some(a))?]),
        (Colors::One(color), Alpha::Each(alphas)) => alphas
            .iter()
            .map(|&a| color.resolve_alpha(Some(a)))
            .collect(),
        (Colors::Many(colors), alpha) => list_to_rgba(&colors, &alpha),
    }
}

fn list_to_rgba(colors: &[Color], alpha: &Alpha) -> Result<Vec<Rgba>, ColorError> {
    let rows = colors
        .iter()
        .map(Color::resolve)
        .collect::<Result<Vec<_>, _>>()?;
    let alphas = match alpha {
        Alpha::Keep => return Ok(rows),
        Alpha::Uniform(a) => vec![*a; rows.len()],
        Alpha::Each(alphas) => alphas.clone(),
    };
    if let Some(&bad) = alphas.iter().find(|a| !(0.0..=1.0).contains(*a)) {
        return Err(ColorError::AlphaOutOfRange(bad));
    }
    // `(colour, is "none")`, a single colour repeated to match the alphas.
    let entries: Vec<(Rgba, bool)> = rows
        .into_iter()
        .zip(colors.iter().map(Color::is_none))
        .collect();
    let entries = match entries.as_slice() {
        [one] => vec![*one; alphas.len()],
        all if all.len() == alphas.len() => entries,
        all => {
            return Err(ColorError::AlphaCount {
                colors: all.len(),
                alphas: alphas.len(),
            });
        }
    };
    Ok(entries
        .into_iter()
        .zip(alphas)
        .map(|((row, none), a)| row.with_alpha(if none { 0.0 } else { a }))
        .collect())
}

/// Whether two colours (or colour lists) are the same RGBA values
/// (matplotlib's `same_color`). Lists must have the same length; a single
/// `"none"` counts as one colour that only equals another `"none"`.
///
/// ```
/// use plt::colors::same_color;
/// assert!(same_color("red", "#ff0000")?);
/// assert!(!same_color("green", "g")?);
/// # Ok::<(), plt::colors::ColorError>(())
/// ```
pub fn same_color(a: impl Into<Colors>, b: impl Into<Colors>) -> Result<bool, ColorError> {
    let a = to_rgba_array(a, Alpha::Keep)?;
    let b = to_rgba_array(b, Alpha::Keep)?;
    let (n_a, n_b) = (a.len().max(1), b.len().max(1));
    if n_a != n_b {
        return Err(ColorError::CountMismatch(n_a, n_b));
    }
    Ok(a == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(r: f64, g: f64, b: f64, a: f64) -> Rgba {
        Rgba::new(r, g, b, a)
    }

    #[test]
    fn single_colours() {
        assert_eq!(
            to_rgba_array("red", Alpha::Keep),
            Ok(vec![Rgba::opaque(1.0, 0.0, 0.0)])
        );
        assert_eq!(to_rgba_array("None", 0.5), Ok(vec![]));
        assert_eq!(
            to_rgba_array("red", vec![0.1, 0.5]),
            Ok(vec![rgba(1.0, 0.0, 0.0, 0.1), rgba(1.0, 0.0, 0.0, 0.5)])
        );
        assert_eq!(to_rgba_array("red", Vec::new()), Ok(vec![]));
        assert!(to_rgba_array("red", 1.5).is_err());
        assert!(to_rgba_array("notacolor", Alpha::Keep).is_err());
    }

    #[test]
    fn lists() {
        let colors = Colors::from(vec!["red", "none", "blue"]);
        assert_eq!(
            to_rgba_array(colors.clone(), vec![0.1, 0.2, 0.3]),
            Ok(vec![
                rgba(1.0, 0.0, 0.0, 0.1),
                rgba(0.0, 0.0, 0.0, 0.0),
                rgba(0.0, 0.0, 1.0, 0.3)
            ])
        );
        assert_eq!(
            to_rgba_array(colors, vec![0.1, 0.2]),
            Err(ColorError::AlphaCount {
                colors: 3,
                alphas: 2
            })
        );
        assert_eq!(
            to_rgba_array(vec![[1.0, 0.0, 0.0]], vec![0.25, 0.75]),
            Ok(vec![rgba(1.0, 0.0, 0.0, 0.25), rgba(1.0, 0.0, 0.0, 0.75)])
        );
        assert_eq!(to_rgba_array(Vec::<Color>::new(), Alpha::Keep), Ok(vec![]));
        assert!(to_rgba_array(vec![[2.0, 0.0, 0.0]], Alpha::Keep).is_err());
    }

    #[test]
    fn sameness() {
        assert_eq!(same_color("C0", "tab:blue"), Ok(true));
        assert_eq!(same_color("red", "#ff0000fe"), Ok(false));
        assert_eq!(same_color("none", "none"), Ok(true));
        assert_eq!(same_color("none", (0.0, 0.0, 0.0, 0.0)), Ok(false));
        assert_eq!(same_color(vec!["red", "blue"], vec!["r", "b"]), Ok(true));
        assert_eq!(
            same_color(vec!["red", "blue"], "red"),
            Err(ColorError::CountMismatch(2, 1))
        );
    }
}
