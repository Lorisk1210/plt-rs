//! Symbols and words: operator spacing, punctuation, spaces, function
//! names, `\operatorname`, `\boldsymbol` and `\text`.

use super::error::{ErrorKind, Fail, NO_MATCH};
use super::{Parsed, Parser};
use crate::mathtext::boxes::{Char, List, Node};
use crate::mathtext::data::{
    delimiters, is_binary_operator, is_digit, is_left_delim, is_punctuation, is_relation,
    is_small_greek, is_space, is_spaced_symbol,
};
use crate::mathtext::font_name::FontName;
use crate::mathtext::fonts::GlyphError;

/// Width of a thin space (`\,`) in em.
pub(super) const THIN_SPACE: f64 = 0.16667;
/// Space around operators and after punctuation, in em.
const OPERATOR_SPACE: f64 = 0.2;

/// Backslash escapes of `\text{...}`.
fn text_escape(c: char) -> char {
    match c {
        't' => '\t',
        'n' => '\n',
        'f' => '\u{c}',
        'r' => '\r',
        other => other,
    }
}

impl Parser<'_, '_> {
    /// A kern of `em` times the quad of the current size.
    pub(super) fn make_space(&mut self, em: f64) -> Result<Node, Fail> {
        let fontsize = self.state().fontsize;
        let quad = self.fonts.quad(FontName::Normal, fontsize)?;
        Ok(Node::kern(quad * em))
    }

    /// The last non-space character before `loc`.
    fn previous_char(&self, loc: usize) -> Option<char> {
        self.src[..loc].iter().rev().find(|&&c| c != ' ').copied()
    }

    /// The first non-space character from `loc`.
    fn next_char(&self, loc: usize) -> Option<char> {
        self.src
            .get(loc..)
            .and_then(|rest| rest.iter().find(|&&c| c != ' ').copied())
    }

    /// The `symbol` action: a character, with thick spaces around
    /// operators, relations and arrows, and a space after punctuation.
    pub(super) fn symbol(&mut self, loc: usize, symbol: &str) -> Result<Node, Fail> {
        let symbol = if symbol == "-" { "\u{2212}" } else { symbol };
        let state = *self.state();
        let node = match Char::new(symbol, &state, self.fonts) {
            Ok(c) => Node::Char(c),
            Err(GlyphError::UnknownSymbol(_)) => {
                let msg = format!("Unknown symbol: {symbol}");
                return Err(self.fatal(ErrorKind::Fatal, &msg, loc));
            }
            Err(err) => return Err(err.into()),
        };
        let previous = self.previous_char(loc);
        if is_spaced_symbol(symbol) {
            let at_start = self.src[..loc].iter().all(|&c| is_space(c));
            let after_opening = previous.is_some_and(|p| {
                let p = p.to_string();
                p == "{" || is_left_delim(&p) || is_relation(&p)
            });
            let unary = is_binary_operator(symbol) && (at_start || after_opening);
            if self.needs_space_after_subsuper || unary {
                return Ok(node);
            }
            let before = self.make_space(OPERATOR_SPACE)?;
            let after = self.make_space(OPERATOR_SPACE)?;
            return Ok(List::hlist(vec![before, node, after], self.fonts)?.into_node());
        }
        if is_punctuation(symbol) {
            let next = self.next_char(loc + 1);
            let in_braces = symbol == "," && previous == Some('{') && next == Some('}');
            let decimal =
                symbol == "." && previous.is_some_and(is_digit) && next.is_some_and(is_digit);
            if in_braces || decimal {
                return Ok(node);
            }
            let after = self.make_space(OPERATOR_SPACE)?;
            return Ok(List::hlist(vec![node, after], self.fonts)?.into_node());
        }
        Ok(node)
    }

    /// A function name (`\sin`, `\lim`, ...): upright letters plus a thin
    /// space unless a delimiter or script follows.
    pub(super) fn function(&mut self, loc: usize, end: usize, name: &str) -> Parsed<Node> {
        self.push_state();
        self.state_mut().set_font(FontName::Rm);
        let state = *self.state();
        let letters = name
            .chars()
            .map(|c| Ok(Node::Char(Char::new(&c.to_string(), &state, self.fonts)?)))
            .collect::<Result<Vec<_>, Fail>>()?;
        let next_loc = loc + name.chars().count() + 1;
        let mut list = self.operator(letters, next_loc)?;
        list.function_name = Some(name.to_owned());
        Ok((end, list.into_node()))
    }

