//! Reading AFM files exactly as matplotlib's `_afm` module does: the header
//! up to `StartCharMetrics` (unknown or unreadable entries are skipped),
//! the character metrics, then kerning pairs and composites wherever they
//! appear. AFM data is untrusted: every malformation matplotlib raises on
//! is an [`AfmError`], and nothing here panics.

use std::collections::HashMap;

use super::error::{AfmError, AfmSection};
use super::metrics::{AfmMetrics, CharMetrics, CharTable, CompositePart, Header, KernTable};

/// Why a number could not be converted: Python's `ValueError` (which the
/// header parser forgives) or `OverflowError` (which it does not).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumberError {
    Invalid,
    Overflow,
}

/// Python's `bytes.isspace` for one byte.
const fn is_py_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// `bytes.rstrip()`.
fn rstrip(line: &[u8]) -> &[u8] {
    let end = line
        .iter()
        .rposition(|&b| !is_py_space(b))
        .map_or(0, |last| last + 1);
    &line[..end]
}

/// `bytes.split()`: the runs of non-whitespace bytes.
fn split_words(line: &[u8]) -> Vec<&[u8]> {
    line.split(|&b| is_py_space(b))
        .filter(|word| !word.is_empty())
        .collect()
}

/// Python's `float(text)`: surrounding whitespace ignored, single
/// underscores allowed between digits, `inf` and `nan` accepted.
fn py_float(text: &str) -> Result<f64, NumberError> {
    let text = text.trim();
    let bytes = text.as_bytes();
    let underscores_valid = bytes.iter().enumerate().all(|(i, &b)| {
        b != b'_'
            || (i > 0
                && bytes[i - 1].is_ascii_digit()
                && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
    });
    if !underscores_valid {
        return Err(NumberError::Invalid);
    }
    text.replace('_', "")
        .parse()
        .map_err(|_| NumberError::Invalid)
}

/// `_to_float`: a decimal comma is read as a point.
fn to_float(text: &str) -> Result<f64, NumberError> {
    py_float(&text.replace(',', "."))
}

/// Raw bytes decoded as Latin-1.
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// `_to_float` of raw bytes, decoded as Latin-1 first.
fn bytes_to_float(bytes: &[u8]) -> Result<f64, NumberError> {
    to_float(&latin1(bytes))
}

/// Python's `int(value)` of a float: truncated, NaN invalid, infinities
/// overflowing.
fn truncate(value: f64) -> Result<i64, NumberError> {
    if value.is_nan() {
        Err(NumberError::Invalid)
    } else if value.is_infinite() {
        Err(NumberError::Overflow)
    } else {
        Ok(value.trunc() as i64)
    }
}

/// `_to_int`: `int(float(text))`, without `_to_float`'s decimal comma.
fn to_int(text: &str) -> Result<i64, NumberError> {
    truncate(py_float(text)?)
}

/// Stores `value` in `slot` when it could be read: matplotlib skips header
/// entries it cannot convert, keeping any earlier value.
fn keep<T>(slot: &mut Option<T>, value: Option<T>) {
    if value.is_some() {
        *slot = value;
    }
}

/// The lines of the file as Python iterates a binary file (each ending
/// at a newline), numbered from 1.
struct Lines<'a> {
    rest: &'a [u8],
    number: usize,
}

impl<'a> Lines<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self {
            rest: data,
            number: 0,
        }
    }

    /// The next line, or the error for data ending inside `section`.
    fn expect(&mut self, section: AfmSection) -> Result<(usize, &'a [u8]), AfmError> {
        self.next().ok_or(AfmError::UnexpectedEnd(section))
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = (usize, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let end = self
            .rest
            .iter()
            .position(|&b| b == b'\n')
            .map_or(self.rest.len(), |newline| newline + 1);
        let (line, rest) = self.rest.split_at(end);
        self.rest = rest;
        self.number += 1;
        Some((self.number, line))
    }
}

