//! FreeType's Adobe charstring engine (`psaux`: `psintrp.c`, `pshints.c`,
//! `psblues.c`, `psft.c`), which FreeType 2.14 loads Type 1, CFF and CFF2
//! glyphs with: the interpreter, its operand stack, the glyph path and the
//! outline builder, the hint maps and the alignment zones (light hinting:
//! the engine only moves vertical coordinates).
//!
//! One interpreter runs the three formats ([`Flavor`]), which differ in
//! the operators they have and in how a glyph is run:
//!
//! * a Type 1 charstring runs twice: a first pass collects the stem hints
//!   and skips drawing, the second draws; it has `hsbw`, `sbw`,
//!   `closepath`, the `seac` operator and the OtherSubrs;
//! * a CFF charstring declares its stems before it draws and runs once;
//!   it has global subroutines, hint masks, and an implied `seac` in
//!   `endchar` with four arguments;
//! * a CFF2 charstring has no width, `endchar` or `return`, and blends
//!   its operands between the masters of a variable font (`blend`,
//!   `vsindex`).
//!
//! What a charstring reaches beyond itself (subroutines, `seac`
//! components, the Private dictionary, the blend vectors) comes from its
//! font through [`Charstrings`].
//!
//! Stem darkening is off in FreeType's default configuration (and
//! matplotlib's) for every format, so path elements carry no offsets.
//!
//! The engine is copyright Adobe Systems Incorporated and the FreeType
//! project, and ported here under the FreeType License (FTL).

mod blues;
mod buffer;
mod builder;
mod draw;
mod error;
mod escape;
mod hintmap;
mod hints;
mod interp;
mod path;
mod stack;

pub(crate) use blues::PrivateDict;
pub(crate) use error::CharstringError;
pub(crate) use interp::{Rendering, decode, engine_rendering};

/// The charstring format the engine runs (FreeType's `isT1` and
/// `isCFF2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flavor {
    /// Type 1 charstrings, decrypted.
    Type1,
    /// Type 2 charstrings of a CFF table.
    Cff,
    /// The charstrings of a CFF2 table.
    Cff2,
}

/// `CF2_OPERAND_STACK_SIZE`: the operand stack of Type 1 and CFF
/// charstrings.
pub(crate) const STACK_SIZE: usize = 48;

/// What a charstring of a font reaches beyond itself: FreeType's
/// `cf2_get*` callbacks into the font's decoder.
pub(crate) trait Charstrings<'a> {
    /// The format of the font's charstrings.
    fn flavor(&self) -> Flavor;

    /// Local subroutine `number`, before the bias of its INDEX is added
    /// (Type 1 subroutines have none).
    fn local_subr(&self, number: i32) -> Option<&'a [u8]>;

    /// Global subroutine `number`, before the bias (none in Type 1).
    fn global_subr(&self, number: i32) -> Option<&'a [u8]>;

    /// The charstring of the glyph Standard Encoding code `code` names,
    /// for a `seac` component.
    ///
    /// # Errors
    ///
    /// When the code is outside the encoding or the font has no such
    /// glyph.
    fn standard_glyph(&self, code: i32) -> Result<&'a [u8], CharstringError>;

    /// The hinting entries of the glyph's Private dictionary.
    fn private(&self) -> &PrivateDict;

    /// The depth of the operand stack (a CFF2 font's `maxstack`).
    fn stack_size(&self) -> usize {
        STACK_SIZE
    }

    /// The weights `blend` gives the default and the regions of item
    /// variation data `vsindex` at the font's instance, in 16.16
    /// (`cff_blend_build_vector`).
    ///
    /// # Errors
    ///
    /// When the font has no variations or no such data.
    fn blend_vector(&self, vsindex: u32) -> Result<Vec<i32>, CharstringError> {
        let _ = vsindex;
        Err(CharstringError::Malformed(
            "blend in a font without variations",
        ))
    }
}