    /// `\operatorname{...}`: the argument set upright.
    pub(super) fn operatorname(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\operatorname")?;
        Some(self.operatorname_body(loc, after))
    }

    fn operatorname_body(&mut self, loc: usize, after: usize) -> Parsed<Node> {
        let (end, mut items) = self.args(after, "\\operatorname{name}", Self::simple_items)?;
        self.push_state();
        self.state_mut().set_font(FontName::Rm);
        for item in &mut items {
            self.refont(item, FontName::Rm)?;
        }
        // matplotlib counts the parsed items, not the characters.
        let next_loc = loc + items.len() + 1 + "operatorname{}".len();
        let list = self.operator(items, next_loc)?;
        Ok((end, list.into_node()))
    }

    /// Finishes an operator in the pushed upright state: the thin space
    /// and the script spacing flag depend on the character at `next_loc`.
    fn operator(&mut self, mut items: Vec<Node>, next_loc: usize) -> Result<List, Fail> {
        let next = self.next_char(next_loc);
        let is_script = matches!(next, Some('^' | '_'));
        let is_delimiter = next.is_some_and(|c| {
            let c = c.to_string();
            delimiters().any(|d| d == c)
        });
        if !is_script && !is_delimiter {
            items.push(self.make_space(THIN_SPACE)?);
        }
        self.pop_state();
        self.needs_space_after_subsuper = is_script;
        Ok(List::hlist(items, self.fonts)?)
    }

    /// `\boldsymbol{...}`: bold, with Latin and Greek letters bold italic.
    pub(super) fn boldsymbol(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\boldsymbol")?;
        Some(self.boldsymbol_body(loc, after))
    }

    fn boldsymbol_body(&mut self, loc: usize, after: usize) -> Parsed<Node> {
        let (end, mut items) = self.args(after, "\\boldsymbol{value}", Self::simple_items)?;
        self.push_state();
        for item in &mut items {
            match item {
                Node::List(list) if !list.vertical => {
                    // matplotlib looks at the second child (the operator
                    // of a spaced symbol) and fails on shorter lists.
                    let Some(second) = list.children.get_mut(1) else {
                        return Err(Fail::miss(loc, NO_MATCH));
                    };
                    self.refont(second, FontName::Bf)?;
                }
                Node::Char(c) => {
                    let symbol = c.symbol.as_str();
                    let latin =
                        symbol.len() == 1 && symbol.chars().all(|c| c.is_ascii_alphabetic());
                    let greek = symbol.get(1..).is_some_and(is_small_greek);
                    let font = if latin || greek {
                        FontName::Bfit
                    } else {
                        FontName::Bf
                    };
                    self.refont(item, font)?;
                }
                _ => {}
            }
        }
        self.pop_state();
        Ok((end, List::hlist(items, self.fonts)?.into_node()))
    }

    /// `\text{...}`: the raw text in the upright font.
    pub(super) fn text(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\text")?;
        Some(self.text_body(after))
    }

    fn text_body(&mut self, after: usize) -> Parsed<Node> {
        let (end, text) = self.args(after, "\\text", |p, loc| p.braced_text(loc))?;
        self.push_state();
        self.state_mut().set_font(FontName::Rm);
        let state = *self.state();
        let chars = text
            .chars()
            .map(|c| Ok(Node::Char(Char::new(&c.to_string(), &state, self.fonts)?)))
            .collect::<Result<Vec<_>, Fail>>()?;
        self.pop_state();
        Ok((end, List::hlist(chars, self.fonts)?.into_node()))
    }

    /// A brace-delimited literal with nested braces (which only group)
    /// and backslash escapes.
    fn braced_text(&self, loc: usize) -> Parsed<String> {
        let mut pos = self.skip_ws(loc);
        if self.at(pos) != Some('{') {
            return Err(Fail::miss(pos, NO_MATCH));
        }
        let mut text = String::new();
        let mut depth = 0usize;
        while let Some(c) = self.at(pos) {
            if c == '\\'
                && let Some(escaped) = self.at(pos + 1)
            {
                text.push(text_escape(escaped));
                pos += 2;
                continue;
            }
            pos += 1;
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((pos, text));
                    }
                }
                c => text.push(c),
            }
        }
        Err(Fail::miss(pos, NO_MATCH))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_escapes() {
        assert_eq!(text_escape('t'), '\t');
        assert_eq!(text_escape('{'), '{');
    }
}
