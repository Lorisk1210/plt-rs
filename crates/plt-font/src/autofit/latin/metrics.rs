//! The global metrics of a latin style (`af_latin_metrics_init`) and their
//! scaling to a size (`af_latin_metrics_scale`), which nudges the vertical
//! scale so that the x-height lands on the pixel grid.

use super::blues::blue_zones;
use crate::autofit::adjust::ReverseCharmap;
use crate::autofit::hints::Dimension;
use crate::autofit::metrics::{MetricsContext, Scaler};
use crate::autofit::widths::{AxisWidths, Width};
use crate::fixed::{mul_div, mul_fix, pix_round};
use crate::types::Tag;

/// Blue zone flags (`AF_LATIN_BLUE_*`).
pub(crate) mod blue_flag {
    /// Less than 3/4 pixel tall at the current size.
    pub(crate) const ACTIVE: u8 = 1 << 0;
    /// A top zone.
    pub(crate) const TOP: u8 = 1 << 1;
    /// A subscript top zone.
    pub(crate) const SUB_TOP: u8 = 1 << 2;
    /// A neutral zone.
    pub(crate) const NEUTRAL: u8 = 1 << 3;
    /// The x-height zone, which adjusts the vertical scale.
    pub(crate) const ADJUSTMENT: u8 = 1 << 4;
    /// The bottom zone of capital letters.
    pub(crate) const BOTTOM: u8 = 1 << 5;
    /// The bottom zone of small letters.
    pub(crate) const BOTTOM_SMALL: u8 = 1 << 6;
}

/// One blue zone (`AF_LatinBlueRec`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LatinBlue {
    /// The flat edge position.
    pub(crate) reference: Width,
    /// The round (overshoot) edge position.
    pub(crate) overshoot: Width,
    /// The extent of the glyphs the zone was measured on, in font units.
    pub(crate) ascender: i64,
    pub(crate) descender: i64,
    /// [`blue_flag`]s.
    pub(crate) flags: u8,
}

/// The metrics of a latin style (`AF_LatinMetricsRec`).
#[derive(Debug)]
pub(crate) struct LatinMetrics {
    pub(crate) units_per_em: u16,
    /// Standard widths of the horizontal and vertical axes.
    pub(crate) widths: [AxisWidths; 2],
    /// The blue zones, sorted as measured.
    pub(crate) blues: Vec<LatinBlue>,
    pub(crate) digits_have_same_width: bool,
    /// Which glyphs get diacritic adjustments.
    pub(crate) reverse_charmap: Option<ReverseCharmap>,
}

impl LatinMetrics {
    /// The metrics of the style of `context`; `None` when it has no blue
    /// zone (FreeType then disables hinting for the style).
    pub(crate) fn new(context: &MetricsContext<'_, '_>) -> Option<Self> {
        let face = context.face;
        let mut metrics = Self {
            units_per_em: face.units_per_em(),
            widths: [AxisWidths::default(); 2],
            blues: Vec::new(),
            digits_have_same_width: false,
            reverse_charmap: None,
        };
        if context.has_unicode() {
            metrics.widths = context.standard_widths();
            metrics.blues = blue_zones(context);
            if metrics.blues.is_empty() {
                return None;
            }
            metrics.digits_have_same_width = context.digits_have_same_width();
        }
        let hinter = context.hinter;
        metrics.reverse_charmap = hinter.unicode.as_ref().map(|unicode| {
            let gsub = face.table(Tag::new(b"GSUB")).zip(face.gsub().copied());
            ReverseCharmap::new(unicode, context.class.script(), gsub, &hinter.substitutions)
        });
        Some(metrics)
    }

    /// The metrics at `scaler`'s size.
    pub(crate) fn scale(&self, scaler: &Scaler) -> ScaledLatin {
        ScaledLatin {
            axis: Dimension::BOTH.map(|dim| self.scale_axis(scaler, dim)),
            x_ppem: scaler.x_ppem,
        }
    }

