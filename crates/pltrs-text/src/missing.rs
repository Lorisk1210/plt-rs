//! Characters no font of a text has: matplotlib warns about each one as it
//! lays the text out ("Glyph 128579 (\N{UPSIDE-DOWN FACE}) missing from
//! font(s) `DejaVu Sans`."), and the engine records each one for its caller
//! to report ([`take_missing_glyphs`]).

use std::cell::RefCell;

/// The most missing glyphs kept until they are taken; later ones are
/// dropped, so that a program that never takes them does not grow.
const MAX_KEPT: usize = 1024;

/// A character a text needs that no font of its fallback chain has: it
/// was drawn from the Last Resort font, or as the missing-glyph box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingGlyph {
    /// The character (the first of its cluster, as matplotlib reports it).
    pub character: char,
    /// The family names of the fonts searched for it, in fallback order
    /// and without repetitions.
    pub families: Vec<String>,
}

thread_local! {
    /// The missing glyphs laid out on this thread and not taken yet.
    static MISSING: RefCell<Vec<MissingGlyph>> = const { RefCell::new(Vec::new()) };
}

/// Records missing glyphs laid out on this thread.
pub(crate) fn record(glyphs: impl IntoIterator<Item = MissingGlyph>) {
    MISSING.with(|missing| {
        let mut missing = missing.borrow_mut();
        let room = MAX_KEPT.saturating_sub(missing.len());
        missing.extend(glyphs.into_iter().take(room));
    });
}

/// The glyphs laid out on this thread since the last call that no font of
/// their text has, in layout order: what matplotlib's `FT2Font.set_text`
/// warns about each time it lays such a text out. At most the first 1024
/// are kept between calls.
pub fn take_missing_glyphs() -> Vec<MissingGlyph> {
    MISSING.with(|missing| std::mem::take(&mut *missing.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(character: char) -> MissingGlyph {
        MissingGlyph {
            character,
            families: vec!["DejaVu Sans".to_owned()],
        }
    }

    #[test]
    fn glyphs_are_taken_once_and_bounded() {
        take_missing_glyphs();
        record([glyph('a'), glyph('b')]);
        assert_eq!(take_missing_glyphs(), [glyph('a'), glyph('b')]);
        assert_eq!(take_missing_glyphs(), Vec::new());
        record((0..2 * MAX_KEPT).map(|_| glyph('x')));
        record([glyph('y')]);
        let kept = take_missing_glyphs();
        assert_eq!(kept.len(), MAX_KEPT);
        assert!(kept.iter().all(|g| g.character == 'x'));
    }
}
