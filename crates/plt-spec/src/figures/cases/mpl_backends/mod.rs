//! `figures/cases_mpl_backends.py`: matplotlib's own image tests of its
//! vector backends — every `@image_comparison` test and the `fig_test`
//! side of every `@check_figures_equal` test of `tests/test_backend_pdf.py`,
//! `tests/test_backend_ps.py` and `tests/test_backend_svg.py`, one
//! submodule each. Case names carry their backend because the three test
//! modules share test names. The six `multi_font_*` tests draw the same
//! figure and differ only in the font type of their document: they share
//! the case `multi_font`, whose vector variants cover the other font types.
//!
//! Cases keep the test's style: the test suite's default `classic` plus
//! `_classic_test_patch` on 8 × 6 in figures (see [`classic`]), or the
//! reference configuration for tests asking for `mpl20`/`default`.
//! `remove_text=True` becomes [`remove_ticks_and_titles`] at the end of
//! the build.

mod pdf;
mod ps;
mod svg;

use plt::figure::Figure;
use plt::text::{HorizontalAlignment, VerticalAlignment};

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// The seed of the tests' `np.random.default_rng`.
const SEED: u64 = 19_680_801;

/// matplotlib's `_gen_multi_font_text()` string: ASCII, then the Latin-1
/// supplement from `Å`, Latin Extended-A and -B and sixteen emoji (outside
/// the Basic Multilingual Plane), 32 characters to a line counted from
/// U+00A0.
fn multi_font_text() -> String {
    const FIRST: u32 = 0xC5;
    let accented: Vec<char> = (FIRST..=0x24F)
        .chain(0x1F600..0x1F610)
        .filter_map(char::from_u32)
        .collect();
    // The first line holds the characters from Å up to U+00DF.
    let first_line = (0x20 - (FIRST - 0xA0) % 0x20) as usize;
    let (head, tail) = accented.split_at(first_line);
    let lines: Vec<String> = std::iter::once(head)
        .chain(tail.chunks(32))
        .map(|line| line.iter().collect())
        .collect();
    format!(
        "There are basic characters\n\
         ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz\n\
         0123456789 !\"#$%&'()*+,-./:;<=>?@[\\]^_`{{|}}~\n\
         and accented characters\n\
         {}\n\
         in between!",
        lines.join("\n")
    )
}

/// The `multi_font_*` figure: [`multi_font_text`] centred.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn multi_font(fig: &Figure) -> CaseResult {
    fig.text(0.5, 0.5, multi_font_text())
        .set_halign(HorizontalAlignment::Center)
        .set_valign(VerticalAlignment::Center);
    Ok(())
}

/// The `multi_font_*` tests of all three modules, in the fonts of
/// `_gen_multi_font_text` at 16 points: `cmr10` lacks the accented
/// letters and the emoji, which fall back to `DejaVu Sans`.
const MULTI_FONT: FigureCase = FigureCase::new("mpl_backends/multi_font", multi_font)
    .sized(8.0, 6.0, 100.0)
    .with_rc(&[("font.family", "cmr10, DejaVu Sans"), ("font.size", "16")]);

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [&[MULTI_FONT], pdf::CASES, ps::CASES, svg::CASES].concat()
}

#[cfg(test)]
mod tests {
    use super::multi_font_text;

    #[test]
    fn multi_font_text_breaks_lines_like_matplotlib() {
        let text = multi_font_text();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 18);
        assert_eq!(lines[4].chars().count(), 27);
        assert!(lines[4].starts_with('Å') && lines[4].ends_with('ß'));
        assert!(lines[5].starts_with('à'));
        assert_eq!(lines[16].chars().last(), Some('\u{1F60F}'));
        assert_eq!(text.chars().count(), 584);
    }
}