    /// `af_latin_metrics_scale_dim`.
    fn scale_axis(&self, scaler: &Scaler, dim: Dimension) -> ScaledAxis {
        let mut scale = scaler.scale(dim);
        if dim == Dimension::Vertical {
            scale = self.align_x_height(scale);
        }
        let widths = self.widths[dim.index()];
        let scaled_widths = widths
            .used()
            .iter()
            .map(|width| {
                let cur = mul_fix(width.org, scale);
                Width {
                    org: width.org,
                    cur,
                    fit: cur,
                }
            })
            .collect();
        // An extra-light axis has stems thinner than 5/8 pixel.
        let extra_light = mul_fix(widths.standard_width, scale) < 32 + 8;
        let blues = if dim == Dimension::Vertical {
            self.scale_blues(scale)
        } else {
            Vec::new()
        };
        ScaledAxis {
            scale,
            widths: scaled_widths,
            extra_light,
            blues,
        }
    }

    /// The vertical scale adjusted so that the x-height zone's overshoot
    /// is rounded to the grid (up from 40/64 of a pixel), unless that moves
    /// the font's extent by two pixels or more.
    fn align_x_height(&self, scale: i64) -> i64 {
        let Some(blue) = self
            .blues
            .iter()
            .find(|blue| blue.flags & blue_flag::ADJUSTMENT != 0)
        else {
            return scale;
        };
        let scaled = mul_fix(blue.overshoot.org, scale);
        let fitted = (scaled + 40) & !63;
        if scaled == fitted {
            return scale;
        }
        let new_scale = mul_div(scale, fitted, scaled);
        let max_height = self
            .blues
            .iter()
            .fold(i64::from(self.units_per_em), |max, blue| {
                max.max(blue.ascender).max(-blue.descender)
            });
        let dist = mul_fix(max_height, new_scale - scale);
        if -128 < dist && dist < 128 {
            new_scale
        } else {
            scale
        }
    }

    /// The blue zones at vertical scale `scale`: zones less than 3/4 pixel
    /// tall are active, with their overshoot rounded to 0, 1/2 or 1 pixel
    /// from the rounded reference; sub-top zones overlapping another zone
    /// are not.
    fn scale_blues(&self, scale: i64) -> Vec<LatinBlue> {
        let mut blues: Vec<LatinBlue> = self
            .blues
            .iter()
            .map(|blue| {
                let mut blue = *blue;
                blue.reference.cur = mul_fix(blue.reference.org, scale);
                blue.reference.fit = blue.reference.cur;
                blue.overshoot.cur = mul_fix(blue.overshoot.org, scale);
                blue.overshoot.fit = blue.overshoot.cur;
                blue.flags &= !blue_flag::ACTIVE;
                let dist = mul_fix(blue.reference.org - blue.overshoot.org, scale);
                if (-48..=48).contains(&dist) {
                    let delta = match dist.abs() {
                        0..32 => 0,
                        32..48 => 32,
                        _ => 64,
                    };
                    let delta = if dist < 0 { -delta } else { delta };
                    blue.reference.fit = pix_round(blue.reference.cur);
                    blue.overshoot.fit = blue.reference.fit - delta;
                    blue.flags |= blue_flag::ACTIVE;
                }
                blue
            })
            .collect();
        let active = |blue: &LatinBlue| blue.flags & blue_flag::ACTIVE != 0;
        let sub_top = |blue: &LatinBlue| blue.flags & blue_flag::SUB_TOP != 0;
        for index in 0..blues.len() {
            let blue = blues[index];
            if !sub_top(&blue) || !active(&blue) {
                continue;
            }
            let overlaps = blues.iter().any(|other| {
                !sub_top(other)
                    && active(other)
                    && other.reference.fit <= blue.overshoot.fit
                    && other.overshoot.fit >= blue.reference.fit
            });
            if overlaps {
                blues[index].flags &= !blue_flag::ACTIVE;
            }
        }
        blues
    }
}

/// One axis of latin metrics at a size.
#[derive(Clone, Debug)]
pub(crate) struct ScaledAxis {
    /// The (adjusted) scale.
    pub(crate) scale: i64,
    /// The standard widths, scaled.
    pub(crate) widths: Vec<Width>,
    /// Whether the standard width is thinner than 5/8 pixel.
    pub(crate) extra_light: bool,
    /// The blue zones (vertical axis only), scaled and fitted.
    pub(crate) blues: Vec<LatinBlue>,
}

/// Latin metrics at a size.
#[derive(Clone, Debug)]
pub(crate) struct ScaledLatin {
    pub(crate) axis: [ScaledAxis; 2],
    /// The size's horizontal pixels per em.
    pub(crate) x_ppem: u16,
}
