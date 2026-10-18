//! HarfBuzz's fallback mark positioning for fonts without `GPOS`: marks are
//! stacked above or below their base by combining class, using glyph
//! extents (`hb-ot-shape-fallback.cc`).

use crate::face::Face;
use crate::layout::buffer::{Buffer, CharProps};
use crate::scale::Scaler;
use crate::types::{Direction, GlyphId};

mod class {
    pub(super) const ATTACHED_BELOW_LEFT: u8 = 200;
    pub(super) const ATTACHED_BELOW: u8 = 202;
    pub(super) const ATTACHED_ABOVE: u8 = 214;
    pub(super) const ATTACHED_ABOVE_RIGHT: u8 = 216;
    pub(super) const BELOW_LEFT: u8 = 218;
    pub(super) const BELOW: u8 = 220;
    pub(super) const BELOW_RIGHT: u8 = 222;
    pub(super) const ABOVE_LEFT: u8 = 228;
    pub(super) const ABOVE: u8 = 230;
    pub(super) const ABOVE_RIGHT: u8 = 232;
    pub(super) const DOUBLE_BELOW: u8 = 233;
    pub(super) const DOUBLE_ABOVE: u8 = 234;
}

/// HarfBuzz's `recategorize_combining_class`: maps the (modified) classes
/// of Hebrew, Arabic, Syriac, Thai, Lao and Tibetan marks to positional
/// classes.
fn recategorize(c: char, combining_class: u8) -> u8 {
    if combining_class >= 200 {
        return combining_class;
    }
    let combining_class = match (c, combining_class) {
        ('\u{0E31}' | '\u{0E34}'..='\u{0E37}' | '\u{0E47}' | '\u{0E4C}'..='\u{0E4E}', 0) => {
            class::ABOVE_RIGHT
        }
        ('\u{0EB1}' | '\u{0EB4}'..='\u{0EB7}' | '\u{0EBB}' | '\u{0ECC}' | '\u{0ECD}', 0) => {
            class::ABOVE
        }
        ('\u{0EBC}', 0) => class::BELOW,
        ('\u{0E3A}', 1..) => class::BELOW_RIGHT,
        _ => combining_class,
    };
    match combining_class {
        // Hebrew points below and meteg; Arabic kasratan and kasra; Lao
        // and Tibetan vowels below.
        15..=25 | 30 | 33 | 118 | 129 | 131 => class::BELOW,
        // Hebrew rafe.
        13 => class::ATTACHED_ABOVE,
        // Hebrew shin dot; Thai mai.
        10 | 107 => class::ABOVE_RIGHT,
        // Hebrew sin dot and holam.
        11 | 14 => class::ABOVE_LEFT,
        // Hebrew point varika; Arabic and Syriac marks above; Lao mai,
        // Tibetan sign i.
        26..=29 | 31 | 32 | 34..=36 | 122 | 132 => class::ABOVE,
        // Thai sara u.
        3 => class::BELOW_RIGHT,
        other => other,
    }
}

/// Positional classes for nonspacing marks (HarfBuzz's
/// `_hb_ot_shape_fallback_mark_position_recategorize_marks`).
pub(crate) fn recategorize_mark(c: char, props: &mut CharProps) {
    if props.is_nonspacing_mark() {
        props.combining_class = recategorize(c, props.combining_class);
    }
}

/// Glyph ink extents in HarfBuzz's convention (`height` is negative for
/// ink above the baseline-relative bearing).
#[derive(Clone, Copy, Debug, Default)]
struct Extents {
    x_bearing: i32,
    y_bearing: i32,
    width: i32,
    height: i32,
}

/// Ink extents of `glyph` as HarfBuzz's FreeType integration reports them
/// (the control box, grid-fitted when hinting), `None` when the glyph
/// cannot be loaded.
fn extents(face: &Face<'_>, scaler: &Scaler<'_>, glyph: GlyphId) -> Option<Extents> {
    let rect = scaler.extents(face, glyph).ok()?;
    Some(rect.map_or_else(Extents::default, |r| Extents {
        x_bearing: r.x_min,
        y_bearing: r.y_max,
        width: r.x_max - r.x_min,
        height: r.y_min - r.y_max,
    }))
}

/// Vertical gap between stacked marks: 1/16 em.
fn y_gap(face: &Face<'_>, scaler: &Scaler<'_>) -> i32 {
    (scaler.em(face).1 / 16) as i32
}

