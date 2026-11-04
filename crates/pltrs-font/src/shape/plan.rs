//! The shaping plan of one run (HarfBuzz's `hb_ot_shape_plan_t`): the
//! script shaper, the feature map with its stages and the positioning
//! strategy.

use crate::aat::{self, KernTable, Morx, UserFeature};
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
use crate::shape::run::cluster_at;
use crate::shape::script::Script;
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
    /// HarfBuzz's "dumber" shaper, which replaces a script's own shaper
    /// when `morx` substitutes: no shaper features, no mark zeroing and no
    /// fallback positioning.
    Dumber,
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
    Dumber,
}

impl ShaperKind {
    /// Whether the shaper gives marks zero advance (HarfBuzz's
    /// `zero_width_marks`, late for every shaper here but the dumber).
    const fn zeroes_marks(self) -> bool {
        !matches!(self, Self::Dumber)
    }

    /// Whether the shaper positions without `GPOS` by fallbacks (HarfBuzz's
    /// `fallback_position`): legacy kerning and mark stacking.
    const fn falls_back(self) -> bool {
        !matches!(self, Self::Dumber)
    }
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "HarfBuzz's independent plan decisions, one flag each"
)]
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
    /// Substitution uses `morx` (or `mort`) instead of `GSUB`, with these
    /// user features and language.
    pub(crate) morx: Option<MorxPlan>,
    /// Positioning uses `GPOS`.
    pub(crate) apply_gpos: bool,
    /// Positioning uses `kerx`.
    pub(crate) apply_kerx: bool,
    /// Positioning uses the `kern` table (either form).
    pub(crate) apply_kern: bool,
    /// The mask bits of the `kern` feature.
    pub(crate) kern_mask: Mask,
    /// Marks get zero advance after positioning (HarfBuzz's
    /// `zero_marks`).
    pub(crate) zero_marks: bool,
    /// Marks keep their pen position when their advance is zeroed
    /// (HarfBuzz's `adjust_mark_positioning_when_zeroing`).
    pub(crate) adjust_mark_positioning: bool,
    /// Marks are stacked from Unicode combining classes after positioning
    /// (HarfBuzz's `fallback_mark_positioning`).
    pub(crate) fallback_mark_positioning: bool,
}

/// What `morx` substitution needs beyond the face: the user's features
/// and the run's language (for language-tagged features).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MorxPlan {
    /// The user's features, in order.
    pub(crate) features: Vec<Feature>,
    /// The run's language, lower-case BCP 47.
    pub(crate) language: Option<String>,
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

/// Who positions the glyphs and how marks are treated (HarfBuzz's
/// positioning decisions in `hb_ot_shape_planner_t::compile`).
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "HarfBuzz's independent positioning decisions, one flag each"
)]
struct Positioning {
    gpos: bool,
    kerx: bool,
    kern: bool,
    zero_marks: bool,
    adjust_marks: bool,
}

impl Positioning {
    /// The decisions for `face` with `map`, for a script whose own shaper
    /// is `kind` (whichever shapes in its place): `kerx` is
    /// preferred to `GPOS` unless the font also substitutes by `GSUB`;
    /// the `kern` table serves when neither kerns.
    fn new(
        face: &Face<'_>,
        map: &Map<Pause>,
        apply_morx: bool,
        gpos_allowed: bool,
        kind: ShaperKind,
    ) -> Self {
        let has_gpos_kern = map.has_lookups(TableKind::Gpos, Tag::new(b"kern"));
        let has_kerx = KernTable::kerx(face).is_some();
        let kern_table = KernTable::kern(face);
        let has_gsub = !apply_morx && face.gsub().is_some();
        let has_gpos = gpos_allowed && face.gpos().is_some();
        let mut kerx = has_kerx && !(has_gsub && has_gpos);
        let gpos = !kerx && has_gpos;
        let mut kern = false;
        if !kerx && (!has_gpos_kern || !gpos) {
            if has_kerx {
                kerx = true;
            } else if kern_table.is_some() {
                kern = kind.falls_back();
            }
        }
        let zero_marks = kind.zeroes_marks()
            && !kerx
            && (!kern || !kern_table.is_some_and(|table| table.has_state_machine()));
        let adjust_marks =
            !gpos && !kerx && (!kern || !kern_table.is_some_and(|table| table.has_cross_stream()));
        Self {
            gpos,
            kerx,
            kern,
            zero_marks,
            adjust_marks,
        }
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
        let apply_morx = Morx::of(face).is_some();
        // The script's own shaper decides kerning and mark positioning;
        // with `morx`, the dumber one shapes in its place.
        let script_kind = Shaper::kind(script, builder.chosen_script(TableKind::Gsub));
        let kind = if apply_morx && script_kind != ShaperKind::Default {
            ShaperKind::Dumber
        } else {
            script_kind
        };
        let arabic_script = script.iso_code() == "Arab";
        // The Hebrew shaper only trusts `GPOS` designed for Hebrew.
        let gpos_allowed = kind != ShaperKind::Hebrew
            || builder.chosen_script(TableKind::Gpos) == Some(Tag::new(b"hebr"));
        collect_features(&mut builder, direction, kind, arabic_script, features);
        let map = builder.compile();
        let kern = Tag::new(b"kern");
        let positioning = Positioning::new(face, &map, apply_morx, gpos_allowed, script_kind);
        let has_gpos_mark = map.one_mask(Tag::new(b"mark")) != 0;
        let (shaper, normalize_hooks) = match kind {
            ShaperKind::Default => (Shaper::Default, Hooks::default()),
            ShaperKind::Dumber => (Shaper::Dumber, Hooks::default()),
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
            morx: apply_morx.then(|| MorxPlan {
                features: features.to_vec(),
                language: language.map(|language| language.canonical().to_owned()),
            }),
            kern_mask: map.value_bits(kern, 0).map_or(0, |(mask, _)| mask),
            map,
            apply_gpos: positioning.gpos,
            apply_kerx: positioning.kerx,
            apply_kern: positioning.kern,
            zero_marks: positioning.zero_marks && kind.zeroes_marks(),
            adjust_mark_positioning: positioning.adjust_marks && !apply_morx,
            fallback_mark_positioning: positioning.adjust_marks && script_kind.falls_back(),
        }
    }

    /// Runs the `GSUB` stages with their pauses.
    pub(crate) fn substitute(
        &self,
        face: &Face<'_>,
        scaler: &Scaler<'_>,
        direction: Direction,
        text: &[(usize, char)],
        buffer: &mut Buffer,
    ) {
        if let Some((plan, morx)) = self.morx.as_ref().zip(Morx::of(face)) {
            let features: Vec<UserFeature> = plan
                .features
                .iter()
                .map(|feature| UserFeature {
                    tag: feature.tag,
                    value: feature.value,
                    start: cluster_at(text, feature.start),
                    end: cluster_at(text, feature.end),
                })
                .collect();
            aat::substitute(
                face,
                &morx,
                buffer,
                direction,
                &features,
                plan.language.as_deref(),
            );
            // Without `GPOS`, deleted glyphs stay until after positioning,
            // where they keep their neighbours from kerning.
            if self.apply_gpos {
                aat::remove_deleted_glyphs(buffer);
            }
            return;
        }
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
