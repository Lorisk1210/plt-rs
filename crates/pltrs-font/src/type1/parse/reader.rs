//! The token-level reading of the dictionary: balanced expressions,
//! binary runs after the `RD` procedure, and the `/Subrs`,
//! `/CharStrings`, `/Encoding` and `/OtherSubrs` entries.

use std::collections::BTreeMap;

use super::Abbreviations;
use crate::type1::error::{Type1Error, Type1Section};
use crate::type1::props::Type1Encoding;
use crate::type1::python::{ascii_lossy, slice};
use crate::type1::token::{Token, Tokenizer};
use crate::type1::value::PsNumber;

/// A section's result and where its definition ends.
type SectionEnd<T> = (T, usize);

/// The declared glyph count and the glyphs' encrypted programs.
pub(super) type RawCharStrings = (i64, Vec<(String, Vec<u8>)>);

pub(super) struct Parser<'a> {
    pub(super) data: &'a [u8],
    pub(super) tokens: Tokenizer<'a>,
    pub(super) abbreviations: Abbreviations,
}

/// The token, or the end-of-data error matplotlib's `StopIteration`
/// stands for.
fn required(token: Option<Token<'_>>) -> Result<Token<'_>, Type1Error> {
    token.ok_or(Type1Error::UnexpectedEnd)
}

impl<'a> Parser<'a> {
    fn next(&mut self) -> Result<Token<'a>, Type1Error> {
        required(self.tokens.next_token()?)
    }

    fn next_matching(
        &mut self,
        wanted: impl Fn(&Token<'a>) -> bool,
    ) -> Result<Token<'a>, Type1Error> {
        required(self.tokens.next_matching(wanted)?)
    }

    /// The source of the balanced expression `initial` opens (matplotlib's
    /// `_expression`).
    pub(super) fn expression(&mut self, initial: Token<'a>) -> Result<String, Type1Error> {
        let mut open: Vec<Token<'a>> = Vec::new();
        let mut token = initial;
        loop {
            if token.is_delimiter() {
                match token.raw {
                    b"[" | b"{" => open.push(token),
                    b"]" | b"}" => {
                        let opening = open
                            .pop()
                            .ok_or(Type1Error::UnbalancedDelimiter(token.pos))?;
                        if Some(opening.raw) != token.opposite() {
                            return Err(Type1Error::UnbalancedDelimiter(token.pos));
                        }
                        if open.is_empty() {
                            break;
                        }
                    }
                    _ => return Err(Type1Error::UnbalancedDelimiter(token.pos)),
                }
            } else if open.is_empty() {
                break;
            }
            token = self.next()?;
        }
        let range = slice(self.data.len(), initial.pos as i64, token.end() as i64);
        Ok(ascii_lossy(&self.data[range]))
    }

    /// A number token, else an error naming what `section` expected.
    fn number(
        &mut self,
        section: Type1Section,
        expected: &'static str,
    ) -> Result<Token<'a>, Type1Error> {
        let token = self.next()?;
        if token.is_number() {
            Ok(token)
        } else {
            Err(Type1Error::UnexpectedToken {
                section,
                expected,
                at: token.pos,
            })
        }
    }

    /// The integer of a number token (a real would make Python raise
    /// `TypeError` where matplotlib uses it as a length or count).
    fn integer(token: &Token<'_>) -> Result<i64, Type1Error> {
        token
            .integer()?
            .ok_or_else(|| Type1Error::InvalidNumber(token.text()))
    }

    /// `<length> RD <binary>`: the binary data after the `RD` token.
    fn binary(&mut self, section: Type1Section) -> Result<Vec<u8>, Type1Error> {
        let length = Self::integer(&self.number(section, "a length")?)?;
        let rd = self.next()?;
        if !rd.is_keyword(&[self.abbreviations.rd.as_str()]) {
            return Err(Type1Error::UnexpectedToken {
                section,
                expected: "the RD procedure",
                at: rd.pos,
            });
        }
        let length = length
            .checked_add(1)
            .ok_or_else(|| Type1Error::InvalidNumber(length.to_string()))?;
        Ok(required(self.tokens.binary(length)?)?.binary_value())
    }

    /// `/Subrs <count> array` followed by `dup <index> <length> RD <binary>
    /// NP` for each subroutine.
    fn subrs(&mut self) -> Result<SectionEnd<Vec<Vec<u8>>>, Type1Error> {
        let section = Type1Section::Subrs;
        let count = Self::integer(&self.number(section, "a count")?)?;
        self.next_matching(|t| t.is_keyword(&["array"]))?;
        // The entries are only allocated as they are read, so a huge count
        // in a few bytes allocates nothing.
        let mut entries = BTreeMap::new();
        let mut integral = true;
        for _ in 0..count.max(0) {
            self.next_matching(|t| t.is_keyword(&["dup"]))?;
            let index = self.number(section, "an index")?;
            let binary = self.binary(section)?;
            match index.number()? {
                PsNumber::Int(index) => {
                    entries.insert(index, binary);
                }
                PsNumber::Real(index) if index.fract() == 0.0 && index.abs() < 9e15 => {
                    entries.insert(index as i64, binary);
                }
                PsNumber::Real(_) => integral = false,
            }
        }
        let covers = integral
            && entries.len() as i64 == count
            && (count == 0
                || (entries.first_key_value().map(|(&k, _)| k) == Some(0)
                    && entries.last_key_value().map(|(&k, _)| k) == Some(count - 1)));
        if !covers {
            return Err(Type1Error::SubrsIndices);
        }
        let end = self.next()?.end();
        Ok((entries.into_values().collect(), end))
    }

    /// `/CharStrings <count> dict dup begin` followed by `/<name> <length>
    /// RD <binary> ND` for each glyph, up to `end`.
    fn char_strings(&mut self) -> Result<SectionEnd<RawCharStrings>, Type1Error> {
        let section = Type1Section::CharStrings;
        let declared = match self.number(section, "a count")?.number()? {
            PsNumber::Int(count) => count,
            // FreeType reads the integer part.
            PsNumber::Real(count) => count.trunc() as i64,
        };
        self.next_matching(|t| t.is_keyword(&["begin"]))?;
        let mut glyphs = Vec::new();
        loop {
            let token = self.next_matching(|t| t.is_keyword(&["end"]) || t.is_slash_name())?;
            if token.raw == b"end" {
                return Ok(((declared, glyphs), token.end()));
            }
            let name = ascii_lossy(&token.raw[1..]);
            glyphs.push((name, self.binary(section)?));
        }
    }

    /// `/Encoding StandardEncoding def`, or an array filled by `dup <code>
    /// /<name> put` up to `def`; malformed entries are skipped.
    fn encoding(&mut self) -> Result<SectionEnd<Type1Encoding>, Type1Error> {
        let mut codes = BTreeMap::new();
        loop {
            let token =
                self.next_matching(|t| t.is_keyword(&["StandardEncoding", "dup", "def"]))?;
            if token.is_keyword(&["StandardEncoding"]) {
                return Ok((Type1Encoding::standard(), token.end()));
            }
            if token.is_keyword(&["def"]) {
                return Ok((Type1Encoding::from_codes(codes), token.end()));
            }
            let code = self.next()?;
            if !code.is_number() {
                continue;
            }
            let name = self.next()?;
            if !name.is_slash_name() {
                continue;
            }
            let code = match code.number()? {
                PsNumber::Int(code) => Some(code),
                PsNumber::Real(code) if code.fract() == 0.0 && code.abs() < 9e15 => {
                    Some(code as i64)
                }
                PsNumber::Real(_) => None,
            };
            if let Some(code) = code {
                codes.insert(code, ascii_lossy(&name.raw[1..]));
            }
        }
    }

    /// The `/OtherSubrs` procedures, from the first token after the key to
    /// the `def`.
    fn other_subrs(&mut self) -> Result<SectionEnd<Vec<u8>>, Type1Error> {
        let mut start = None;
        loop {
            let token = self.next()?;
            let start = *start.get_or_insert(token.pos);
            if token.is_delimiter() {
                self.expression(token)?;
            } else if token.is_keyword(&["def", self.abbreviations.nd.as_str()]) {
                let range = slice(self.data.len(), start as i64, token.end() as i64);
                return Ok((self.data[range].to_vec(), token.end()));
            }
        }
    }
}

