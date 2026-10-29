//! Tokenizer tests: matplotlib's `test_tokenize` data, binary runs, errors,
//! token kinds and positions.

use super::*;

/// matplotlib's `test_tokenize` data.
const DATA: &[u8] =
    b"1234/abc false -9.81  Foo <<[0 1 2]<0 1ef a\t>>>\n(string with(nested\t\\) par)ens\\\\)";

fn tokens(data: &[u8], skip: bool) -> Vec<(PsTokenKind, PsValue)> {
    ps_tokenize(data, skip)
        .unwrap()
        .into_iter()
        .map(|token| (token.kind, token.value))
        .collect()
}

fn int(value: i64) -> PsValue {
    PsValue::Number(PsNumber::Int(value))
}

#[test]
fn matplotlibs_tokenize_example() {
    use PsTokenKind::{Boolean, Delimiter, Keyword, Name, Number, String, Whitespace};
    let expression = |text: &str| PsValue::Expression(text.to_owned());
    let correct = vec![
        (Number, int(1234)),
        (Name, PsValue::Name("abc".into())),
        (Whitespace, expression(" ")),
        (Boolean, PsValue::Bool(false)),
        (Whitespace, expression(" ")),
        (Number, PsValue::Number(PsNumber::Real(-9.81))),
        (Whitespace, expression("  ")),
        (Keyword, expression("Foo")),
        (Whitespace, expression(" ")),
        (Delimiter, expression("<<")),
        (Delimiter, expression("[")),
        (Number, int(0)),
        (Whitespace, expression(" ")),
        (Number, int(1)),
        (Whitespace, expression(" ")),
        (Number, int(2)),
        (Delimiter, expression("]")),
        (String, PsValue::Bytes(vec![0x01, 0xEF, 0xA0])),
        (Delimiter, expression(">>")),
        (Whitespace, expression("\n")),
        (
            String,
            PsValue::Text("string with(nested\t) par)ens\\".into()),
        ),
    ];
    assert_eq!(tokens(DATA, false), correct);
    let without: Vec<_> = correct
        .into_iter()
        .filter(|(kind, _)| *kind != Whitespace)
        .collect();
    assert_eq!(tokens(DATA, true), without);
}

#[test]
fn binary_tokens_after_any_token() {
    let count = tokens(DATA, true).len();
    for n in 1..count {
        let mut tokenizer = Tokenizer::new(DATA, true);
        for _ in 0..n {
            tokenizer.next_token().unwrap();
        }
        let binary = tokenizer.binary(10).unwrap().unwrap();
        assert_eq!(binary.kind, PsTokenKind::Binary);
        assert!(binary.raw.len() <= 10);
    }
}

#[test]
fn tokenize_errors() {
    let error = |data: &[u8]| ps_tokenize(data, true).err();
    assert_eq!(
        error(b"1234 (this (string) is unterminated\\)"),
        Some(Type1Error::UnterminatedString(5))
    );
    assert_eq!(
        error(b"/Foo<01234"),
        Some(Type1Error::UnterminatedHexString(4))
    );
    assert_eq!(
        error(b"/Foo<01234abcg>/Bar"),
        Some(Type1Error::MalformedHexString(4))
    );
    assert_eq!(
        error(b"(trailing \\"),
        Some(Type1Error::UnterminatedString(0))
    );
    assert_eq!(error(b"/a ///b % comment\n{x}"), None);
    assert_eq!(error(b"1e5"), Some(Type1Error::InvalidNumber("1e5".into())));
    assert_eq!(PsTokenKind::Delimiter.name(), "delimiter");
}

#[test]
fn token_classification() {
    use PsTokenKind::{Delimiter, Keyword, Name, Number};
    let kinds = |data: &[u8]| -> Vec<PsTokenKind> {
        let mut tokenizer = Tokenizer::new(data, true);
        std::iter::from_fn(|| tokenizer.next_token().unwrap())
            .map(|t| t.kind)
            .collect()
    };
    assert_eq!(
        kinds(b"//x / ) >"),
        vec![Name, Delimiter, Delimiter, Delimiter]
    );
    assert_eq!(
        kinds(b"1e5 inf |- RD"),
        vec![Number, Number, Keyword, Keyword]
    );
    let mut tokenizer = Tokenizer::new(b"1e5 2.5", true);
    let first = tokenizer.next_token().unwrap().unwrap();
    assert!(first.number().is_err());
    let second = tokenizer.next_token().unwrap().unwrap();
    assert_eq!(second.number(), Ok(PsNumber::Real(2.5)));
    assert_eq!(second.integer(), Ok(None));
}

#[test]
fn negative_lengths_move_back() {
    let mut tokenizer = Tokenizer::new(b"ab cd ef", true);
    tokenizer.next_token().unwrap();
    tokenizer.next_token().unwrap();
    let empty = tokenizer.binary(-3).unwrap().unwrap();
    assert_eq!(empty.raw, b"");
    assert_eq!(tokenizer.next_token().unwrap().unwrap().raw, b"cd");
    assert_eq!(tokenizer.binary(-50), Err(Type1Error::NegativeLength(-50)));
    let mut looping = Tokenizer::new(b"ab cd", true);
    let looped = (0..1000).find_map(|_| {
        looping
            .next_token()
            .err()
            .or_else(|| looping.binary(-2).err())
    });
    assert_eq!(looped, Some(Type1Error::ParseLimit));
}

#[test]
fn hex_strings_are_recorded_at_their_end() {
    let mut tokenizer = Tokenizer::new(b"x <ab> y", true);
    tokenizer.next_token().unwrap();
    let hex = tokenizer.next_token().unwrap().unwrap();
    assert_eq!((hex.pos, hex.raw), (6, &b"<ab>"[..]));
    assert_eq!(hex.end(), 10);
    assert_eq!(unescape("(\\101\\x\\5)"), "A\\x\u{5}");
}
