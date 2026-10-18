//! The shaping plan of one run (HarfBuzz's `hb_ot_shape_plan_t`): the
//! script shaper, the feature map with its stages and the positioning
//! strategy.

use crate::face::Face;
use crate::layout::apply::{Applier, TableKind};
use crate::layout::buffer::{Buffer, Mask};
use crate::layout::feature::Feature;
use crate::layout::map::{FeatureFlags, Map, MapBuilder};
use crate::layout::ot_tags::OtTags;
use crate::scale::Scaler;
use crate::shape::arabic::{self, ArabicPlan};
use crate::shape::hebrew;
use crate::shape::language::LanguageTag;
use crate::shape::normalize::Hooks;
use crate::shape::script::Script;
use crate::tables::kern::KernSubtable;
use crate::types::{Direction, Tag};

/// A shaper hook run after a `GSUB` stage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pause {
    /// Marks the glyphs the `stch` feature multiplied.
    RecordStretch,
    /// Applies the synthesized Arabic presentation-form lookups.
    ArabicFallback,
}

/// The script-specific shaper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Shaper {
    /// HarfBuzz's default shaper.
    Default,
    /// The Arabic shaper (Arabic and Syriac).
    Arabic(ArabicPlan),
    /// The Hebrew shaper.
    Hebrew,
}

impl Shaper {
    /// HarfBuzz's `hb_ot_shaper_categorize` for the shapers implemented
    /// here: Arabic always uses the Arabic shaper, Syriac when the font has
    /// a Syriac script, Hebrew the Hebrew shaper.
    fn kind(script: Script, gsub_script: Option<Tag>) -> ShaperKind {
        match script.iso_code() {
            "Arab" => ShaperKind::Arabic,
            "Syrc" if gsub_script.is_some_and(|tag| tag != Tag::new(b"DFLT")) => ShaperKind::Arabic,
            "Hebr" => ShaperKind::Hebrew,
            _ => ShaperKind::Default,
        }
    }
}

/// [`Shaper`] before its plan data exists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ShaperKind {
    Default,
    Arabic,
    Hebrew,
}

/// Features every run gets, after the shaper's (HarfBuzz's
/// `common_features` and `horizontal_features`).
const COMMON_FEATURES: [(&[u8; 4], FeatureFlags); 14] = [
    (b"abvm", FeatureFlags::NONE),
    (b"blwm", FeatureFlags::NONE),
    (b"ccmp", FeatureFlags::NONE),
    (b"locl", FeatureFlags::NONE),
    (b"mark", FeatureFlags::MANUAL_JOINERS),
    (b"mkmk", FeatureFlags::MANUAL_JOINERS),
    (b"rlig", FeatureFlags::NONE),
    (b"calt", FeatureFlags::NONE),
    (b"clig", FeatureFlags::NONE),
    (b"curs", FeatureFlags::NONE),
    (b"dist", FeatureFlags::NONE),
    (b"kern", FeatureFlags::HAS_FALLBACK),
    (b"liga", FeatureFlags::NONE),
    (b"rclt", FeatureFlags::NONE),
];

/// The mask bits a feature set for some characters only gives them
/// (HarfBuzz's `set_masks` in `hb_ot_shape_setup_masks`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct RangedMask {
    /// The feature's mask bits.
    pub(crate) mask: Mask,
    /// Its value within those bits.
    pub(crate) value: Mask,
    /// Index of the first character.
    pub(crate) start: u32,
    /// Index one past the last character.
    pub(crate) end: u32,
}

/// Everything needed to shape one run of one script and direction.
#[derive(Clone, Debug)]
pub(crate) struct ShapePlan {
    pub(crate) script: Script,
    /// The run's direction (before any switch to the script's native
    /// direction).
    pub(crate) direction: Direction,
    pub(crate) map: Map<Pause>,
    pub(crate) shaper: Shaper,
    /// Mask of `rtlm`, given to characters that are not mirrored.
    pub(crate) rtlm_mask: Mask,
    /// The masks of the user's features that cover part of the text, in
    /// the order they were given.
    pub(crate) ranged_masks: Vec<RangedMask>,
    /// The shaper's normalization hooks.
    pub(crate) normalize_hooks: Hooks,
    /// Positioning uses `GPOS`.
    pub(crate) apply_gpos: bool,
    /// Positioning uses the legacy `kern` table.
    pub(crate) apply_kern: bool,
    /// Without `GPOS`, marks keep their pen position when their advance is
    /// zeroed and are then stacked from Unicode combining classes
    /// (HarfBuzz's `adjust_mark_positioning_when_zeroing` and
    /// `fallback_mark_positioning`, equal for the shapers here).
    pub(crate) fallback_mark_positioning: bool,
}

