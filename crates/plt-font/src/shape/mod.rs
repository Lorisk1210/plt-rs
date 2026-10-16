//! Text shaping: characters to positioned glyphs.
//!
//! This reproduces the pipeline matplotlib uses (libraqm + HarfBuzz) for
//! left-to-right text: the text is split into runs of one script (libraqm's
//! rules for Common/Inherited characters); each run is normalized like
//! HarfBuzz does (decomposing characters the font lacks, recomposing base +
//! mark sequences, substituting missing spaces), mapped through the
//! character map, substituted with the `GSUB` lookups of HarfBuzz's default
//! features (`ccmp`, `locl`, `rlig`, `calt`, `clig`, `liga`, ...) and
//! positioned with `GPOS` (`kern`, `mark`, `mkmk`, ...) or, for fonts without
//! `GPOS` kerning, the legacy `kern` table. Mark glyphs get zero advance;
//! fonts without `GPOS` get HarfBuzz's fallback mark stacking.

mod chain;
mod fallback;
mod normalize;
mod script;
mod spaces;
mod unicode_data;

pub use chain::{FontChain, LaidOutGlyph, TextLayout};
pub use script::Script;

use crate::face::Face;
use crate::layout::apply::{Applier, TableKind};
use crate::layout::buffer::{
    AttachKind, Buffer, CharProps, GlyphInfo, GlyphPos, gdef_props, props,
};
use crate::layout::plan::{Feature, Plan, plan};
use crate::scale::{Scaler, Size};
use crate::types::{GlyphId, Tag};
use normalize::{ClusterChar, combining_class, normalize};
use script::{is_default_ignorable, mark_kind, resolve_scripts};
use spaces::SpaceWidth;

/// Longest chain of attached marks followed (HarfBuzz's nesting limit).
const MAX_ATTACHMENT_DEPTH: u8 = 64;

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
    scaler: Scaler,
) -> Vec<ShapedGlyph> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut glyphs = Vec::with_capacity(chars.len());
    for (start, end, script) in script_runs(&chars, options.script) {
        glyphs.extend(shape_run(face, &chars[start..end], script, options, scaler));
    }
    glyphs
}

/// Splits characters into `(start, end, script)` runs of one script.
pub(crate) fn script_runs(
    chars: &[(usize, char)],
    forced: Option<Script>,
) -> Vec<(usize, usize, Script)> {
    if chars.is_empty() {
        return Vec::new();
    }
    if let Some(script) = forced {
        return vec![(0, chars.len(), script)];
    }
    let text: Vec<char> = chars.iter().map(|&(_, c)| c).collect();
    let scripts = resolve_scripts(&text);
    let mut runs: Vec<(usize, usize, Script)> = Vec::new();
    for (index, script) in scripts.into_iter().enumerate() {
        match runs.last_mut() {
            Some(run) if run.2 == script => run.1 = index + 1,
            _ => runs.push((index, index + 1, script)),
        }
    }
    runs
}

fn char_props(c: char) -> CharProps {
    let mark = mark_kind(c);
    CharProps {
        nonspacing_mark: mark == Some(true),
        default_ignorable: is_default_ignorable(c),
        mark: mark.is_some(),
        combining_class: combining_class(c),
    }
}

/// Characters continuing the grapheme of the preceding one (HarfBuzz's
/// `continuation` property): marks, emoji modifiers and tag characters.
fn is_continuation(c: char) -> bool {
    mark_kind(c).is_some() || matches!(u32::from(c), 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F)
}

/// Gives continuation characters the cluster of their base (HarfBuzz's
/// grapheme-level `form_clusters`).
fn form_clusters(chars: &[(usize, char)]) -> Vec<ClusterChar> {
    let mut out: Vec<ClusterChar> = Vec::with_capacity(chars.len());
    for &(offset, c) in chars {
        let cluster = match out.last() {
            Some(previous) if is_continuation(c) => previous.cluster,
            _ => offset,
        };
        out.push(ClusterChar::new(c, cluster));
    }
    out
}