/// `_parse_header`: the entries matplotlib uses, up to `StartCharMetrics`.
fn parse_header(lines: &mut Lines<'_>) -> Result<Header, AfmError> {
    let mut header = Header::default();
    let mut first_line = true;
    loop {
        let (number, raw) = lines.expect(AfmSection::Header)?;
        let line = rstrip(raw);
        if line.starts_with(b"Comment") {
            continue;
        }
        let (key, value) = match line.iter().position(|&b| b == b' ') {
            Some(space) => (&line[..space], &line[space + 1..]),
            None => (line, &b""[..]),
        };
        if first_line {
            if key != b"StartFontMetrics" {
                return Err(AfmError::NotAfm);
            }
            first_line = false;
        }
        let overflow = AfmError::BadLine {
            section: AfmSection::Header,
            line: number,
        };
        let text = || std::str::from_utf8(value).ok().map(str::to_owned);
        let float = || bytes_to_float(value).ok();
        match key {
            b"FontName" => keep(&mut header.font_name, text()),
            b"FullName" => keep(&mut header.full_name, text()),
            b"FamilyName" => keep(&mut header.family_name, text()),
            b"Weight" => keep(&mut header.weight, text()),
            b"ItalicAngle" => keep(&mut header.italic_angle, float()),
            b"Ascender" => keep(&mut header.ascender, float()),
            b"Descender" => keep(&mut header.descender, float()),
            b"CapHeight" => keep(&mut header.cap_height, float()),
            b"XHeight" => keep(&mut header.x_height, float()),
            b"UnderlineThickness" => keep(&mut header.underline_thickness, float()),
            // Only an overflowing integer matters in the entries matplotlib
            // reads but never uses; `StartCharMetrics` ends the header once
            // its count reads as a number.
            b"FontBBox" => {
                let words = latin1(value).replace(',', " ");
                if words
                    .split_whitespace()
                    .any(|word| to_int(word) == Err(NumberError::Overflow))
                {
                    return Err(overflow);
                }
            }
            b"Characters" | b"StartCharMetrics" => match to_int(&latin1(value)) {
                Ok(_) if key == b"StartCharMetrics" => return Ok(header),
                Err(NumberError::Overflow) => return Err(overflow),
                Ok(_) | Err(NumberError::Invalid) => {}
            },
            _ => {}
        }
    }
}

/// The `key value` entries of a character metrics line (separated by
/// semicolons; a later key replaces an earlier one).
fn metric_entries(line: &str) -> Option<HashMap<&str, &str>> {
    line.split(';')
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.trim().split_once(' '))
        .collect()
}

/// One character metrics line: its character code and metrics.
fn char_metrics_line(line: &str) -> Option<(i64, CharMetrics)> {
    let entries = metric_entries(line)?;
    let code = to_int(entries.get("C")?).ok()?;
    let width = to_float(entries.get("WX")?).ok()?;
    let name = (*entries.get("N")?).to_owned();
    let bbox: Vec<f64> = entries
        .get("B")?
        .split_whitespace()
        .map(|v| to_float(v).and_then(truncate).map(|v| v as f64))
        .collect::<Result<_, _>>()
        .ok()?;
    let bbox: [f64; 4] = bbox.try_into().ok()?;
    // matplotlib files the Euro and the minus sign under their WinAnsi and
    // Unicode code points.
    let code = match name.as_str() {
        "Euro" => 128,
        "minus" => 0x2212,
        _ => code,
    };
    Some((code, CharMetrics { width, name, bbox }))
}

/// `_parse_char_metrics`: every character up to `EndCharMetrics`.
fn parse_char_metrics(lines: &mut Lines<'_>) -> Result<CharTable, AfmError> {
    let mut table = CharTable::default();
    loop {
        let (number, raw) = lines.expect(AfmSection::CharMetrics)?;
        let bad = AfmError::BadLine {
            section: AfmSection::CharMetrics,
            line: number,
        };
        let line = std::str::from_utf8(rstrip(raw)).map_err(|_| bad.clone())?;
        if line.starts_with("EndCharMetrics") {
            return Ok(table);
        }
        let (code, metrics) = char_metrics_line(line).ok_or(bad)?;
        table.insert(code, metrics);
    }
}

