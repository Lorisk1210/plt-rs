//! `_native.type1font`: matplotlib's `_type1font`, PostScript Type 1 fonts
//! as its PDF backend reads, rewrites and subsets them, by pltrs-font's
//! [`Type1Font`], and the tokenizer its parser reads them with
//! ([`PsTokenStream`]).

use pltrs_font::{
    PsNumber, PsTokenStream, PsValue, Type1Cipher, Type1Effects, Type1Encoding, Type1Error,
    Type1Font, Type1Parts,
};

use crate::binding::new_dict;
use crate::binding::prelude::*;

impl From<Type1Error> for PyErr {
    /// matplotlib's exceptions: `ValueError` from the tokenizer,
    /// `RuntimeError` from the parser, with its messages where tests
    /// match them.
    fn from(err: Type1Error) -> Self {
        match err {
            Type1Error::UnterminatedString(at) => {
                PyErr::value_error(format!("Unterminated string starting at {at}"))
            }
            Type1Error::UnterminatedHexString(at) => {
                PyErr::value_error(format!("Unterminated hex string starting at {at}"))
            }
            Type1Error::MalformedHexString(at) => {
                PyErr::value_error(format!("Malformed hex string starting at {at}"))
            }
            Type1Error::Incomplete(section) => {
                PyErr::runtime_error(format!("Malformed Type1 font file: Incomplete {section}"))
            }
            Type1Error::SubrsIndices => PyErr::runtime_error(
                "Malformed Type1 font file: /Subrs indices do not cover the declared count",
            ),
            other => PyErr::runtime_error(other.to_string()),
        }
    }
}

