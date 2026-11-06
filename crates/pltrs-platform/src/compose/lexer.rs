//! The tokens of Compose files, as libxkbcommon's scanner reads them:
//! keysyms in angle brackets, strings with C escapes, identifiers, `:`,
//! `!` and `~`, and include paths with their `%` expansions.

use std::path::Path;

/// The longest token, in bytes (libxkbcommon's scanner buffer).
const MAX_TOKEN: usize = 1023;

/// A token of a Compose file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    /// The end of a line.
    EndOfLine,
    /// The end of the file.
    EndOfFile,
    /// `<name>`: a keysym of a sequence.
    Keysym(String),
    /// `:`, between a sequence and what it types.
    Colon,
    /// `!`, before modifiers that must match exactly.
    Bang,
    /// `~`, before a modifier that must not be held.
    Tilde,
    /// A string literal, escapes resolved.
    Text(String),
    /// An identifier: a keysym name, a modifier or `None`.
    Ident(String),
    /// `include`.
    Include,
    /// Something that is no token, or a token too long; the rest of the
    /// line is skipped.
    Error,
}

/// What `%` expands to in include paths: `%H` the home directory, `%L`
/// the locale's Compose file, `%S` the X locale directory.
#[derive(Debug, Clone, Copy)]
pub(super) struct Expansions<'a> {
    /// `%H`.
    pub(super) home: Option<&'a Path>,
    /// `%L`.
    pub(super) locale_file: Option<&'a Path>,
    /// `%S`.
    pub(super) locale_dir: &'a Path,
}

/// Reads tokens off a Compose file's bytes.
#[derive(Debug)]
pub(super) struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// Whether the last token ended its line (an end of line or file).
    line_ended: bool,
}

const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t'..=b'\r')
}