/// What the fallback positioner needs besides the buffer.
struct Context<'f, 'a> {
    face: &'f Face<'a>,
    scaler: &'f Scaler<'f>,
    direction: Direction,
    adjust_offsets: bool,
}

/// Positions mark `index` relative to `base` (HarfBuzz's `position_mark`),
/// updating `base` for further stacking.
fn position_mark(
    context: &Context<'_, '_>,
    buffer: &mut Buffer,
    base: &mut Extents,
    index: usize,
    combining_class: u8,
) {
    let Some(mark) = extents(context.face, context.scaler, buffer.info[index].glyph) else {
        return;
    };
    let y_gap = y_gap(context.face, context.scaler);
    let pos = &mut buffer.pos[index];
    pos.x_offset = 0;
    pos.y_offset = 0;
    let centered = base.x_bearing + (base.width - mark.width) / 2 - mark.x_bearing;
    pos.x_offset += match (combining_class, context.direction) {
        (class::DOUBLE_BELOW | class::DOUBLE_ABOVE, Direction::LeftToRight) => {
            base.x_bearing + base.width - mark.width / 2 - mark.x_bearing
        }
        (class::DOUBLE_BELOW | class::DOUBLE_ABOVE, Direction::RightToLeft) => {
            base.x_bearing - mark.width / 2 - mark.x_bearing
        }
        (class::ATTACHED_BELOW_LEFT | class::BELOW_LEFT | class::ABOVE_LEFT, _) => {
            base.x_bearing - mark.x_bearing
        }
        (class::ATTACHED_ABOVE_RIGHT | class::BELOW_RIGHT | class::ABOVE_RIGHT, _) => {
            base.x_bearing + base.width - mark.width - mark.x_bearing
        }
        _ => centered,
    };
    match combining_class {
        class::DOUBLE_BELOW
        | class::BELOW_LEFT
        | class::BELOW
        | class::BELOW_RIGHT
        | class::ATTACHED_BELOW_LEFT
        | class::ATTACHED_BELOW => {
            if !matches!(
                combining_class,
                class::ATTACHED_BELOW_LEFT | class::ATTACHED_BELOW
            ) {
                base.height -= y_gap;
            }
            pos.y_offset = base.y_bearing + base.height - mark.y_bearing;
            // Never shift "below" marks up.
            if (y_gap > 0) == (pos.y_offset > 0) {
                base.height -= pos.y_offset;
                pos.y_offset = 0;
            }
            base.height += mark.height;
        }
        class::DOUBLE_ABOVE
        | class::ABOVE_LEFT
        | class::ABOVE
        | class::ABOVE_RIGHT
        | class::ATTACHED_ABOVE
        | class::ATTACHED_ABOVE_RIGHT => {
            if !matches!(
                combining_class,
                class::ATTACHED_ABOVE | class::ATTACHED_ABOVE_RIGHT
            ) {
                base.y_bearing += y_gap;
                base.height -= y_gap;
            }
            pos.y_offset = base.y_bearing - (mark.y_bearing + mark.height);
            // Don't shift "above" marks down too much.
            if (y_gap > 0) != (pos.y_offset > 0) {
                let correction = -pos.y_offset / 2;
                base.y_bearing += correction;
                base.height -= correction;
                pos.y_offset += correction;
            }
            base.y_bearing -= mark.height;
            base.height += mark.height;
        }
        _ => {}
    }
}

/// Zeroes the advances of the nonspacing marks in `start..end` (when the
/// base has no extents).
fn zero_mark_advances(context: &Context<'_, '_>, buffer: &mut Buffer, start: usize, end: usize) {
    for (info, pos) in buffer.info[start..end]
        .iter()
        .zip(&mut buffer.pos[start..end])
    {
        if info.chars.is_nonspacing_mark() {
            if context.adjust_offsets {
                pos.x_offset -= pos.x_advance;
                pos.y_offset -= pos.y_advance;
            }
            pos.x_advance = 0;
            pos.y_advance = 0;
        }
    }
}