/// One `KPX first second value` line.
fn kern_pair(line: &[u8]) -> Option<(String, String, f64)> {
    let [keyword, first, second, value] = split_words(line).try_into().ok()?;
    if keyword != b"KPX" {
        return None;
    }
    let name = |bytes: &[u8]| std::str::from_utf8(bytes).ok().map(str::to_owned);
    Some((name(first)?, name(second)?, bytes_to_float(value).ok()?))
}

/// `_parse_kern_pairs`, after `StartKernData`: the pairs up to
/// `EndKernPairs` and the (unchecked) line after it.
fn parse_kern_pairs(lines: &mut Lines<'_>) -> Result<KernTable, AfmError> {
    let (number, start) = lines.expect(AfmSection::KernPairs)?;
    if !start.starts_with(b"StartKernPairs") {
        return Err(AfmError::BadLine {
            section: AfmSection::KernPairs,
            line: number,
        });
    }
    let mut table = KernTable::default();
    loop {
        let (number, raw) = lines.expect(AfmSection::KernPairs)?;
        let line = rstrip(raw);
        if line.is_empty() {
            continue;
        }
        if line.starts_with(b"EndKernPairs") {
            lines.expect(AfmSection::KernPairs)?;
            return Ok(table);
        }
        let (first, second, value) = kern_pair(line).ok_or(AfmError::BadLine {
            section: AfmSection::KernPairs,
            line: number,
        })?;
        table.insert(first, second, value);
    }
}

/// One `CC name parts ; PCC part dx dy ; ...` line.
fn composite(line: &[u8]) -> Option<(String, Vec<CompositePart>)> {
    let fields: Vec<&[u8]> = line.split(|&b| b == b';').collect();
    let head = split_words(fields.first()?);
    let name = String::from_utf8_lossy(head.get(1)?).into_owned();
    let count = to_int(&latin1(head.get(2)?)).ok()?;
    if i64::try_from(fields.len()).ok()? != count.checked_add(2)? {
        return None;
    }
    let parts = fields
        .get(1..fields.len() - 1)
        .unwrap_or_default()
        .iter()
        .map(|field| {
            let words = split_words(field);
            Some(CompositePart {
                name: String::from_utf8_lossy(words.get(1)?).into_owned(),
                dx: bytes_to_float(words.get(2)?).ok()?,
                dy: bytes_to_float(words.get(3)?).ok()?,
            })
        })
        .collect::<Option<_>>()?;
    Some((name, parts))
}

/// `_parse_composites`, after `StartComposites`.
fn parse_composites(
    lines: &mut Lines<'_>,
) -> Result<HashMap<String, Vec<CompositePart>>, AfmError> {
    let mut composites = HashMap::new();
    loop {
        let (number, raw) = lines.expect(AfmSection::Composites)?;
        let line = rstrip(raw);
        if line.is_empty() {
            continue;
        }
        if line.starts_with(b"EndComposites") {
            return Ok(composites);
        }
        let (name, parts) = composite(line).ok_or(AfmError::BadLine {
            section: AfmSection::Composites,
            line: number,
        })?;
        composites.insert(name, parts);
    }
}

