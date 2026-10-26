//! Rewriting a font's source: matplotlib's `Type1Font.transform` (the
//! `SlantFont` and `ExtendFont` effects of `psfonts.map`),
//! `Type1Font.with_encoding` and the `_replace` they share with `subset`.

use super::container::Type1Parts;
use super::crypt::{Type1Cipher, encrypt};
use super::error::Type1Error;
use super::font::Type1Font;
use super::props::Type1Encoding;
use super::python::{
    format_approx, is_python_space, latin1, numpy_round, parse_float, repr, slice, slice_from,
};

/// The effects pdfTeX's font map applies to a font.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Type1Effects {
    /// `SlantFont`: the tangent of the angle the glyphs lean right by
    /// (negative leans left).
    pub slant: Option<f64>,
    /// `ExtendFont`: the factor applied to glyph widths (below 1
    /// condenses).
    pub extend: Option<f64>,
}

/// A replacement of the source between two offsets.
pub(crate) type Replacement = ((usize, usize), Vec<u8>);

/// Python's `int(value)` of a float as decimal digits.
fn truncated(value: f64, what: &'static str) -> Result<String, Type1Error> {
    if !value.is_finite() {
        return Err(Type1Error::InvalidValue(what));
    }
    let value = value.trunc();
    Ok(if value == 0.0 {
        "0".to_owned()
    } else {
        format!("{value:.0}")
    })
}

impl Type1Font {
    /// A copy of the font slanted and extended as matplotlib's
    /// `Type1Font.transform` does it: the effects are appended to the font
    /// name, `ItalicAngle` decreases by the slant's angle, the
    /// `FontMatrix` is multiplied by the effects and `UniqueID` is
    /// removed.
    pub fn transform(&self, effects: Type1Effects) -> Result<Self, Type1Error> {
        let prop = self.prop();
        let mut font_name = prop.font_name().to_string();
        let mut italic_angle = prop.italic_angle().to_string();
        let textual = prop.font_name().as_text().is_some();
        let matrix = prop
            .get("FontMatrix")
            .ok_or(Type1Error::MissingKey("FontMatrix"))?
            .as_text()
            .ok_or(Type1Error::InvalidValue("FontMatrix"))?;
        let values = matrix
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(is_python_space)
            .filter(|item| !item.is_empty())
            .map(|item| parse_float(item).ok_or_else(|| Type1Error::InvalidNumber(item.to_owned())))
            .collect::<Result<Vec<f64>, _>>()?;
        let [a, b, c, d, e, f] = values[..] else {
            return Err(Type1Error::InvalidValue("FontMatrix"));
        };
        // The effects matrix [[extend, 0, 0], [slant, 1, 0], [0, 0, 1]]
        // times the font matrix [[a, b, 0], [c, d, 0], [e, f, 1]].
        let (mut extend, mut slant) = (1.0, 0.0);
        if let Some(value) = effects.slant {
            if !textual {
                return Err(Type1Error::InvalidValue("FontName"));
            }
            font_name.push_str("_Slant_");
            font_name.push_str(&truncated(1000.0 * value, "slant")?);
            let angle = prop
                .italic_angle()
                .to_f64()
                .ok_or(Type1Error::InvalidValue("ItalicAngle"))?;
            let tilt = value.atan() / std::f64::consts::PI * 180.0;
            italic_angle = repr(numpy_round(angle - tilt, 5));
            slant = value;
        }
        if let Some(value) = effects.extend {
            if !textual {
                return Err(Type1Error::InvalidValue("FontName"));
            }
            font_name.push_str("_Extend_");
            font_name.push_str(&truncated(1000.0 * value, "extend")?);
            extend = value;
        }
        let product = [
            extend * a + 0.0 * c + 0.0 * e,
            extend * b + 0.0 * d + 0.0 * f,
            slant * a + c + 0.0 * e,
            slant * b + d + 0.0 * f,
            0.0 * a + 0.0 * c + e,
            0.0 * b + 0.0 * d + f,
        ];
        let matrix = product
            .iter()
            .map(|&x| format_approx(x, 6))
            .collect::<Vec<_>>()
            .join(" ");
        let mut replacements =
            self.replace_defined("FontName", &format!("/FontName/{font_name} def"))?;
        replacements.extend(
            self.replace_defined("ItalicAngle", &format!("/ItalicAngle {italic_angle} def"))?,
        );
        replacements.extend(self.replace_defined(
            "FontMatrix",
            &format!("/FontMatrix [{matrix}] readonly def"),
        )?);
        replacements.extend(self.replace_all("UniqueID", "")?);
        self.rewrite(replacements)
    }

