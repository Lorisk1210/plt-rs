//! PostScript Type 1 fonts (`.pfa`, `.pfb`), the fonts TeX typesets with
//! and matplotlib's `text.usetex` draws and embeds.
//!
//! Two views of a font, as matplotlib has them:
//!
//! * [`Type1Font`] — matplotlib's `_type1font.Type1Font`, which its PDF
//!   backend embeds: the PFA/PFB containers, the cleartext and encrypted
//!   parts with `eexec` and charstring decryption, the dictionary entries
//!   ([`Type1Properties`]) and where they are defined, and the rewriting
//!   operations [`transform`](Type1Font::transform) (the `SlantFont` and
//!   `ExtendFont` effects of `psfonts.map`),
//!   [`with_encoding`](Type1Font::with_encoding) and
//!   [`subset`](Type1Font::subset), which reproduce matplotlib's bytes.
//! * [`Type1Face`] — the font as FreeType's Type 1 driver loads it for
//!   matplotlib's `FT2Font` (`get_font` of a `.pfb` file in `textpath`
//!   and `dviread`): glyph indices and names in FreeType's order, the
//!   face metrics, the encoding vector, and glyph outlines and advances
//!   decoded by a port of FreeType's Adobe charstring engine
//!   ([`Type1Glyph`]): unscaled (`FT_LOAD_NO_SCALE`), scaled unhinted
//!   (`FT_LOAD_NO_HINTING`), or hinted at a size with the engine's
//!   alignment zones and hint maps (`FT_LOAD_TARGET_LIGHT`, as `textpath`
//!   loads usetex glyphs).
//!
//! Font data is untrusted: every operation returns a [`Type1Error`] where
//! matplotlib raises, and never panics or runs unbounded.

mod cf2;
mod container;
mod crypt;
mod edit;
mod error;
mod face;
mod font;
mod parse;
mod props;
mod python;
mod standard;
mod subset;
#[cfg(test)]
mod test_support;
mod token;
mod value;

pub use container::Type1Parts;
pub use crypt::Type1Cipher;
pub use edit::Type1Effects;
pub use error::{Type1Error, Type1Section};
pub use face::{Type1Face, Type1Glyph};
pub use font::Type1Font;
pub use parse::Abbreviations;
pub use props::{CharStrings, Program, Type1Encoding, Type1Properties};
pub use token::{PsToken, PsTokenKind, ps_tokenize};
pub use value::{PsNumber, PsValue};
