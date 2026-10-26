//! [`Type1Font`]: matplotlib's `_type1font.Type1Font`.

use std::path::Path;

use super::container::{Type1Parts, decode, split};
use super::crypt::{Type1Cipher, decrypt, encrypt};
use super::error::Type1Error;
use super::parse::{Abbreviations, Spans, parse};
use super::props::Type1Properties;
use crate::error::{FontError, Result};

/// A Type 1 font as matplotlib reads it for its PDF backend: the three
/// parts of the font program, the decrypted private part, the dictionary
/// entries and where each definition lies, so that the font can be
/// slanted, extended, re-encoded and subset by rewriting its source.
#[derive(Clone, Debug)]
pub struct Type1Font {
    parts: Type1Parts,
    decrypted: Vec<u8>,
    prop: Type1Properties,
    spans: Spans,
    abbreviations: Abbreviations,
}

impl Type1Font {
    /// Reads a font from the contents of a PFA or PFB file.
    pub fn from_bytes(data: &[u8]) -> Result<Self, Type1Error> {
        Self::from_parts(split(&decode(data)?)?)
    }

    /// Reads the PFA or PFB file at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|error| FontError::Io {
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;
        Ok(Self::from_bytes(&data)?)
    }

    /// A font from already split parts (matplotlib's `Type1Font(parts)`).
    pub fn from_parts(parts: Type1Parts) -> Result<Self, Type1Error> {
        let decrypted = decrypt(&parts.encrypted, Type1Cipher::Eexec, 4);
        let mut data = parts.cleartext.clone();
        data.extend_from_slice(&decrypted);
        let parsed = parse(&data)?;
        Ok(Self {
            parts,
            decrypted,
            prop: parsed.prop,
            spans: parsed.spans,
            abbreviations: parsed.abbreviations,
        })
    }

    /// The cleartext part, the encrypted part and the finale.
    pub fn parts(&self) -> &Type1Parts {
        &self.parts
    }

    /// The decrypted form of the encrypted part.
    pub fn decrypted(&self) -> &[u8] {
        &self.decrypted
    }

    /// The font's dictionary entries.
    pub fn prop(&self) -> &Type1Properties {
        &self.prop
    }

    /// Where each definition of `key` starts and ends, as offsets into
    /// the cleartext part followed by the decrypted part (matplotlib's
    /// `_pos[key]`); empty for keys the font does not define.
    pub fn spans(&self, key: &str) -> &[(usize, usize)] {
        self.spans.get(key).map_or(&[], Vec::as_slice)
    }

    /// The keys with recorded definitions, in sorted order.
    pub fn span_keys(&self) -> impl Iterator<Item = &str> {
        self.spans.keys().map(String::as_str)
    }

    /// The font's names for the `RD`, `ND` and `NP` procedures.
    pub fn abbreviations(&self) -> &Abbreviations {
        &self.abbreviations
    }

    /// Encrypts `plaintext` after four NUL bytes (matplotlib's
    /// `Type1Font._encrypt`).
    pub fn encrypt(plaintext: &[u8], cipher: Type1Cipher) -> Vec<u8> {
        encrypt(plaintext, cipher, 4)
    }

    /// Decrypts `ciphertext` and drops its first four bytes (matplotlib's
    /// `Type1Font._decrypt`).
    pub fn decrypt(ciphertext: &[u8], cipher: Type1Cipher) -> Vec<u8> {
        decrypt(ciphertext, cipher, 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type1::test_support::{fixture, pfa};

    #[test]
    fn cmr10_parts_and_entries() {
        let raw = fixture("cmr10.pfb");
        let font = Type1Font::from_bytes(&raw).unwrap();
        assert_eq!(font.parts().cleartext, raw[0x0006..0x10c5].to_vec());
        assert_eq!(font.parts().encrypted, raw[0x10cb..0x897f].to_vec());
        assert_eq!(font.parts().finale, raw[0x8985..0x8ba6].to_vec());
        assert!(
            font.decrypted()
                .starts_with(b"dup\n/Private 18 dict dup begin")
        );
        assert!(font.decrypted().ends_with(b"mark currentfile closefile\n"));
        assert_eq!(font.spans("UniqueID"), [(797, 818), (4483, 4504)]);
        let len0 = font.parts().cleartext.len();
        for key in font.span_keys() {
            for &(start, end) in font.spans(key) {
                let text = if start < len0 {
                    &font.parts().cleartext[start..end]
                } else {
                    &font.decrypted()[start - len0..end - len0]
                };
                assert!(text.starts_with(format!("/{key}").as_bytes()), "{key}");
            }
        }
        let prop = font.prop();
        assert_eq!(prop.weight().as_text(), Some("Medium"));
        assert_eq!(prop.is_fixed_pitch(), &crate::type1::PsValue::Bool(false));
        assert_eq!(prop.italic_angle().to_f64(), Some(0.0));
        assert_eq!(prop.encoding().and_then(|e| e.get(5)), Some("Pi"));
        assert!(prop.char_strings().get("Pi").is_some());
        assert_eq!(font.abbreviations().nd, "ND");
    }

    #[test]
    fn courier_uses_the_standard_encoding() {
        let font = Type1Font::from_bytes(&fixture("Courier10PitchBT-Bold.pfb")).unwrap();
        assert_eq!(font.prop().weight().as_text(), Some("Bold"));
        assert_eq!(
            font.prop().is_fixed_pitch(),
            &crate::type1::PsValue::Bool(true)
        );
        assert_eq!(font.prop().encoding().and_then(|e| e.get(65)), Some("A"));
        let [(start, end)] = font.spans("Encoding") else {
            panic!("one Encoding definition");
        };
        assert_eq!(
            &font.parts().cleartext[*start..*end],
            b"/Encoding StandardEncoding"
        );
        assert_eq!(font.abbreviations().nd, "|-");
    }

    #[test]
    fn a_minimal_pfa_font() {
        let private = b"/CharStrings 1 begin\n/.notdef 5 RD \x00\x01\x02\x03\x04 ND\nend\n";
        let font = Type1Font::from_bytes(&pfa(private)).unwrap();
        assert_eq!(
            font.parts().cleartext,
            b"%!PS-AdobeFont-1.0: X 001.000\neexec\n".to_vec()
        );
        assert!(font.decrypted().starts_with(b"/FontName /X def"));
        assert_eq!(font.prop().char_strings().len(), 1);
        assert_eq!(font.spans("FontName").len(), 1);
        assert_eq!(font.spans("UniqueID"), []);
        assert!(font.span_keys().any(|key| key == "CharStrings"));
        assert_eq!(font.abbreviations().rd, "RD");
        assert!(matches!(
            Type1Font::open("/nonexistent/font.pfb"),
            Err(FontError::Io { .. })
        ));
    }
}
