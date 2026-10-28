//! A safe parser for the Python expressions that appear in rc files:
//! literals (numbers, strings, `None`, booleans, tuples, lists) and the
//! cycler grammar (calls, `+`, `*`, slicing), as matplotlib's
//! `ast.literal_eval` / `_parse_cycler_string` accept them.

use std::fmt;

/// A parsed Python expression.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    /// `None`.
    None,
    /// `True` / `False`.
    Bool(bool),
    /// An integer literal.
    Int(i64),
    /// A float literal.
    Number(f64),
    /// A string literal.
    Str(String),
    /// `(a, b, ...)`.
    Tuple(Vec<Expr>),
    /// `[a, b, ...]`.
    List(Vec<Expr>),
    /// `name(args..., key=value...)`.
    Call {
        /// Function name.
        name: String,
        /// Positional arguments.
        args: Vec<Expr>,
        /// Keyword arguments in order.
        kwargs: Vec<(String, Expr)>,
    },
    /// `left + right` (`true`) or `left * right` (`false`).
    BinOp {
        /// Whether the operator is `+` (otherwise `*`).
        add: bool,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// `value[start:stop:step]`.
    Slice {
        /// The sliced expression.
        value: Box<Expr>,
        /// Optional bounds and step.
        bounds: [Option<i64>; 3],
    },
}

