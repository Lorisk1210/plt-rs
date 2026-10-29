//! Tokens of the XKB text format, splitting them into statements, and
//! reading the assignments and values statements hold.
//!
//! XKB files are C-like: statements end with `;`, blocks are braced, and
//! comments run from `//` or `#` to the end of the line or between
//! `/*` and `*/`.

use crate::error::Error;

/// The most tokens read from one keymap (compositor keymaps have about
/// 20 000); more means the input is not a keymap.
const MAX_TOKENS: usize = 1 << 20;

/// A token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token<'a> {
    /// A run of letters, digits and underscores: keywords, names,
    /// keysyms and numbers.
    Word(&'a str),
    /// A key name, without its angle brackets (`<AC01>` is `AC01`).
    KeyName(&'a str),
    /// A quoted string, escapes resolved.
    Str(String),
    /// Any other character.
    Punct(char),
}

impl<'a> Token<'a> {
    /// Whether this is the punctuation `c`.
    pub(crate) fn is(&self, c: char) -> bool {
        *self == Token::Punct(c)
    }

    /// The text of a word.
    pub(crate) fn word(&self) -> Option<&'a str> {
        match self {
            Token::Word(word) => Some(*word),
            _ => None,
        }
    }
}

/// Splits `text` into tokens.
pub(crate) fn tokenize(text: &str) -> Result<Vec<Token<'_>>, Error> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if tokens.len() > MAX_TOKENS {
            return Err(Error::Malformed("keymap too long"));
        }
        if c.is_whitespace() || c == '\0' {
            rest = &rest[c.len_utf8()..];
        } else if rest.starts_with("//") || c == '#' {
            rest = rest.find('\n').map_or("", |end| &rest[end..]);
        } else if rest.starts_with("/*") {
            let end = rest[2..]
                .find("*/")
                .ok_or(Error::Malformed("unterminated keymap comment"))?;
            rest = &rest[end + 4..];
        } else if c == '<' {
            let end = rest
                .find('>')
                .ok_or(Error::Malformed("unterminated key name"))?;
            tokens.push(Token::KeyName(&rest[1..end]));
            rest = &rest[end + 1..];
        } else if c == '"' {
            let (string, len) = string(rest)?;
            tokens.push(Token::Str(string));
            rest = &rest[len..];
        } else if c.is_ascii_alphanumeric() || c == '_' {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            tokens.push(Token::Word(&rest[..end]));
            rest = &rest[end..];
        } else {
            tokens.push(Token::Punct(c));
            rest = &rest[c.len_utf8()..];
        }
    }
    Ok(tokens)
}

/// The string at the start of `text` (which starts with `"`), and the
/// bytes it takes.
fn string(text: &str) -> Result<(String, usize), Error> {
    let mut value = String::new();
    let mut chars = text.char_indices().skip(1);
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => return Ok((value, at + 1)),
            '\\' => match chars.next() {
                Some((_, 'n')) => value.push('\n'),
                Some((_, 't')) => value.push('\t'),
                Some((_, escaped)) => value.push(escaped),
                None => break,
            },
            c => value.push(c),
        }
    }
    Err(Error::Malformed("unterminated keymap string"))
}

/// Splits `tokens` into statements at the semicolons outside brackets,
/// dropping empty ones.
pub(crate) fn statements<'t, 'a>(tokens: &'t [Token<'a>]) -> Vec<&'t [Token<'a>]> {
    split(tokens, ';')
}

/// Splits `tokens` at `separator` outside brackets, dropping empty parts.
pub(crate) fn split<'t, 'a>(tokens: &'t [Token<'a>], separator: char) -> Vec<&'t [Token<'a>]> {
    let mut parts = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    for (at, token) in tokens.iter().enumerate() {
        match token {
            Token::Punct('{' | '[' | '(') => depth += 1,
            Token::Punct('}' | ']' | ')') => depth = depth.saturating_sub(1),
            Token::Punct(c) if *c == separator && depth == 0 => {
                if at > start {
                    parts.push(&tokens[start..at]);
                }
                start = at + 1;
            }
            _ => {}
        }
    }
    if tokens.len() > start {
        parts.push(&tokens[start..]);
    }
    parts
}

