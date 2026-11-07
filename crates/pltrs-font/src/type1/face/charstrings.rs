//! What the charstrings of a Type 1 face reach beyond themselves, for the
//! Adobe engine: their subroutines, the `seac` components looked up by
//! their Standard Encoding names, and the alignment zones.

use crate::psaux::{CharstringError, Charstrings, Flavor, PrivateDict};
use crate::type1::standard::standard_name;

/// The decrypted programs of a Type 1 face and its Private dictionary.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphSet<'a> {
    pub(crate) names: &'a [String],
    pub(crate) charstrings: &'a [Vec<u8>],
    pub(crate) subrs: &'a [Vec<u8>],
    pub(crate) private: &'a PrivateDict,
}

impl<'a> Charstrings<'a> for GlyphSet<'a> {
    fn flavor(&self) -> Flavor {
        Flavor::Type1
    }

    fn local_subr(&self, number: i32) -> Option<&'a [u8]> {
        let number = usize::try_from(number).ok()?;
        self.subrs.get(number).map(Vec::as_slice)
    }

    fn global_subr(&self, _number: i32) -> Option<&'a [u8]> {
        None
    }

    /// The glyph whose name the Standard Encoding gives `code`, as
    /// `t1_lookup_glyph_by_stdcharcode_ps` finds it (`.notdef` for
    /// unencoded codes).
    fn standard_glyph(&self, code: i32) -> Result<&'a [u8], CharstringError> {
        let name = u8::try_from(code)
            .map(standard_name)
            .map_err(|_| CharstringError::NotStandardCode(i64::from(code)))?;
        self.names
            .iter()
            .position(|glyph| glyph == name)
            .and_then(|glyph| self.charstrings.get(glyph))
            .map(Vec::as_slice)
            .ok_or_else(|| CharstringError::MissingGlyph(name.to_owned()))
    }

    fn private(&self) -> &PrivateDict {
        self.private
    }
}
