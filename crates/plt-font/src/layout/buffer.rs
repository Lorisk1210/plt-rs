//! The glyph buffer that `GSUB` and `GPOS` lookups operate on.

use crate::layout::gdef::{Gdef, GlyphClass};
use crate::shape::category::GeneralCategory;
use crate::types::GlyphId;

/// Feature mask bits of a glyph: the lookups of a feature only apply to
/// glyphs carrying the feature's bits (HarfBuzz's `hb_mask_t`).
pub(crate) type Mask = u32;

/// Glyph property bits; the class bits deliberately coincide with the
/// `IgnoreBaseGlyphs`/`IgnoreLigatures`/`IgnoreMarks` lookup flags, and the
/// mark attachment class lives in the high byte, as in HarfBuzz.
pub(crate) mod props {
    pub(crate) const BASE: u16 = 0x02;
    pub(crate) const LIGATURE: u16 = 0x04;
    pub(crate) const MARK: u16 = 0x08;
    pub(crate) const CLASS_MASK: u16 = BASE | LIGATURE | MARK;
}

/// How a default-ignorable character takes part in lookup matching.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ignorable {
    /// ZERO WIDTH NON-JOINER: skipped in context, not in `GSUB` input.
    Zwnj,
    /// ZERO WIDTH JOINER: skipped unless a feature handles joiners itself.
    Zwj,
    /// Hidden during `GPOS` only (CGJ, Mongolian variation selectors, tags).
    Hidden,
    /// Any other default-ignorable character.
    Other,
}

/// Unicode properties of the character a glyph came from (HarfBuzz's
/// `unicode_props`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct CharProps {
    pub(crate) category: GeneralCategory,
    /// HarfBuzz's modified combining class; zero for non-marks.
    pub(crate) combining_class: u8,
    /// Set for default-ignorable characters (hidden by shaping).
    pub(crate) ignorable: Option<Ignorable>,
}

impl CharProps {
    /// General category Mn, Mc or Me.
    pub(crate) const fn is_mark(self) -> bool {
        self.category.is_mark()
    }

    /// General category Mn.
    pub(crate) fn is_nonspacing_mark(self) -> bool {
        self.category == GeneralCategory::NonspacingMark
    }

    /// Hidden from `GSUB` skipping (see [`Ignorable::Hidden`]).
    pub(crate) fn is_hidden(self) -> bool {
        self.ignorable == Some(Ignorable::Hidden)
    }
}

/// One glyph being shaped.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct GlyphInfo {
    pub(crate) glyph: GlyphId,
    /// Byte offset of the cluster start in the source text.
    pub(crate) cluster: usize,
    pub(crate) chars: CharProps,
    /// Features enabled for this glyph.
    pub(crate) mask: Mask,
    /// Class bits plus mark attachment class in the high byte.
    pub(crate) props: u16,
    /// Ligature id shared by a ligature and the marks attached to it.
    pub(crate) lig_id: u8,
    /// For marks: the ligature component they belong to (1-based); for
    /// ligatures: the number of components; for the glyphs of a multiple
    /// substitution: their index in the sequence.
    pub(crate) lig_comp: u8,
    /// Whether this glyph is a ligature formed during shaping.
    pub(crate) lig_base: bool,
    /// Whether a `GSUB` lookup replaced this glyph.
    pub(crate) substituted: bool,
    /// Whether this glyph came out of a multiple substitution.
    pub(crate) multiplied: bool,
    /// Per-glyph value owned by the script shaper and carried through
    /// substitutions (HarfBuzz's auxiliary shaper variable).
    pub(crate) aux: u8,
}

impl GlyphInfo {
    pub(crate) fn new(glyph: GlyphId, cluster: usize, chars: CharProps, mask: Mask) -> Self {
        Self {
            glyph,
            cluster,
            chars,
            mask,
            props: 0,
            lig_id: 0,
            lig_comp: 0,
            lig_base: false,
            substituted: false,
            multiplied: false,
            aux: 0,
        }
    }

    /// How the glyph's character is ignorable; characters stop being
    /// default-ignorable once substituted (HarfBuzz's
    /// `_hb_glyph_info_is_default_ignorable`).
    pub(crate) fn ignorable(&self) -> Option<Ignorable> {
        self.chars.ignorable.filter(|_| !self.substituted)
    }

    pub(crate) fn is_default_ignorable(&self) -> bool {
        self.ignorable().is_some()
    }

    pub(crate) const fn is_mark(&self) -> bool {
        self.props & props::MARK != 0
    }

    pub(crate) const fn is_ligature(&self) -> bool {
        self.props & props::LIGATURE != 0
    }

