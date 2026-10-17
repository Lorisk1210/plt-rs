//! The outlines of mathtext markers such as `"$\alpha$"` (matplotlib's
//! `TextPath`, as used by `MarkerStyle._set_mathtext_path`).

use plt_core::{Path, Point};

use super::spec::MarkerError;
use crate::fonts::FontProperties;

/// The outline of `text` as `TextPath((0, 0), text)`: typeset with the
/// default font and scaled to the `font.size` rcParam, in points.
pub(super) fn text_path(text: &str) -> Result<Path, MarkerError> {
    let font = FontProperties::from_rc(&crate::rc::params());
    crate::text::text_path(Point::ORIGIN, text, None, &font)
        .map_err(|err| MarkerError::Math(err.to_string()))
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
