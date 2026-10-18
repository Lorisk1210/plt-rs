//! The CJK writing system (FreeType's `afcjk.c`), which also hints the
//! Indic scripts without blue zones (`afindic.c`): segments as in the
//! latin system, stems linked by distance, edges snapped to vertical blue
//! zones and stems kept apart from each other.
//!
//! matplotlib's FreeType has no horizontal CJK blue zones
//! (`AF_CONFIG_OPTION_CJK_BLUE_HANI_VERT` is off), so every blue zone here
//! is vertical.

mod edges;
mod hint;

use self::edges::{compute_blue_edges, compute_edges, link_segments};
use self::hint::{align_edge_points, hint_edges};
use super::hints::{Dimension, FontOutline, GlyphHints, scaler_flag};
use super::latin::segments::compute_segments;
use super::metrics::{MetricsContext, Scaler};
use super::shaper::{Clusters, shape_cluster};
use super::tables::blue;
use super::widths::{AxisWidths, Width};
use crate::fixed::{div_fix, mul_fix, pix_round};
use crate::types::GlyphId;

/// A CJK blue zone (`AF_CJKBlueRec`): its reference and undershoot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CjkBlue {
    pub(crate) reference: Width,
    pub(crate) overshoot: Width,
    /// A top zone.
    pub(crate) top: bool,
    /// Less than 3/4 pixel tall at the current size.
    pub(crate) active: bool,
}

/// The metrics of a CJK or Indic style (`AF_CJKMetricsRec`).
#[derive(Debug)]
pub(crate) struct CjkMetrics {
    pub(crate) units_per_em: u16,
    /// Standard widths of both axes. FreeType never scales them for CJK,
    /// so their scaled values stay zero.
    pub(crate) widths: [AxisWidths; 2],
    /// The vertical blue zones.
    pub(crate) blues: Vec<CjkBlue>,
    pub(crate) digits_have_same_width: bool,
}

impl CjkMetrics {
    /// The metrics of the style of `context`; Indic styles have no
    /// `blues` (`af_cjk_metrics_init`, `af_indic_metrics_init`).
    pub(crate) fn new(context: &MetricsContext<'_, '_>, blues: bool) -> Self {
        let mut metrics = Self {
            units_per_em: context.face.units_per_em(),
            widths: [AxisWidths::default(); 2],
            blues: Vec::new(),
            digits_have_same_width: false,
        };
        if context.has_unicode() {
            metrics.widths = context.standard_widths();
            if blues {
                metrics.blues = blue_zones(context);
            }
            metrics.digits_have_same_width = context.digits_have_same_width();
        }
        metrics
    }

    /// The vertical blue zones at `scale` (`af_cjk_metrics_scale_dim`):
    /// zones less than 3/4 pixel tall are active, with the undershoot
    /// rounded to whole pixels from the rounded reference.
    pub(crate) fn scale_blues(&self, scale: i64) -> Vec<CjkBlue> {
        self.blues
            .iter()
            .map(|blue| {
                let mut blue = *blue;
                blue.reference.cur = mul_fix(blue.reference.org, scale);
                blue.reference.fit = blue.reference.cur;
                blue.overshoot.cur = mul_fix(blue.overshoot.org, scale);
                blue.overshoot.fit = blue.overshoot.cur;
                blue.active = false;
                let dist = mul_fix(blue.reference.org - blue.overshoot.org, scale);
                if (-48..=48).contains(&dist) {
                    blue.reference.fit = pix_round(blue.reference.cur);
                    let delta1 = div_fix(blue.reference.fit, scale) - blue.overshoot.org;
                    let delta2 = mul_fix(delta1.abs(), scale);
                    let delta2 = if delta2 < 32 { 0 } else { pix_round(delta2) };
                    let delta2 = if delta1 < 0 { -delta2 } else { delta2 };
                    blue.overshoot.fit = blue.reference.fit - delta2;
                    blue.active = true;
                }
                blue
            })
            .collect()
    }

