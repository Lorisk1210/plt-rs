//! Lexical matching on the string being parsed: whitespace, literals,
//! command names and the terminal regexes of the grammar.

use super::Parser;
use crate::mathtext::data::{
    self, SPACE_WIDTHS, accent_names, delimiters, font_names, is_font_name, is_function_name,
};
use crate::mathtext::font_name::FontName;

/// pyparsing's default whitespace.
fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// Characters that are symbols by themselves.
fn is_symbol_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || " +-*/<>=:,.;!?&'@()[]|".contains(c)
        || ('\u{80}'..='\u{1ffff}').contains(&c)
}

/// Accents written with a punctuation character (`\"`, `\'`, `\^`, ...).
const PUNCTUATION_ACCENTS: [char; 6] = ['"', '`', '\'', '~', '.', '^'];

impl Parser<'_, '_> {
    /// Number of characters in the current string.
    pub(super) fn len(&self) -> usize {
        self.src.len()
    }

    /// The character at `loc`.
    pub(super) fn at(&self, loc: usize) -> Option<char> {
        self.src.get(loc).copied()
    }

    /// The first position at or after `loc` that is not whitespace.
    pub(super) fn skip_ws(&self, loc: usize) -> usize {
        let mut pos = loc;
        while self.at(pos).is_some_and(is_whitespace) {
            pos += 1;
        }
        pos
    }

    /// The characters `from..to` as a string.
    pub(super) fn slice(&self, from: usize, to: usize) -> String {
        self.src[from..to].iter().collect()
    }

    /// Where `literal` ends if it starts at `loc`.
    pub(super) fn literal(&self, loc: usize, literal: &str) -> Option<usize> {
        let mut pos = loc;
        for c in literal.chars() {
            if self.at(pos) != Some(c) {
                return None;
            }
            pos += 1;
        }
        Some(pos)
    }

    /// The longest of `options` starting at `loc` (pyparsing's `one_of`).
    pub(super) fn one_of<'s>(
        &self,
        loc: usize,
        options: impl Iterator<Item = &'s str>,
    ) -> Option<(usize, &'s str)> {
        options
            .filter_map(|option| self.literal(loc, option).map(|end| (end, option)))
            .max_by_key(|&(end, _)| end)
    }

    /// `\` followed by one or more ASCII letters: the end and the letters.
    pub(super) fn command(&self, loc: usize) -> Option<(usize, String)> {
        if self.at(loc) != Some('\\') {
            return None;
        }
        let mut end = loc + 1;
        while self.at(end).is_some_and(|c| c.is_ascii_alphabetic()) {
            end += 1;
        }
        (end > loc + 1).then(|| (end, self.slice(loc + 1, end)))
    }

    /// The `symbol` regex: a plain character, an escaped special
    /// character, or a known TeX symbol name.
    pub(super) fn scan_symbol(&self, loc: usize) -> Option<(usize, String)> {
        let c = self.at(loc)?;
        if is_symbol_char(c) {
            return Some((loc + 1, c.to_string()));
        }
        if c != '\\' {
            return None;
        }
        let next = self.at(loc + 1)?;
        if "%${}[]_|".contains(next) {
            return Some((loc + 2, format!("\\{next}")));
        }
        if let Some((end, name)) = self.command(loc) {
            return data::tex2uni(&name)
                .is_some()
                .then(|| (end, format!("\\{name}")));
        }
        let letter_follows = self.at(loc + 2).is_some_and(|c| c.is_ascii_alphabetic());
        (next == '#' && !letter_follows).then(|| (loc + 2, "\\#".to_owned()))
    }

    /// A spacing command (`\,`, `\quad`, `~`, ...) and its width in em.
    pub(super) fn space_command(&self, loc: usize) -> Option<(usize, f64)> {
        let (end, name) = self.one_of(loc, SPACE_WIDTHS.iter().map(|(name, _)| *name))?;
        let width = SPACE_WIDTHS.iter().find(|(n, _)| *n == name)?.1;
        Some((end, width))
    }

    /// A font switch such as `\rm` or `\bf`.
    pub(super) fn font_command(&self, loc: usize) -> Option<(usize, FontName)> {
        let (end, name) = self.command(loc)?;
        if !is_font_name(&name) {
            return None;
        }
        FontName::parse(&name).map(|font| (end, font))
    }

    /// A font name after `\math` (`rm`, `bfit`, ...).
    pub(super) fn font_name(&self, loc: usize) -> Option<(usize, FontName)> {
        let (end, name) = self.one_of(loc, font_names())?;
        FontName::parse(name).map(|font| (end, font))
    }

    /// A function name such as `\sin` or `\lim`.
    pub(super) fn function_command(&self, loc: usize) -> Option<(usize, String)> {
        self.command(loc).filter(|(_, name)| is_function_name(name))
    }

    /// An accent command: `\hat`, `\widehat`, `\"`, ...
    pub(super) fn accent_command(&self, loc: usize) -> Option<(usize, String)> {
        if let Some((end, name)) = self.command(loc) {
            return accent_names()
                .any(|accent| accent == name)
                .then_some((end, name));
        }
        if self.at(loc) != Some('\\') {
            return None;
        }
        let c = self
            .at(loc + 1)
            .filter(|c| PUNCTUATION_ACCENTS.contains(c))?;
        Some((loc + 2, c.to_string()))
    }

    /// A delimiter for `\left`, `\middle` and `\right`.
    pub(super) fn delimiter(&self, loc: usize) -> Option<(usize, &'static str)> {
        self.one_of(loc, delimiters())
    }

    /// pyparsing's `float_literal`: `[-+]?([0-9]+\.?[0-9]*|\.[0-9]+)`.
    pub(super) fn float_literal(&self, loc: usize) -> Option<(usize, f64)> {
        let mut pos = loc;
        if matches!(self.at(pos), Some('+' | '-')) {
            pos += 1;
        }
        let digits = |mut p: usize| {
            while self.at(p).is_some_and(|c| c.is_ascii_digit()) {
                p += 1;
            }
            p
        };
        let int_end = digits(pos);
        let end = if int_end > pos {
            if self.at(int_end) == Some('.') {
                digits(int_end + 1)
            } else {
                int_end
            }
        } else if self.at(pos) == Some('.') && digits(pos + 1) > pos + 1 {
            digits(pos + 1)
        } else {
            return None;
        };
        self.slice(loc, end).parse().ok().map(|value| (end, value))
    }

    /// `\\`, the row separator of `\substack`.
    pub(super) fn row_separator(&self, loc: usize) -> Option<usize> {
        self.literal(loc, "\\\\")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_characters() {
        assert!(is_symbol_char('a') && is_symbol_char(' ') && is_symbol_char('é'));
        assert!(!is_symbol_char('{') && !is_symbol_char('^') && !is_symbol_char('~'));
        assert!(!is_symbol_char('\u{20000}'));
        assert!(is_whitespace('\t') && !is_whitespace('\u{a0}'));
    }
}