impl<'a> Lexer<'a> {
    /// A lexer at the start of `bytes`, past a UTF-8 byte order mark.
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
        Self {
            bytes,
            pos: 0,
            line_ended: false,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// Takes the next byte if it is `byte`.
    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.pos += 1;
        }
        found
    }

    fn at_line_end(&self) -> bool {
        matches!(self.peek(), None | Some(b'\n'))
    }

    /// Skips to the end of the line, leaving the newline.
    fn skip_to_line_end(&mut self) {
        while !self.at_line_end() {
            self.pos += 1;
        }
    }

    /// Skips spaces and comments; `true` when a newline was passed.
    fn skip_blanks(&mut self) -> bool {
        loop {
            while let Some(byte) = self.peek().filter(|&byte| is_space(byte)) {
                self.pos += 1;
                if byte == b'\n' {
                    return true;
                }
            }
            if !self.eat(b'#') {
                return false;
            }
            self.skip_to_line_end();
        }
    }

    /// The next token.
    pub(super) fn next_token(&mut self) -> Token {
        let token = self.token();
        self.line_ended = matches!(token, Token::EndOfLine | Token::EndOfFile);
        token
    }

    /// Skips what is left of the line the last token was on.
    pub(super) fn skip_line(&mut self) {
        while !self.line_ended {
            self.next_token();
        }
    }

    fn token(&mut self) -> Token {
        if self.skip_blanks() {
            return Token::EndOfLine;
        }
        let Some(byte) = self.peek() else {
            return Token::EndOfFile;
        };
        match byte {
            b'<' => {
                self.pos += 1;
                self.keysym()
            }
            b':' | b'!' | b'~' => {
                self.pos += 1;
                match byte {
                    b':' => Token::Colon,
                    b'!' => Token::Bang,
                    _ => Token::Tilde,
                }
            }
            b'"' => {
                self.pos += 1;
                self.text()
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => self.ident(),
            _ => {
                self.skip_to_line_end();
                Token::Error
            }
        }
    }

    fn keysym(&mut self) -> Token {
        let start = self.pos;
        while !self.at_line_end() && self.peek() != Some(b'>') {
            self.pos += 1;
        }
        let name = &self.bytes[start..self.pos];
        if !self.eat(b'>') || name.len() > MAX_TOKEN {
            return Token::Error;
        }
        // Keysym names are ASCII: anything else names no keysym.
        Token::Keysym(String::from_utf8_lossy(name).into_owned())
    }

    fn ident(&mut self) -> Token {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            self.pos += 1;
        }
        let name = &self.bytes[start..self.pos];
        if name.len() > MAX_TOKEN {
            return Token::Error;
        }
        if name == b"include" {
            return Token::Include;
        }
        Token::Ident(String::from_utf8_lossy(name).into_owned())
    }

    /// A string literal after its opening quote: `\\`, `\"`, `\x` with
    /// one or two hexadecimal digits and `\` with one to three octal
    /// ones are escapes; other escapes and escapes of NUL are dropped.
    /// The bytes must be UTF-8.
    fn text(&mut self) -> Token {
        let mut bytes = Vec::new();
        while !self.at_line_end() && self.peek() != Some(b'"') {
            if !self.eat(b'\\') {
                bytes.push(self.bytes[self.pos]);
                self.pos += 1;
                continue;
            }
            if self.eat(b'\\') {
                bytes.push(b'\\');
            } else if self.eat(b'"') {
                bytes.push(b'"');
            } else if self.eat(b'x') || self.eat(b'X') {
                if let Some(value) = self.number(16, 2).filter(|&value| value != 0) {
                    bytes.push(value);
                }
            } else if let Some(value) = self.number(8, 3).filter(|&value| value != 0) {
                bytes.push(value);
            }
        }
        if !self.eat(b'"') || bytes.len() > MAX_TOKEN {
            return Token::Error;
        }
        String::from_utf8(bytes).map_or(Token::Error, Token::Text)
    }

    /// Up to `digits` digits in `radix`, as a byte (`None` without
    /// digits or above 255).
    fn number(&mut self, radix: u32, digits: usize) -> Option<u8> {
        let mut value: u32 = 0;
        let mut read = 0;
        while read < digits {
            let Some(digit) = self
                .peek()
                .and_then(|byte| char::from(byte).to_digit(radix))
            else {
                break;
            };
            value = value * radix + digit;
            self.pos += 1;
            read += 1;
        }
        if read == 0 {
            return None;
        }
        u8::try_from(value).ok()
    }

    /// The path of an include statement, after `include`: a string whose
    /// `%%`, `%H`, `%L` and `%S` are expanded. `None` for anything else,
    /// or an expansion that has no value.
    pub(super) fn include_path(&mut self, expansions: Expansions<'_>) -> Option<String> {
        self.line_ended = self.skip_blanks();
        if self.line_ended || !self.eat(b'"') {
            return None;
        }
        let mut path = Vec::new();
        while !self.at_line_end() && self.peek() != Some(b'"') {
            if !self.eat(b'%') {
                path.push(self.bytes[self.pos]);
                self.pos += 1;
                continue;
            }
            let expanded = match self.peek() {
                Some(b'%') => Some(Path::new("%")),
                Some(b'H') => expansions.home,
                Some(b'L') => expansions.locale_file,
                Some(b'S') => Some(expansions.locale_dir),
                _ => None,
            }?;
            self.pos += 1;
            path.extend_from_slice(expanded.to_string_lossy().as_bytes());
        }
        if !self.eat(b'"') || path.len() > MAX_TOKEN {
            return None;
        }
        String::from_utf8(path).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(text.as_bytes());
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            let end = token == Token::EndOfFile;
            tokens.push(token);
            if end {
                return tokens;
            }
        }
    }

    fn ident(name: &str) -> Token {
        Token::Ident(name.to_owned())
    }

    #[test]
    fn productions_are_tokenized() {
        assert_eq!(
            tokens("<dead_acute> <e> : \"é\" eacute # LATIN SMALL LETTER E WITH ACUTE\n"),
            [
                Token::Keysym("dead_acute".into()),
                Token::Keysym("e".into()),
                Token::Colon,
                Token::Text("é".into()),
                ident("eacute"),
                Token::EndOfLine,
                Token::EndOfFile,
            ]
        );
        assert_eq!(
            tokens("# only a comment\n\n!Ctrl ~Shift <a>"),
            [
                Token::EndOfLine,
                Token::EndOfLine,
                Token::Bang,
                ident("Ctrl"),
                Token::Tilde,
                ident("Shift"),
                Token::Keysym("a".into()),
                Token::EndOfFile,
            ]
        );
        assert_eq!(tokens("include")[0], Token::Include);
        assert_eq!(tokens("\u{feff}<a>")[0], Token::Keysym("a".into()), "a BOM");
    }

    #[test]
    fn strings_resolve_escapes() {
        let text = |source: &str| tokens(source).swap_remove(0);
        assert_eq!(text(r#""\"\\""#), Token::Text("\"\\".into()));
        assert_eq!(text(r#""\x41\X42c""#), Token::Text("ABc".into()));
        assert_eq!(text(r#""\101\1012""#), Token::Text("AA2".into()));
        assert_eq!(
            text(r#""\303\251""#),
            Token::Text("é".into()),
            "UTF-8 in octal"
        );
        assert_eq!(text(r#""\q\0\x""#), Token::Text("q".into()), "dropped");
        assert_eq!(text(r#""\777""#), Token::Text(String::new()), "above 255");
        assert_eq!(text("\"\\xff\""), Token::Error, "not UTF-8");
        assert_eq!(text("\"open\n\""), Token::Error, "unterminated");
        assert_eq!(text("<open\n>"), Token::Error);
        assert_eq!(text("$"), Token::Error);
        let long = format!("\"{}\"", "a".repeat(MAX_TOKEN + 1));
        assert_eq!(text(&long), Token::Error);
    }

    #[test]
    fn include_paths_expand_percent_escapes() {
        let expansions = Expansions {
            home: Some(Path::new("/home/u")),
            locale_file: Some(Path::new("/x/en_US.UTF-8/Compose")),
            locale_dir: Path::new("/x"),
        };
        let path = |source: &str| Lexer::new(source.as_bytes()).include_path(expansions);
        assert_eq!(
            path(r#" "%H/.compose" "#).as_deref(),
            Some("/home/u/.compose")
        );
        assert_eq!(path(r#""%L""#).as_deref(), Some("/x/en_US.UTF-8/Compose"));
        assert_eq!(
            path(r#""%S/a/Compose%%""#).as_deref(),
            Some("/x/a/Compose%")
        );
        assert_eq!(path(r#""%Q""#), None, "unknown expansion");
        assert_eq!(path("\n\"a\""), None, "on the next line");
        assert_eq!(path("a"), None);
        let homeless = Expansions {
            home: None,
            ..expansions
        };
        assert_eq!(
            Lexer::new(br#""%H/x""#).include_path(homeless),
            None,
            "no home"
        );
    }
}
