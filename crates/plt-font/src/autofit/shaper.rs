//! HarfBuzz as FreeType's autohinter uses it (`afshaper.c` built with
//! `FT_CONFIG_OPTION_USE_HARFBUZZ`, as matplotlib's FreeType is): blue
//! and standard character clusters are shaped, possibly with a style's
//! feature, and glyphs only `GSUB` reaches are assigned to styles from the
//! lookups of each script's features.

use std::collections::BTreeSet;

use super::cmap::UnicodeMap;
use super::globals::STYLE_UNASSIGNED;
use super::tables::{Coverage, STYLES, StyleClass};
use crate::face::Face;
use crate::layout::collect::{
    collect_lookups, gpos_input_glyphs, gsub_output_glyphs, would_substitute,
};
use crate::layout::feature::Feature;
use crate::shape::shape_segment;
use crate::types::{GlyphId, Tag};

/// The OpenType script tags HarfBuzz tries for a script (its ISO 15924
/// tag, `None` for no script), at most `max` of them
/// (`hb_ot_tags_from_script_and_language`): for the Indic scripts with
/// newer shaping the version 3 and 2 tags come first.
pub(crate) fn script_tags(script: Option<Tag>, max: usize) -> Vec<Tag> {
    let Some(script) = script else {
        return Vec::new();
    };
    let new = match &script.0 {
        b"Beng" => Some(*b"bng2"),
        b"Deva" => Some(*b"dev2"),
        b"Gujr" => Some(*b"gjr2"),
        b"Guru" => Some(*b"gur2"),
        b"Knda" => Some(*b"knd2"),
        b"Mlym" => Some(*b"mlm2"),
        b"Orya" => Some(*b"ory2"),
        b"Taml" => Some(*b"tml2"),
        b"Telu" => Some(*b"tel2"),
        b"Mymr" => Some(*b"mym2"),
        _ => None,
    };
    let old = match &script.0 {
        b"Hira" => *b"kana",
        b"Laoo" => *b"lao ",
        b"Yiii" => *b"yi  ",
        b"Nkoo" => *b"nko ",
        b"Vaii" => *b"vai ",
        b"Zmth" => *b"math",
        tag => [tag[0].to_ascii_lowercase(), tag[1], tag[2], tag[3]],
    };
    let mut tags = Vec::new();
    if let Some(new) = new {
        if &new != b"mym2" {
            tags.push(Tag([new[0], new[1], new[2], b'3']));
        }
        tags.push(Tag(new));
    }
    tags.push(Tag(old));
    tags.truncate(max);
    tags
}

/// A blue or standard character string, walked as FreeType does: clusters
/// of characters separated by spaces.
#[derive(Clone, Debug)]
pub(crate) struct Clusters<'a> {
    rest: &'a str,
}

impl<'a> Clusters<'a> {
    pub(crate) const fn new(text: &'a str) -> Self {
        Self { rest: text }
    }

    /// Consumes a `|` (after spaces), which separates the CJK fill and
    /// flat characters.
    pub(crate) fn take_separator(&mut self) -> bool {
        self.rest = self.rest.trim_start_matches(' ');
        match self.rest.strip_prefix('|') {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }
}

impl<'a> Iterator for Clusters<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        self.rest = self.rest.trim_start_matches(' ');
        if self.rest.is_empty() {
            return None;
        }
        let end = self.rest.find(' ').unwrap_or(self.rest.len());
        let (cluster, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some(cluster)
    }
}

/// One glyph of a shaped cluster, in font units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClusterGlyph {
    pub(crate) glyph: GlyphId,
    pub(crate) advance: i32,
    pub(crate) y_offset: i32,
}

/// `af_shaper_get_cluster`: the glyphs of `cluster`, shaped by HarfBuzz
/// with the coverage's feature (`face` has HarfBuzz's character map).
/// With a feature, nothing comes out when the feature changes no glyph:
/// the cluster is not available in the feature.
pub(crate) fn shape_cluster(
    face: &Face<'_>,
    cluster: &str,
    coverage: Coverage,
) -> Vec<ClusterGlyph> {
    let shape = |features: &[Feature]| -> Vec<ClusterGlyph> {
        shape_segment(face, cluster, features)
            .iter()
            .map(|glyph| ClusterGlyph {
                glyph: glyph.glyph_id,
                advance: glyph.x_advance,
                y_offset: glyph.y_offset,
            })
            .collect()
    };
    let Some(feature) = coverage.feature() else {
        return shape(&[]);
    };
    let with = shape(&[Feature::new(feature, 1)]);
    let without = shape(&[]);
    let same =
        with.len() == without.len() && with.iter().zip(&without).all(|(a, b)| a.glyph == b.glyph);
    if same { Vec::new() } else { with }
}