/// Parses AFM data (matplotlib's `AFM(fh)`).
pub(super) fn parse(data: &[u8]) -> Result<AfmMetrics, AfmError> {
    let mut lines = Lines::new(data);
    let header = parse_header(&mut lines)?;
    let chars = parse_char_metrics(&mut lines)?;
    let mut kern = KernTable::default();
    let mut composites = HashMap::new();
    while let Some((_, raw)) = lines.next() {
        match split_words(rstrip(raw)).first().copied() {
            Some(b"StartKernData") => kern = parse_kern_pairs(&mut lines)?,
            Some(b"StartComposites") => composites = parse_composites(&mut lines)?,
            _ => {}
        }
    }
    Ok(AfmMetrics {
        header,
        chars,
        kern,
        composites,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_follow_python() {
        assert_eq!(to_float(" 1,5 "), Ok(1.5));
        assert_eq!(to_float("1_000"), Ok(1000.0));
        assert_eq!(to_float("1__0"), Err(NumberError::Invalid));
        assert_eq!(to_float("_1"), Err(NumberError::Invalid));
        assert_eq!(to_float("x"), Err(NumberError::Invalid));
        assert_eq!(to_int("-2.9"), Ok(-2));
        assert_eq!(to_int("2,5"), Err(NumberError::Invalid), "no decimal comma");
        assert_eq!(to_int("nan"), Err(NumberError::Invalid));
        assert_eq!(to_int("-inf"), Err(NumberError::Overflow));
        assert_eq!(bytes_to_float(b"\xe912"), Err(NumberError::Invalid));
    }

    #[test]
    fn lines_split_like_python_files() {
        let lines: Vec<(usize, &[u8])> = Lines::new(b"a\r\nb\n\nc").collect();
        assert_eq!(
            lines,
            [(1, &b"a\r\n"[..]), (2, b"b\n"), (3, b"\n"), (4, b"c")]
        );
        assert_eq!(Lines::new(b"a\n").count(), 1);
        assert_eq!(rstrip(b"x \t\x0b\x0c\r\n"), b"x");
        assert_eq!(
            split_words(b" KPX  A\tV -80 "),
            [&b"KPX"[..], b"A", b"V", b"-80"]
        );
    }

    #[test]
    fn character_metrics_lines() {
        let (code, metrics) =
            char_metrics_line("C 65 ; WX 667 ; N A ; B 14 0 654 718 ;").expect("valid");
        assert_eq!(code, 65);
        assert_eq!(metrics.name, "A");
        assert_eq!(metrics.bbox, [14.0, 0.0, 654.0, 718.0]);
        let (code, _) = char_metrics_line("C -1 ; WX 556 ; N Euro ; B 0 0 1 1 ;").expect("valid");
        assert_eq!(code, 128);
        let (_, metrics) =
            char_metrics_line("C 1 ; WX 5,5 ; N x ; B 0.9 -1.9 2 3 ; L f fi").expect("valid");
        assert_eq!((metrics.width, metrics.bbox[1]), (5.5, -1.0));
        assert!(char_metrics_line("C 65 ; WX 667 ; N A").is_none(), "no box");
        assert!(char_metrics_line("C 65 ; WX 667 ; N A ; B 1 2 3").is_none());
        assert!(char_metrics_line("C 65 ; WX 667 ; N A ; B 1 2 3 nan").is_none());
        assert!(char_metrics_line("C 65 ;  ; WX 1 ; N A ; B 1 2 3 4").is_none());
        assert!(char_metrics_line("").is_none());
    }

    #[test]
    fn kern_pairs_and_composites() {
        assert_eq!(
            kern_pair(b"KPX A V -70"),
            Some(("A".to_owned(), "V".to_owned(), -70.0))
        );
        assert_eq!(kern_pair(b"KPY A V -70"), None);
        assert_eq!(kern_pair(b"KPX A -70"), None);
        let (name, parts) =
            composite(b"CC Aacute 2 ; PCC A 0 0 ; PCC acute 160 170 ;").expect("valid");
        assert_eq!(name, "Aacute");
        assert_eq!(parts[1].name, "acute");
        assert_eq!((parts[1].dx, parts[1].dy), (160.0, 170.0));
        assert!(composite(b"CC Aacute 3 ; PCC A 0 0 ;").is_none());
        assert!(composite(b"CC Aacute").is_none());
        assert!(composite(b"CC Aacute 1 ; PCC A 0 ;").is_none());
        let (_, parts) = composite(b"CC x -1").expect("matplotlib accepts it");
        assert_eq!(parts, []);
    }
}
