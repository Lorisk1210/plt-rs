//! Text work that may go deep, run where the stack has room for it
//! ([`pltrs_core::stack::with_room`]): shaping a line of text the engine
//! has not shaped yet, at the bottom of a figure's layout, and typesetting
//! a formula, whose depth grows with its nesting.
//!
//! Where the caller's stack is short the work runs on a thread of its
//! own. What it notes for the caller there (the font lookups and the
//! messages of a formula's typesetting, thread-locals of this crate) is
//! handed back to the caller's thread, as if noted there. Neither kind of work touches
//! another thread-local nor takes a lock its callers hold: the font
//! database is only read, and the engine's memos compute unlocked.

use crate::{lookups, mathtext_log};

/// The stack shaping a line of text needs: the font chain made (faces
/// parsed, a cold one read from its file) and the text shaped. Measured:
/// at most 12 KiB below `TextEngine::shape_line` in an unoptimised build
/// (an SVG save of seed 22 of pltrs (Python)'s fuzzer), 6 KiB optimised.
pub(crate) const SHAPING: usize = 64 << 10;

/// The stack typesetting a formula needs before its nesting: a formula
/// of one level takes at most 20 KiB (unoptimised, its fonts read cold).
const FORMULA: usize = 64 << 10;

/// The stack each level of a formula's nesting may add. A level opens at
/// a `{`, a `\`, a `^` or a `_`; the deepest level per opening measured
/// is a brace group's, 5.7 KiB unoptimised (`\frac{` is 7.3 KiB for its
/// three openings, `\sum_{\substack{` 12.3 KiB for five).
const PER_OPENING: usize = 8 << 10;

/// The most levels a formula nests: the parser refuses deeper ones
/// (`parser::MAX_NESTING`, and one more for the level refused).
const MOST_LEVELS: usize = crate::mathtext::MAX_NESTING + 1;

/// The stack typesetting `source` needs: what one level takes, and what
/// each further level may add, for as many levels as `source` can open
/// (at most as many as the parser accepts).
pub(crate) fn formula_need(source: &str) -> usize {
    let openings = source
        .chars()
        .filter(|c| matches!(c, '{' | '\\' | '^' | '_'))
        .count();
    FORMULA + openings.min(MOST_LEVELS) * PER_OPENING
}

/// Runs `work`, which needs `need` bytes of stack, where the stack has
/// room for it ([`pltrs_core::stack::with_room`]), handing what it noted
/// on a thread of its own back to the caller's thread.
pub(crate) fn with_room<R: Send>(need: usize, work: impl FnOnce() -> R + Send) -> R {
    let caller = std::thread::current().id();
    let every = lookups::keeps_every();
    let (result, noted) = pltrs_core::stack::with_room(need, move || {
        if std::thread::current().id() == caller {
            return (work(), None);
        }
        lookups::keep_every_lookup(every);
        let result = work();
        let noted = Noted {
            fonts: crate::take_font_notes(),
            messages: crate::take_mathtext_log(),
        };
        (result, Some(noted))
    });
    if let Some(noted) = noted {
        noted.adopt();
    }
    result
}

/// What work noted on a thread of its own, for its caller's thread.
struct Noted {
    fonts: Vec<lookups::FontNote>,
    messages: Vec<String>,
}

impl Noted {
    /// Notes it all on the calling thread, as the work would have there.
    fn adopt(self) {
        lookups::adopt(self.fonts);
        self.messages.into_iter().for_each(mathtext_log::record);
    }
}

#[cfg(test)]
mod tests {
    use pltrs_font::{FontDatabase, FontQuery};

    use super::*;
    use crate::mathtext::{MAX_NESTING, MathStyle};
    use crate::{FontRequest, TextEngine};

    /// Runs `work` as if `left` bytes were left of the calling thread's
    /// stack, so that work needing more runs on a stack of its own.
    fn with_left<R>(left: usize, work: impl FnOnce() -> R) -> R {
        let marker = 0_u8;
        pltrs_core::stack::set_end(Some(std::ptr::from_ref(&marker).addr() - left));
        let result = work();
        pltrs_core::stack::set_end(None);
        result
    }

    #[test]
    fn a_formula_needs_more_stack_the_more_it_can_nest() {
        assert_eq!(formula_need("$x$"), FORMULA);
        assert_eq!(
            formula_need(r"$\mathdefault{10^{3}}$"),
            FORMULA + 4 * PER_OPENING
        );
        let deep = format!("${}x{}$", "{".repeat(1000), "}".repeat(1000));
        assert_eq!(formula_need(&deep), FORMULA + MOST_LEVELS * PER_OPENING);
    }

    #[test]
    fn formulas_as_deep_as_the_parser_takes_fit_the_stack_they_are_given() {
        // Each kind of nesting at the parser's limit, typeset on a stack of
        // the size `formula_need` gives it (the caller's being short): a
        // level that took more than it allows would overflow it.
        let levels = MAX_NESTING - 1;
        let nested =
            |open: &str, close: &str| format!("${}x{}$", open.repeat(levels), close.repeat(levels));
        let style = MathStyle::default().with_dpi(72.0);
        for source in [
            nested("{", "}"),
            nested(r"\frac{", "}{y}"),
            nested(r"\sqrt{", "}"),
            nested("x^{", "}"),
            nested(r"\left(", r"\right)"),
            nested(r"\overline{", "}"),
            nested(r"\sum_{\substack{", "}}"),
            nested(r"\hat{", "}"),
        ] {
            let typeset = with_left(16 << 10, || {
                TextEngine::global().typeset_math(&source, &style)
            });
            // The parser's own limit may refuse a kind before this depth.
            if let Err(error) = &typeset {
                assert!(error.to_string().contains("recursion"), "{source}: {error}");
            }
        }
        let deeper = nested("{", "}").replace('x', "{{x}}");
        let refused = with_left(16 << 10, || {
            TextEngine::global().typeset_math(&deeper, &style)
        });
        assert!(refused.is_err_and(|error| error.to_string().contains("recursion")));
    }

    #[test]
    fn work_on_a_stack_of_its_own_gives_what_it_would_have_here() {
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let request = FontRequest::new(FontQuery {
            size: 10.0,
            ..FontQuery::families(["DejaVu Sans"])
        });
        let shaped = with_left(16 << 10, || {
            engine.layout_line("Elsewhere", &request, 100.0)
        });
        let fresh = TextEngine::new(FontDatabase::with_bundled_fonts());
        assert_eq!(
            shaped.unwrap(),
            fresh.layout_line("Elsewhere", &request, 100.0).unwrap()
        );

        // A formula's font notes and its message reach the caller, as
        // they are noted where the formula is typeset here.
        let style = MathStyle::default().with_dpi(72.0);
        let noted = |left: usize| {
            crate::take_font_notes();
            crate::take_mathtext_log();
            crate::keep_every_lookup(true);
            let tree = with_left(left, || engine.math_box_tree("$y\u{1f60e}$", &style));
            crate::keep_every_lookup(false);
            assert!(tree.is_ok());
            (crate::take_font_notes(), crate::take_mathtext_log())
        };
        let (elsewhere, here) = (noted(16 << 10), noted(16 << 20));
        assert_eq!(elsewhere, here);
        assert!(!here.0.is_empty(), "a lookup kept");
        assert_eq!(here.1.len(), 1, "the dummy symbol's message");
    }
}
