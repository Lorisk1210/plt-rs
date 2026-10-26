//! FreeType's Adobe charstring engine (`psintrp.c`, `pshints.c`,
//! `psft.c`) in Type 1 mode, which FreeType 2.14 loads Type 1 glyphs
//! with: the interpreter, its operand stack, the glyph path and the
//! outline builder, the hint maps and the alignment zones (light hinting:
//! the engine only moves vertical coordinates).
//!
//! The engine runs a Type 1 charstring twice: a first pass collects the
//! stem hints and skips drawing, the second draws. Type 2 operators are
//! accepted too, as FreeType's shared interpreter does.

mod blues;
mod buffer;
mod builder;
mod escape;
mod hintmap;
mod hints;
mod interp;
mod path;
mod stack;

pub(crate) use blues::PrivateDict;
pub(crate) use interp::{GlyphSet, Rendering, decode};
