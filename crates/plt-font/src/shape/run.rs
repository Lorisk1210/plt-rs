//! Shaping one run (HarfBuzz's `hb_ot_shape_internal`): Unicode
//! properties, dotted circles, clusters, the script's native direction,
//! mirroring, normalization, shaper masks, glyph mapping, substitution,
//! positioning and the final visual order.

use std::ops::Range;

use crate::face::Face;
use crate::layout::buffer::{Buffer, GlyphInfo, gdef_props, props};
use crate::scale::Scaler;
use crate::shape::ShapedGlyph;
use crate::shape::bidi::mirror;
use crate::shape::category::GeneralCategory;
use crate::shape::fallback::recategorize_mark;
use crate::shape::normalize::{ClusterChar, normalize};
use crate::shape::plan::{ShapePlan, Shaper};
use crate::shape::position::position;
use crate::shape::spaces::SpaceWidth;
use crate::shape::unicode::continuations;
use crate::types::{Direction, GlyphId};

/// Characters of context HarfBuzz keeps on each side of a run.
const CONTEXT_LENGTH: usize = 5;
/// DOTTED CIRCLE, the base shown for a mark that starts the text.
const DOTTED_CIRCLE: char = '\u{25CC}';

/// Groups `items` into graphemes (each starting where `continues` is
/// false) and reverses their order, keeping each grapheme's characters in
/// order (HarfBuzz's `_hb_ot_layout_reverse_graphemes`).
fn reverse_graphemes(items: &mut Vec<ClusterChar>, continues: &[bool]) {
    let mut graphemes: Vec<&[ClusterChar]> = Vec::new();
    let mut start = 0;
    for index in 1..=items.len() {
        if index == items.len() || !continues[index] {
            graphemes.push(&items[start..index]);
            start = index;
        }
    }
    *items = graphemes.into_iter().rev().flatten().copied().collect();
}

/// HarfBuzz's `hb_ensure_native_direction`: the direction to shape in.
/// Runs of digits (or regional indicators) without letters keep their
/// left-to-right direction even in right-to-left scripts.
fn native_direction(plan: &ShapePlan, items: &[ClusterChar]) -> Direction {
    let requested = plan.direction;
    let mut native = plan.script.horizontal_direction();
    if native == Some(Direction::RightToLeft) && requested == Direction::LeftToRight {
        let mut found_number = false;
        let mut found_letter = false;
        for item in items {
            let category = item.props.category;
            if category.is_letter() {
                found_letter = true;
                break;
            }
            found_number |= category == GeneralCategory::DecimalNumber
                || matches!(item.c, '\u{1F1E6}'..='\u{1F1FF}');
        }
        if found_number && !found_letter {
            native = Some(Direction::LeftToRight);
        }
    }
    native.unwrap_or(requested)
}

/// The characters of `text[range]` after HarfBuzz's pre-shaping steps, in
/// the order they are shaped, plus that direction.
fn prepare(
    face: &Face<'_>,
    text: &[(usize, char)],
    range: Range<usize>,
    plan: &ShapePlan,
) -> (Vec<ClusterChar>, Direction) {
    let mask = plan.map.global_mask();
    let mut items: Vec<ClusterChar> = text[range.clone()]
        .iter()
        .map(|&(offset, c)| ClusterChar::new(c, offset, mask))
        .collect();
    if let Some(first) = items
        .first()
        .copied()
        .filter(|first| range.start == 0 && first.props.is_mark())
        .filter(|_| face.glyph_index(DOTTED_CIRCLE).is_some())
    {
        items.insert(
            0,
            ClusterChar::new(DOTTED_CIRCLE, first.cluster, first.mask),
        );
    }
    let chars: Vec<char> = items.iter().map(|item| item.c).collect();
    let continues = continuations(&chars);
    for index in 1..items.len() {
        if continues[index] {
            items[index].cluster = items[index - 1].cluster;
        }
    }
    let direction = native_direction(plan, &items);
    if direction != plan.direction {
        reverse_graphemes(&mut items, &continues);
    }
    if plan.direction == Direction::RightToLeft {
        for item in &mut items {
            match mirror(item.c).filter(|&m| face.glyph_index(m).is_some()) {
                Some(mirrored) => item.c = mirrored,
                None => item.mask |= plan.rtlm_mask,
            }
        }
    }
    let has_glyph = |c: char| face.glyph_index(c).is_some();
    let mut items = normalize(&items, &has_glyph, plan.normalize_hooks);
    if let Shaper::Arabic(arabic) = &plan.shaper {
        let pre: Vec<char> = text[..range.start]
            .iter()
            .rev()
            .take(CONTEXT_LENGTH)
            .map(|&(_, c)| c)
            .collect();
        let post: Vec<char> = text[range.end..]
            .iter()
            .take(CONTEXT_LENGTH)
            .map(|&(_, c)| c)
            .collect();
        arabic.setup_masks(&mut items, &pre, &post);
    }
    if plan.fallback_mark_positioning {
        for item in &mut items {
            recategorize_mark(item.c, &mut item.props);
        }
    }
    (items, direction)
}

