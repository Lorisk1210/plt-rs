//! HarfBuzz's Arabic shaper (`hb-ot-shaper-arabic.cc`), used for the
//! Arabic and Syriac scripts: the joining state machine selects one of the
//! positional features (`isol`, `fina`, `fin2`, `fin3`, `medi`, `med2`,
//! `init`) per character, features are applied in the stages the Arabic
//! specification orders (`stch`, `ccmp`/`locl`, each positional feature,
//! `rlig`, `calt`, `liga`/`clig`/`mset`), fonts without the positional
//! features get synthesized presentation-form lookups, and modifier
//! combining marks move ahead of other marks.

mod fallback;
mod joining;
mod stretch;

use crate::face::Face;
use crate::layout::buffer::{Buffer, Mask};
use crate::layout::map::{FeatureFlags, Map, MapBuilder};
use crate::scale::Scaler;
use crate::shape::normalize::{ClusterChar, merge_clusters};
use crate::shape::plan::Pause;
use crate::types::{Direction, Tag};
use fallback::FallbackPlan;

/// Tags of the positional features, in `joining::Action` order.
const FEATURES: [&[u8; 4]; 7] = [
    b"isol", b"fina", b"fin2", b"fin3", b"medi", b"med2", b"init",
];

/// HarfBuzz's modified combining class of reordered modifier marks below.
const MODIFIER_BELOW_CLASS: u8 = 25;
/// HarfBuzz's modified combining class of reordered modifier marks above.
const MODIFIER_ABOVE_CLASS: u8 = 26;

/// Whether a positional feature is Syriac-only (`fin2`, `fin3`, `med2`).
fn is_syriac(tag: [u8; 4]) -> bool {
    matches!(tag[3], b'2' | b'3')
}

/// Adds the Arabic shaper's features and pauses (HarfBuzz's
/// `collect_features_arabic`); `arabic_script` enables the fallback.
pub(crate) fn collect_features(map: &mut MapBuilder<'_, Pause>, arabic_script: bool) {
    map.enable_feature(Tag::new(b"stch"), FeatureFlags::NONE);
    map.add_gsub_pause(Some(Pause::RecordStretch));
    map.enable_feature(Tag::new(b"ccmp"), FeatureFlags::MANUAL_ZWJ);
    map.enable_feature(Tag::new(b"locl"), FeatureFlags::MANUAL_ZWJ);
    map.add_gsub_pause(None);
    for tag in FEATURES {
        let fallback = if arabic_script && !is_syriac(*tag) {
            FeatureFlags::HAS_FALLBACK
        } else {
            FeatureFlags::NONE
        };
        map.add_feature(Tag::new(tag), FeatureFlags::MANUAL_ZWJ | fallback, 1);
        map.add_gsub_pause(None);
    }
    // HarfBuzz releases its joining scratch data here: an empty stage.
    map.add_gsub_pause(None);
    map.enable_feature(
        Tag::new(b"rlig"),
        FeatureFlags::MANUAL_ZWJ | FeatureFlags::HAS_FALLBACK,
    );
    if arabic_script {
        map.add_gsub_pause(Some(Pause::ArabicFallback));
    }
    map.enable_feature(Tag::new(b"calt"), FeatureFlags::MANUAL_ZWJ);
    if !map.has_feature(Tag::new(b"rclt")) {
        map.add_gsub_pause(None);
    }
    for tag in [b"liga", b"clig", b"mset"] {
        map.enable_feature(Tag::new(tag), FeatureFlags::MANUAL_ZWJ);
    }
}

/// Per-plan data of the Arabic shaper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArabicPlan {
    /// Mask of each positional feature, in `joining::Action` order.
    masks: [Mask; 7],
    /// Whether the font lacks the positional features and the
    /// presentation-form fallback runs.
    do_fallback: bool,
}

impl ArabicPlan {
    /// HarfBuzz's `data_create_arabic`.
    pub(crate) fn new(map: &Map<Pause>, arabic_script: bool) -> Self {
        let masks = FEATURES.map(|tag| map.one_mask(Tag::new(tag)));
        let do_fallback = arabic_script
            && FEATURES
                .iter()
                .all(|&tag| is_syriac(*tag) || map.needs_fallback(Tag::new(tag)));
        Self { masks, do_fallback }
    }