/// The raw (not yet decrypted) special sections.
#[derive(Default)]
pub(super) struct Sections {
    pub(super) encoding: Option<Type1Encoding>,
    pub(super) char_strings: Option<RawCharStrings>,
    pub(super) subrs: Option<Vec<Vec<u8>>>,
    pub(super) other_subrs: Option<Vec<u8>>,
}

/// Reads the special section `key` names, returning where it ends;
/// `None` when `key` is a plain entry.
pub(super) fn read_section(
    parser: &mut Parser<'_>,
    key: &str,
    sections: &mut Sections,
) -> Result<Option<usize>, Type1Error> {
    let section = match key {
        "Subrs" => Type1Section::Subrs,
        "CharStrings" => Type1Section::CharStrings,
        "Encoding" => Type1Section::Encoding,
        "OtherSubrs" => Type1Section::OtherSubrs,
        _ => return Ok(None),
    };
    let incomplete = |err| match err {
        Type1Error::UnexpectedEnd => Type1Error::Incomplete(section),
        err => err,
    };
    let end = match section {
        Type1Section::Subrs => {
            let (subrs, end) = parser.subrs().map_err(incomplete)?;
            sections.subrs = Some(subrs);
            end
        }
        Type1Section::CharStrings => {
            let (glyphs, end) = parser.char_strings().map_err(incomplete)?;
            sections.char_strings = Some(glyphs);
            end
        }
        Type1Section::Encoding => {
            let (encoding, end) = parser.encoding().map_err(incomplete)?;
            sections.encoding = Some(encoding);
            end
        }
        Type1Section::OtherSubrs => {
            let (source, end) = parser.other_subrs().map_err(incomplete)?;
            sections.other_subrs = Some(source);
            end
        }
    };
    Ok(Some(end))
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use super::super::tests::{charstring, font};
    use crate::type1::error::{Type1Error, Type1Section};

    #[test]
    fn subrs_indices_must_cover_the_count() {
        let subrs = |indices: [i32; 2]| {
            let body = indices
                .iter()
                .map(|i| format!("dup {i} 5 RD \x00\x01\x02\x03\x04 NP\n"))
                .collect::<Vec<_>>()
                .concat();
            parse(&font(&format!(
                "/Subrs 2 array\n{body}ND\n{}",
                charstring("")
            )))
        };
        assert_eq!(subrs([0, 1]).unwrap().prop.subrs().map(<[_]>::len), Some(2));
        for bad in [[0, 5], [0, 0], [-1, 1]] {
            assert_eq!(subrs(bad).unwrap_err(), Type1Error::SubrsIndices);
        }
        // A huge count with an empty body fails without allocating.
        assert_eq!(
            parse(&font("/Subrs 5000000 array\n")).unwrap_err(),
            Type1Error::Incomplete(Type1Section::Subrs)
        );
    }

    #[test]
    fn incomplete_sections() {
        let cases = [
            (
                "/CharStrings 1 begin\n/.notdef 5 RD \x00\x01\x02\x03\x04 ND\n",
                Type1Section::CharStrings,
            ),
            ("/CharStrings 1\n", Type1Section::CharStrings),
            ("/CharStrings 1 begin\n", Type1Section::CharStrings),
            (
                "/Encoding 1 array\ndup 0 /.notdef put\n",
                Type1Section::Encoding,
            ),
            ("/OtherSubrs [ {} {} \n", Type1Section::OtherSubrs),
        ];
        for (private, section) in cases {
            assert_eq!(
                parse(&font(private)).unwrap_err(),
                Type1Error::Incomplete(section)
            );
        }
    }
}
