//! Parse failures and matplotlib's (pyparsing's) error messages.

use crate::error::TextError;
use crate::mathtext::data::is_letter;
use crate::mathtext::fonts::GlyphError;

/// What pyparsing reports when no alternative of a placeable matches.
pub(super) const EXPECTED_PLACEABLE: &str = "Expected {phantom | llap | rlap | accent | symbol | \
     function | operatorname | group | frac | dfrac | binom | genfrac | overset | underset | \
     sqrt | overline | underline | text | boldsymbol | substack | Forward: {{{{'\\\\left' -} \
     {delim | Empty} [{{simple | : ...} | {'\\\\middle' -} {delim | Empty}}]...} '\\\\right'} -} \
     {delim | Empty}}";
/// A group that is not closed.
pub(super) const EXPECTED_END_GROUP: &str = "Expected end_group";
/// Anything else that does not match.
pub(super) const NO_MATCH: &str = "no match";

/// A recoverable mismatch: where the furthest alternative failed and what
/// it expected (pyparsing's `ParseException`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Miss {
    /// Character index of the failure.
    pub(super) loc: usize,
    /// The message pyparsing would report.
    pub(super) msg: &'static str,
}

/// Why a grammar element did not produce a result.
#[derive(Debug)]
pub(super) enum Fail {
    /// The element does not match here; alternatives may still.
    Soft(Miss),
    /// Parsing stops.
    Fatal(TextError),
}

impl Fail {
    /// Python's `max()` of an empty sequence, which matplotlib raises for
    /// empty `\substack`s and delimiters around nothing but `\middle`.
    pub(super) fn empty_max() -> Self {
        Self::Fatal(TextError::Math(
            "max() iterable argument is empty".to_owned(),
        ))
    }

    /// A soft failure at `loc`.
    pub(super) fn miss(loc: usize, msg: &'static str) -> Self {
        Self::Soft(Miss { loc, msg })
    }
}

impl From<GlyphError> for Fail {
    fn from(err: GlyphError) -> Self {
        Self::Fatal(err.into())
    }
}

impl From<TextError> for Fail {
    fn from(err: TextError) -> Self {
        Self::Fatal(err)
    }
}

/// The pyparsing exception class named in messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ErrorKind {
    /// `ParseException`: the input as a whole does not parse.
    Parse,
    /// `ParseFatalException`: raised by a parse action.
    Fatal,
    /// `ParseSyntaxException`: an element after a `-` failed.
    Syntax,
}

impl ErrorKind {
    fn name(self) -> &'static str {
        match self {
            Self::Parse => "ParseException",
            Self::Fatal => "ParseFatalException",
            Self::Syntax => "ParseSyntaxException",
        }
    }
}

/// Characters pyparsing groups into the "found" word of a message.
fn is_word_char(c: char) -> bool {
    let code = u32::from(c);
    let in_sets = code <= 0x24f
        || (0x370..=0x3ff).contains(&code)
        || (0x1f00..=0x1fff).contains(&code)
        || (0x400..=0x52f).contains(&code);
    in_sets && (is_letter(code) || c.is_numeric())
}

/// Python's `repr` of a short string, with backslashes not doubled.
fn python_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => {
                out.push_str("\\x");
                let code = u32::from(c);
                for digit in [code >> 4, code & 0xf] {
                    out.extend(char::from_digit(digit, 16));
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// What pyparsing says it found at `loc`: the next word (up to 16
/// characters) or character, or the end of the text.
fn found(text: &[char], loc: usize) -> String {
    let Some(&first) = text.get(loc) else {
        return "end of text".to_owned();
    };
    let word: String = if is_word_char(first) {
        text[loc..]
            .iter()
            .take(16)
            .take_while(|&&c| is_word_char(c))
            .collect()
    } else {
        first.to_string()
    };
    python_repr(&word)
}

/// matplotlib's `ValueError` text for a parse error:
/// `"\n" + exception.explain(0)`.
pub(super) fn explain(kind: ErrorKind, msg: &str, text: &[char], loc: usize) -> TextError {
    let line_start = text[..loc.min(text.len())]
        .iter()
        .rposition(|&c| c == '\n')
        .map_or(0, |i| i + 1);
    let line_end = text[loc.min(text.len())..]
        .iter()
        .position(|&c| c == '\n')
        .map_or(text.len(), |i| loc + i);
    let line: String = text[line_start..line_end].iter().collect();
    let lineno = text[..loc.min(text.len())]
        .iter()
        .filter(|&&c| c == '\n')
        .count()
        + 1;
    let column = if loc > 0 && loc < text.len() && text[loc - 1] == '\n' {
        1
    } else {
        loc - line_start + 1
    };
    TextError::Math(format!(
        "\n{line}\n{marker:>column$}\n{kind}: {msg}, found {found}  (at char {loc}), \
         (line:{lineno}, col:{column})",
        marker = "^",
        kind = kind.name(),
        found = found(text, loc),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    #[test]
    fn messages_match_pyparsing() {
        let err = explain(
            ErrorKind::Fatal,
            "Unknown symbol: \\foo",
            &chars("\\foo"),
            0,
        );
        assert_eq!(
            err,
            TextError::Math(
                "\n\\foo\n^\nParseFatalException: Unknown symbol: \\foo, found '\\'  \
                 (at char 0), (line:1, col:1)"
                    .to_owned()
            )
        );
        let err = explain(
            ErrorKind::Syntax,
            "Expected \\text",
            &chars("\\textbf{x}"),
            5,
        );
        assert!(
            matches!(&err, TextError::Math(m) if m.contains("found 'bf'  (at char 5), (line:1, col:6)"))
        );
        let err = explain(ErrorKind::Parse, "Expected end of text", &chars("x"), 1);
        assert!(matches!(&err, TextError::Math(m) if m.contains("found end of text")));
    }

    #[test]
    fn found_words_and_quotes() {
        assert_eq!(found(&chars("abc^2"), 0), "'abc'");
        assert_eq!(found(&chars("'"), 0), "\"'\"");
        assert_eq!(found(&chars("é1"), 0), "'é1'");
        assert_eq!(found(&chars("∑"), 0), "'∑'");
    }
}