/// Normalizes the characters for `face`, maps them to glyphs and assigns
/// glyph classes (from `GDEF`, or synthesized from Unicode like HarfBuzz).
/// Also returns the clusters of spaces drawn with the fallback space glyph.
fn initial_buffer(face: &Face<'_>, chars: &[(usize, char)]) -> (Buffer, Vec<(usize, SpaceWidth)>) {
    let has_glyph = |c: char| face.glyph_index(c).is_some();
    let normalized = normalize(&form_clusters(chars), &has_glyph);
    let spaces = normalized
        .iter()
        .filter_map(|item| Some((item.cluster, item.space?)))
        .collect();
    let gdef = face.gdef();
    let infos = normalized
        .iter()
        .map(|item| {
            let glyph = face.glyph_index(item.c).unwrap_or(GlyphId::NOTDEF);
            let chars = char_props(item.c);
            let mut info = GlyphInfo::new(glyph, item.cluster, chars);
            info.props = if gdef.has_glyph_classes() {
                gdef_props(gdef, glyph)
            } else if chars.nonspacing_mark && !chars.default_ignorable {
                props::MARK
            } else {
                props::BASE
            };
            info
        })
        .collect();
    (Buffer::new(infos), spaces)
}

fn shape_run(
    face: &Face<'_>,
    chars: &[(usize, char)],
    script: Script,
    options: &ShapeOptions,
    scaler: Scaler,
) -> Vec<ShapedGlyph> {
    let (mut buffer, spaces) = initial_buffer(face, chars);
    let plan = plan(
        face,
        script.opentype_tag(),
        options.language,
        &options.features,
    );
    if let Some(table) = face.gsub() {
        let mut applier = Applier::new(face, *table, TableKind::Gsub, &scaler, &mut buffer);
        for &lookup in &plan.gsub {
            applier.apply_lookup(lookup);
        }
    }
    position(face, &plan, &scaler, &spaces, &mut buffer);
    hide_default_ignorables(face, &mut buffer);
    buffer
        .info
        .iter()
        .zip(&buffer.pos)
        .map(|(info, pos)| ShapedGlyph {
            glyph_id: info.glyph,
            cluster: info.cluster,
            x_advance: pos.x_advance,
            y_advance: pos.y_advance,
            x_offset: pos.x_offset,
            y_offset: pos.y_offset,
        })
        .collect()
}

/// HarfBuzz's `hb_ot_position`: default advances, `GPOS` or legacy kerning,
/// late zeroing of mark advances and attachment offset propagation.
fn position(
    face: &Face<'_>,
    plan: &Plan,
    scaler: &Scaler,
    spaces: &[(usize, SpaceWidth)],
    buffer: &mut Buffer,
) {
    for (info, pos) in buffer.info.iter().zip(buffer.pos.iter_mut()) {
        pos.x_advance = scaler.advance(face, info.glyph);
    }
    let space_glyph = face.glyph_index(' ');
    for &(cluster, width) in spaces {
        let found = buffer
            .info
            .iter()
            .position(|info| info.cluster == cluster && Some(info.glyph) == space_glyph);
        if let Some(index) = found {
            buffer.pos[index].x_advance =
                spaces::advance(width, face, scaler, buffer.pos[index].x_advance);
        }
    }
    if let Some(table) = face.gpos().filter(|_| plan.apply_gpos) {
        let mut applier = Applier::new(face, *table, TableKind::Gpos, scaler, buffer);
        for &lookup in &plan.gpos {
            applier.apply_lookup(lookup);
        }
    }
    if plan.apply_kern_table {
        apply_kern_table(face, scaler, buffer);
    }
    let adjust_offsets = !plan.apply_gpos;
    for (info, pos) in buffer.info.iter().zip(buffer.pos.iter_mut()) {
        if info.is_mark() {
            if adjust_offsets {
                pos.x_offset -= pos.x_advance;
                pos.y_offset -= pos.y_advance;
            }
            pos.x_advance = 0;
            pos.y_advance = 0;
        }
        if info.chars.default_ignorable {
            *pos = GlyphPos::default();
        }
    }
    for index in 0..buffer.len() {
        propagate_attachment(buffer, index, MAX_ATTACHMENT_DEPTH);
    }
    if !plan.apply_gpos {
        fallback::position_marks(face, scaler, buffer);
    }
}

