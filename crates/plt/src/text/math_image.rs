//! A math expression rendered alone on a figure just its size
//! (matplotlib's `mathtext.math_to_image`).

use std::fmt;

use plt_text::TextEngine;

use super::TextError;
use super::math::math_style;
use crate::colors::Color;
use crate::figure::{Figure, SaveError, SaveFormat, SaveOptions};
use crate::fonts::FontProperties;

/// How [`math_to_image`] renders an expression (matplotlib's keyword
/// arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct MathImageOptions {
    /// The font, whose size and math fonts the expression is set in
    /// (`None` = the rc defaults; matplotlib's `prop`).
    pub font: Option<FontProperties>,
    /// The output resolution (`None` = `savefig.dpi`).
    pub dpi: Option<f64>,
    /// The file format.
    pub format: SaveFormat,
    /// The text colour (`None` = `text.color`).
    pub color: Option<Color>,
}

impl Default for MathImageOptions {
    /// A PNG in the rc font, resolution and text colour.
    fn default() -> Self {
        Self {
            font: None,
            dpi: None,
            format: SaveFormat::Png,
            color: None,
        }
    }
}

/// An expression rendered by [`math_to_image`].
#[derive(Clone, Debug, PartialEq)]
pub struct MathImage {
    /// The file, in the format asked for.
    pub data: Vec<u8>,
    /// How far the expression reaches below its baseline, in points.
    pub depth: f64,
}

/// Why an expression could not be rendered.
#[derive(Debug)]
pub enum MathImageError {
    /// The expression could not be typeset.
    Text(TextError),
    /// The figure could not be saved.
    Save(SaveError),
}

impl fmt::Display for MathImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(err) => write!(f, "cannot typeset the expression: {err}"),
            Self::Save(err) => write!(f, "cannot save the expression: {err}"),
        }
    }
}

impl std::error::Error for MathImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Text(err) => Some(err),
            Self::Save(err) => Some(err),
        }
    }
}

impl From<TextError> for MathImageError {
    fn from(err: TextError) -> Self {
        Self::Text(err)
    }
}

impl From<SaveError> for MathImageError {
    fn from(err: SaveError) -> Self {
        Self::Save(err)
    }
}

/// Renders `text`, whose math is enclosed in dollar signs, in a figure of
/// exactly its size (its typeset width and height at 72 dpi), with the
/// baseline as far up as the expression reaches below it (matplotlib's
/// `mathtext.math_to_image`).
///
/// ```
/// use plt::figure::SaveFormat;
/// use plt::image::imread_bytes;
/// use plt::text::{MathImageOptions, math_to_image};
/// let image = math_to_image("$x^2$", MathImageOptions::default())?;
/// assert!(image.depth >= 0.0);
/// let (rows, columns) = imread_bytes(&image.data)?.shape();
/// assert!(rows > 0 && columns > rows);
/// let svg = MathImageOptions { format: SaveFormat::Svg, ..MathImageOptions::default() };
/// assert!(math_to_image("$x^2$", svg)?.data.starts_with(b"<?xml"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the expression cannot be typeset or the figure cannot be saved in
/// the format asked for.
pub fn math_to_image(text: &str, options: MathImageOptions) -> Result<MathImage, MathImageError> {
    let rc = crate::rc::params();
    let font = options.font.unwrap_or_else(|| FontProperties::from_rc(&rc));
    let run = TextEngine::global().layout_math(text, &math_style(&font, 72.0, &rc))?;
    let (width, height, depth) = (run.width, run.height, run.descent);
    let fig = Figure::with_size(width / 72.0, height / 72.0, rc.figure_dpi);
    let label = fig.text(0.0, depth / height, text);
    label.set_font(font);
    if let Some(color) = options.color {
        label.set_color(color);
    }
    let save = SaveOptions {
        dpi: options.dpi,
        ..SaveOptions::default()
    };
    let data = fig.to_format(options.format, save)?;
    Ok(MathImage { data, depth })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::imread_bytes;

    #[test]
    fn the_figure_is_the_size_of_the_expression() {
        let options = MathImageOptions {
            dpi: Some(72.0),
            ..MathImageOptions::default()
        };
        let image = math_to_image("$x^2$", options).expect("typeset");
        let rc = crate::rc::params();
        let style = math_style(&FontProperties::from_rc(&rc), 72.0, &rc);
        let run = TextEngine::global()
            .layout_math("$x^2$", &style)
            .expect("typeset");
        let (rows, columns) = imread_bytes(&image.data).expect("a PNG").shape();
        assert_eq!((columns as f64, rows as f64), (run.width, run.height));
        assert_eq!(image.depth, run.descent);
    }

    #[test]
    fn malformed_math_is_an_error() {
        let result = math_to_image(r"$\frac{1$", MathImageOptions::default());
        assert!(matches!(result, Err(MathImageError::Text(_))));
    }
}
