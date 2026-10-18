//! Loading a glyph through the autohinter (FreeType's
//! `af_loader_load_glyph`): the outline in font units is hinted by its
//! style's writing system, the side bearings are rounded with the hinted
//! stems (outside light mode), and the advance width follows.

use super::AutohintTarget;
use super::globals::{Autohinter, GlyphStyle};
use super::hints::{Dimension, FontOutline, GlyphHints, point_flag};
use super::latin::{self, LatinGlyph};
use super::metrics::{Scaler, StyleMetrics};
use crate::error::Result;
use crate::face::Face;
use crate::fixed::{mul_fix, pix_round};
use crate::hinting::HintedGlyph;
use crate::outline::{Outline, PointKind};
use crate::scale::Size;
use crate::types::GlyphId;

impl Autohinter {
    /// Loads `glyph` of `face` (the face this autohinter analysed),
    /// autohinted at `size` for `target`, as `FT_Load_Glyph` does when
    /// FreeType's autohinter takes the glyph.
    ///
    /// # Errors
    ///
    /// When the glyph does not exist or its outline is malformed.
    pub fn load_glyph(
        &self,
        face: &Face<'_>,
        size: &Size,
        glyph: GlyphId,
        target: AutohintTarget,
    ) -> Result<HintedGlyph> {
        face.check_glyph(glyph)?;
        let (outline, advance) = FontOutline::load(face, glyph)?;
        let style = self
            .glyph_style(face, glyph)
            .expect("checked glyphs have a style");
        let scaler = Scaler {
            x_scale: size.x_scale(),
            y_scale: size.y_scale(),
            x_ppem: size.x_ppem(),
            target,
        };
        let hints = hint(face, glyph, &outline, style, &scaler);
        Ok(finish(face, &outline, advance, &hints, style, &scaler))
    }
}

/// Hints `outline` with the writing system of `style`. Glyphs without
/// points are only scaled (their hints hold no points).
fn hint(
    face: &Face<'_>,
    glyph: GlyphId,
    outline: &FontOutline,
    style: GlyphStyle<'_>,
    scaler: &Scaler,
) -> GlyphHints {
    let units_per_em = face.units_per_em();
    let has_points = !outline.points.is_empty();
    match style.metrics {
        StyleMetrics::Latin(metrics) => {
            let scaled = metrics.scale(scaler);
            let mut hints = latin::hints_init(&scaled, scaler.target, face.style_flags().italic);
            if has_points {
                let glyph = LatinGlyph {
                    glyph,
                    outline,
                    nonbase: style.nonbase,
                    top_to_bottom: style.top_to_bottom,
                };
                latin::hints_apply(&mut hints, metrics, &scaled, &glyph);
            }
            hints
        }
        StyleMetrics::Cjk(metrics) if has_points => metrics.hint(scaler, outline),
        StyleMetrics::Cjk(_) => GlyphHints::new(scaler.x_scale, scaler.y_scale, 0),
        StyleMetrics::Dummy | StyleMetrics::NoBlueZones => {
            let mut hints = GlyphHints::new(scaler.x_scale, scaler.y_scale, 0);
            if has_points {
                hints.reload(outline, units_per_em);
            }
            hints
        }
    }
}

/// The hinted glyph: the outline moved so that the rounded left side
/// bearing is at the origin, and the advance width.
fn finish(
    face: &Face<'_>,
    outline: &FontOutline,
    advance: i64,
    hints: &GlyphHints,
    style: GlyphStyle<'_>,
    scaler: &Scaler,
) -> HintedGlyph {
    let light = scaler.target == AutohintTarget::Light;
    let unhinted_advance = mul_fix(advance, hints.x_scale);
    let (pp1, pp2) = if outline.points.is_empty() {
        (0, unhinted_advance)
    } else if light {
        (0, pix_round(unhinted_advance))
    } else {
        side_bearings(hints, unhinted_advance)
    };
    let mut hinted = Outline::default();
    let mut start = 0;
    for &last in &outline.contour_ends {
        for point in &hints.points[start..=last] {
            let kind = if point.flags & point_flag::CONIC != 0 {
                PointKind::Quadratic
            } else if point.flags & point_flag::CUBIC != 0 {
                PointKind::Cubic
            } else {
                PointKind::OnCurve
            };
            hinted.push((point.x - pp1) as f64, point.y as f64, kind);
        }
        hinted.end_contour();
        start = last + 1;
    }
    // Monospaced fonts, and digits when all have one width, keep the
    // rounded unhinted advance.
    let fixed_width = face.post().is_some_and(|post| post.is_fixed_pitch);
    let keeps_advance = fixed_width || (style.digit && style.metrics.digits_have_same_width());
    let advance = if !light && keeps_advance {
        mul_fix(advance, scaler.x_scale)
    } else if advance != 0 {
        pp2 - pp1
    } else {
        0
    };
    let advance = pix_round(advance).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    HintedGlyph::new(hinted, advance)
}

/// The rounded phantom points (origin and advance) of a glyph hinted
/// horizontally: placed by the outermost hinted edges when there are
/// several, preferring too much space to too little at small sizes.
fn side_bearings(hints: &GlyphHints, unhinted_advance: i64) -> (i64, i64) {
    let edges = &hints.axis(Dimension::Horizontal).edges;
    let (Some(first), Some(last)) = (edges.first(), edges.last()) else {
        return (0, pix_round(unhinted_advance));
    };
    if edges.len() < 2 || !hints.do_advance() {
        return (0, pix_round(unhinted_advance));
    }
    let old_rsb = unhinted_advance - last.opos;
    let old_lsb = first.opos;
    let new_lsb = first.pos;
    let mut pp1_unhinted = new_lsb - old_lsb;
    let mut pp2_unhinted = last.pos + old_rsb;
    if old_lsb < 24 {
        pp1_unhinted -= 8;
    }
    if old_rsb < 24 {
        pp2_unhinted += 8;
    }
    let mut pp1 = pix_round(pp1_unhinted);
    let mut pp2 = pix_round(pp2_unhinted);
    if pp1 >= new_lsb && old_lsb > 0 {
        pp1 -= 64;
    }
    if pp2 <= last.pos && old_rsb > 0 {
        pp2 += 64;
    }
    (pp1, pp2)
}

impl StyleMetrics {
    /// Whether the style's digits share one advance width.
    const fn digits_have_same_width(&self) -> bool {
        match self {
            Self::Latin(metrics) => metrics.digits_have_same_width,
            Self::Cjk(metrics) => metrics.digits_have_same_width,
            Self::Dummy | Self::NoBlueZones => false,
        }
    }
}
