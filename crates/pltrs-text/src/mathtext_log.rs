//! The warnings matplotlib's mathtext logs while it typesets: a list it
//! packs to a size its glue cannot reach (`List._set_glue`: "Underful
//! Hlist: Hlist<w=nan ...>[...]", the list's box tree after it, which a
//! list of NaN sizes, a formula typeset at a resolution of NaN, meets at
//! every level), and a symbol no font of the set has, which a dummy
//! symbol stands for (`UnicodeFonts._get_glyph`).
//!
//! matplotlib logs them while it parses a formula, and its parser keeps
//! what it parsed (`MathTextParser._parse_cached`), so a formula laid out
//! again logs nothing. Each parse also makes the formula's fontset, whose
//! fonts the font manager looks up (`findfont`, which logs about a family
//! no font has once per font properties and font rcParams). pltrs
//! typesets a formula wherever it needs it, so typesetting only notes each
//! message ([`record`]) and each font lookup ([`claim`]) for the
//! typesetting that [`collecting`] runs, which keeps them with what it
//! typeset; the caller says them again ([`log_mathtext`]) where
//! matplotlib's parser would parse, and the engine hands what was logged
//! to the program ([`take_mathtext_log`],
//! [`take_font_notes`](crate::take_font_notes)).

use std::cell::RefCell;

use crate::lookups::{self, FontNote};
use crate::mathtext::Fontset;

/// The most messages kept until they are taken; later ones are dropped,
/// so that a program that never takes them does not grow.
const MAX_KEPT: usize = 1024;

/// What parsing a formula says, in order: a message mathtext logs, or a
/// font lookup (or a font opened) of the fontset the parse makes.
#[derive(Clone, Debug, PartialEq)]
pub enum MathNote {
    /// A message mathtext logs.
    Message(String),
    /// A note of the fonts the parse looks up or opens.
    Font(FontNote),
}

/// The notes of a formula being typeset ([`collecting`]).
struct Collection {
    /// The notes, in typesetting order.
    notes: Vec<MathNote>,
    /// The formula's fontset, the math font family of its font lookups.
    fontset: Option<Fontset>,
}

thread_local! {
    /// The messages logged on this thread and not taken yet.
    static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The notes of the typesetting [`collecting`] runs, if it runs.
    static COLLECTED: RefCell<Option<Collection>> = const { RefCell::new(None) };
}

/// Notes a message of a formula typeset on this thread: kept with the
/// typesetting [`collecting`] runs, logged at once outside it.
pub(crate) fn record(message: String) {
    let unclaimed = COLLECTED.with_borrow_mut(|collected| match collected {
        Some(collected) => {
            collected.notes.push(MathNote::Message(message));
            None
        }
        None => Some(message),
    });
    if let Some(message) = unclaimed {
        log_mathtext(&[MathNote::Message(message)]);
    }
}

/// The font `note` made now, as the formula [`collecting`] typesets (if
/// it runs) makes it: its lookups name the formula's fontset as their
/// math font family, and a copy is kept with the formula, to be said
/// again wherever it is parsed. (Said now too: what the typesetting looks
/// up, matplotlib looks up as it parses the formula, and the font
/// manager's cache keeps repeated lookups quiet.)
pub(crate) fn in_formula(mut note: FontNote) -> FontNote {
    COLLECTED.with_borrow_mut(|collected| {
        if let Some(collected) = collected {
            if let FontNote::Lookup(run) = &mut note
                && run.lookup.math_fontfamily.is_none()
            {
                run.lookup.math_fontfamily = collected.fontset;
            }
            collected.notes.push(MathNote::Font(note.clone()));
        }
    });
    note
}

/// Runs `typeset`, the typesetting of a formula of `fontset`, keeping the
/// messages it notes instead of logging them, and a copy of its font
/// notes: what `typeset` returns, and the notes in typesetting order.
pub(crate) fn collecting<R>(
    fontset: Option<Fontset>,
    typeset: impl FnOnce() -> R,
) -> (R, Vec<MathNote>) {
    /// Puts back the collection around this one, also when it unwinds.
    struct Restore(Option<Collection>);
    impl Drop for Restore {
        fn drop(&mut self) {
            COLLECTED.set(self.0.take());
        }
    }
    let restore = Restore(COLLECTED.replace(Some(Collection {
        notes: Vec::new(),
        fontset,
    })));
    let result = typeset();
    let collected = COLLECTED
        .take()
        .map(|collected| collected.notes)
        .unwrap_or_default();
    drop(restore);
    (result, collected)
}

/// Says again `notes`, what typesetting a formula noted (a
/// [`MathLayout`](crate::mathtext::MathLayout)'s `logged`), as
/// matplotlib's parser logs its messages and looks its fonts up when it
/// parses the formula.
pub fn log_mathtext(notes: &[MathNote]) {
    for note in notes {
        match note {
            MathNote::Message(message) => WARNINGS.with_borrow_mut(|warnings| {
                if warnings.len() < MAX_KEPT {
                    warnings.push(message.clone());
                }
            }),
            MathNote::Font(note) => lookups::adopt(vec![note.clone()]),
        }
    }
}

/// The messages matplotlib's mathtext logs about the formulas typeset on
/// this thread since the last call, in the order they were logged. At
/// most the first 1024 are kept between calls.
pub fn take_mathtext_log() -> Vec<String> {
    WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_are_taken_once_and_bounded() {
        take_mathtext_log();
        record("Underful Hlist: Hlist".to_owned());
        assert_eq!(take_mathtext_log(), ["Underful Hlist: Hlist"]);
        assert_eq!(take_mathtext_log(), Vec::<String>::new());
        for _ in 0..=MAX_KEPT {
            record(String::new());
        }
        assert_eq!(take_mathtext_log().len(), MAX_KEPT);
    }

    #[test]
    fn collected_messages_are_kept_until_logged() {
        take_mathtext_log();
        let message = |text: &str| MathNote::Message(text.to_owned());
        let ((), collected) = collecting(None, || {
            record("outer".to_owned());
            let ((), inner) = collecting(None, || record("inner".to_owned()));
            assert_eq!(inner, [message("inner")]);
        });
        assert_eq!(collected, [message("outer")]);
        assert_eq!(take_mathtext_log(), Vec::<String>::new());
        log_mathtext(&collected);
        log_mathtext(&collected);
        assert_eq!(take_mathtext_log(), ["outer", "outer"]);
    }

    #[test]
    fn a_formulas_font_lookups_are_said_again_at_each_parse() {
        use crate::{FontQuery, LookupKind, LookupRun, take_font_notes};
        take_font_notes();
        let lookup = crate::FontLookup {
            kind: LookupKind::Font,
            query: FontQuery::families(["Corp14Missing"]),
            generics: std::sync::Arc::default(),
            weight_keyword: None,
            stretch_keyword: None,
            math_fontfamily: None,
        };
        let ((), collected) = collecting(Some(Fontset::StixSans), || {
            lookups::note(lookup.clone(), &[None]);
        });
        let lookup = crate::FontLookup {
            math_fontfamily: Some(Fontset::StixSans),
            ..lookup
        };
        assert_eq!(
            take_font_notes(),
            [FontNote::Lookup(LookupRun {
                lookup: lookup.clone(),
                times: 1
            })],
            "made now, of the formula's fontset"
        );
        log_mathtext(&collected);
        log_mathtext(&collected);
        assert_eq!(
            take_font_notes(),
            [FontNote::Lookup(LookupRun { lookup, times: 2 })]
        );
    }
}