/// HarfBuzz's kern-table machine: kerns each glyph with the next non-mark
/// glyph, splitting the value between the two advances.
fn apply_kern_table(face: &Face<'_>, scaler: &Scaler, buffer: &mut Buffer) {
    let skippable = |info: &GlyphInfo| info.is_mark() || info.chars.default_ignorable;
    for subtable in face
        .kern_table()
        .subtables()
        .iter()
        .filter(|s| s.is_horizontal())
    {
        let mut i = 0;
        while i < buffer.len() {
            let Some(j) = (i + 1..buffer.len()).find(|&j| !skippable(&buffer.info[j])) else {
                break;
            };
            if let Some(value) = subtable
                .get(buffer.info[i].glyph, buffer.info[j].glyph)
                .filter(|&v| v != 0)
            {
                let kern = scaler.x(i32::from(value));
                if subtable.is_cross_stream() {
                    buffer.pos[j].y_offset = kern;
                } else {
                    let first = kern >> 1;
                    let second = kern - first;
                    buffer.pos[i].x_advance += first;
                    buffer.pos[j].x_advance += second;
                    buffer.pos[j].x_offset += second;
                }
            }
            i = j;
        }
    }
}

/// Adds the offsets of the glyph a mark is attached to, minus the advances
/// in between (HarfBuzz's `propagate_attachment_offsets`).
fn propagate_attachment(buffer: &mut Buffer, index: usize, depth: u8) {
    let pos = buffer.pos[index];
    if pos.attach_chain == 0 || depth == 0 {
        return;
    }
    buffer.pos[index].attach_chain = 0;
    let Some(target) = index
        .checked_add_signed(pos.attach_chain as isize)
        .filter(|&t| t < buffer.len())
    else {
        return;
    };
    propagate_attachment(buffer, target, depth - 1);
    if pos.attach_kind == AttachKind::Mark && target < index {
        let base = buffer.pos[target];
        let (dx, dy) = buffer.pos[target..index]
            .iter()
            .fold((0, 0), |(x, y), p| (x + p.x_advance, y + p.y_advance));
        let current = &mut buffer.pos[index];
        current.x_offset += base.x_offset - dx;
        current.y_offset += base.y_offset - dy;
    }
}

/// Replaces default-ignorable characters by the (zero-width) space glyph,
/// or removes them when the font has no space.
fn hide_default_ignorables(face: &Face<'_>, buffer: &mut Buffer) {
    if !buffer.info.iter().any(|info| info.chars.default_ignorable) {
        return;
    }
    match face.glyph_index(' ') {
        Some(space) => {
            for info in buffer
                .info
                .iter_mut()
                .filter(|info| info.chars.default_ignorable)
            {
                info.glyph = space;
            }
        }
        None => {
            for index in (0..buffer.len()).rev() {
                if buffer.info[index].chars.default_ignorable {
                    buffer.remove(index);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_follow_scripts() {
        let chars: Vec<(usize, char)> = "Ab αβ".char_indices().collect();
        let runs = script_runs(&chars, None);
        assert_eq!(runs.len(), 2);
        assert_eq!((runs[0].0, runs[0].1), (0, 3));
        assert_eq!(runs[1].2, Script::of('α'));
        assert_eq!(script_runs(&chars, Some(Script::COMMON)).len(), 1);
        assert_eq!(script_runs(&[], None), Vec::new());
    }

    #[test]
    fn continuation_characters() {
        assert!(is_continuation('\u{301}'));
        assert!(is_continuation('\u{1F3FB}'));
        assert!(!is_continuation('a'));
        let props = char_props('\u{200D}');
        assert!(props.default_ignorable && !props.nonspacing_mark);
        let acute = char_props('\u{301}');
        assert!(acute.nonspacing_mark && acute.mark && acute.combining_class == 230);
        let clusters = form_clusters(&[(0, 'a'), (1, '\u{301}'), (3, 'b')]);
        assert_eq!(
            clusters.iter().map(|c| c.cluster).collect::<Vec<_>>(),
            vec![0, 0, 3]
        );
    }
}
