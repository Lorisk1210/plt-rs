//! PDF objects and matplotlib's `pdfRepr` serialization, including its
//! line filling of arrays, dictionaries and operator sequences.

use crate::vector::{VectorError, trimmed};

/// A PDF object, or an operator or verbatim content in a content stream
/// (what matplotlib's `pdfRepr` writes: numbers, names, strings, arrays,
/// dictionaries, references, `null`, `Op`s and `Verbatim` content).
///
/// ```
/// use pltrs_render::PdfObject;
/// let page = PdfObject::Dict(vec![
///     ("Type".to_owned(), PdfObject::name("Page")),
///     ("Parent".to_owned(), PdfObject::Ref(2)),
///     ("Rotate".to_owned(), PdfObject::Null),
/// ]);
/// assert_eq!(page.repr()?, b"<< /Type /Page /Parent 2 0 R /Rotate null >>");
/// # Ok::<(), pltrs_render::VectorError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    /// `null`.
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A real number (written with up to ten decimals).
    Real(f64),
    /// A name (`/Name`).
    Name(String),
    /// A literal string.
    Str(Vec<u8>),
    /// An array.
    Array(Vec<Obj>),
    /// A dictionary with keys in insertion order.
    Dict(Vec<(String, Obj)>),
    /// An indirect reference (`12 0 R`).
    Ref(u32),
    /// Objects separated by spaces without brackets (how a `Bbox` is
    /// written: `x y width height`).
    Seq(Vec<Obj>),
    /// Bytes written as they are: operators and path data.
    Raw(Vec<u8>),
}

impl Obj {
    /// A name object.
    pub fn name(name: impl Into<String>) -> Self {
        Self::Name(name.into())
    }

    /// An operator.
    pub fn op(op: &str) -> Self {
        Self::Raw(op.as_bytes().to_vec())
    }