impl IntoPy for Type1Font {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

impl IntoPy for PsTokenStream {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

/// A number as Python holds it: `int` or `float`.
fn number(py: Python<'_>, value: PsNumber) -> PyResult<Bound<'_>> {
    match value {
        PsNumber::Int(value) => value.into_py(py),
        PsNumber::Real(value) => value.into_py(py),
    }
}

/// A token or dictionary value as matplotlib holds it: numbers, booleans,
/// `str` for names, `(...)` strings and expressions, `bytes` for `<...>`
/// strings and binary runs.
fn value<'py>(py: Python<'py>, value: &PsValue) -> PyResult<Bound<'py>> {
    match value {
        PsValue::Number(value) => number(py, *value),
        PsValue::Bool(value) => value.into_py(py),
        PsValue::Name(text) | PsValue::Text(text) | PsValue::Expression(text) => text.into_py(py),
        PsValue::Bytes(bytes) => Bytes(bytes.clone()).into_py(py),
    }
}

/// The cipher matplotlib names `key` (`'eexec'` or `'charstring'`).
fn cipher(key: &str) -> PyResult<Type1Cipher> {
    match key {
        "eexec" => Ok(Type1Cipher::Eexec),
        "charstring" => Ok(Type1Cipher::Charstring),
        _ => Err(PyErr::value_error(format!(
            "{key:?} is not a valid value for key; supported values are 'eexec', 'charstring'"
        ))),
    }
}

/// `Type1Font(filename)`: the font in the contents of a PFA or PFB file.
fn read(data: Bytes) -> PyResult<Type1Font> {
    Ok(Type1Font::from_bytes(&data.0)?)
}

/// `Type1Font((cleartext, encrypted, finale))`: a font from its parts.
fn from_parts(cleartext: Bytes, encrypted: Bytes, finale: Bytes) -> PyResult<Type1Font> {
    Ok(Type1Font::from_parts(Type1Parts {
        cleartext: cleartext.0,
        encrypted: encrypted.0,
        finale: finale.0,
    })?)
}

/// `Type1Font.parts`: the cleartext part, the encrypted part and the
/// finale.
fn parts(font: Native<Type1Font>) -> PyResult<(Bytes, Bytes, Bytes)> {
    let parts = font.borrow().parts().clone();
    Ok((
        Bytes(parts.cleartext),
        Bytes(parts.encrypted),
        Bytes(parts.finale),
    ))
}

/// `Type1Font.decrypted`: the decrypted encrypted part.
fn decrypted(font: Native<Type1Font>) -> PyResult<Bytes> {
    Ok(Bytes(font.borrow().decrypted().to_vec()))
}

/// `Type1Font.prop`: the dictionary entries, with `FontBBox` as a list of
/// numbers, `Encoding` a dict of codes to glyph names, `CharStrings` a
/// dict of glyph names to decrypted programs, `Subrs` a list of decrypted
/// programs and `OtherSubrs` the procedures' source.
fn prop(font: Native<Type1Font>) -> PyResult<Owned> {
    let font = font.borrow();
    let prop = font.prop();
    Python::with_gil(|py| {
        let dict = new_dict(py)?;
        for (key, entry) in prop.entries() {
            dict.set_item(key, value(py, entry)?)?;
        }
        let bbox = prop
            .font_bbox()
            .into_iter()
            .map(|n| number(py, n))
            .collect::<PyResult<Vec<_>>>()?;
        dict.set_item("FontBBox", bbox)?;
        if let Some(encoding) = prop.encoding() {
            let codes = new_dict(py)?;
            for (code, glyph) in encoding.iter() {
                codes.set_item(code, glyph)?;
            }
            dict.set_item("Encoding", codes)?;
        }
        let char_strings = new_dict(py)?;
        for name in prop.char_strings().names() {
            if let Some(program) = prop.char_strings().get(name) {
                char_strings.set_item(name, Bytes(program.to_vec()))?;
            }
        }
        dict.set_item("CharStrings", char_strings)?;
        if let Some(subrs) = prop.subrs() {
            let programs: Vec<Bytes> = subrs
                .iter()
                .map(|p| Bytes(p.decrypted().to_vec()))
                .collect();
            dict.set_item("Subrs", programs)?;
        }
        if let Some(other) = prop.other_subrs() {
            dict.set_item("OtherSubrs", Bytes(other.to_vec()))?;
        }
        Ok(dict.unbind())
    })
}

/// The definitions of each key: where they start and end.
type Spans = Vec<(String, Vec<(usize, usize)>)>;

/// `Type1Font._pos`: where each definition starts and ends, as offsets
/// into the cleartext part followed by the decrypted part.
fn positions(font: Native<Type1Font>) -> PyResult<Spans> {
    let font = font.borrow();
    Ok(font
        .span_keys()
        .map(|key| (key.to_owned(), font.spans(key).to_vec()))
        .collect())
}

/// `Type1Font._abbr`: the font's names for the `RD`, `ND` and `NP`
/// procedures.
fn abbreviations(font: Native<Type1Font>) -> PyResult<Vec<(String, String)>> {
    let names = font.borrow().abbreviations().clone();
    Ok(vec![
        ("RD".to_owned(), names.rd),
        ("ND".to_owned(), names.nd),
        ("NP".to_owned(), names.np),
    ])
}

/// `Type1Font.transform(effects)`: the font slanted and extended.
fn transform(
    font: Native<Type1Font>,
    slant: Option<f64>,
    extend: Option<f64>,
) -> PyResult<Type1Font> {
    Ok(font.borrow().transform(Type1Effects { slant, extend })?)
}

/// `Type1Font.with_encoding(encoding)`: the font re-encoded with the
/// `(code, glyph name)` pairs of `encoding`.
fn with_encoding(font: Native<Type1Font>, encoding: Vec<(i64, String)>) -> PyResult<Type1Font> {
    Ok(font
        .borrow()
        .with_encoding(&Type1Encoding::from_codes(encoding))?)
}

/// `Type1Font.subset(characters, name_prefix)`: the font reduced to the
/// glyphs the character codes need.
fn subset(
    font: Native<Type1Font>,
    characters: Vec<u32>,
    name_prefix: String,
) -> PyResult<Type1Font> {
    Ok(font.borrow().subset(characters, &name_prefix)?)
}

/// `Type1Font._encrypt(plaintext, key, ndiscard)`.
fn encrypt(plaintext: Bytes, key: String, discard: i64) -> PyResult<Bytes> {
    Ok(Bytes(cipher(&key)?.encrypt(&plaintext.0, discard)))
}

/// `Type1Font._decrypt(ciphertext, key, ndiscard)`.
fn decrypt(ciphertext: Bytes, key: String, discard: i64) -> PyResult<Bytes> {
    Ok(Bytes(cipher(&key)?.decrypt(&ciphertext.0, discard)))
}

/// The tokens of `data` for `_tokenize`, read one at a time.
fn tokenizer(data: Bytes, skip_whitespace: bool) -> PyResult<PsTokenStream> {
    Ok(PsTokenStream::new(data.0, skip_whitespace))
}

/// The next token of `stream` as `(kind, pos, raw, value)`, the next
/// `binary` bytes when given (what `_tokenize` was sent); `None` at the
/// end.
fn next_token(stream: Native<PsTokenStream>, binary: Option<i64>) -> PyResult<Owned> {
    let token = stream.borrow_mut().next_token(binary)?;
    Python::with_gil(|py| match token {
        None => Ok(Bound::none(py).unbind()),
        Some(token) => Ok((
            token.kind.name(),
            token.pos,
            Bytes(token.raw),
            value(py, &token.value)?,
        )
            .into_py(py)?
            .unbind()),
    })
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("read", read)?;
    module.add("from_parts", from_parts)?;
    module.add("parts", parts)?;
    module.add("decrypted", decrypted)?;
    module.add("prop", prop)?;
    module.add("positions", positions)?;
    module.add("abbreviations", abbreviations)?;
    module.add("transform", transform)?;
    module.add("with_encoding", with_encoding)?;
    module.add("subset", subset)?;
    module.add("encrypt", encrypt)?;
    module.add("decrypt", decrypt)?;
    module.add("tokenizer", tokenizer)?;
    module.add("next_token", next_token)
}

#[cfg(test)]
mod tests {
    use crate::binding::testing::{eval, extension, with_python};