    /// Hints `outline` at `scaler`'s size (`af_cjk_hints_init` and
    /// `af_cjk_hints_apply`); the advance width is never hinted.
    pub(crate) fn hint(&self, scaler: &Scaler, outline: &FontOutline) -> GlyphHints {
        let mut hints = GlyphHints::new(scaler.x_scale, scaler.y_scale, scaler_flag::NO_ADVANCE);
        hints.other_flags = scaler.target.hinter_flags();
        hints.reload(outline, self.units_per_em);
        let blues = self.scale_blues(scaler.y_scale);
        // Horizontal edges have no blue zones to look for.
        self.detect_features(&mut hints, Dimension::Horizontal);
        self.detect_features(&mut hints, Dimension::Vertical);
        compute_blue_edges(&mut hints, &blues, self.units_per_em);
        for dim in Dimension::BOTH {
            hint_edges(&mut hints, &self.widths[dim.index()], dim);
            align_edge_points(&mut hints, dim);
            hints.align_strong_points(dim);
            hints.align_weak_points(dim);
        }
        hints
    }

    /// `af_cjk_hints_detect_features`. FreeType's CJK segment pass only
    /// adds a roundness test over the segments that existed before it ran,
    /// and there are none then.
    fn detect_features(&self, hints: &mut GlyphHints, dim: Dimension) {
        compute_segments(hints, dim, self.units_per_em);
        link_segments(hints, dim, self.units_per_em);
        compute_edges(hints, dim, self.widths[dim.index()].edge_distance_threshold);
    }
}

/// The vertical blue zones of the style (`af_cjk_metrics_init_blues`):
/// the characters before a `|` give the reference (fill) positions, those
/// after it the undershoot (flat) ones.
fn blue_zones(context: &MetricsContext<'_, '_>) -> Vec<CjkBlue> {
    let mut blues = Vec::new();
    for string in context.class.blues {
        let top = string.has(blue::CJK_TOP);
        let (mut fills, mut flats) = (Vec::new(), Vec::new());
        let mut fill = true;
        let mut clusters = Clusters::new(string.chars);
        loop {
            if clusters.take_separator() {
                fill = false;
                continue;
            }
            let Some(cluster) = clusters.next() else {
                break;
            };
            let glyphs = shape_cluster(context.shaping, cluster, context.coverage());
            if glyphs.len() > 1 {
                continue;
            }
            let Some(glyph) = glyphs
                .first()
                .map(|g| g.glyph)
                .filter(|&g| g != GlyphId::NOTDEF)
            else {
                continue;
            };
            let Ok((outline, _)) = FontOutline::load(context.face, glyph) else {
                continue;
            };
            if outline.points.len() <= 2 {
                continue;
            }
            let position = extreme_y(&outline, top);
            if fill {
                fills.push(position);
            } else {
                flats.push(position);
            }
        }
        if fills.is_empty() && flats.is_empty() {
            continue;
        }
        fills.sort_unstable();
        flats.sort_unstable();
        let median = |values: &[i64]| values[values.len() / 2];
        let (mut reference, mut overshoot) = if flats.is_empty() {
            (median(&fills), median(&fills))
        } else if fills.is_empty() {
            (median(&flats), median(&flats))
        } else {
            (median(&fills), median(&flats))
        };
        if overshoot != reference && top ^ (overshoot < reference) {
            reference = i64::midpoint(overshoot, reference);
            overshoot = reference;
        }
        let width = |org| Width {
            org,
            cur: 0,
            fit: 0,
        };
        blues.push(CjkBlue {
            reference: width(reference),
            overshoot: width(overshoot),
            top,
            active: false,
        });
    }
    blues
}

/// The highest (or lowest) point of the contours of more than one point;
/// zero when there is none.
fn extreme_y(outline: &FontOutline, top: bool) -> i64 {
    let mut best: Option<i64> = None;
    let mut start = 0;
    for &last in &outline.contour_ends {
        let first = start;
        start = last + 1;
        if last <= first {
            continue;
        }
        for &(_, y) in &outline.points[first..=last] {
            if best.is_none_or(|best| if top { y > best } else { y < best }) {
                best = Some(y);
            }
        }
    }
    best.unwrap_or(0)
}
