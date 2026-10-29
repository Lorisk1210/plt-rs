//! The global metrics of a style, per writing system, and what the latin
//! and CJK writing systems compute alike: the standard stem widths
//! (`af_latin_metrics_init_widths`, `af_cjk_metrics_init_widths`) and
//! whether the digits share one advance width
//! (`af_latin_metrics_check_digits`, `af_cjk_metrics_check_digits`).

use super::AutohintTarget;
use super::cjk::CjkMetrics;
use super::cmap::harfbuzz_face;
use super::globals::Autohinter;
use super::hints::{Dimension, FontOutline, GlyphHints};
use super::latin::LatinMetrics;
use super::latin::segments::{compute_segments, latin_constant, link_segments};
use super::shaper::{Clusters, shape_cluster};
use super::tables::{Coverage, STYLES, StyleClass, WritingSystem};
use super::widths::{AxisWidths, MAX_WIDTHS};
use crate::face::Face;
use crate::types::GlyphId;

/// The size and target glyphs are hinted for (`AF_ScalerRec`; FreeType's
/// deltas are always zero for the autohinter).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scaler {
    /// Scales from font units to 26.6, as 16.16.
    pub(crate) x_scale: i64,
    pub(crate) y_scale: i64,
    /// The rounded horizontal pixels per em of the face's size.
    pub(crate) x_ppem: u16,
    pub(crate) target: AutohintTarget,
}

impl Scaler {
    pub(crate) const fn scale(&self, dim: Dimension) -> i64 {
        match dim {
            Dimension::Horizontal => self.x_scale,
            Dimension::Vertical => self.y_scale,
        }
    }
}

/// The global metrics of one style.
#[derive(Debug)]
pub(crate) enum StyleMetrics {
    /// Unhinted: the outline is only scaled (`afdummy.c`).
    Dummy,
    Latin(LatinMetrics),
    /// The CJK writing system, and the Indic one (CJK without blue
    /// zones).
    Cjk(CjkMetrics),
    /// A latin style that found no blue zone: FreeType hints its glyphs
    /// with the unhinted style instead.
    NoBlueZones,
}

impl StyleMetrics {
    /// Computes the metrics of style `style` of `face`.
    pub(crate) fn new(face: &Face<'_>, hinter: &Autohinter, style: usize) -> Self {
        let class = &STYLES[style];
        let shaping = harfbuzz_face(face);
        let context = MetricsContext {
            face,
            shaping: &shaping,
            hinter,
            class,
        };
        match class.writing_system {
            WritingSystem::Dummy => Self::Dummy,
            WritingSystem::Latin => {
                LatinMetrics::new(&context).map_or(Self::NoBlueZones, Self::Latin)
            }
            WritingSystem::Cjk => Self::Cjk(CjkMetrics::new(&context, true)),
            WritingSystem::Indic => Self::Cjk(CjkMetrics::new(&context, false)),
        }
    }

    /// Whether the style is replaced by the unhinted one.
    pub(crate) const fn is_unhintable(&self) -> bool {
        matches!(self, Self::NoBlueZones)
    }
}

/// What computing a style's metrics needs.
pub(crate) struct MetricsContext<'a, 'f> {
    pub(crate) face: &'a Face<'f>,
    /// The face with HarfBuzz's character map, for shaping.
    pub(crate) shaping: &'a Face<'f>,
    pub(crate) hinter: &'a Autohinter,
    pub(crate) class: &'static StyleClass,
}

impl MetricsContext<'_, '_> {
    /// Whether the face has a Unicode character map, without which
    /// FreeType skips widths, blue zones and digits.
    pub(crate) fn has_unicode(&self) -> bool {
        self.hinter.unicode.is_some()
    }

    /// The glyph of the first cluster of `text` that shapes to a single
    /// existing glyph.
    fn first_single_glyph(&self, text: &str) -> Option<GlyphId> {
        Clusters::new(text).find_map(|cluster| {
            let glyphs = shape_cluster(self.shaping, cluster, self.class.coverage);
            if glyphs.len() > 1 {
                return None;
            }
            glyphs
                .first()
                .map(|g| g.glyph)
                .filter(|&g| g != GlyphId::NOTDEF)
        })
    }

    /// The standard stem widths of both axes, measured on the stems of
    /// the script's first available standard character; when there is
    /// none, 50 units per 2048.
    pub(crate) fn standard_widths(&self) -> [AxisWidths; 2] {
        let units_per_em = self.face.units_per_em();
        let mut axes = [AxisWidths::default(); 2];
        if let Some(glyph) = self.first_single_glyph(self.class.script().standard_chars) {
            self.measure_widths(glyph, units_per_em, &mut axes);
        }
        for axis in &mut axes {
            axis.set_standard(latin_constant(units_per_em, 50));
        }
        axes
    }

    /// The stem widths of `glyph`, in font units.
    fn measure_widths(&self, glyph: GlyphId, units_per_em: u16, axes: &mut [AxisWidths; 2]) {
        let Ok((outline, _)) = FontOutline::load(self.face, glyph) else {
            return;
        };
        if outline.points.is_empty() {
            return;
        }
        let mut hints = GlyphHints::new(0x10000, 0x10000, 0);
        hints.reload(&outline, units_per_em);
        for dim in Dimension::BOTH {
            compute_segments(&mut hints, dim, units_per_em);
            link_segments(&mut hints, dim, &[], units_per_em);
            let axis = &mut axes[dim.index()];
            let segments = &hints.axis(dim).segments;
            let mut count = 0;
            for (index, segment) in segments.iter().enumerate() {
                let Some(link) = segment.link else {
                    continue;
                };
                if segments[link].link == Some(index) && link > index && count < MAX_WIDTHS {
                    axis.widths[count].org =
                        (i64::from(segment.pos) - i64::from(segments[link].pos)).abs();
                    count += 1;
                }
            }
            axis.quantize(count, i64::from(units_per_em) / 100);
        }
    }

    /// Whether the ASCII digits all have the same advance width, as
    /// HarfBuzz shapes them.
    pub(crate) fn digits_have_same_width(&self) -> bool {
        let mut first: Option<i32> = None;
        for cluster in Clusters::new("0 1 2 3 4 5 6 7 8 9") {
            let glyphs = shape_cluster(self.shaping, cluster, self.class.coverage);
            if glyphs.len() > 1 {
                continue;
            }
            let Some(digit) = glyphs.first().filter(|g| g.glyph != GlyphId::NOTDEF) else {
                continue;
            };
            match first {
                Some(advance) if advance != digit.advance => return false,
                Some(_) => {}
                None => first = Some(digit.advance),
            }
        }
        true
    }

    /// The style's coverage.
    pub(crate) const fn coverage(&self) -> Coverage {
        self.class.coverage
    }
}
