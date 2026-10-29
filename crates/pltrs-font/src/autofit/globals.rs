//! What the autohinter knows about a whole face (FreeType's
//! `afglobal.c`): the style of every glyph, from the character map
//! ranges of each script, the glyphs `GSUB` features reach and the
//! fallback style; and each style's global metrics, computed when a glyph
//! of the style is first hinted.

use std::sync::OnceLock;

use super::cmap::UnicodeMap;
use super::gsub::SubstitutionLookups;
use super::metrics::StyleMetrics;
use super::shaper::assign_coverage;
use super::tables::{Coverage, DEFAULT_SCRIPT, FALLBACK_STYLE, NONE_STYLE, STYLES};
use crate::face::Face;
use crate::types::{GlyphId, Tag};

/// The bits of a glyph style entry holding the style index.
pub(crate) const STYLE_MASK: u16 = 0x1FFF;
/// The style entry of a glyph no style claims yet.
pub(crate) const STYLE_UNASSIGNED: u16 = STYLE_MASK;
/// An ASCII digit.
const DIGIT: u16 = 0x8000;
/// A non-base character, which blue zones do not apply to.
const NONBASE: u16 = 0x4000;

/// The autohinter's analysis of one face (`AF_FaceGlobalsRec`).
///
/// It is computed from a face and owns all its data, so that it can be
/// cached next to the font; every method takes that same face again.
#[derive(Debug)]
pub struct Autohinter {
    /// Style index and flags of every glyph.
    styles: Vec<u16>,
    /// FreeType's Unicode character map, if the face has one.
    pub(crate) unicode: Option<UnicodeMap>,
    /// The `GSUB` substitutions diacritic adjustments follow.
    pub(crate) substitutions: SubstitutionLookups,
    metrics: Vec<OnceLock<StyleMetrics>>,
}

/// A glyph's style for hinting, once its metrics are known.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphStyle<'a> {
    pub(crate) metrics: &'a StyleMetrics,
    /// An ASCII digit (for the fixed digit advance).
    pub(crate) digit: bool,
    /// A non-base character (blue zones do not apply).
    pub(crate) nonbase: bool,
    /// Whether the style's script hints vertical edges top to bottom.
    pub(crate) top_to_bottom: bool,
}

impl Autohinter {
    /// Analyses `face`: assigns a style to every glyph
    /// (`af_face_globals_compute_style_coverage`).
    pub fn new(face: &Face<'_>) -> Self {
        let unicode = UnicodeMap::new(face);
        let substitutions = face
            .table(Tag::new(b"GSUB"))
            .map(SubstitutionLookups::new)
            .unwrap_or_default();
        let mut styles = vec![STYLE_UNASSIGNED; usize::from(face.num_glyphs())];
        if let Some(unicode) = &unicode {
            assign_styles(face, unicode, &mut styles);
        }
        for entry in &mut styles {
            if *entry & STYLE_MASK == STYLE_UNASSIGNED {
                *entry = (*entry & !STYLE_MASK) | FALLBACK_STYLE as u16;
            }
        }
        Self {
            styles,
            unicode,
            substitutions,
            metrics: STYLES.iter().map(|_| OnceLock::new()).collect(),
        }
    }

    /// The style of `glyph` with its metrics, computed on first use
    /// (`af_face_globals_get_metrics`). A latin style without blue zones
    /// is unhinted: its glyphs fall back to the unhinted style, losing
    /// their flags, as FreeType rewrites their entries.
    pub(crate) fn glyph_style(&self, face: &Face<'_>, glyph: GlyphId) -> Option<GlyphStyle<'_>> {
        let entry = *self.styles.get(usize::from(glyph.0))?;
        let style = usize::from(entry & STYLE_MASK);
        let metrics = self.metrics(face, style);
        if metrics.is_unhintable() {
            return Some(GlyphStyle {
                metrics: self.metrics(face, NONE_STYLE),
                digit: false,
                nonbase: false,
                top_to_bottom: false,
            });
        }
        Some(GlyphStyle {
            metrics,
            digit: entry & DIGIT != 0,
            nonbase: entry & NONBASE != 0,
            top_to_bottom: STYLES[style].script().top_to_bottom,
        })
    }

    fn metrics(&self, face: &Face<'_>, style: usize) -> &StyleMetrics {
        self.metrics[style].get_or_init(|| StyleMetrics::new(face, self, style))
    }
}

/// The style pass over a face with a Unicode character map.
fn assign_styles(face: &Face<'_>, unicode: &UnicodeMap, styles: &mut [u16]) {
    for (index, class) in STYLES.iter().enumerate() {
        if class.coverage != Coverage::Default {
            // Glyphs not directly addressable by the character map.
            assign_coverage(face, unicode, index, false, styles);
            continue;
        }
        let script = class.script();
        for &(first, last) in script.ranges {
            for glyph in unicode.range(first, last) {
                let entry = &mut styles[usize::from(glyph.0)];
                if *entry & STYLE_MASK == STYLE_UNASSIGNED {
                    *entry = index as u16;
                }
            }
        }
        for &(first, last) in script.nonbase_ranges {
            for glyph in unicode.range(first, last) {
                let entry = &mut styles[usize::from(glyph.0)];
                if usize::from(*entry & STYLE_MASK) == index {
                    *entry |= NONBASE;
                }
            }
        }
    }
    // The remaining default OpenType features, then those of the default
    // script.
    for (index, class) in STYLES.iter().enumerate() {
        if class.coverage == Coverage::Default {
            assign_coverage(face, unicode, index, false, styles);
        }
    }
    let default = STYLES
        .iter()
        .position(|class| class.coverage == Coverage::Default && class.script == DEFAULT_SCRIPT)
        .expect("the default script has a default style");
    assign_coverage(face, unicode, default, true, styles);
    for digit in '0'..='9' {
        if let Some(glyph) = unicode.glyph(u32::from(digit)) {
            styles[usize::from(glyph.0)] |= DIGIT;
        }
    }
}
