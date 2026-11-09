//! Lists mathtext packs to a size their glue cannot reach: matplotlib's
//! `List._set_glue` logs each as a warning ("Underful Hlist: Hlist<w=nan
//! ...>[...]", the list's box tree after it), which a list of NaN sizes
//! (a formula typeset at a resolution of NaN) meets at every level. The
//! packing notes each such list ([`record`]), and the engine keeps the
//! messages for the caller to log ([`take_glue_warnings`]).

use std::cell::RefCell;

/// The most messages kept until they are taken; later ones are dropped,
/// so that a program that never takes them does not grow.
const MAX_KEPT: usize = 1024;

thread_local! {
    /// The messages of the lists packed on this thread and not taken yet.
    static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Records the message about a list packed on this thread.
pub(crate) fn record(message: String) {
    WARNINGS.with(|warnings| {
        let mut warnings = warnings.borrow_mut();
        if warnings.len() < MAX_KEPT {
            warnings.push(message);
        }
    });
}

/// The messages matplotlib's mathtext logs about the lists packed on this
/// thread since the last call, in packing order. At most the first 1024
/// are kept between calls.
pub fn take_glue_warnings() -> Vec<String> {
    WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_are_taken_once_and_bounded() {
        take_glue_warnings();
        record("Underful Hlist: Hlist".to_owned());
        assert_eq!(take_glue_warnings(), ["Underful Hlist: Hlist"]);
        assert_eq!(take_glue_warnings(), Vec::<String>::new());
        for _ in 0..=MAX_KEPT {
            record(String::new());
        }
        assert_eq!(take_glue_warnings().len(), MAX_KEPT);
    }
}
