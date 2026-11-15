//! The mathtext parser: a recursive-descent port of matplotlib's pyparsing
//! grammar together with its parse actions, which build the box model as
//! they go.
//!
//! The port keeps pyparsing's observable behaviour: which elements skip
//! whitespace, the order alternatives are tried in, where errors are
//! reported and with which message, and the side effects of parse actions
//! (font switches leaking out of `{...}` arguments, operator spacing that
//! depends on the preceding characters, ...).

mod commands;
mod delims;
mod error;
mod fraction;
mod scan;
mod script;
mod symbol;
mod token;

use std::collections::HashMap;

use error::{ErrorKind, Fail, explain};

use super::boxes::{Char, List, Node, State};
use super::font_name::FontName;
use super::fonts::Fonts;
use crate::error::TextError;

/// Deepest nesting of placeables accepted. matplotlib's parser runs out of
/// Python stack (`RecursionError`) well before this.
pub(crate) const MAX_NESTING: usize = 100;

/// The result of a grammar element: where it ended and what it built.
type Parsed<T> = Result<(usize, T), Fail>;

/// Parses `text` (plain text with `$...$` math) into the box to ship.
pub(crate) fn parse(
    text: &str,
    fonts: &mut Fonts<'_>,
    fontsize: f64,
    math_default: FontName,
) -> Result<List, TextError> {
    let root = State {
        font: FontName::Default,
        font_class: FontName::Rm,
        fontsize,
    };
    let mut parser = Parser {
        fonts,
        src: expand_tabs(text),
        root,
        stack: Vec::new(),
        needs_space_after_subsuper: false,
        math_default,
        group_failures: HashMap::new(),
        nesting: 0,
    };
    parser.main()
}

/// Python's `str.expandtabs()` (pyparsing expands tabs before parsing).
fn expand_tabs(text: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(text.len());
    let mut column = 0;
    for c in text.chars() {
        match c {
            '\t' => {
                let spaces = 8 - column % 8;
                out.extend(std::iter::repeat_n(' ', spaces));
                column += spaces;
            }
            '\n' | '\r' => {
                out.push(c);
                column = 0;
            }
            c => {
                out.push(c);
                column += 1;
            }
        }
    }
    out
}

/// Parser state: the text being parsed, the font state stack and the
/// flags parse actions share.
struct Parser<'a, 'e> {
    fonts: &'a mut Fonts<'e>,
    /// The string being parsed: the whole text, then each math segment.
    src: Vec<char>,
    root: State,
    stack: Vec<State>,
    /// Set by function names followed by a script: scripts typeset no
    /// operator spacing and a thin space follows them.
    needs_space_after_subsuper: bool,
    math_default: FontName,
    /// Groups that are missing their `}`: start → end of their content.
    group_failures: HashMap<usize, usize>,
    /// Placeables being parsed inside each other.
    nesting: usize,
}

impl Parser<'_, '_> {
    fn state(&self) -> &State {
        self.stack.last().unwrap_or(&self.root)
    }

    fn state_mut(&mut self) -> &mut State {
        self.stack.last_mut().unwrap_or(&mut self.root)
    }

    fn push_state(&mut self) {
        self.stack.push(*self.state());
    }

    fn pop_state(&mut self) {
        self.stack.pop();
    }

    /// A fatal error at `loc` of the current string.
    fn fatal(&self, kind: ErrorKind, msg: &str, loc: usize) -> Fail {
        Fail::Fatal(explain(kind, msg, &self.src, loc))
    }

    /// `non_math + (math_string + non_math)* + StringEnd`.
    fn main(&mut self) -> Result<List, TextError> {
        let mut items = Vec::new();
        let mut loc = 0;
        loop {
            let end = self.non_math_end(loc);
            let plain: String = self.src[loc..end].iter().collect();
            items.push(self.non_math(&plain)?);
            loc = end;
            let Some(close) = self.math_string_end(loc) else {
                break;
            };
            let content = self.src[loc + 1..close - 1].to_vec();
            match self.math(content) {
                Ok(node) => items.push(node),
                Err(Fail::Soft(_)) => break,
                Err(Fail::Fatal(err)) => return Err(err),
            }
            loc = close;
        }
        if loc != self.src.len() {
            return Err(explain(
                ErrorKind::Parse,
                "Expected end of text",
                &self.src,
                loc,
            ));
        }
        Ok(List::hlist(items, self.fonts)?)
    }

