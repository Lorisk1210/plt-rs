//! HarfBuzz's Arabic fallback shaping: for fonts that lack the positional
//! features but map the Unicode presentation forms, single substitutions
//! to the forms and ligature substitutions (lam-alef etc.) are synthesized
//! from the character map.

use crate::face::Face;
use crate::layout::apply::{Applier, TableKind};
use crate::layout::buffer::{Buffer, Mask};
use crate::layout::common::lookup_flag;
use crate::layout::map::{LookupMap, Map};
use crate::layout::synthetic::{Ligature, SyntheticGsub, SyntheticLookup};
use crate::scale::Scaler;
use crate::shape::ucd_tables::{
    ARABIC_FORMS, ARABIC_LIGATURES, ARABIC_LIGATURES_3, ARABIC_MARK_LIGATURES, Ligatures,
};
use crate::types::{Direction, GlyphId, Tag};

use super::Pause;

/// Features of the synthesized lookups, in application order: the four
/// columns of `ARABIC_FORMS`, then three ligature lookups.
const FEATURES: [&[u8; 4]; 7] = [
    b"init", b"medi", b"fina", b"isol", b"rlig", b"rlig", b"rlig",
];

/// The glyph of `code` (a presentation form or base character).
fn glyph(face: &Face<'_>, code: u16) -> Option<GlyphId> {
    char::from_u32(u32::from(code)).and_then(|c| face.glyph_index(c))
}

/// Single substitutions from base characters to the presentation forms of
/// column `form` of `ARABIC_FORMS`.
fn single_lookup(face: &Face<'_>, form: usize) -> Option<SyntheticLookup> {
    let pairs: Vec<(GlyphId, GlyphId)> = ARABIC_FORMS
        .iter()
        .filter(|(_, forms)| forms[form] != 0)
        .filter_map(|&(base, forms)| Some((glyph(face, base)?, glyph(face, forms[form])?)))
        .filter(|(base, substitute)| base != substitute)
        .collect();
    (!pairs.is_empty()).then_some(SyntheticLookup::Single {
        flags: lookup_flag::IGNORE_MARKS,
        pairs,
    })
}

/// Ligature substitutions from a table of `(first, [(components,
/// ligature)])` entries.
fn ligature_lookup<const N: usize>(
    face: &Face<'_>,
    table: &[(u16, Ligatures<N>)],
    flags: u16,
) -> Option<SyntheticLookup> {
    let sets: Vec<(GlyphId, Vec<Ligature>)> = table
        .iter()
        .filter_map(|&(first, ligatures)| {
            let ligatures = ligatures
                .iter()
                .filter_map(|(components, ligature)| {
                    Some(Ligature {
                        components: components
                            .iter()
                            .map(|&c| glyph(face, c))
                            .collect::<Option<_>>()?,
                        glyph: glyph(face, *ligature)?,
                    })
                })
                .collect();
            Some((glyph(face, first)?, ligatures))
        })
        .collect();
    sets.iter()
        .any(|(_, ligatures)| !ligatures.is_empty())
        .then_some(SyntheticLookup::Ligature { flags, sets })
}

/// The lookup HarfBuzz synthesizes for `FEATURES[index]`.
fn synthesize(face: &Face<'_>, index: usize) -> Option<SyntheticLookup> {
    match index {
        0..=3 => single_lookup(face, index),
        4 => ligature_lookup(face, &ARABIC_LIGATURES_3, lookup_flag::IGNORE_MARKS),
        5 => ligature_lookup(face, &ARABIC_LIGATURES, lookup_flag::IGNORE_MARKS),
        _ => ligature_lookup(face, &ARABIC_MARK_LIGATURES, 0),
    }
}

/// The synthesized lookups of one font, each with its feature mask.
#[derive(Clone, Debug)]
pub(super) struct FallbackPlan {
    gsub: SyntheticGsub,
    masks: Vec<Mask>,
}

impl FallbackPlan {
    /// Synthesizes the lookups whose features are in `map`; `None` when
    /// the font maps no presentation forms.
    pub(super) fn new(face: &Face<'_>, map: &Map<Pause>) -> Option<Self> {
        let (lookups, masks): (Vec<SyntheticLookup>, Vec<Mask>) = FEATURES
            .iter()
            .enumerate()
            .filter_map(|(index, &tag)| {
                let mask = map.one_mask(Tag::new(tag));
                (mask != 0).then(|| synthesize(face, index).map(|lookup| (lookup, mask)))?
            })
            .unzip();
        (!lookups.is_empty()).then(|| Self {
            gsub: SyntheticGsub::new(&lookups),
            masks,
        })
    }

    /// Applies the lookups to `buffer`.
    pub(super) fn apply(
        &self,
        face: &Face<'_>,
        scaler: &Scaler<'_>,
        direction: Direction,
        buffer: &mut Buffer,
    ) {
        let Ok(table) = self.gsub.table() else {
            return;
        };
        let mut applier = Applier::new(face, table, TableKind::Gsub, scaler, direction, buffer);
        for (index, &mask) in self.masks.iter().enumerate() {
            let index = u16::try_from(index).expect("at most seven fallback lookups");
            applier.apply_lookup(LookupMap {
                mask,
                ..LookupMap::global(index)
            });
        }
    }
}