    /// Enables the positional feature each character's joining calls for;
    /// `pre` and `post` are the characters around the run (closest first).
    pub(crate) fn setup_masks(&self, chars: &mut [ClusterChar], pre: &[char], post: &[char]) {
        let text: Vec<char> = chars.iter().map(|item| item.c).collect();
        for (item, action) in chars.iter_mut().zip(joining::actions(&text, pre, post)) {
            if let Some(action) = action {
                item.mask |= self.masks[action.index()];
            }
        }
    }

    /// Runs a pause registered by [`collect_features`].
    pub(crate) fn pause(
        &self,
        pause: Pause,
        face: &Face<'_>,
        map: &Map<Pause>,
        scaler: &Scaler<'_>,
        direction: Direction,
        buffer: &mut Buffer,
    ) {
        match pause {
            Pause::RecordStretch => {
                if map.one_mask(Tag::new(b"stch")) != 0 {
                    stretch::record(buffer);
                }
            }
            Pause::ArabicFallback => {
                if let Some(plan) = self
                    .do_fallback
                    .then(|| FallbackPlan::new(face, map))
                    .flatten()
                {
                    plan.apply(face, scaler, direction, buffer);
                }
            }
        }
    }
}

/// Post-processing after positioning: stretches `stch` tiles.
pub(crate) fn postprocess(
    face: &Face<'_>,
    scaler: &Scaler<'_>,
    direction: Direction,
    buffer: &mut Buffer,
) {
    stretch::apply(face, scaler, direction, buffer);
}

/// Modifier combining marks (UAX #53) that HarfBuzz moves before the
/// other marks of their combining class.
fn is_modifier_mark(c: char) -> bool {
    matches!(
        c,
        '\u{0654}'
            | '\u{0655}'
            | '\u{0658}'
            | '\u{06DC}'
            | '\u{06E3}'
            | '\u{06E7}'
            | '\u{06E8}'
            | '\u{08CA}'
            | '\u{08CB}'
            | '\u{08CD}'
            | '\u{08CE}'
            | '\u{08CF}'
            | '\u{08D3}'
            | '\u{08F3}'
    )
}

/// HarfBuzz's `reorder_marks_arabic`: within the canonically ordered run of
/// marks `start..end` of `chars`, modifier marks of class 220 and 230 move
/// to the front and get classes that keep the run sorted.
pub(crate) fn reorder_marks(chars: &mut [ClusterChar], start: usize, end: usize) {
    let mut start = start;
    let mut i = start;
    for (class, new_class) in [(220, MODIFIER_BELOW_CLASS), (230, MODIFIER_ABOVE_CLASS)] {
        while i < end && chars[i].props.combining_class < class {
            i += 1;
        }
        if i == end {
            break;
        }
        if chars[i].props.combining_class > class {
            continue;
        }
        let j = (i..end)
            .find(|&j| chars[j].props.combining_class != class || !is_modifier_mark(chars[j].c))
            .unwrap_or(end);
        if i == j {
            continue;
        }
        merge_clusters(chars, start, j);
        chars[start..j].rotate_right(j - i);
        let moved = start + j - i;
        for mark in &mut chars[start..moved] {
            mark.props.combining_class = new_class;
        }
        start = moved;
        i = j;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(text: &str) -> Vec<ClusterChar> {
        text.chars().map(|c| ClusterChar::new(c, 0, 0)).collect()
    }

    #[test]
    fn modifier_marks_move_first() {
        // KASRA (220 after modification: 33) stays; HAMZA BELOW (220) and
        // HAMZA ABOVE (230) are modifiers.
        let mut run = marks("\u{628}\u{650}\u{655}\u{64E}\u{654}");
        run[1..].sort_by_key(|m| m.props.combining_class);
        reorder_marks(&mut run, 1, 5);
        let text: String = run.iter().map(|m| m.c).collect();
        assert_eq!(text, "\u{628}\u{655}\u{654}\u{64E}\u{650}");
        let classes: Vec<u8> = run.iter().map(|m| m.props.combining_class).collect();
        assert_eq!(classes, vec![0, 25, 26, 31, 33]);
        let mut plain = marks("\u{64E}\u{650}");
        reorder_marks(&mut plain, 0, 2);
        assert_eq!(plain[0].c, '\u{64E}');
    }

    #[test]
    fn plan_masks_and_fallback() {
        assert!(is_syriac(*b"fin2") && !is_syriac(*b"fina"));
        assert!(is_modifier_mark('\u{654}') && !is_modifier_mark('\u{64E}'));
    }
}
