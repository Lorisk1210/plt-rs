//! Text shaping: characters to positioned glyphs.
//!
//! This reproduces the pipeline matplotlib uses (libraqm + `SheenBidi` +
//! HarfBuzz). Text is resolved with the Unicode Bidirectional Algorithm
//! and split into runs of one level, script (libraqm's rules for Common
//! and Inherited characters) and font, ordered visually. Each run is
//! shaped like HarfBuzz does: switched to its script's native direction,
//! mirrored when right to left, normalized (decomposing characters the
//! font lacks, reordering and recomposing marks), mapped through the
//! character map, given per-glyph feature masks by the script shaper
//! (Arabic joining), substituted with the `GSUB` lookups of HarfBuzz's
//! features stage by stage (with the Arabic presentation-form fallback for
//! fonts without the features) and positioned with `GPOS` (`kern`,
//! `curs`, `mark`, `mkmk`, ...) or the legacy `kern` table. Mark glyphs get
//! zero advance; fonts without `GPOS` get HarfBuzz's fallback mark
//! stacking. Right-to-left runs come out in visual order.

mod arabic;
mod bidi;
pub(crate) mod category;
mod chain;
mod fallback;
mod hebrew;
mod itemize;
mod normalize;
mod plan;
mod position;
mod run;
mod script;
mod spaces;
mod ucd_tables;
mod unicode;
mod unicode_data;

pub use chain::{FontChain, LaidOutGlyph, TextLayout};
pub use script::Script;

use crate::face::Face;
use crate::layout::feature::Feature;
use crate::scale::{Scaler, Size};
use crate::types::{Direction, GlyphId, Tag};
use itemize::{itemize, scripts};
use plan::ShapePlan;
use run::shape_run;

/// One positioned glyph. Units are font units for [`shape`] and
/// [`shape_with`], 26.6 pixels for [`shape_at_size`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShapedGlyph {
    /// The glyph.
    pub glyph_id: GlyphId,
    /// Byte offset in the source text of the cluster this glyph renders.
    pub cluster: usize,
    /// Horizontal pen advance after drawing this glyph.
    pub x_advance: i32,
    /// Vertical pen advance (zero for horizontal text).
    pub y_advance: i32,
    /// Horizontal displacement of the glyph from the pen position.
    pub x_offset: i32,
    /// Vertical displacement of the glyph from the pen position.
    pub y_offset: i32,
}

/// Shaping settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShapeOptions {
    /// Feature settings applied on top of HarfBuzz's defaults (e.g.
    /// `-kern` to disable kerning, `smcp` to enable small capitals).
    pub features: Vec<Feature>,
    /// OpenType language system tag (e.g. `TRK `); `None` uses the
    /// script's default language system.
    pub language: Option<Tag>,
    /// Forces one script for the whole text instead of itemizing it.
    pub script: Option<Script>,
    /// Paragraph direction; `None` takes it from the first strong
    /// character (left to right when there is none), like matplotlib.
    pub direction: Option<Direction>,
}

/// Shapes `text` with default settings; positions are in font units.
pub fn shape(face: &Face<'_>, text: &str) -> Vec<ShapedGlyph> {
    shape_with(face, text, &ShapeOptions::default())
}

/// Shapes `text`; positions are in font units.
pub fn shape_with(face: &Face<'_>, text: &str, options: &ShapeOptions) -> Vec<ShapedGlyph> {
    shape_scaled(face, text, options, Scaler::Units)
}

/// Shapes `text` at `size` with HarfBuzz's pixel arithmetic; positions are
/// in 26.6 pixels and match matplotlib's (`FT2Font.set_text`) exactly.
pub fn shape_at_size(
    face: &Face<'_>,
    text: &str,
    size: &Size,
    options: &ShapeOptions,
) -> Vec<ShapedGlyph> {
    shape_scaled(face, text, options, Scaler::Pixels(*size))
}

fn shape_scaled(
    face: &Face<'_>,
    text: &str,
    options: &ShapeOptions,
    scaler: Scaler<'_>,
) -> Vec<ShapedGlyph> {
    let text: Vec<(usize, char)> = text.char_indices().collect();
    let faces = std::slice::from_ref(face);
    shape_text(&text, faces, &vec![0; text.len()], options, &|_| scaler)
        .into_iter()
        .map(|(_, glyph)| glyph)
        .collect()
}

/// Shapes `text` as a single segment the way FreeType's autohinter calls
/// HarfBuzz (`hb_buffer_guess_segment_properties` then `hb_shape`): the
/// script is that of the first character with a script of its own (none
/// for text of Common and Inherited characters only), the direction the
/// script's, the language system the default one, and no
/// beginning-of-text dotted circle is inserted. `features` apply on top
/// of HarfBuzz's defaults; positions are in font units, glyphs in visual
/// order.
pub(crate) fn shape_segment(face: &Face<'_>, text: &str, features: &[Feature]) -> Vec<ShapedGlyph> {
    let text: Vec<(usize, char)> = text.char_indices().collect();
    let script = text
        .iter()
        .map(|&(_, c)| Script::of(c))
        .find(|script| script.opentype_tag().is_some())
        .unwrap_or(Script::COMMON);
    let direction = script
        .horizontal_direction()
        .unwrap_or(Direction::LeftToRight);
    let plan = ShapePlan::new(face, script, direction, None, features);
    shape_run(face, &text, 0..text.len(), &plan, Scaler::Units, false)
}

/// Shapes `text` with `faces[assignment[i]]` for character `i`; returns
/// `(face index, glyph)` in visual order. `scaler` gives each face's
/// scaling.
fn shape_text<'h>(
    text: &[(usize, char)],
    faces: &[Face<'_>],
    assignment: &[usize],
    options: &ShapeOptions,
    scaler: &dyn Fn(usize) -> Scaler<'h>,
) -> Vec<(usize, ShapedGlyph)> {
    let chars: Vec<char> = text.iter().map(|&(_, c)| c).collect();
    let scripts = scripts(&chars, options.script);
    let mut glyphs = Vec::with_capacity(text.len());
    for run in itemize(&chars, &scripts, assignment, options.direction) {
        let face = &faces[run.face];
        let plan = ShapePlan::new(
            face,
            run.script,
            run.direction,
            options.language,
            &options.features,
        );
        let shaped = shape_run(
            face,
            text,
            run.start..run.end,
            &plan,
            scaler(run.face),
            run.start == 0,
        );
        glyphs.extend(shaped.into_iter().map(|glyph| (run.face, glyph)));
    }
    glyphs
}
