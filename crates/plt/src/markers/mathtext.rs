//! The outlines of mathtext markers such as `"$\alpha$"` (matplotlib's
//! `TextPath`, as used by `MarkerStyle._set_mathtext_path`).

use plt_core::{Affine2D, Path};
use plt_text::TextEngine;

use super::spec::MarkerError;
use crate::fonts::FontProperties;
use crate::text::math_style;

/// The size text is typeset at before scaling to the font size
/// (matplotlib's `TextToPath.FONT_SCALE`).
const FONT_SCALE: f64 = 100.0;

/// The resolution text paths are typeset at (`TextToPath.DPI`).
const DPI: f64 = 72.0;

/// The outline of `text` as `TextPath((0, 0), text)`: typeset with the
/// default font at [`FONT_SCALE`] points and [`DPI`], then scaled to the
/// `font.size` rcParam, in points.
pub(super) fn text_path(text: &str) -> Result<Path, MarkerError> {
    let rc = crate::rc::params();
    let mut font = FontProperties::from_rc(&rc);
    let size = font.size;
    font.size = FONT_SCALE;
    let engine = TextEngine::global();
    let run = engine
        .layout_math(text, &math_style(&font, DPI, &rc))
        .map_err(|err| MarkerError::Math(err.to_string()))?;
    let scale = size / FONT_SCALE;
    Ok(run
        .outline(engine)
        .transformed(&Affine2D::scaling(scale, scale)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_become_outlines_at_the_font_size() {
        let path = text_path(r"$\heartsuit$").unwrap();
        let extents = path.extents();
        assert!(!extents.is_null());
        // A 10 pt symbol is at most 10 points tall.
        assert!(extents.height() > 1.0 && extents.height() < 10.0);
    }

    #[test]
    fn malformed_math_is_an_error() {
        assert!(matches!(text_path(r"$\frac$"), Err(MarkerError::Math(_))));
    }
}