    /// A dictionary from `(key, value)` pairs.
    pub(super) fn dict<const N: usize>(entries: [(&str, Obj); N]) -> Self {
        Self::Dict(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }

    /// An array of reals.
    pub(super) fn reals(values: impl IntoIterator<Item = f64>) -> Self {
        Self::Array(values.into_iter().map(Self::Real).collect())
    }

    /// The object in PDF syntax (matplotlib's `pdfRepr`): reals with up to
    /// ten decimals, containers filled into lines under 75 columns.
    ///
    /// # Errors
    ///
    /// When a number is not finite, which PDF cannot write.
    pub fn repr(&self) -> Result<Vec<u8>, VectorError> {
        Ok(match self {
            Self::Null => b"null".to_vec(),
            Self::Bool(b) => {
                if *b {
                    b"true".to_vec()
                } else {
                    b"false".to_vec()
                }
            }
            Self::Int(i) => i.to_string().into_bytes(),
            Self::Real(r) => {
                if !r.is_finite() {
                    return Err(VectorError::NonFinite("number in a PDF"));
                }
                trimmed(*r, 10).into_bytes()
            }
            Self::Name(name) => name_repr(name),
            Self::Str(bytes) => string_repr(bytes),
            Self::Array(items) => {
                let mut parts = vec![b"[".to_vec()];
                for item in items {
                    parts.push(item.repr()?);
                }
                parts.push(b"]".to_vec());
                fill(&parts)
            }
            Self::Dict(entries) => {
                let mut parts = vec![b"<<".to_vec()];
                for (key, value) in entries {
                    let mut part = name_repr(key);
                    part.push(b' ');
                    part.extend(value.repr()?);
                    parts.push(part);
                }
                parts.push(b">>".to_vec());
                fill(&parts)
            }
            Self::Ref(id) => format!("{id} 0 R").into_bytes(),
            Self::Seq(items) => fill(&items.iter().map(Obj::repr).collect::<Result<Vec<_>, _>>()?),
            Self::Raw(bytes) => bytes.clone(),
        })
    }
}

/// A name: bytes outside `!`–`~` as `#xx` (matplotlib's `Name`).
fn name_repr(name: &str) -> Vec<u8> {
    let mut out = vec![b'/'];
    for c in name.chars() {
        match u32::from(c) {
            0x21..=0x7e => out.push(c as u8),
            code => out.extend(format!("#{:02x}", code & 0xff).into_bytes()),
        }
    }
    out
}

/// A literal string with backslashes, parentheses and line breaks
/// escaped.
fn string_repr(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 2);
    out.push(b'(');
    for &b in bytes {
        match b {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'(' => out.extend_from_slice(b"\\("),
            b')' => out.extend_from_slice(b"\\)"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            _ => out.push(b),
        }
    }
    out.push(b')');
    out
}

/// A string as `pdfRepr` writes Python text: ASCII as is, anything else
/// as UTF-16BE with a byte-order mark.
///
/// ```
/// use pltrs_render::{PdfObject, pdf_text_string};
/// assert_eq!(pdf_text_string("ab"), PdfObject::Str(b"ab".to_vec()));
/// assert_eq!(pdf_text_string("é"), PdfObject::Str(vec![0xfe, 0xff, 0, 0xe9]));
/// ```
pub fn text_string(text: &str) -> Obj {
    if text.is_ascii() {
        Obj::Str(text.as_bytes().to_vec())
    } else {
        let mut bytes = vec![0xfe, 0xff];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        Obj::Str(bytes)
    }
}

/// One line of `objects` as matplotlib's `PdfFile.output` writes it: their
/// syntax joined by `_fill`, then a newline.
///
/// ```
/// use pltrs_render::{PdfObject, pdf_line};
/// assert_eq!(pdf_line(&[PdfObject::Int(1), PdfObject::name("j")]).unwrap(), b"1 /j\n");
/// ```
///
/// # Errors
///
/// When an object holds a non-finite number.
pub fn line(objects: &[Obj]) -> Result<Vec<u8>, VectorError> {
    let parts = objects
        .iter()
        .map(Obj::repr)
        .collect::<Result<Vec<_>, _>>()?;
    let mut line = fill(&parts);
    line.push(b'\n');
    Ok(line)
}

/// `_fill`: joins `parts` with spaces, breaking lines before a part that
/// would reach 75 columns.
pub(super) fn fill(parts: &[Vec<u8>]) -> Vec<u8> {
    const LINE: usize = 75;
    let mut lines: Vec<Vec<u8>> = Vec::new();
    let (mut position, mut start) = (0, 0);
    for (i, part) in parts.iter().enumerate() {
        if position + part.len() < LINE {
            position += part.len() + 1;
        } else {
            lines.push(parts[start..i].join(&b' '));
            start = i;
            position = part.len();
        }
    }
    lines.push(parts[start..].join(&b' '));
    lines.join(&b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(obj: &Obj) -> String {
        String::from_utf8(obj.repr().unwrap()).unwrap()
    }

    #[test]
    fn scalars() {
        assert_eq!(text(&Obj::Real(31.0 / 255.0)), "0.1215686275");
        assert_eq!(text(&Obj::Real(-0.0)), "-0");
        assert_eq!(text(&Obj::Real(460.8)), "460.8");
        assert_eq!(text(&Obj::Int(42)), "42");
        assert_eq!(text(&Obj::name("A b#")), "/A#20b#");
        assert_eq!(text(&Obj::Str(b"a(b)\\\n".to_vec())), "(a\\(b\\)\\\\\\n)");
        assert_eq!(text(&Obj::Ref(3)), "3 0 R");
        assert!(Obj::Real(f64::NAN).repr().is_err());
        assert_eq!(text_string("é"), Obj::Str(vec![0xfe, 0xff, 0, 0xe9]));
    }

    #[test]
    fn containers_fill_lines() {
        let dict = Obj::dict([
            ("Type", Obj::name("Page")),
            ("MediaBox", Obj::reals([0.0, 0.0, 460.8, 345.6])),
        ]);
        assert_eq!(
            text(&dict),
            "<< /Type /Page /MediaBox [ 0 0 460.8 345.6 ] >>"
        );
        let long = vec![b"x".repeat(80)];
        assert_eq!(fill(&long), [b"\n".to_vec(), b"x".repeat(80)].concat());
        let words: Vec<Vec<u8>> = (0..30).map(|i| format!("w{i:02}").into_bytes()).collect();
        let filled = String::from_utf8(fill(&words)).unwrap();
        assert!(filled.lines().all(|line| line.len() < 75));
        assert_eq!(filled.lines().count(), 2);
    }
}
