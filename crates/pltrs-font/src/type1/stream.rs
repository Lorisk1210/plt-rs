//! [`PsTokenStream`]: matplotlib's `_tokenize` generator, the tokens of
//! PostScript data read one at a time.

use super::error::Type1Error;
use super::token::{Cursor, PsToken, Token, Tokenizer};

/// The tokens of PostScript data read one at a time, as matplotlib's
/// `_type1font._tokenize` generator yields them: the consumer may ask for
/// the next token to be a run of binary bytes (`generator.send(n)`), as
/// the parser does for charstrings.
#[derive(Clone, Debug)]
pub struct PsTokenStream {
    data: Vec<u8>,
    skip_whitespace: bool,
    cursor: Cursor,
}

impl PsTokenStream {
    /// The tokens of `data`, white space and comments included unless
    /// `skip_whitespace` is set.
    pub fn new(data: Vec<u8>, skip_whitespace: bool) -> Self {
        let cursor = Cursor::start(&data);
        Self {
            data,
            skip_whitespace,
            cursor,
        }
    }

    /// The next token, `None` at the end of the data: the next `binary`
    /// bytes (fewer at the end) as a binary token when `binary` is given,
    /// else the next PostScript token. A negative length gives an empty
    /// token and moves back, as matplotlib's position arithmetic does.
    pub fn next_token(&mut self, binary: Option<i64>) -> Result<Option<PsToken>, Type1Error> {
        let mut tokenizer = Tokenizer::resume(&self.data, self.skip_whitespace, self.cursor);
        let token = match binary {
            Some(length) => tokenizer.binary(length)?,
            None => tokenizer.next_token()?,
        };
        self.cursor = tokenizer.cursor();
        token.map(Token::to_ps_token).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type1::{PsNumber, PsTokenKind, PsValue, ps_tokenize};

    /// matplotlib's `test_tokenize` data.
    const DATA: &[u8] =
        b"1234/abc false -9.81  Foo <<[0 1 2]<0 1ef a\t>>>\n(string with(nested\t\\) par)ens\\\\)";

    fn drain(stream: &mut PsTokenStream) -> Vec<PsToken> {
        std::iter::from_fn(|| stream.next_token(None).unwrap()).collect()
    }

    #[test]
    fn a_stream_yields_what_tokenize_returns() {
        for skip in [false, true] {
            let mut stream = PsTokenStream::new(DATA.to_vec(), skip);
            assert_eq!(drain(&mut stream), ps_tokenize(DATA, skip).unwrap());
            assert_eq!(stream.next_token(None), Ok(None));
        }
    }

    #[test]
    fn positions_and_source_are_matplotlibs() {
        let tokens = ps_tokenize(b"/a <ab> 12", true).unwrap();
        let summary: Vec<_> = tokens
            .iter()
            .map(|token| (token.pos, token.raw.as_slice()))
            .collect();
        assert_eq!(summary, [(0, &b"/a"[..]), (7, b"<ab>"), (8, b"12")]);
        assert_eq!(tokens[2].value, PsValue::Number(PsNumber::Int(12)));
    }

    #[test]
    fn binary_runs_are_read_on_request() {
        let mut stream = PsTokenStream::new(b"3 RD abc ND".to_vec(), true);
        stream.next_token(None).unwrap();
        stream.next_token(None).unwrap();
        let binary = stream.next_token(Some(4)).unwrap().unwrap();
        assert_eq!(binary.kind, PsTokenKind::Binary);
        assert_eq!((binary.pos, binary.raw.as_slice()), (4, &b" abc"[..]));
        assert_eq!(binary.value, PsValue::Bytes(b"abc".to_vec()));
        let rest = drain(&mut stream);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].raw, b"ND");
        assert_eq!(stream.next_token(Some(3)), Ok(None));
    }

    #[test]
    fn errors_surface_where_matplotlib_raises() {
        let mut stream = PsTokenStream::new(b"/Foo<01234".to_vec(), true);
        assert!(stream.next_token(None).unwrap().is_some());
        assert_eq!(
            stream.next_token(None),
            Err(Type1Error::UnterminatedHexString(4))
        );
        let mut back = PsTokenStream::new(b"ab".to_vec(), true);
        assert_eq!(
            back.next_token(Some(-5)),
            Err(Type1Error::NegativeLength(-5))
        );
    }
}
