//! TrueType hinting as matplotlib's bundled FreeType 2.14 does it.
//!
//! matplotlib's default `text.hinting` loads glyphs with FreeType's
//! `FT_LOAD_DEFAULT`, which runs a TrueType font's bytecode with the
//! version 40 interpreter ("minimal subpixel hinting"): instructions may
//! not move points along x, `DELTAP`/`SHPIX` only touch points already
//! grid-fitted in y, nothing moves once both `IUP`s ran, and the advance
//! is the unhinted one rounded to whole pixels. This module ports that
//! interpreter (`ttinterp.c`) and the parts of the TrueType driver around
//! it (`ttobjs.c`, `ttgload.c`): the font and control value programs, the
//! control value scaling, phantom points, the twilight zone, composite
//! glyph hinting and the grid-fitted advance.
//!
//! * [`Hinting`] — FreeType's load flags behind `text.hinting`, and which
//!   glyphs they load with the interpreter.
//! * [`HintingInstance`] — one face at one size after its font and control
//!   value programs ran; [`HintingInstance::load_glyph`] hints glyphs.
//! * [`HintedGlyph`] — the grid-fitted outline and advance.
//!
//! What is left out, deliberately: FreeType's autohinter (glyphs FreeType
//! would autohint load unhinted) and the CFF hinter; the bytecode of
//! "tricky" fonts (a handful of CJK fonts FreeType recognises by name);
//! variation fonts; the `hdmx` advances fonts outside backward
//! compatibility mode may use; and non-square pixels, which plt-rs never
//! asks for. Every glyph starts from the state the control value program
//! left (FreeType keeps twilight points and some control value writes from
//! one glyph to the next until the size is set again, which matplotlib
//! does before rendering every glyph).
//!
//! The bytecode comes from untrusted font files: it runs within FreeType's
//! limits (a million instructions per program, bounded loops, calls and
//! backward jumps, the `maxp` sizes for stack, storage, definitions and
//! twilight points) plus a work budget on the points instructions visit,
//! so it can neither crash, nor hang, nor allocate without bound.

mod error;
mod interp;
mod layout;
mod loader;
mod program;
mod state;
#[cfg(test)]
mod tests;
mod zone;

pub use error::{BytecodeError, BytecodeProgram};
pub(crate) use layout::HintedFace;
pub use loader::HintedGlyph;
pub use program::HintingInstance;

use crate::face::Face;
use crate::types::Tag;

/// How glyph outlines are grid-fitted: the FreeType load flags
/// matplotlib's `text.hinting` selects.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Hinting {
    /// No grid fitting (`FT_LOAD_NO_HINTING`; `none` or `no_hinting`).
    #[default]
    None,
    /// FreeType's default (`FT_LOAD_DEFAULT`; `default` or `either`):
    /// native hinting for fonts with bytecode, the autohinter otherwise.
    Default,
    /// Native hinting only (`FT_LOAD_NO_AUTOHINT`; `native` or
    /// `no_autohint`), even for fonts without bytecode.
    Native,
    /// The autohinter for every font (`FT_LOAD_FORCE_AUTOHINT`; `auto` or
    /// `force_autohint`).
    Auto,
}

impl Hinting {
    /// Whether FreeType loads `face`'s glyphs with the TrueType bytecode
    /// interpreter in this mode. `axis_aligned` tells whether the glyph
    /// transform is a rotation by a multiple of 90 degrees: FreeType only
    /// autohints such glyphs and hints all others natively.
    ///
    /// Glyphs FreeType would give to its autohinter, or to its CFF hinter,
    /// are loaded unhinted: those hinters are not ported.
    pub fn uses_bytecode(self, face: &Face<'_>, axis_aligned: bool) -> bool {
        if face.glyf().is_none() {
            return false;
        }
        match self {
            Self::None => false,
            Self::Native => true,
            Self::Default => !axis_aligned || has_bytecode(face),
            Self::Auto => !axis_aligned,
        }
    }
}

/// Whether a TrueType face carries hinting programs: FreeType's test
/// before autohinting under `FT_LOAD_DEFAULT` (a font program, or a
/// control value program longer than a trivial one).
fn has_bytecode(face: &Face<'_>) -> bool {
    let length = |tag: &[u8; 4]| face.table(Tag::new(tag)).map_or(0, <[u8]>::len);
    length(b"fpgm") > 0 || length(b"prep") > 7
}
