//! Apple Advanced Typography, as HarfBuzz shapes with it (and so as
//! matplotlib lays out text in the fonts of macOS): glyph metamorphosis
//! by `morx` (or `mort`) in place of `GSUB`, and kerning by `kerx` and by
//! the Apple form of `kern`.
//!
//! * [`lookup`] and [`state`]: the lookup, class and state tables the AAT
//!   tables share, and the machine that runs state tables.
//! * [`buffer`]: the glyph buffer as subtables change it.
//! * [`morx`]: the metamorphosis chains and their five subtable types.
//! * [`kern`]: `kern` (both forms) and `kerx`.
//! * [`features`]: the user's OpenType features as AAT feature settings.
//! * [`trak`]: tracking, which HarfBuzz applies to fonts with a `STAT`
//!   table (whatever else they substitute and position with).

mod buffer;
mod features;
mod kern;
mod lookup;
mod morx;
mod state;
mod trak;

pub(crate) use buffer::remove_deleted_glyphs;
pub(crate) use features::UserFeature;
pub(crate) use kern::{KernContext, KernTable};
pub(crate) use morx::Morx;
pub(crate) use trak::Trak;

use crate::face::Face;
use crate::layout::buffer::Buffer;
use crate::types::{Direction, Tag};
use buffer::AatBuffer;

/// Substitutes the glyphs of `buffer` with `morx`, the table of `face`
/// (HarfBuzz's `hb_aat_layout_substitute`): the chains' flags come from
/// the user's `features` and the text's `language` (lower-case BCP 47).
/// Deleted glyphs are left in place, for [`remove_deleted_glyphs`].
pub(crate) fn substitute(
    face: &Face<'_>,
    morx: &Morx<'_>,
    buffer: &mut Buffer,
    direction: Direction,
    features: &[UserFeature],
    language: Option<&str>,
) {
    let ltag = face.table(Tag::new(b"ltag"));
    let aat_features = features::aat_features(face.table(Tag::new(b"feat")), features);
    let chain_flags = features::compile(&aat_features, |active| {
        morx.chain_flags(active, language, ltag)
    });
    let mut buffer = AatBuffer::new(buffer, face.gdef());
    morx.apply(&mut buffer, &chain_flags, face.num_glyphs(), direction);
}