    /// A copy of the font with `encoding` written as its encoding array and
    /// `UniqueID` removed (matplotlib's `Type1Font.with_encoding`).
    pub fn with_encoding(&self, encoding: &Type1Encoding) -> Result<Self, Type1Error> {
        let mut replacements = self.replace_all("UniqueID", "")?;
        replacements.push((self.first_span("Encoding")?, postscript_encoding(encoding)?));
        self.rewrite(replacements)
    }

    /// The first definition of `key`, which must exist.
    pub(crate) fn first_span(&self, key: &'static str) -> Result<(usize, usize), Type1Error> {
        self.spans(key)
            .first()
            .copied()
            .ok_or(Type1Error::MissingKey(key))
    }

    /// Replaces every definition of `key`, which the font must define,
    /// with `text`.
    pub(crate) fn replace_defined(
        &self,
        key: &'static str,
        text: &str,
    ) -> Result<Vec<Replacement>, Type1Error> {
        self.first_span(key)?;
        self.replace_all(key, text)
    }

    /// Replaces every definition of `key` with `text`.
    pub(crate) fn replace_all(
        &self,
        key: &str,
        text: &str,
    ) -> Result<Vec<Replacement>, Type1Error> {
        let bytes = latin1(text)?;
        Ok(self
            .spans(key)
            .iter()
            .map(|&span| (span, bytes.clone()))
            .collect())
    }

    /// The font with the source between each pair of offsets replaced,
    /// from the last pair to the first; the decrypted part is encrypted
    /// again (matplotlib's `_replace` and the `Type1Font` it builds).
    pub(crate) fn rewrite(&self, mut replacements: Vec<Replacement>) -> Result<Self, Type1Error> {
        let parts = self.parts();
        let clear_len = parts.cleartext.len();
        let mut data = parts.cleartext.clone();
        data.extend_from_slice(self.decrypted());
        let mut split = clear_len as i64;
        replacements.sort_by(|x, y| y.cmp(x));
        for ((start, end), value) in replacements {
            let range = slice(data.len(), start as i64, end as i64);
            if start < clear_len {
                if end >= clear_len {
                    return Err(Type1Error::SpansEexecBoundary);
                }
                split += value.len() as i64 - end as i64 + start as i64;
            }
            data.splice(range, value);
        }
        let cleartext = data[slice(data.len(), 0, split)].to_vec();
        let private = &data[slice_from(data.len(), split)..];
        Self::from_parts(Type1Parts {
            cleartext,
            encrypted: encrypt(private, Type1Cipher::Eexec, 4),
            finale: parts.finale.clone(),
        })
    }
}