/// HarfBuzz's `hb_ot_shape_collect_features`.
fn collect_features(
    builder: &mut MapBuilder<'_, Pause>,
    direction: Direction,
    kind: ShaperKind,
    arabic_script: bool,
    features: &[Feature],
) {
    builder.enable_feature(Tag::new(b"rvrn"), FeatureFlags::NONE);
    builder.add_gsub_pause(None);
    match direction {
        Direction::LeftToRight => {
            builder.enable_feature(Tag::new(b"ltra"), FeatureFlags::NONE);
            builder.enable_feature(Tag::new(b"ltrm"), FeatureFlags::NONE);
        }
        Direction::RightToLeft => {
            builder.enable_feature(Tag::new(b"rtla"), FeatureFlags::NONE);
            builder.add_feature(Tag::new(b"rtlm"), FeatureFlags::NONE, 1);
        }
    }
    if kind == ShaperKind::Arabic {
        arabic::collect_features(builder, arabic_script);
    }
    for (tag, flags) in COMMON_FEATURES {
        builder.enable_feature(Tag::new(tag), flags);
    }
    for feature in features {
        let flags = if feature.is_global() {
            FeatureFlags::GLOBAL
        } else {
            FeatureFlags::NONE
        };
        builder.add_feature(feature.tag, flags, feature.value);
    }
}

impl ShapePlan {
    /// Plans a run of `script` in `direction` with `language` (`None`:
    /// the default language system) and the user's `features` on top of
    /// HarfBuzz's defaults.
    pub(crate) fn new(
        face: &Face<'_>,
        script: Script,
        direction: Direction,
        language: Option<&LanguageTag>,
        features: &[Feature],
    ) -> Self {
        let tags = OtTags::new(script, language);
        let mut builder = MapBuilder::new(face, &tags.scripts, &tags.languages);
        let kind = Shaper::kind(script, builder.chosen_script(TableKind::Gsub));
        let arabic_script = script.iso_code() == "Arab";
        // The Hebrew shaper only trusts `GPOS` designed for Hebrew.
        let gpos_allowed = kind != ShaperKind::Hebrew
            || builder.chosen_script(TableKind::Gpos) == Some(Tag::new(b"hebr"));
        collect_features(&mut builder, direction, kind, arabic_script, features);
        let map = builder.compile();
        let kern = Tag::new(b"kern");
        let apply_gpos = gpos_allowed && face.gpos().is_some();
        let apply_kern = (!apply_gpos || !map.has_lookups(TableKind::Gpos, kern))
            && map.one_mask(kern) != 0
            && face.has_legacy_kerning();
        let cross_kerning = face
            .kern_table()
            .subtables()
            .iter()
            .any(KernSubtable::is_cross_stream);
        let has_gpos_mark = map.one_mask(Tag::new(b"mark")) != 0;
        let (shaper, normalize_hooks) = match kind {
            ShaperKind::Default => (Shaper::Default, Hooks::default()),
            ShaperKind::Arabic => (
                Shaper::Arabic(ArabicPlan::new(&map, arabic_script)),
                Hooks {
                    reorder_marks: Some(arabic::reorder_marks),
                    compose: None,
                },
            ),
            ShaperKind::Hebrew => (
                Shaper::Hebrew,
                Hooks {
                    reorder_marks: Some(hebrew::reorder_marks),
                    compose: (!has_gpos_mark).then_some(hebrew::compose),
                },
            ),
        };
        let ranged_masks = features
            .iter()
            .filter(|feature| !feature.is_global())
            .filter_map(|feature| {
                let (mask, value) = map.value_bits(feature.tag, feature.value)?;
                Some(RangedMask {
                    mask,
                    value,
                    start: feature.start,
                    end: feature.end,
                })
            })
            .collect();
        Self {
            script,
            direction,
            rtlm_mask: map.one_mask(Tag::new(b"rtlm")),
            ranged_masks,
            normalize_hooks,
            shaper,
            map,
            apply_gpos,
            apply_kern,
            fallback_mark_positioning: !apply_gpos && (!apply_kern || !cross_kerning),
        }
    }

    /// Runs the `GSUB` stages with their pauses.
    pub(crate) fn substitute(
        &self,
        face: &Face<'_>,
        scaler: &Scaler<'_>,
        direction: Direction,
        buffer: &mut Buffer,
    ) {
        for stage in self.map.stages(TableKind::Gsub) {
            if let Some(table) = face.gsub() {
                let mut applier =
                    Applier::new(face, *table, TableKind::Gsub, scaler, direction, buffer);
                for lookup in &stage.lookups {
                    applier.apply_lookup(*lookup);
                }
            }
            if let (Some(pause), Shaper::Arabic(plan)) = (stage.pause, &self.shaper) {
                plan.pause(pause, face, &self.map, scaler, direction, buffer);
            }
        }
    }

    /// The shaper's post-processing, after positioning.
    pub(crate) fn postprocess(
        &self,
        face: &Face<'_>,
        scaler: &Scaler<'_>,
        direction: Direction,
        buffer: &mut Buffer,
    ) {
        if let Shaper::Arabic(_) = self.shaper {
            arabic::postprocess(face, scaler, direction, buffer);
        }
    }
}
