//! HarfBuzz's fallback mark positioning for fonts without `GPOS`: marks are
//! stacked above or below their base by combining class, using glyph
//! extents (`hb-ot-shape-fallback.cc`).

use crate::face::Face;
use crate::layout::buffer::Buffer;
use crate::scale::Scaler;
use crate::types::GlyphId;

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
/// (the unhinted control box), `None` when the glyph cannot be loaded.
fn extents(face: &Face<'_>, scaler: &Scaler, glyph: GlyphId) -> Option<Extents> {
    let rect = match scaler {
        Scaler::Units => face.glyph_bbox(glyph),
        Scaler::Pixels(size) => size.glyph_cbox(face, glyph),
    }
    .ok()?;
    Some(rect.map_or_else(Extents::default, |r| Extents {
        x_bearing: r.x_min,
        y_bearing: r.y_max,
        width: r.x_max - r.x_min,
        height: r.y_min - r.y_max,
    }))
}

/// Vertical gap between stacked marks: 1/16 em.
fn y_gap(face: &Face<'_>, scaler: &Scaler) -> i32 {
    match scaler {
        Scaler::Units => i32::from(face.units_per_em()) / 16,
        Scaler::Pixels(size) => (size.hb_scales().1 / 16) as i32,
    }
}

/// Positions mark `index` relative to `base` (HarfBuzz's `position_mark`),
/// updating `base` for further stacking.
fn position_mark(
    face: &Face<'_>,
    scaler: &Scaler,
    buffer: &mut Buffer,
    base: &mut Extents,
    index: usize,
    combining_class: u8,
) {
    let Some(mark) = extents(face, scaler, buffer.info[index].glyph) else {
        return;
    };
    let y_gap = y_gap(face, scaler);
    let pos = &mut buffer.pos[index];
    pos.x_offset = 0;
    pos.y_offset = 0;
    pos.x_offset += match combining_class {
        class::DOUBLE_BELOW | class::DOUBLE_ABOVE => {
            base.x_bearing + base.width - mark.width / 2 - mark.x_bearing
        }
        class::ATTACHED_BELOW_LEFT | class::BELOW_LEFT | class::ABOVE_LEFT => {
            base.x_bearing - mark.x_bearing
        }
        class::ATTACHED_ABOVE_RIGHT | class::BELOW_RIGHT | class::ABOVE_RIGHT => {
            base.x_bearing + base.width - mark.width - mark.x_bearing
        }
        _ => base.x_bearing + (base.width - mark.width) / 2 - mark.x_bearing,
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

/// HarfBuzz's `position_around_base` for left-to-right text.
fn position_around_base(
    face: &Face<'_>,
    scaler: &Scaler,
    buffer: &mut Buffer,
    base: usize,
    end: usize,
) {
    let base_info = buffer.info[base];
    let Some(mut base_extents) = extents(face, scaler, base_info.glyph) else {
        for pos in &mut buffer.pos[base + 1..end] {
            pos.x_advance = 0;
            pos.y_advance = 0;
        }
        return;
    };
    base_extents.y_bearing += buffer.pos[base].y_offset;
    base_extents.x_bearing = 0;
    base_extents.width = scaler.advance(face, base_info.glyph);
    let components = i32::from(base_info.lig_num_comps());
    let (mut x_offset, mut y_offset) = (-buffer.pos[base].x_advance, -buffer.pos[base].y_advance);
    let mut component_extents = base_extents;
    let mut last_component: Option<i32> = None;
    let mut last_class: Option<u8> = None;
    let mut cluster_extents = base_extents;
    for i in base + 1..end {
        let combining_class = buffer.info[i].chars.combining_class;
        if combining_class == 0 {
            x_offset -= buffer.pos[i].x_advance;
            y_offset -= buffer.pos[i].y_advance;
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
                component_extents.x_bearing += component * component_extents.width / components;
                component_extents.width /= components;
            }
        }
        if last_class != Some(combining_class) {
            last_class = Some(combining_class);
            cluster_extents = component_extents;
        }
        position_mark(
            face,
            scaler,
            buffer,
            &mut cluster_extents,
            i,
            combining_class,
        );
        let pos = &mut buffer.pos[i];
        pos.x_advance = 0;
        pos.y_advance = 0;
        pos.x_offset += x_offset;
        pos.y_offset += y_offset;
    }
}

/// Positions every base + marks sequence of a cluster.
fn position_cluster(
    face: &Face<'_>,
    scaler: &Scaler,
    buffer: &mut Buffer,
    start: usize,
    end: usize,
) {
    if end - start < 2 {
        return;
    }
    let mut i = start;
    while i < end {
        if buffer.info[i].chars.mark {
            i += 1;
            continue;
        }
        let marks_end = (i + 1..end)
            .find(|&j| !buffer.info[j].chars.mark)
            .unwrap_or(end);
        position_around_base(face, scaler, buffer, i, marks_end);
        i = marks_end;
    }
}

/// HarfBuzz's `_hb_ot_shape_fallback_mark_position`.
pub(crate) fn position_marks(face: &Face<'_>, scaler: &Scaler, buffer: &mut Buffer) {
    let mut start = 0;
    for i in 1..buffer.len() {
        if buffer.info[i].chars.combining_class == 0 {
            position_cluster(face, scaler, buffer, start, i);
            start = i;
        }
    }
    position_cluster(face, scaler, buffer, start, buffer.len());
}