/// Maps characters to glyphs and assigns glyph classes (from `GDEF`, or
/// synthesized from Unicode like HarfBuzz).
fn glyph_buffer(face: &Face<'_>, items: &[ClusterChar]) -> Buffer {
    let gdef = face.gdef();
    let infos = items
        .iter()
        .map(|item| {
            let glyph = face.glyph_index(item.c).unwrap_or(GlyphId::NOTDEF);
            let mut info = GlyphInfo::new(glyph, item.cluster, item.props, item.mask);
            info.props = if gdef.has_glyph_classes() {
                gdef_props(gdef, glyph)
            } else if item.props.is_nonspacing_mark() && item.props.ignorable.is_none() {
                props::MARK
            } else {
                props::BASE
            };
            info
        })
        .collect();
    Buffer::new(infos)
}

/// Replaces default-ignorable characters by the (zero-width) space glyph,
/// or removes them when the font has no space.
fn hide_default_ignorables(face: &Face<'_>, buffer: &mut Buffer) {
    match face.glyph_index(' ') {
        Some(space) => {
            for info in buffer
                .info
                .iter_mut()
                .filter(|info| info.is_default_ignorable())
            {
                info.glyph = space;
            }
        }
        None => {
            for index in (0..buffer.len()).rev() {
                if buffer.info[index].is_default_ignorable() {
                    buffer.remove(index);
                }
            }
        }
    }
}

/// Shapes `text[range]` (the rest of `text` is context) with `plan`;
/// glyphs come out in visual order.
pub(crate) fn shape_run(
    face: &Face<'_>,
    text: &[(usize, char)],
    range: Range<usize>,
    plan: &ShapePlan,
    scaler: Scaler<'_>,
) -> Vec<ShapedGlyph> {
    let (items, direction) = prepare(face, text, range, plan);
    let spaces: Vec<(usize, SpaceWidth)> = items
        .iter()
        .filter_map(|item| Some((item.cluster, item.space?)))
        .collect();
    let mut buffer = glyph_buffer(face, &items);
    plan.substitute(face, &scaler, direction, &mut buffer);
    position(face, plan, direction, &scaler, &spaces, &mut buffer);
    if !direction.is_forward() {
        buffer.reverse();
    }
    hide_default_ignorables(face, &mut buffer);
    plan.postprocess(face, &scaler, direction, &mut buffer);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn items(text: &str) -> Vec<ClusterChar> {
        text.char_indices()
            .map(|(cluster, c)| ClusterChar::new(c, cluster, 1))
            .collect()
    }

    #[test]
    fn graphemes_reverse_as_units() {
        let mut text = items("ab\u{301}c");
        let continues = [false, false, true, false];
        reverse_graphemes(&mut text, &continues);
        let order: String = text.iter().map(|item| item.c).collect();
        assert_eq!(order, "cb\u{301}a");
        let mut empty = Vec::new();
        reverse_graphemes(&mut empty, &[]);
        assert_eq!(empty, vec![]);
    }
}
