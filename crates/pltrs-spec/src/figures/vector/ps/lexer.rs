//! PostScript tokens: numbers, names, literal names, strings, procedure
//! and array/dictionary delimiters, with raw hex reading for image data.

use crate::figures::vector::postscript::is_delimiter;

/// One token.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Tok {
    Num(f64),
    /// An executable name (or operator).
    Name(String),
    /// A literal name (`/name`).
    Lit(String),
    Str(Vec<u8>),
    ProcStart,
    ProcEnd,
}

/// A cursor over a document.
pub(super) struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub(super) const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn skip_space(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else if b == b'%' {
                while self.peek().is_some_and(|b| b != b'\n') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Reads `count` bytes of hex data following an image operator.
    pub(super) fn read_hex(&mut self, count: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(count);
        let mut high: Option<u8> = None;
        while out.len() < count {
            let Some(b) = self.peek() else { break };
            self.pos += 1;
            let Some(digit) = (b as char).to_digit(16) else {
                continue;
            };
            match high.take() {
                Some(h) => out.push(h << 4 | digit as u8),
                None => high = Some(digit as u8),
            }
        }
        out
    }

    fn word(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(|b| !is_delimiter(b)) {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.data[start..self.pos]).into_owned()
    }

    fn literal_string(&mut self) -> Vec<u8> {
        self.pos += 1;
        let mut out = Vec::new();
        let mut depth = 1;
        while let Some(b) = self.peek() {
            self.pos += 1;
            match b {
                b'\\' => {
                    if let Some(e) = self.peek() {
                        self.pos += 1;
                        out.push(match e {
                            b'n' => b'\n',
                            b'r' => b'\r',
                            b't' => b'\t',
                            other => other,
                        });
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(b);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(b);
                }
                _ => out.push(b),
            }
        }
        out
    }
}

impl Iterator for Lexer<'_> {
    type Item = Tok;

    fn next(&mut self) -> Option<Tok> {
        self.skip_space();
        let b = self.peek()?;
        Some(match b {
            b'{' => {
                self.pos += 1;
                Tok::ProcStart
            }
            b'}' => {
                self.pos += 1;
                Tok::ProcEnd
            }
            b'[' | b']' => {
                self.pos += 1;
                Tok::Name(char::from(b).to_string())
            }
            b'<' if self.data.get(self.pos + 1) == Some(&b'<') => {
                self.pos += 2;
                Tok::Name("<<".to_owned())
            }
            b'>' if self.data.get(self.pos + 1) == Some(&b'>') => {
                self.pos += 2;
                Tok::Name(">>".to_owned())
            }
            b'<' => {
                self.pos += 1;
                let start = self.pos;
                while self.peek().is_some_and(|b| b != b'>') {
                    self.pos += 1;
                }
                let hex = self.data[start..self.pos].to_vec();
                self.pos += 1;
                Tok::Str(Lexer::new(&hex).read_hex(hex.len()))
            }
            b'(' => Tok::Str(self.literal_string()),
            b'/' => {
                self.pos += 1;
                Tok::Lit(self.word())
            }
            _ => {
                let word = self.word();
                if word.is_empty() {
                    self.pos += 1;
                    return self.next();
                }
                match word.parse::<f64>() {
                    Ok(number) => Tok::Num(number),
                    Err(_) => Tok::Name(word),
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens() {
        let tokens: Vec<Tok> =
            Lexer::new(b"%%Comment\n/o { 1.5 -2 m } bind def <0aFf> << /A [1] >>").collect();
        assert_eq!(tokens[0], Tok::Lit("o".to_owned()));
        assert_eq!(tokens[1], Tok::ProcStart);
        assert_eq!(tokens[2], Tok::Num(1.5));
        assert_eq!(tokens[4], Tok::Name("m".to_owned()));
        assert_eq!(tokens[8], Tok::Str(vec![0x0a, 0xff]));
        assert_eq!(tokens[9], Tok::Name("<<".to_owned()));
        assert_eq!(tokens[11], Tok::Name("[".to_owned()));
        let mut lexer = Lexer::new(b"ff00\n7f x");
        assert_eq!(lexer.read_hex(3), [0xff, 0x00, 0x7f]);
        assert_eq!(lexer.next(), Some(Tok::Name("x".to_owned())));
    }
}