/// The PostScript encoding array for `encoding` (matplotlib's
/// `_postscript_encoding`).
pub(crate) fn postscript_encoding(encoding: &Type1Encoding) -> Result<Vec<u8>, Type1Error> {
    let mut lines = vec![
        "/Encoding 256 array".to_owned(),
        "0 1 255 { 1 index exch /.notdef put} for".to_owned(),
    ];
    lines.extend(
        encoding
            .iter()
            .filter(|&(_, glyph)| glyph != ".notdef")
            .map(|(code, glyph)| format!("dup {code} /{glyph} put")),
    );
    lines.push("readonly def\n".to_owned());
    latin1(&lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type1::test_support::{fixture, pfa};

    fn cmr10() -> Type1Font {
        Type1Font::from_bytes(&fixture("cmr10.pfb")).unwrap()
    }

    fn lines(bytes: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Lines of `after` that are not in `before` (`+`) and of `before`
    /// that are not in `after` (`-`), like the lines of a `difflib` diff.
    fn changes(before: &[u8], after: &[u8]) -> (Vec<String>, Vec<String>) {
        let (before, after) = (lines(before), lines(after));
        let added = after
            .iter()
            .filter(|l| !before.contains(l))
            .cloned()
            .collect();
        let removed = before
            .iter()
            .filter(|l| !after.contains(l))
            .cloned()
            .collect();
        (added, removed)
    }

    #[test]
    fn slanted_cmr10() {
        let font = cmr10();
        let slanted = font
            .transform(Type1Effects {
                slant: Some(1.0),
                extend: None,
            })
            .unwrap();
        assert!(
            slanted
                .decrypted()
                .starts_with(b"dup\n/Private 18 dict dup begin")
        );
        assert!(
            slanted
                .decrypted()
                .ends_with(b"mark currentfile closefile\n")
        );
        assert_eq!(slanted.spans("UniqueID"), []);
        assert_eq!(slanted.prop().italic_angle().to_f64(), Some(-45.0));
        let (added, removed) = changes(&font.parts().cleartext, &slanted.parts().cleartext);
        for line in [
            "/UniqueID 5000793 def",
            "/FontName /CMR10 def",
            "/FontMatrix [0.001 0 0 0.001 0 0 ]readonly def",
            " /ItalicAngle 0 def",
        ] {
            assert!(removed.contains(&line.to_owned()), "{line}");
        }
        for line in [
            "/FontName/CMR10_Slant_1000 def",
            "/FontMatrix [0.001 0 0.001 0.001 0 0] readonly def",
            " /ItalicAngle -45.0 def",
        ] {
            assert!(added.contains(&line.to_owned()), "{line}");
        }
    }

    #[test]
    fn condensed_and_overprecise_effects() {
        let font = cmr10();
        let condensed = font
            .transform(Type1Effects {
                slant: None,
                extend: Some(0.5),
            })
            .unwrap();
        let (added, _) = changes(&font.parts().cleartext, &condensed.parts().cleartext);
        assert!(added.contains(&"/FontName/CMR10_Extend_500 def".to_owned()));
        assert!(added.contains(&"/FontMatrix [0.0005 0 0 0.001 0 0] readonly def".to_owned()));
        let slanted = font
            .transform(Type1Effects {
                slant: Some(0.167),
                extend: None,
            })
            .unwrap();
        let text = String::from_utf8_lossy(&slanted.parts().cleartext).into_owned();
        assert!(text.contains("/FontMatrix [0.001 0 0.000167 0.001 0 0] readonly def"));
        assert!(text.contains("/ItalicAngle -9.4809 def"));
    }

    #[test]
    fn new_encodings_and_errors() {
        let font = cmr10();
        let encoding =
            Type1Encoding::from_codes([(65, "Gamma".to_owned()), (66, ".notdef".to_owned())]);
        let encoded = font.with_encoding(&encoding).unwrap();
        let prop_encoding = encoded.prop().encoding().unwrap();
        assert_eq!(
            (prop_encoding.len(), prop_encoding.get(65)),
            (1, Some("Gamma"))
        );
        assert_eq!(encoded.spans("UniqueID"), []);
        assert_eq!(
            postscript_encoding(&encoding).unwrap(),
            b"/Encoding 256 array\n0 1 255 { 1 index exch /.notdef put} for\ndup 65 /Gamma put\nreadonly def\n".to_vec()
        );
        let bare = Type1Font::from_bytes(&pfa(b"/CharStrings 0 begin end\n")).unwrap();
        assert_eq!(
            bare.transform(Type1Effects::default()).unwrap_err(),
            Type1Error::MissingKey("FontMatrix")
        );
        assert_eq!(
            bare.with_encoding(&encoding).unwrap_err(),
            Type1Error::MissingKey("Encoding")
        );
        assert_eq!(truncated(-0.4, "x").unwrap(), "0");
        assert_eq!(truncated(166.99, "x").unwrap(), "166");
        assert!(truncated(f64::NAN, "x").is_err());
    }
}