    /// End of `(\\$|[^$])*` from `loc`.
    fn non_math_end(&self, loc: usize) -> usize {
        let mut pos = loc;
        while let Some(&c) = self.src.get(pos) {
            match c {
                '$' => break,
                '\\' if self.src.get(pos + 1) == Some(&'$') => pos += 2,
                _ => pos += 1,
            }
        }
        pos
    }

    /// End (after the closing `$`) of a `$...$` string at `loc`
    /// (pyparsing's `QuotedString('$', '\\')`: a backslash escapes any
    /// character but `\n`; unescaped line breaks end the match).
    fn math_string_end(&self, loc: usize) -> Option<usize> {
        if self.src.get(loc) != Some(&'$') {
            return None;
        }
        let mut pos = loc + 1;
        loop {
            match *self.src.get(pos)? {
                '$' => return Some(pos + 1),
                '\n' | '\r' => return None,
                '\\' => match self.src.get(pos + 1) {
                    Some('\n') | None => return None,
                    Some(_) => pos += 2,
                },
                _ => pos += 1,
            }
        }
    }

    /// Text outside math: one character per glyph in the current (text)
    /// font; math that follows starts in `mathtext.default`.
    fn non_math(&mut self, plain: &str) -> Result<Node, TextError> {
        let state = *self.state();
        let chars = plain
            .replace("\\$", "$")
            .chars()
            .map(|c| Ok(Node::Char(Char::new(&c.to_string(), &state, self.fonts)?)))
            .collect::<Result<Vec<_>, TextError>>()?;
        let list = List::hlist(chars, self.fonts)?;
        self.push_state();
        let font = self.math_default;
        self.state_mut().set_font(font);
        Ok(list.into_node())
    }

    /// Parses one math segment (the content between dollars).
    fn math(&mut self, content: Vec<char>) -> Result<Node, Fail> {
        let outer = std::mem::replace(&mut self.src, content);
        let saved_failures = std::mem::take(&mut self.group_failures);
        let result = self.math_tokens();
        self.src = outer;
        self.group_failures = saved_failures;
        result
    }

    /// `OneOrMore(token)` over the whole segment.
    fn math_tokens(&mut self) -> Result<Node, Fail> {
        let (end, nodes, count) = self.tokens(0)?;
        if count == 0 {
            return Err(Fail::miss(0, error::NO_MATCH));
        }
        let list = List::hlist(nodes, self.fonts)?;
        self.pop_state();
        if self.skip_ws(end) != self.src.len() {
            return Err(Fail::miss(end, error::NO_MATCH));
        }
        Ok(list.into_node())
    }
}

#[cfg(test)]
mod tests {
    use pltrs_font::{FontDatabase, FontQuery};

    use super::*;
    use crate::engine::{FontRequest, TextEngine};
    use crate::mathtext::{MathLayout, MathStyle};

    fn typeset(source: &str) -> Result<MathLayout, TextError> {
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let style = MathStyle {
            font: FontRequest::new(FontQuery {
                size: 10.0,
                ..FontQuery::families(["DejaVu Sans"])
            }),
            dpi: 100.0,
            ..MathStyle::default()
        };
        engine.typeset_math(source, &style)
    }

    fn width(source: &str) -> f64 {
        typeset(source).expect("valid math").width
    }

    fn message(source: &str) -> String {
        match typeset(source) {
            Err(TextError::Math(msg)) => msg,
            other => panic!("expected a math error, got {other:?}"),
        }
    }

    #[test]
    fn tabs_expand_to_eight_columns() {
        let expanded: String = expand_tabs("a\tb\n\tc").into_iter().collect();
        assert_eq!(expanded, "a       b\n        c");
    }

    #[test]
    fn superscripts_shrink_and_rise() {
        let layout = typeset("$x^2$").expect("valid");
        let [x, two] = layout.glyphs[..] else {
            panic!("two glyphs expected")
        };
        assert_eq!((x.size, two.size), (10.0, 7.0));
        assert!(two.y < x.y && two.x > x.x);
    }