/// The tokens inside the first bracketed group opened by `open` in
/// `tokens`, and those after it; `None` without a complete group.
pub(crate) fn group<'t, 'a>(
    tokens: &'t [Token<'a>],
    open: char,
) -> Option<(&'t [Token<'a>], &'t [Token<'a>])> {
    let start = tokens.iter().position(|token| token.is(open))?;
    let mut depth = 0_usize;
    for (at, token) in tokens.iter().enumerate().skip(start) {
        match token {
            Token::Punct('{' | '[' | '(') => depth += 1,
            Token::Punct('}' | ']' | ')') => {
                depth -= 1;
                if depth == 0 {
                    return Some((&tokens[start + 1..at], &tokens[at + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// A modifier expression (`Shift+LevelThree`, `none`, `all`): its names.
pub(crate) type ModNames = Vec<String>;

/// The names of a modifier expression.
pub(crate) fn mod_names(tokens: &[Token<'_>]) -> ModNames {
    tokens
        .iter()
        .filter_map(Token::word)
        .map(str::to_owned)
        .collect()
}

/// `True`/`Yes`/`On` or `False`/`No`/`Off`.
pub(crate) fn boolean(tokens: &[Token<'_>]) -> Option<bool> {
    match tokens.first()?.word()?.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// A `field[index]= value` assignment: the field, the index tokens and
/// the value tokens.
pub(crate) type Assignment<'t, 'a> = (&'a str, Option<&'t [Token<'a>]>, &'t [Token<'a>]);

/// The assignment `tokens` make, if they make one.
pub(crate) fn assignment<'t, 'a>(tokens: &'t [Token<'a>]) -> Option<Assignment<'t, 'a>> {
    let field = tokens.first()?.word()?;
    let (index, rest) = match tokens.get(1) {
        Some(token) if token.is('[') => {
            let (index, rest) = group(tokens, '[')?;
            (Some(index), rest)
        }
        _ => (None, &tokens[1..]),
    };
    let (eq, value) = rest.split_first()?;
    eq.is('=').then_some((field, index, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_of_each_kind() {
        let tokens =
            tokenize("key <AE01> { [ 1, exclam ] }; // one\n# two\n/* three */ \"a\\\"b\"")
                .unwrap();
        assert_eq!(
            tokens,
            [
                Token::Word("key"),
                Token::KeyName("AE01"),
                Token::Punct('{'),
                Token::Punct('['),
                Token::Word("1"),
                Token::Punct(','),
                Token::Word("exclam"),
                Token::Punct(']'),
                Token::Punct('}'),
                Token::Punct(';'),
                Token::Str(String::from("a\"b")),
            ]
        );
    }

    #[test]
    fn unterminated_input_is_an_error() {
        assert!(tokenize("<AE01").is_err());
        assert!(tokenize("\"open").is_err());
        assert!(tokenize("\"esc\\").is_err());
        assert!(tokenize("/* open").is_err());
        assert_eq!(tokenize("// only a comment").unwrap(), []);
    }

    #[test]
    fn statements_and_groups_respect_brackets() {
        let tokens = tokenize("a { b; c }; ; d [ e; f ];").unwrap();
        let parts = statements(&tokens);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 6);
        let (inside, after) = group(parts[0], '{').unwrap();
        assert_eq!(statements(inside).len(), 2);
        assert_eq!(after, []);
        assert!(group(&tokens[..3], '{').is_none(), "unclosed");
        assert!(group(&tokens, '(').is_none());
        let unbalanced = tokenize("} } a; b").unwrap();
        assert_eq!(statements(&unbalanced).len(), 2);
    }
}
