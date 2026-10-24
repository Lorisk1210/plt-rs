//! `figures/cases_mpl_tests_ft2font.py`: matplotlib's own tests of
//! `matplotlib.ft2font` that draw (`test_ft2font.py`). The 12 px `M` of
//! `test_ft2font_drawing`, autohinted and rendered without anti-aliasing,
//! as figure text on a 10 × 11 pixel canvas at 72 dpi.

use plt::figure::Figure;

use crate::figures::{CaseResult, FigureCase};

/// `FT2Font`'s default `FORCE_AUTOHINT` load flag, without anti-aliasing.
const AUTOHINTED_ALIASED: &[(&str, &str)] = &[
    ("text.hinting", "force_autohint"),
    ("text.antialiased", "False"),
];

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        FigureCase::new("mpl_tests_ft2font/drawing", drawing)
            .sized(10.0 / 72.0, 11.0 / 72.0, 72.0)
            .with_rc(AUTOHINTED_ALIASED),
    ]
}

/// `test_ft2font_drawing`: one `M` at 12 pt, on the baseline one pixel up.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case builder returns a CaseResult"
)]
fn drawing(fig: &Figure) -> CaseResult {
    fig.text(0.0, 1.0 / 11.0, "M").set_fontsize(12.0);
    Ok(())
}