    pub(crate) const fn is_base(&self) -> bool {
        self.props & props::BASE != 0
    }

    /// HarfBuzz's `_hb_glyph_info_get_lig_comp`.
    pub(crate) const fn lig_comp(&self) -> u8 {
        if self.lig_base { 0 } else { self.lig_comp }
    }

    /// HarfBuzz's `_hb_glyph_info_get_lig_num_comps`.
    pub(crate) const fn lig_num_comps(&self) -> u8 {
        if self.is_ligature() && self.lig_base {
            self.lig_comp
        } else {
            1
        }
    }

    pub(crate) fn set_lig_props_for_mark(&mut self, lig_id: u8, comp: u8) {
        self.lig_id = lig_id;
        self.lig_comp = comp & 0x0F;
        self.lig_base = false;
    }

    pub(crate) fn set_lig_props_for_ligature(&mut self, lig_id: u8, num_comps: u8) {
        self.lig_id = lig_id;
        self.lig_comp = num_comps & 0x0F;
        self.lig_base = true;
    }

    /// Substitutes the glyph and recomputes the class bits. Fonts with GDEF
    /// glyph classes use them; otherwise `guess` (ligature or the old
    /// class) is kept, mirroring HarfBuzz's `_set_glyph_class`.
    pub(crate) fn set_glyph(&mut self, glyph: GlyphId, gdef: &Gdef<'_>, guess: Option<u16>) {
        self.glyph = glyph;
        self.substituted = true;
        if gdef.has_glyph_classes() {
            self.props = gdef_props(gdef, glyph);
        } else if let Some(class) = guess {
            self.props = class;
        }
    }
}

/// Glyph properties from `GDEF`.
pub(crate) fn gdef_props(gdef: &Gdef<'_>, glyph: GlyphId) -> u16 {
    match gdef.glyph_class(glyph) {
        GlyphClass::Base => props::BASE,
        GlyphClass::Ligature => props::LIGATURE,
        GlyphClass::Mark => props::MARK | (gdef.mark_attach_class(glyph) << 8),
        GlyphClass::Component | GlyphClass::Unclassified => 0,
    }
}

/// Position of one glyph; units depend on the scaler used.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct GlyphPos {
    pub(crate) x_advance: i32,
    pub(crate) y_advance: i32,
    pub(crate) x_offset: i32,
    pub(crate) y_offset: i32,
    /// Relative index of the glyph this one is attached to (negative:
    /// earlier), 0 when unattached.
    pub(crate) attach_chain: i32,
    pub(crate) attach_kind: AttachKind,
}

/// Kind of GPOS attachment.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum AttachKind {
    #[default]
    None,
    Mark,
    Cursive,
}

/// Glyphs plus positions, with a cursor used while applying lookups.
#[derive(Clone, Debug, Default)]
pub(crate) struct Buffer {
    pub(crate) info: Vec<GlyphInfo>,
    pub(crate) pos: Vec<GlyphPos>,
    pub(crate) idx: usize,
    next_lig_id: u8,
}