/// `af_shaper_get_coverage`: gives style `style` to the still unassigned
/// glyphs the `GSUB` lookups of its script's coverage feature (all
/// features for the default coverage) can produce. `default_script` adds
/// the `DFLT` script, for the default script's last pass.
///
/// A feature's glyphs are skipped when no blue character is substituted
/// by it, and those it also positions in `GPOS` are left alone (they are
/// likely shifted versions of other glyphs).
pub(crate) fn assign_coverage(
    face: &Face<'_>,
    unicode: &UnicodeMap,
    style: usize,
    default_script: bool,
    styles: &mut [u16],
) {
    let class = &STYLES[style];
    let mut scripts = script_tags(class.script().harfbuzz, 3);
    let dflt = Tag::new(b"DFLT");
    if default_script {
        match scripts.len() {
            0 | 1 => scripts.push(dflt),
            _ if scripts[1] != dflt => {
                scripts.truncate(2);
                scripts.push(dflt);
            }
            _ => {}
        }
    }
    let Some(gsub) = face.gsub() else {
        return;
    };
    let feature = class.coverage.feature();
    let features = feature.map(|tag| [tag]);
    let lookups = collect_lookups(gsub, &scripts, features.as_ref().map(<[Tag; 1]>::as_slice));
    if lookups.is_empty() {
        return;
    }
    let mut glyphs = BTreeSet::new();
    for &lookup in &lookups {
        gsub_output_glyphs(gsub, lookup, &mut glyphs);
    }
    if class.coverage != Coverage::Default {
        if !substitutes_blue_characters(face, unicode, class, &lookups) {
            return;
        }
        let mut positioned = BTreeSet::new();
        if let Some(gpos) = face.gpos() {
            let features = features.as_ref().map(<[Tag; 1]>::as_slice);
            for lookup in collect_lookups(gpos, &scripts, features) {
                gpos_input_glyphs(gpos, lookup, &mut positioned);
            }
        }
        glyphs.retain(|glyph| !positioned.contains(glyph));
    }
    for glyph in glyphs {
        if let Some(slot) = styles.get_mut(glyph as usize)
            && *slot == STYLE_UNASSIGNED
        {
            *slot = style as u16;
        }
    }
}

/// Whether some character of the style's blue strings (spaces included,
/// as FreeType walks them) is substituted by one of `lookups`.
fn substitutes_blue_characters(
    face: &Face<'_>,
    unicode: &UnicodeMap,
    class: &StyleClass,
    lookups: &BTreeSet<u16>,
) -> bool {
    let Some(gsub) = face.gsub() else {
        return false;
    };
    class.blues.iter().any(|blue| {
        blue.chars.chars().any(|c| {
            let glyph = unicode.glyph(u32::from(c)).unwrap_or(GlyphId::NOTDEF);
            lookups
                .iter()
                .any(|&lookup| would_substitute(gsub, lookup, glyph))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harfbuzz_script_tags() {
        let tags = |iso: &[u8; 4], max| script_tags(Some(Tag::new(iso)), max);
        assert_eq!(tags(b"Latn", 3), [Tag::new(b"latn")]);
        assert_eq!(
            tags(b"Beng", 3),
            [Tag::new(b"bng3"), Tag::new(b"bng2"), Tag::new(b"beng")]
        );
        assert_eq!(tags(b"Beng", 1), [Tag::new(b"bng3")]);
        assert_eq!(tags(b"Mymr", 3), [Tag::new(b"mym2"), Tag::new(b"mymr")]);
        assert_eq!(tags(b"Laoo", 3), [Tag::new(b"lao ")]);
        assert_eq!(script_tags(None, 3), []);
    }

    #[test]
    fn clusters_split_on_spaces() {
        let mut clusters = Clusters::new(" a bc  d |e ");
        assert_eq!(clusters.next(), Some("a"));
        assert_eq!(clusters.next(), Some("bc"));
        assert!(!clusters.take_separator());
        assert_eq!(clusters.next(), Some("d"));
        assert!(clusters.take_separator());
        assert_eq!(clusters.next(), Some("e"));
        assert_eq!(clusters.next(), None);
    }
}
