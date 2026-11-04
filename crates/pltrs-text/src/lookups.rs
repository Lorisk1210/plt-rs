//! Font lookups matplotlib's font manager logs about: a family of a text
//! that no font has (`findfont: Font family 'xkcd' not found.`), or a best
//! face of another weight than asked for (`findfont: Failed to find font
//! weight light for DejaVu Sans, now using 400.`). matplotlib looks the
//! fonts of a text up each time it lays the text out or draws it, through
//! `_find_fonts_by_props`; the engine records each lookup it would log
//! about for its caller to report ([`take_font_lookups`]), where the
//! program's font manager can log as matplotlib's does.

use std::cell::RefCell;

use pltrs_font::{FontQuery, GenericFamilies};

/// The most lookups kept until they are taken; later ones are dropped, so
/// that a program that never takes them does not grow.
const MAX_KEPT: usize = 1024;

/// How the fonts of a lookup were asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookupKind {
    /// The best face of each family, for a fallback chain (matplotlib's
    /// `_find_fonts_by_props`).
    Fonts,
    /// The single best face (matplotlib's `findfont`).
    Font,
}

/// A font lookup matplotlib's font manager logs about.
#[derive(Clone, Debug, PartialEq)]
pub struct FontLookup {
    /// How the fonts were asked for.
    pub kind: LookupKind,
    /// What was looked up.
    pub query: FontQuery,
    /// The generic-family lists in force.
    pub generics: GenericFamilies,
    /// The keyword the weight was given as, if it was (matplotlib's
    /// message names it).
    pub weight_keyword: Option<&'static str>,
}

thread_local! {
    /// The lookups made on this thread and not taken yet.
    static LOOKUPS: RefCell<Vec<FontLookup>> = const { RefCell::new(Vec::new()) };
}

/// Records `lookup` if matplotlib's font manager logs about it: a family
/// of the query has no face (`found` holds the weight of the best face of
/// each family, `None` for none), or a face found has another weight than
/// the query asks for.
pub(crate) fn note(lookup: FontLookup, found: &[Option<u16>]) {
    let weight = lookup.query.weight.0;
    let logged = found
        .iter()
        .any(|found| found.is_none_or(|found| found != weight));
    if !logged {
        return;
    }
    LOOKUPS.with(|lookups| {
        let mut lookups = lookups.borrow_mut();
        if lookups.len() < MAX_KEPT {
            lookups.push(lookup);
        }
    });
}

/// The lookups made on this thread since the last call that matplotlib's
/// font manager logs about, in the order they were made. At most the first
/// 1024 are kept between calls.
pub fn take_font_lookups() -> Vec<FontLookup> {
    LOOKUPS.with(|lookups| std::mem::take(&mut *lookups.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use pltrs_font::Weight;

    use super::*;

    fn lookup(weight: u16) -> FontLookup {
        FontLookup {
            kind: LookupKind::Fonts,
            query: FontQuery {
                weight: Weight(weight),
                ..FontQuery::families(["xkcd", "DejaVu Sans"])
            },
            generics: GenericFamilies::default(),
            weight_keyword: None,
        }
    }

    #[test]
    fn only_lookups_matplotlib_logs_about_are_kept() {
        take_font_lookups();
        note(lookup(400), &[Some(400), Some(400)]);
        assert_eq!(take_font_lookups(), [], "every family found at its weight");
        note(lookup(400), &[None, Some(400)]);
        note(lookup(300), &[Some(400)]);
        assert_eq!(take_font_lookups(), [lookup(400), lookup(300)]);
        assert_eq!(take_font_lookups(), []);
    }
}