/// HarfBuzz's `position_around_base`.
fn position_around_base(context: &Context<'_, '_>, buffer: &mut Buffer, base: usize, end: usize) {
    let base_info = buffer.info[base];
    let Some(mut base_extents) = extents(context.face, context.scaler, base_info.glyph) else {
        zero_mark_advances(context, buffer, base + 1, end);
        return;
    };
    base_extents.y_bearing += buffer.pos[base].y_offset;
    base_extents.x_bearing = 0;
    base_extents.width = context.scaler.advance(context.face, base_info.glyph);
    let components = i32::from(base_info.lig_num_comps());
    let forward = context.direction.is_forward();
    let (mut x_offset, mut y_offset) = if forward {
        (-buffer.pos[base].x_advance, -buffer.pos[base].y_advance)
    } else {
        (0, 0)
    };
    let mut component_extents = base_extents;
    let mut last_component: Option<i32> = None;
    let mut last_class: Option<u8> = None;
    let mut cluster_extents = base_extents;
    for i in base + 1..end {
        let combining_class = buffer.info[i].chars.combining_class;
        if combining_class == 0 {
            let (dx, dy) = (buffer.pos[i].x_advance, buffer.pos[i].y_advance);
            if forward {
                x_offset -= dx;
                y_offset -= dy;
            } else {
                x_offset += dx;
                y_offset += dy;
            }
            continue;
        }
        if components > 1 {
            let info = buffer.info[i];
            let mut component = i32::from(info.lig_comp()) - 1;
            if base_info.lig_id == 0 || base_info.lig_id != info.lig_id || component >= components {
                component = components - 1;
            }
            if last_component != Some(component) {
                last_component = Some(component);
                last_class = None;
                component_extents = base_extents;
                let slot = if forward {
                    component
                } else {
                    components - 1 - component
                };
                component_extents.x_bearing += slot * component_extents.width / components;
                component_extents.width /= components;
            }
        }
        if last_class != Some(combining_class) {
            last_class = Some(combining_class);
            cluster_extents = component_extents;
        }
        position_mark(context, buffer, &mut cluster_extents, i, combining_class);
        let pos = &mut buffer.pos[i];
        pos.x_advance = 0;
        pos.y_advance = 0;
        pos.x_offset += x_offset;
        pos.y_offset += y_offset;
    }
}

/// Whether glyph `index` starts a new cluster for mark positioning.
fn starts_cluster(buffer: &Buffer, index: usize) -> bool {
    let info = &buffer.info[index];
    !info.chars.is_mark() && !info.chars.is_hidden() && !info.is_default_ignorable()
}

/// Positions every base + marks sequence of a cluster.
fn position_cluster(context: &Context<'_, '_>, buffer: &mut Buffer, start: usize, end: usize) {
    if end - start < 2 {
        return;
    }
    let mut i = start;
    while i < end {
        if buffer.info[i].chars.is_mark() {
            i += 1;
            continue;
        }
        let marks_end = (i + 1..end)
            .find(|&j| {
                let info = &buffer.info[j];
                !info.chars.is_hidden() && !info.is_default_ignorable() && !info.chars.is_mark()
            })
            .unwrap_or(end);
        position_around_base(context, buffer, i, marks_end);
        i = marks_end;
    }
}

/// HarfBuzz's `_hb_ot_shape_fallback_mark_position` for a run in
/// `direction` (logical order).
pub(crate) fn position_marks(
    face: &Face<'_>,
    scaler: &Scaler<'_>,
    direction: Direction,
    adjust_offsets: bool,
    buffer: &mut Buffer,
) {
    let context = Context {
        face,
        scaler,
        direction,
        adjust_offsets,
    };
    let mut start = 0;
    for i in 1..buffer.len() {
        if starts_cluster(buffer, i) {
            position_cluster(&context, buffer, start, i);
            start = i;
        }
    }
    position_cluster(&context, buffer, start, buffer.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recategorized_classes() {
        // Arabic fatha (modified 31) sits above, kasra (33) below.
        assert_eq!(recategorize('\u{64E}', 31), class::ABOVE);
        assert_eq!(recategorize('\u{650}', 33), class::BELOW);
        // Hebrew shin dot (modified 10) above right, qamats (21) below.
        assert_eq!(recategorize('\u{5C1}', 10), class::ABOVE_RIGHT);
        assert_eq!(recategorize('\u{5B8}', 21), class::BELOW);
        // Thai: mai ek has class 107, sara i class 0.
        assert_eq!(recategorize('\u{E48}', 107), class::ABOVE_RIGHT);
        assert_eq!(recategorize('\u{E34}', 0), class::ABOVE_RIGHT);
        assert_eq!(recategorize('\u{E3A}', 9), class::BELOW_RIGHT);
        assert_eq!(recategorize('\u{301}', 230), 230);
        assert_eq!(recategorize('x', 0), 0);
    }
}