impl Expr {
    /// Parses a complete expression; comma-separated expressions without
    /// brackets make a tuple, as in Python (`4, 2` is `(4, 2)`).
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut parser = Parser {
            tokens: tokenize(text)?,
            pos: 0,
        };
        let first = parser.sum()?;
        if !parser.eat(',') {
            return match parser.peek() {
                None => Ok(first),
                Some(token) => Err(format!("unexpected {token} after expression")),
            };
        }
        let mut items = vec![first];
        while parser.peek().is_some() {
            items.push(parser.sum()?);
            if !parser.eat(',')
                && let Some(token) = parser.peek()
            {
                return Err(format!("unexpected {token} after expression"));
            }
        }
        Ok(Self::Tuple(items))
    }

    /// The elements of a tuple or list literal.
    pub(crate) fn as_sequence(&self) -> Option<&[Self]> {
        match self {
            Self::Tuple(items) | Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// The value of a number literal, integer or float.
    pub(crate) fn as_number(&self) -> Option<f64> {
        match self {
            Self::Int(n) => Some(*n as f64),
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Int(i64),
    Number(f64),
    Str(String),
    Ident(String),
    Punct(char),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(n) => write!(f, "integer {n}"),
            Self::Number(n) => write!(f, "number {n}"),
            Self::Str(s) => write!(f, "string {s:?}"),
            Self::Ident(name) => write!(f, "name {name}"),
            Self::Punct(c) => write!(f, "'{c}'"),
        }
    }
}

/// Reads the string literal whose opening quote is at `chars[start]`,
/// with Python's escape sequences; returns its text and the index after
/// the closing quote.
fn string_literal(chars: &[char], start: usize) -> Result<(String, usize), String> {
    let quote = chars[start];
    let mut text = String::new();
    let mut i = start + 1;
    loop {
        match *chars.get(i).ok_or("unterminated string literal")? {
            c if c == quote => return Ok((text, i + 1)),
            '\\' => {
                let escaped = *chars.get(i + 1).ok_or("unterminated string literal")?;
                i += 2;
                let hex = |len: usize, i: &mut usize| -> Result<char, String> {
                    let digits: String = chars
                        .get(*i..*i + len)
                        .ok_or("truncated escape")?
                        .iter()
                        .collect();
                    *i += len;
                    u32::from_str_radix(&digits, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or_else(|| format!("invalid escape \\{escaped}{digits}"))
                };
                match escaped {
                    '\\' | '\'' | '"' => text.push(escaped),
                    'n' => text.push('\n'),
                    't' => text.push('\t'),
                    'r' => text.push('\r'),
                    'a' => text.push('\u{7}'),
                    'b' => text.push('\u{8}'),
                    'f' => text.push('\u{c}'),
                    'v' => text.push('\u{b}'),
                    'x' => text.push(hex(2, &mut i)?),
                    'u' => text.push(hex(4, &mut i)?),
                    'U' => text.push(hex(8, &mut i)?),
                    '\n' => {}
                    // Python keeps unknown escapes as they are.
                    other => {
                        text.push('\\');
                        text.push(other);
                    }
                }
            }
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
}

fn tokenize(text: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    // A line break outside brackets ends a Python expression.
    let mut depth = 0_usize;
    let mut line_ended = false;
    while let Some(&c) = chars.get(i) {
        if !c.is_whitespace() && line_ended {
            return Err("a line break ends the expression".to_owned());
        }
        match c {
            '\n' | '\r' => {
                line_ended = depth == 0 && !tokens.is_empty();
                i += 1;
            }
            c if c.is_whitespace() => i += 1,
            '(' | ')' | '[' | ']' | ',' | '=' | '+' | '*' | ':' => {
                match c {
                    '(' | '[' => depth += 1,
                    ')' | ']' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                tokens.push(Token::Punct(c));
                i += 1;
            }
            '\'' | '"' => {
                let (literal, next) = string_literal(&chars, i)?;
                tokens.push(Token::Str(literal));
                i = next;
            }
            c if c.is_ascii_digit() || c == '.' || (c == '-' && starts_number(&chars, i + 1)) => {
                let start = i;
                i += 1;
                while chars.get(i).is_some_and(|&d| {
                    d.is_ascii_alphanumeric()
                        || d == '.'
                        || d == '_'
                        || ((d == '-' || d == '+') && matches!(chars[i - 1], 'e' | 'E'))
                }) {
                    i += 1;
                }
                let literal: String = chars[start..i].iter().filter(|&&d| d != '_').collect();
                // Python's int literals are digits only; anything else
                // that parses is a float.
                let token = match literal.parse::<i64>() {
                    Ok(value) => Token::Int(value),
                    Err(_) => literal
                        .parse::<f64>()
                        .map(Token::Number)
                        .map_err(|_| format!("malformed number {literal:?}"))?,
                };
                tokens.push(token);
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while chars
                    .get(i)
                    .is_some_and(|d| d.is_alphanumeric() || *d == '_')
                {
                    i += 1;
                }
                tokens.push(Token::Ident(chars[start..i].iter().collect()));
            }
            other => return Err(format!("unexpected character {other:?}")),
        }
    }
    Ok(tokens)
}

fn starts_number(chars: &[char], i: usize) -> bool {
    chars
        .get(i)
        .is_some_and(|c| c.is_ascii_digit() || *c == '.')
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn eat(&mut self, punct: char) -> bool {
        let found = self.peek() == Some(&Token::Punct(punct));
        if found {
            self.pos += 1;
        }
        found
    }

    fn expect(&mut self, punct: char) -> Result<(), String> {
        if self.eat(punct) {
            Ok(())
        } else {
            Err(format!("expected '{punct}'"))
        }
    }

    fn sum(&mut self) -> Result<Expr, String> {
        let mut left = self.product()?;
        while self.eat('+') {
            let right = self.product()?;
            // Python concatenates strings and sequences.
            left = match (left, right) {
                (Expr::Str(a), Expr::Str(b)) => Expr::Str(a + &b),
                (Expr::List(mut a), Expr::List(b)) => {
                    a.extend(b);
                    Expr::List(a)
                }
                (Expr::Tuple(mut a), Expr::Tuple(b)) => {
                    a.extend(b);
                    Expr::Tuple(a)
                }
                (left, right) => Expr::BinOp {
                    add: true,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn product(&mut self) -> Result<Expr, String> {
        let mut left = self.postfix()?;
        while self.eat('*') {
            let right = self.postfix()?;
            left = Expr::BinOp {
                add: false,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut value = self.primary()?;
        while self.eat('[') {
            let mut bounds = [None; 3];
            let mut colons = 0;
            for (slot, bound) in bounds.iter_mut().enumerate() {
                if let Some(Token::Int(n)) = self.peek() {
                    *bound = Some(*n);
                    self.pos += 1;
                }
                if slot == 2 || !self.eat(':') {
                    break;
                }
                colons += 1;
            }
            if colons == 0 {
                return Err("only slicing is supported, not indexing".to_owned());
            }
            self.expect(']')?;
            value = Expr::Slice {
                value: Box::new(value),
                bounds,
            };
        }
        Ok(value)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.next().ok_or("unexpected end of expression")? {
            Token::Int(n) => Ok(Expr::Int(n)),
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::Str(s) => Ok(Expr::Str(s)),
            Token::Ident(name) => match name.as_str() {
                "None" => Ok(Expr::None),
                "True" => Ok(Expr::Bool(true)),
                "False" => Ok(Expr::Bool(false)),
                _ if self.eat('(') => self.call(name),
                _ => Err(format!("unknown name {name:?}")),
            },
            Token::Punct('(') => {
                let (items, trailing_comma) = self.sequence(')')?;
                Ok(if items.len() == 1 && !trailing_comma {
                    items
                        .into_iter()
                        .next()
                        .expect("the sequence holds one item")
                } else {
                    Expr::Tuple(items)
                })
            }
            Token::Punct('[') => Ok(Expr::List(self.sequence(']')?.0)),
            token @ Token::Punct(_) => Err(format!("unexpected {token}")),
        }
    }

    /// Comma-separated expressions up to `close`; reports a trailing comma.
    fn sequence(&mut self, close: char) -> Result<(Vec<Expr>, bool), String> {
        let mut items = Vec::new();
        let mut trailing_comma = false;
        while !self.eat(close) {
            items.push(self.sum()?);
            trailing_comma = self.eat(',');
            if !trailing_comma {
                self.expect(close)?;
                break;
            }
        }
        Ok((items, trailing_comma))
    }

    fn call(&mut self, name: String) -> Result<Expr, String> {
        let mut args = Vec::new();
        let mut kwargs = Vec::new();
        while !self.eat(')') {
            let keyword = match (self.peek(), self.tokens.get(self.pos + 1)) {
                (Some(Token::Ident(key)), Some(Token::Punct('='))) => Some(key.clone()),
                _ => None,
            };
            if let Some(key) = keyword {
                self.pos += 2;
                kwargs.push((key, self.sum()?));
            } else {
                args.push(self.sum()?);
            }
            if !self.eat(',') {
                self.expect(')')?;
                break;
            }
        }
        Ok(Expr::Call { name, args, kwargs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals() {
        assert_eq!(Expr::parse("None"), Ok(Expr::None));
        assert_eq!(Expr::parse("-1.5e-3"), Ok(Expr::Number(-1.5e-3)));
        assert_eq!(Expr::parse("'a b'"), Ok(Expr::Str("a b".into())));
        assert_eq!(
            Expr::parse("(0, (3.5, 1))"),
            Ok(Expr::Tuple(vec![
                Expr::Int(0),
                Expr::Tuple(vec![Expr::Number(3.5), Expr::Int(1)])
            ]))
        );
        assert_eq!(Expr::parse("(5,)"), Ok(Expr::Tuple(vec![Expr::Int(5)])));
        assert_eq!(Expr::parse("(5)"), Ok(Expr::Int(5)));
        assert_eq!(Expr::parse("5.0"), Ok(Expr::Number(5.0)));
        assert_eq!(Expr::parse("1_000"), Ok(Expr::Int(1000)));
        assert_eq!(
            Expr::parse("4, 2"),
            Ok(Expr::Tuple(vec![Expr::Int(4), Expr::Int(2)]))
        );
        assert_eq!(Expr::parse("4,"), Ok(Expr::Tuple(vec![Expr::Int(4)])));
        assert!(Expr::parse("4, 2 3").is_err());
        assert_eq!(Expr::parse("[]"), Ok(Expr::List(Vec::new())));
    }

    #[test]
    fn strings_follow_python() {
        assert_eq!(Expr::parse(r"'a\\b'"), Ok(Expr::Str(r"a\b".into())));
        assert_eq!(Expr::parse(r"'\x41é\n'"), Ok(Expr::Str("Aé\n".into())));
        assert_eq!(
            Expr::parse(r"'\oO'"),
            Ok(Expr::Str(r"\oO".into())),
            "unknown escapes stay"
        );
        assert_eq!(Expr::parse(r"'it\'s'"), Ok(Expr::Str("it's".into())));
        assert_eq!(Expr::parse("'r' + 'gb'"), Ok(Expr::Str("rgb".into())));
        assert_eq!(
            Expr::parse("[1] + [2]"),
            Ok(Expr::List(vec![Expr::Int(1), Expr::Int(2)]))
        );
    }

    #[test]
    fn line_breaks_end_expressions_outside_brackets() {
        assert!(Expr::parse("1 +\n 2").is_err());
        assert_eq!(Expr::parse("(1 +\n 2)").map(|e| e.as_number()), Ok(None));
        assert_eq!(
            Expr::parse("[1,\n 2]").map(|e| e.as_sequence().map(<[Expr]>::len)),
            Ok(Some(2))
        );
        assert_eq!(Expr::parse("3\n"), Ok(Expr::Int(3)));
    }

    #[test]
    fn cycler_expressions() {
        let expr =
            Expr::parse("cycler('color', ['r', 'g']) + cycler(lw=[1, 2])[::2]").expect("parses");
        let Expr::BinOp {
            add: true,
            left,
            right,
        } = expr
        else {
            panic!("expected addition");
        };
        assert!(
            matches!(*left, Expr::Call { ref name, ref args, .. } if name == "cycler" && args.len() == 2)
        );
        assert!(matches!(
            *right,
            Expr::Slice {
                bounds: [None, None, Some(2)],
                ..
            }
        ));
    }

    #[test]
    fn precedence_and_errors() {
        let expr = Expr::parse("a(x=1) + b(y=2) * c(z=3)").expect("parses");
        assert!(
            matches!(expr, Expr::BinOp { add: true, ref right, .. } if matches!(**right, Expr::BinOp { add: false, .. }))
        );
        assert!(Expr::parse("(1, 2").is_err());
        assert!(Expr::parse("'open").is_err());
        assert!(Expr::parse("1 2").is_err());
        assert!(Expr::parse("os.system").is_err());
        assert!(Expr::parse("c()[2]").is_err(), "indexing is not slicing");
    }
}