    #[test]
    fn a_space_after_a_caret_is_the_superscript() {
        let layout = typeset("$x ^ 2$").expect("valid");
        assert_eq!(layout.glyphs.len(), 3);
        assert_eq!(layout.glyphs[1].code, u32::from(' '));
        assert_eq!(layout.glyphs[2].size, 10.0);
    }

    #[test]
    fn operators_are_spaced_unless_unary() {
        assert!(width("$a+b$") > width("$a{+}b$"));
        assert_eq!(width("$-1$"), width("${-}1$"));
        let minus = typeset("$-$").expect("valid");
        assert_eq!(minus.glyphs[0].code, 0x2212);
        assert!(width("$1,5$") > width("$1.5$"));
    }

    #[test]
    fn functions_are_upright_with_a_thin_space() {
        assert!(width(r"$\sin x$") > width(r"$\mathrm{sin}x$"));
        assert_eq!(width(r"$\sin(x)$"), width(r"$\mathrm{sin}(x)$"));
        let limit = typeset(r"$\lim_{n}$").expect("valid");
        assert!(limit.glyphs[3].y > limit.glyphs[0].y, "limits go below");
    }

    #[test]
    fn fractions_radicals_and_delimiters() {
        assert_eq!(typeset(r"$\frac{a}{b}$").expect("valid").rects.len(), 1);
        let binom = typeset(r"$\binom{a}{b}$").expect("valid");
        assert_eq!((binom.glyphs.len(), binom.rects.len()), (4, 0));
        let root = typeset(r"$\sqrt{x}$").expect("valid");
        assert_eq!((root.glyphs.len(), root.rects.len()), (2, 1));
        let tall = typeset(r"$\left(\frac{a}{b}\right)$").expect("valid");
        let paren = tall.glyphs[0];
        assert!(paren.ink.height() > typeset("$(x)$").expect("valid").glyphs[0].ink.height());
    }

    #[test]
    fn font_switches_leak_out_of_arguments() {
        let leaked = typeset(r"$\frac{\rm a}{b} c$").expect("valid");
        let plain = typeset(r"$\frac{a}{b} c$").expect("valid");
        assert_ne!(leaked.glyphs[2].font, plain.glyphs[2].font);
        let grouped = typeset(r"${\rm a} c$").expect("valid");
        assert_eq!(grouped.glyphs[1].font, plain.glyphs[2].font);
    }

    #[test]
    fn primes_phantoms_and_text() {
        let prime = typeset("$f'$").expect("valid");
        assert_eq!((prime.glyphs.len(), prime.glyphs[1].size), (2, 7.0));
        let phantom = typeset(r"$\phantom{x}y$").expect("valid");
        assert_eq!(phantom.glyphs.len(), 1);
        assert!(phantom.glyphs[0].x > 0.0);
        let mixed = typeset("a $b$").expect("valid");
        assert_eq!(mixed.glyphs.len(), 3);
        assert_eq!(typeset("").expect("empty").width, 0.0);
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        let depth = MAX_NESTING + 5;
        let nested = format!("${}x{}$", "{".repeat(depth), "}".repeat(depth));
        assert!(message(&nested).contains("recursion"));
        let scripts = format!("${}x{}$", "x^{".repeat(depth), "}".repeat(depth));
        assert!(message(&scripts).contains("recursion"));
        let fine = format!("${}x{}$", "{".repeat(20), "}".repeat(20));
        assert!(typeset(&fine).is_ok());
    }

    #[test]
    fn errors_carry_matplotlib_messages() {
        assert!(message(r"$\foo$").contains("ParseFatalException: Unknown symbol: \\foo"));
        assert!(message("$x^2^3$").contains("Double superscript, found 'x'"));
        assert!(message("$x_1_2$").contains("Double subscript"));
        assert!(message(r"$\frac{a}$").contains("Expected \\frac{num}{den}"));
        assert!(message(r"$\left(x$").contains("Expected '\\\\right'"));
        assert!(message("${$").contains("Expected '}'"));
        assert!(message("$x$ $").contains("ParseException: Expected end of text"));
        assert!(message(r"$x^$").starts_with("\nx^\n  ^\nParseSyntaxException: Expected {phantom"));
    }
}
