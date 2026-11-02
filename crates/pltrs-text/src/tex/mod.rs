//! TeX typesetting for `text.usetex`: matplotlib's `texmanager` and
//! `dviread`.
//!
//! * [`TexManager`] writes the LaTeX document a string is typeset in
//!   (preamble, font selection from `font.family` and the family lists,
//!   `text.latex.preamble`, `\fontsize`), runs `latex` (and `dvipng` for
//!   Agg's `latex+dvipng` engine) in a cache directory, `tex.cache`, and
//!   measures the typeset text. A missing or failing program is a
//!   [`TexError`] with matplotlib's message.
//! * [`read_dvi`] reads the DVI file `latex` writes into pages of glyphs
//!   and rules ([`RawPage`], [`DviPage`] at a resolution), expanding
//!   virtual fonts ([`Vf`]) and measuring with TeX font metrics
//!   ([`Tfm`]). It also reads the DVI files of XeTeX and LuaTeX, whose
//!   native fonts ([`NativeFont`]) are OpenType and TrueType files
//!   measured from the fonts themselves ([`NativeFace`]).
//! * [`PsfontsMap`] reads `pdftex.map`, which gives each TeX font its Type 1
//!   file, `SlantFont`/`ExtendFont` effects and encoding ([`parse_enc`]);
//!   the program can replace it for the thread ([`set_font_map_lookup`]).
//! * [`TexFiles`] finds TeX's files as matplotlib's `find_tex_file` does
//!   (a long-lived `luatex --luaonly` helper, else `kpsewhich`), keeps
//!   what it read, and maps DVI glyphs to the glyphs of their Type 1
//!   faces ([`TexFontFile`]).
//! * [`TemporaryDirectory`] and [`format_command`] are the temporary
//!   directories and program helpers TeX runs with, which the PostScript
//!   backend's external programs share.
//! * [`TexText`] is a line of usetex text as backends receive it; its
//!   [`glyphs`](TexText::glyphs) are resolved to Type 1 faces, and its
//!   [`text_path`](TexText::text_path) is the outline matplotlib's
//!   `TextToPath` makes of it.
//!
//! Everything read from TeX's files is untrusted: malformed DVI, VF, TFM,
//! map and encoding files give errors, never panics.

mod bytes;
mod command;
mod dvi;
mod error;
mod files;
mod font_file;
#[cfg(test)]
mod fuzz_tests;
mod kpathsea;
mod manager;
mod map_override;
mod native;
mod psfonts;
mod source;
mod tempdir;
#[cfg(test)]
mod test_support;
mod text;
mod tfm;
mod vf;

pub use command::format_command;
pub use dvi::{
    DviBox, DviFont, DviMetrics, DviPage, DviText, RawGlyph, RawPage, RawRule, read_dvi,
};
pub use error::{TexError, TexFile};
pub use files::{FileFinder, FontSource, TexFiles};
pub use font_file::{TexFontFile, TexHinting};
pub use manager::{GreyImage, TexEngine, TexManager, TexRgba};
pub use map_override::{FontMapLookup, set_font_map_lookup};
pub use native::{NativeEffects, NativeFace, NativeFont};
pub use psfonts::{PsFont, PsfontsMap, parse_enc};
pub use source::{TexFonts, tex_source, usepackage_if_not_loaded};
pub use tempdir::TemporaryDirectory;
pub use text::{TexGlyph, TexText};
pub use tfm::{TexMetrics, Tfm};
pub use vf::{Vf, VfPacket};