    #[test]
    fn a_font_is_read_and_tokens_are_streamed() {
        with_python(|py| {
            let native = extension(py);
            let module = native.getattr("type1font").unwrap();
            let encrypted = module
                .getattr("encrypt")
                .unwrap()
                .call(
                    &[
                        eval(py, "", "b'secret'"),
                        eval(py, "", "'eexec'"),
                        eval(py, "", "4"),
                    ],
                    &[],
                )
                .unwrap();
            let decrypted = module
                .getattr("decrypt")
                .unwrap()
                .call(
                    &[encrypted, eval(py, "", "'eexec'"), eval(py, "", "4")],
                    &[],
                )
                .unwrap();
            assert_eq!(decrypted.repr().unwrap(), "b'secret'");
            let stream = module
                .getattr("tokenizer")
                .unwrap()
                .call(&[eval(py, "", "b'/a 1 RD xy'"), eval(py, "", "True")], &[])
                .unwrap();
            let next = module.getattr("next_token").unwrap();
            let none = || crate::binding::Bound::none(py);
            let first = next.call(&[stream.clone(), none()], &[]).unwrap();
            assert_eq!(first.repr().unwrap(), "('name', 0, b'/a', 'a')");
            next.call(&[stream.clone(), none()], &[]).unwrap();
            next.call(&[stream.clone(), none()], &[]).unwrap();
            let binary = next
                .call(&[stream.clone(), eval(py, "", "3")], &[])
                .unwrap();
            assert_eq!(binary.repr().unwrap(), "('binary', 7, b' xy', b'xy')");
            assert!(next.call(&[stream, none()], &[]).unwrap().is_none());
            let error = module
                .getattr("tokenizer")
                .unwrap()
                .call(&[eval(py, "", "b'(open'"), eval(py, "", "True")], &[])
                .and_then(|stream| next.call(&[stream, none()], &[]))
                .unwrap_err();
            assert_eq!(error.class_name(py), "ValueError");
        });
    }
}