impl Buffer {
    pub(crate) fn new(info: Vec<GlyphInfo>) -> Self {
        let pos = vec![GlyphPos::default(); info.len()];
        Self {
            info,
            pos,
            idx: 0,
            next_lig_id: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.info.len()
    }

    /// Allocates a ligature id in `1..=7`, cycling like HarfBuzz.
    pub(crate) fn allocate_lig_id(&mut self) -> u8 {
        self.next_lig_id = self.next_lig_id % 7 + 1;
        self.next_lig_id
    }

    /// Replaces the glyph at `index` by `glyphs` (possibly empty, which
    /// deletes it); the copies inherit the original's properties.
    /// Returns the number of glyphs inserted.
    pub(crate) fn splice(
        &mut self,
        index: usize,
        glyphs: impl Iterator<Item = GlyphInfo>,
    ) -> usize {
        let glyphs: Vec<GlyphInfo> = glyphs.collect();
        let count = glyphs.len();
        self.info.splice(index..=index, glyphs);
        self.pos.splice(
            index..=index,
            std::iter::repeat_n(GlyphPos::default(), count),
        );
        count
    }

    /// Removes the glyph at `index`.
    pub(crate) fn remove(&mut self, index: usize) {
        self.info.remove(index);
        self.pos.remove(index);
    }

    /// Reverses glyphs and positions (HarfBuzz's `hb_buffer_reverse`).
    pub(crate) fn reverse(&mut self) {
        self.info.reverse();
        self.pos.reverse();
    }

    /// Merges the clusters of `start..end` (see [`merge_clusters`]).
    pub(crate) fn merge_clusters(&mut self, start: usize, end: usize) {
        merge_clusters(&mut self.info, start, end, |info| &mut info.cluster);
    }
}

/// HarfBuzz's `merge_clusters`: gives `items[start..end]` their smallest
/// cluster, extending the range over neighbours that shared the cluster
/// of its first or last item.
pub(crate) fn merge_clusters<T>(
    items: &mut [T],
    start: usize,
    end: usize,
    cluster: fn(&mut T) -> &mut usize,
) {
    let end = end.min(items.len());
    if end <= start + 1 {
        return;
    }
    let get = |items: &mut [T], index: usize| *cluster(&mut items[index]);
    let merged = (start..end)
        .map(|index| get(items, index))
        .min()
        .unwrap_or_default();
    let (mut start, mut end) = (start, end);
    if merged != get(items, end - 1) {
        while end < items.len() && get(items, end - 1) == get(items, end) {
            end += 1;
        }
    }
    if merged != get(items, start) {
        while start > 0 && get(items, start - 1) == get(items, start) {
            start -= 1;
        }
    }
    for item in &mut items[start..end] {
        *cluster(item) = merged;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(glyph: u16, cluster: usize) -> GlyphInfo {
        GlyphInfo::new(GlyphId(glyph), cluster, CharProps::default(), 1)
    }

    #[test]
    fn splicing_and_clusters() {
        let mut buffer = Buffer::new(vec![info(1, 0), info(2, 1), info(3, 2)]);
        let template = buffer.info[1];
        let added = buffer.splice(
            1,
            [5, 6]
                .map(|g| GlyphInfo {
                    glyph: GlyphId(g),
                    ..template
                })
                .into_iter(),
        );
        assert_eq!(added, 2);
        assert_eq!(
            buffer.info.iter().map(|i| i.glyph.0).collect::<Vec<_>>(),
            vec![1, 5, 6, 3]
        );
        assert_eq!(buffer.pos.len(), 4);
        buffer.merge_clusters(1, 4);
        assert_eq!(
            buffer.info.iter().map(|i| i.cluster).collect::<Vec<_>>(),
            vec![0, 1, 1, 1]
        );
        buffer.reverse();
        assert_eq!(
            buffer.info.iter().map(|i| i.glyph.0).collect::<Vec<_>>(),
            vec![3, 6, 5, 1]
        );
        buffer.remove(0);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.splice(0, std::iter::empty()), 0);
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn merging_extends_over_shared_clusters() {
        let mut clusters = vec![0, 2, 2, 3, 3, 5];
        merge_clusters(&mut clusters, 2, 4, |c| c);
        assert_eq!(clusters, vec![0, 2, 2, 2, 2, 5]);
        let mut clusters = vec![1, 1, 4, 0];
        merge_clusters(&mut clusters, 1, 4, |c| c);
        assert_eq!(clusters, vec![0, 0, 0, 0]);
        merge_clusters(&mut clusters, 3, 3, |c| c);
        merge_clusters(&mut clusters, 2, 9, |c| c);
        assert_eq!(clusters, vec![0; 4]);
    }

    #[test]
    fn ligature_properties() {
        let mut buffer = Buffer::default();
        let ids: Vec<_> = (0..8).map(|_| buffer.allocate_lig_id()).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5, 6, 7, 1]);
        let mut lig = info(1, 0);
        lig.props = props::LIGATURE;
        lig.set_lig_props_for_ligature(3, 2);
        assert_eq!((lig.lig_comp(), lig.lig_num_comps()), (0, 2));
        let mut mark = info(2, 1);
        mark.props = props::MARK;
        mark.set_lig_props_for_mark(3, 2);
        assert_eq!((mark.lig_comp(), mark.lig_num_comps()), (2, 1));
        assert!(mark.is_mark() && lig.is_ligature() && !lig.is_base());
    }

    #[test]
    fn character_properties() {
        let zwj = CharProps {
            category: GeneralCategory::Format,
            combining_class: 0,
            ignorable: Some(Ignorable::Zwj),
        };
        assert!(!zwj.is_hidden() && !zwj.is_mark());
        let mut glyph = GlyphInfo::new(GlyphId(3), 0, zwj, 1);
        assert_eq!(glyph.ignorable(), Some(Ignorable::Zwj));
        glyph.set_glyph(GlyphId(4), &Gdef::default(), None);
        assert!(!glyph.is_default_ignorable());
        let acute = CharProps {
            category: GeneralCategory::NonspacingMark,
            combining_class: 230,
            ignorable: None,
        };
        assert!(acute.is_mark() && acute.is_nonspacing_mark());
        let cgj = CharProps {
            ignorable: Some(Ignorable::Hidden),
            ..acute
        };
        assert!(cgj.is_hidden());
    }
}
