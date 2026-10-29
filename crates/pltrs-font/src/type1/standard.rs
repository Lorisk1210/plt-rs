//! The Adobe Standard Encoding, shared with the CFF tables.

use crate::cff::{STANDARD_ENCODING, STANDARD_STRINGS};

/// The glyph name of `code` in the Standard Encoding, `.notdef` where it
/// has none (FreeType's `adobe_std_strings(adobe_std_encoding[code])`).
pub(crate) fn standard_name(code: u8) -> &'static str {
    STANDARD_STRINGS[usize::from(STANDARD_ENCODING[usize::from(code)])]
}

/// The entries of matplotlib's `_StandardEncoding`: every encoded code,
/// plus code 0 for `.notdef`.
pub(crate) fn standard_encoding() -> impl Iterator<Item = (u8, &'static str)> {
    (0..=u8::MAX)
        .filter(|&code| code == 0 || STANDARD_ENCODING[usize::from(code)] != 0)
        .map(|code| (code, standard_name(code)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matplotlibs_standard_encoding() {
        let table: Vec<_> = standard_encoding().collect();
        // 149 encoded codes plus `.notdef` at 0.
        assert_eq!(table.len(), 150);
        assert_eq!(standard_name(65), "A");
        assert_eq!(standard_name(39), "quoteright");
        assert_eq!(standard_name(251), "germandbls");
        assert_eq!(standard_name(1), ".notdef");
        assert_eq!(table[0], (0, ".notdef"));
    }
}
