//! The warnings matplotlib's mathtext logs while it typesets: a list it
//! packs to a size its glue cannot reach (`List._set_glue`: "Underful
//! Hlist: Hlist<w=nan ...>[...]", the list's box tree after it, which a
//! list of NaN sizes, a formula typeset at a resolution of NaN, meets at
//! every level), and a symbol no font of the set has, which a dummy
//! symbol stands for (`UnicodeFonts._get_glyph`).
//!
//! matplotlib logs them while it parses a formula, and its parser keeps
//! what it parsed (`MathTextParser._parse_cached`), so a formula laid out
//! again logs nothing. pltrs typesets a formula wherever it needs it, so
//! typesetting only notes each message ([`record`]) for the typesetting
//! that [`collecting`] runs, which keeps them with what it typeset; the
//! caller logs them ([`log_mathtext`]) where matplotlib's parser would
//! parse, and the engine hands what was logged to the program
//! ([`take_mathtext_log`]).

use std::cell::RefCell;

/// The most messages kept until they are taken; later ones are dropped,
/// so that a program that never takes them does not grow.
const MAX_KEPT: usize = 1024;

thread_local! {
    /// The messages logged on this thread and not taken yet.
    static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The messages of the typesetting [`collecting`] runs, if it runs.
    static COLLECTED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Notes a message of a formula typeset on this thread: kept with the
/// typesetting [`collecting`] runs, logged at once outside it.
pub(crate) fn record(message: String) {
    let unclaimed = COLLECTED.with_borrow_mut(|collected| match collected {
        Some(collected) => {
            collected.push(message);
            None
        }
        None => Some(message),
    });
    if let Some(message) = unclaimed {
        log_mathtext(&[message]);
    }
}

/// Runs `typeset`, keeping the messages it notes instead of logging
/// them: what `typeset` returns, and the messages in typesetting order.
pub(crate) fn collecting<R>(typeset: impl FnOnce() -> R) -> (R, Vec<String>) {
    /// Puts back the collection around this one, also when it unwinds.
    struct Restore(Option<Vec<String>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            COLLECTED.set(self.0.take());
        }
    }
    let restore = Restore(COLLECTED.replace(Some(Vec::new())));
    let result = typeset();
    let collected = COLLECTED.take().unwrap_or_default();
    drop(restore);
    (result, collected)
}

/// Logs `messages`, the messages typesetting a formula noted (a
/// [`MathLayout`](crate::mathtext::MathLayout)'s `logged`), as matplotlib's
/// parser logs them when it parses the formula.
pub fn log_mathtext(messages: &[String]) {
    WARNINGS.with_borrow_mut(|warnings| {
        let room = MAX_KEPT.saturating_sub(warnings.len());
        warnings.extend(messages.iter().take(room).cloned());
    });
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
        let ((), collected) = collecting(|| {
            record("outer".to_owned());
            let ((), inner) = collecting(|| record("inner".to_owned()));
            assert_eq!(inner, ["inner"]);
        });
        assert_eq!(collected, ["outer"]);
        assert_eq!(take_mathtext_log(), Vec::<String>::new());
        log_mathtext(&collected);
        log_mathtext(&collected);
        assert_eq!(take_mathtext_log(), ["outer", "outer"]);
    }
}
