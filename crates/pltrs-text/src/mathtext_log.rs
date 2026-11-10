//! The warnings matplotlib's mathtext logs while it typesets: a list it
//! packs to a size its glue cannot reach (`List._set_glue`: "Underful
//! Hlist: Hlist<w=nan ...>[...]", the list's box tree after it, which a
//! list of NaN sizes, a formula typeset at a resolution of NaN, meets at
//! every level), and a symbol no font of the set has, which a dummy
//! symbol stands for (`UnicodeFonts._get_glyph`). Typesetting notes each
//! ([`record`]), and the engine keeps the messages for the caller to log
//! ([`take_mathtext_log`]).

use std::cell::RefCell;
use std::fmt::Write;

/// The most messages kept until they are taken; later ones are dropped,
/// so that a program that never takes them does not grow.
const MAX_KEPT: usize = 1024;

thread_local! {
    /// The messages of the formulas typeset on this thread and not taken
    /// yet.
    static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Records a message of a formula typeset on this thread.
pub(crate) fn record(message: String) {
    WARNINGS.with(|warnings| {
        let mut warnings = warnings.borrow_mut();
        if warnings.len() < MAX_KEPT {
            warnings.push(message);
        }
    });
}

/// Python's `ascii()` of `text`: its `repr`, with what is no printable
/// ASCII escaped (`'\\aleph'` for `\aleph`, `'\xe9'` for `é`), as
/// matplotlib's messages format symbols (`%a`).
pub(crate) fn ascii_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in text.chars() {
        let code = u32::from(c);
        let written = match c {
            '\\' => out.write_str("\\\\"),
            '\n' => out.write_str("\\n"),
            '\r' => out.write_str("\\r"),
            '\t' => out.write_str("\\t"),
            c if c == quote => write!(out, "\\{c}"),
            ' '..='~' => out.write_char(c),
            _ if code < 0x100 => write!(out, "\\x{code:02x}"),
            _ if code < 0x1_0000 => write!(out, "\\u{code:04x}"),
            _ => write!(out, "\\U{code:08x}"),
        };
        written.expect("writing to a string does not fail");
    }
    out.push(quote);
    out
}

/// The messages matplotlib's mathtext logs about the formulas typeset on
/// this thread since the last call, in typesetting order. At most the
/// first 1024 are kept between calls.
pub fn take_mathtext_log() -> Vec<String> {
    WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_are_formatted_as_pythons_ascii() {
        assert_eq!(ascii_repr("\\aleph"), "'\\\\aleph'");
        assert_eq!(ascii_repr("x"), "'x'");
        assert_eq!(
            ascii_repr("\u{e9}\u{2135}\u{1d400}"),
            "'\\xe9\\u2135\\U0001d400'"
        );
        assert_eq!(ascii_repr("'"), "\"'\"");
    }

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
}
