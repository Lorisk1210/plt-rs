//! OpenType features as AAT features (HarfBuzz's `hb_aat_map_builder_t`
//! and the feature mapping of `hb-aat-layout.cc`): the user's features
//! become AAT feature types and settings the font's `feat` table names,
//! which switch the flags of `morx` chains, per range of clusters.

use crate::aat::state::RangeFlags;
use crate::reader::peek;
use crate::types::Tag;

/// The AAT feature type of character alternatives (`aalt`).
const CHARACTER_ALTERNATIVES: u16 = 17;
/// The AAT feature types and settings small capitals moved between.
pub(crate) const LETTER_CASE: u16 = 3;
/// The deprecated small-capitals setting of [`LETTER_CASE`].
pub(crate) const SMALL_CAPS: u16 = 3;
/// The current small-capitals feature type.
pub(crate) const LOWER_CASE: u16 = 37;
/// The current small-capitals setting of [`LOWER_CASE`].
pub(crate) const LOWER_CASE_SMALL_CAPS: u16 = 1;
/// The AAT feature type selecting a language (by `ltag` index).
pub(crate) const LANGUAGE_TAG_TYPE: u16 = 39;
/// The `feat` flag of features whose settings exclude each other.
const EXCLUSIVE: u16 = 0x8000;

/// OpenType feature tags with the AAT feature type and the settings that
/// enable and disable it (HarfBuzz's `feature_mappings`, from Apple).
const MAPPINGS: [(&[u8; 4], u16, u16, u16); 78] = [
    (b"afrc", 11, 1, 0),
    (b"c2pc", 38, 2, 0),
    (b"c2sc", 38, 1, 0),
    (b"calt", 36, 0, 1),
    (b"case", 33, 0, 1),
    (b"clig", 1, 18, 19),
    (b"cpsp", 33, 2, 3),
    (b"cswh", 36, 4, 5),
    (b"dlig", 1, 4, 5),
    (b"expt", 20, 10, 16),
    (b"frac", 11, 2, 0),
    (b"fwid", 22, 1, 7),
    (b"halt", 22, 6, 7),
    (b"hist", 40, 0, 1),
    (b"hkna", 34, 0, 1),
    (b"hlig", 1, 20, 21),
    (b"hngl", 23, 1, 0),
    (b"hojo", 20, 12, 16),
    (b"hwid", 22, 2, 7),
    (b"ital", 32, 2, 3),
    (b"jp04", 20, 11, 16),
    (b"jp78", 20, 2, 16),
    (b"jp83", 20, 3, 16),
    (b"jp90", 20, 4, 16),
    (b"liga", 1, 2, 3),
    (b"lnum", 21, 1, 2),
    (b"mgrk", 15, 10, 11),
    (b"nlck", 20, 13, 16),
    (b"onum", 21, 0, 2),
    (b"ordn", 10, 3, 0),
    (b"palt", 22, 5, 7),
    (b"pcap", 37, 2, 0),
    (b"pkna", 22, 0, 7),
    (b"pnum", 6, 1, 4),
    (b"pwid", 22, 0, 7),
    (b"qwid", 22, 4, 7),
    (b"rlig", 1, 0, 1),
    (b"ruby", 28, 2, 3),
    (b"sinf", 10, 4, 0),
    (b"smcp", 37, 1, 0),
    (b"smpl", 20, 1, 16),
    (b"ss01", 35, 2, 3),
    (b"ss02", 35, 4, 5),
    (b"ss03", 35, 6, 7),
    (b"ss04", 35, 8, 9),
    (b"ss05", 35, 10, 11),
    (b"ss06", 35, 12, 13),
    (b"ss07", 35, 14, 15),
    (b"ss08", 35, 16, 17),
    (b"ss09", 35, 18, 19),
    (b"ss10", 35, 20, 21),
    (b"ss11", 35, 22, 23),
    (b"ss12", 35, 24, 25),
    (b"ss13", 35, 26, 27),
    (b"ss14", 35, 28, 29),
    (b"ss15", 35, 30, 31),
    (b"ss16", 35, 32, 33),
    (b"ss17", 35, 34, 35),
    (b"ss18", 35, 36, 37),
    (b"ss19", 35, 38, 39),
    (b"ss20", 35, 40, 41),
    (b"subs", 10, 2, 0),
    (b"sups", 10, 1, 0),
    (b"swsh", 36, 2, 3),
    (b"titl", 19, 4, 0),
    (b"tnam", 20, 14, 16),
    (b"tnum", 6, 0, 4),
    (b"trad", 20, 0, 16),
    (b"twid", 22, 3, 7),
    (b"unic", 3, 14, 15),
    (b"valt", 22, 5, 7),
    (b"vert", 4, 0, 1),
    (b"vhal", 22, 6, 7),
    (b"vkna", 34, 2, 3),
    (b"vpal", 22, 5, 7),
    (b"vrt2", 4, 0, 1),
    (b"vrtr", 4, 2, 3),
    (b"zero", 14, 4, 5),
];

/// An AAT feature setting the user asked for (HarfBuzz's
/// `feature_info_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AatFeature {
    /// The feature type.
    pub(crate) kind: u16,
    /// The setting (selector).
    pub(crate) setting: u16,
    /// Whether the type's settings exclude each other.
    exclusive: bool,
    /// The order it was asked in, for a stable sort.
    seq: usize,
}

impl AatFeature {
    /// HarfBuzz's sort order: by type, then (for non-exclusive types) by
    /// setting pair, then in the order asked.
    fn sort_order(&self, other: &Self) -> std::cmp::Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| {
                if !self.exclusive && (self.setting & !1) != (other.setting & !1) {
                    self.setting.cmp(&other.setting)
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then(self.seq.cmp(&other.seq))
    }

    /// Whether `self` and `other` set the same type and setting.
    const fn same_setting(&self, other: &Self) -> bool {
        self.kind == other.kind && self.setting == other.setting
    }
}

/// The AAT features active somewhere, in HarfBuzz's sorted order, as a
/// chain's flags are compiled from them.
#[derive(Clone, Debug, Default)]
pub(crate) struct ActiveFeatures(Vec<AatFeature>);

impl ActiveFeatures {
    /// Whether `kind` with `setting` is active, by HarfBuzz's binary
    /// search over the sorted features.
    pub(crate) fn contains(&self, kind: u16, setting: u16) -> bool {
        let key = (kind, setting);
        let (mut min, mut max) = (0_isize, self.0.len() as isize - 1);
        while min <= max {
            let mid = isize::midpoint(min, max);
            let feature = self.0[mid as usize];
            match key.cmp(&(feature.kind, feature.setting)) {
                std::cmp::Ordering::Less => max = mid - 1,
                std::cmp::Ordering::Greater => min = mid + 1,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }
}

/// An AAT feature set for a range of clusters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FeatureRange {
    /// The feature setting.
    pub(crate) feature: AatFeature,
    /// The first cluster.
    pub(crate) start: usize,
    /// The cluster after the last.
    pub(crate) end: usize,
}

/// A feature setting as the user gave it: tag, value and the clusters it
/// covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UserFeature {
    /// The OpenType tag.
    pub(crate) tag: Tag,
    /// The value (0 disables).
    pub(crate) value: u32,
    /// The first cluster.
    pub(crate) start: usize,
    /// The cluster after the last.
    pub(crate) end: usize,
}

/// The `feat` table: the feature types a font has, with their settings.
#[derive(Clone, Copy, Debug)]
struct Feat<'a> {
    data: &'a [u8],
}

impl Feat<'_> {
    /// The number of settings and the flags of feature type `kind`; zero
    /// settings when the font does not name it.
    fn feature(&self, kind: u16) -> (u16, u16) {
        let count = peek::<u16>(self.data, 4).unwrap_or(0);
        let (mut min, mut max) = (0_i32, i32::from(count) - 1);
        while min <= max {
            let mid = i32::midpoint(min, max);
            let at = 12 + 12 * mid as usize;
            let Some(found) = peek::<u16>(self.data, at) else {
                break;
            };
            match kind.cmp(&found) {
                std::cmp::Ordering::Less => max = mid - 1,
                std::cmp::Ordering::Greater => min = mid + 1,
                std::cmp::Ordering::Equal => {
                    return (
                        peek::<u16>(self.data, at + 2).unwrap_or(0),
                        peek::<u16>(self.data, at + 8).unwrap_or(0),
                    );
                }
            }
        }
        (0, 0)
    }
}

/// The user's `features` as AAT features of a font with the `feat` table
/// `feat` (HarfBuzz's `hb_aat_map_builder_t::add_feature`): features the
/// font does not name are dropped.
pub(crate) fn aat_features(feat: Option<&[u8]>, features: &[UserFeature]) -> Vec<FeatureRange> {
    let Some(feat) = feat
        .map(|data| Feat { data })
        .filter(|feat| peek::<u32>(feat.data, 0).is_some_and(|version| version != 0))
    else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    for feature in features {
        let seq = ranges.len() + 1;
        if feature.tag == Tag::new(b"aalt") {
            if feat.feature(CHARACTER_ALTERNATIVES).0 == 0 {
                continue;
            }
            ranges.push(FeatureRange {
                feature: AatFeature {
                    kind: CHARACTER_ALTERNATIVES,
                    setting: feature.value as u16,
                    exclusive: true,
                    seq,
                },
                start: feature.start,
                end: feature.end,
            });
            continue;
        }
        let Some(&(_, kind, enable, disable)) = MAPPINGS
            .iter()
            .find(|(tag, ..)| Tag::new(tag) == feature.tag)
        else {
            continue;
        };
        let (mut settings, mut flags) = feat.feature(kind);
        if settings == 0 {
            // Small capitals may live under their deprecated type.
            if kind == LOWER_CASE && enable == LOWER_CASE_SMALL_CAPS {
                (settings, flags) = feat.feature(LETTER_CASE);
            }
            if settings == 0 {
                continue;
            }
        }
        ranges.push(FeatureRange {
            feature: AatFeature {
                kind,
                setting: if feature.value != 0 { enable } else { disable },
                exclusive: flags & EXCLUSIVE != 0,
                seq,
            },
            start: feature.start,
            end: feature.end,
        });
    }
    ranges
}

/// The flags of each chain, per range of clusters (HarfBuzz's
/// `hb_aat_map_builder_t::compile`): `chain_flags` gives the flags of
/// every chain for the features active in a range.
pub(crate) fn compile(
    features: &[FeatureRange],
    chain_flags: impl Fn(&ActiveFeatures) -> Vec<u32>,
) -> Vec<Vec<RangeFlags>> {
    let ranges = |flags: Vec<u32>, first: usize, last: usize| -> Vec<RangeFlags> {
        flags
            .into_iter()
            .map(|flags| RangeFlags { flags, first, last })
            .collect()
    };
    if features.is_empty() {
        return ranges(chain_flags(&ActiveFeatures::default()), 0, usize::MAX)
            .into_iter()
            .map(|range| vec![range])
            .collect();
    }
    // Start and end events, sorted, then a final one past every cluster.
    let mut events: Vec<(usize, bool, AatFeature)> = features
        .iter()
        .filter(|range| range.start != range.end)
        .flat_map(|range| {
            [
                (range.start, true, range.feature),
                (range.end, false, range.feature),
            ]
        })
        .collect();
    events.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then_with(|| a.2.sort_order(&b.2))
    });
    let last = AatFeature {
        kind: 0,
        setting: 0,
        exclusive: false,
        seq: features.len() + 1,
    };
    events.push((usize::MAX, false, last));
    let mut chains: Vec<Vec<RangeFlags>> = Vec::new();
    let mut active: Vec<AatFeature> = Vec::new();
    let mut last_index = 0;
    for (index, start, feature) in events {
        if index != last_index {
            let mut current = active.clone();
            current.sort_by(AatFeature::sort_order);
            current.dedup_by(|later, kept| {
                later.kind == kept.kind
                    && (later.exclusive || (later.setting & !1) == (kept.setting & !1))
            });
            let flags = chain_flags(&ActiveFeatures(current));
            if chains.is_empty() {
                chains = vec![Vec::new(); flags.len()];
            }
            for (chain, flags) in chains.iter_mut().zip(flags) {
                chain.push(RangeFlags {
                    flags,
                    first: last_index,
                    last: index.saturating_sub(1),
                });
            }
            last_index = index;
        }
        if start {
            active.push(feature);
        } else if let Some(at) = active.iter().position(|a| a.same_setting(&feature)) {
            active.remove(at);
        }
    }
    for chain in &mut chains {
        if let Some(range) = chain.last_mut() {
            range.last = usize::MAX;
        }
    }
    chains
}

/// The language the `ltag` table `ltag` lists at `index`, lower-cased as
/// HarfBuzz keeps languages.
pub(crate) fn ltag_language(ltag: &[u8], index: u32) -> Option<String> {
    let count = peek::<u32>(ltag, 8)?;
    if index >= count {
        return None;
    }
    let at = 12 + 4 * usize::try_from(index).ok()?;
    let offset = usize::from(peek::<u16>(ltag, at)?);
    let length = usize::from(peek::<u16>(ltag, at + 2)?);
    let bytes = ltag.get(offset..offset + length)?;
    Some(String::from_utf8_lossy(bytes).to_ascii_lowercase())
}

/// HarfBuzz's `hb_language_matches`: whether `language` is `specific` or
/// a prefix of it ending at a subtag.
pub(crate) fn language_matches(language: &str, specific: &str) -> bool {
    specific
        .strip_prefix(language)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `feat` table naming ligatures (non-exclusive, 4 settings) and
    /// letter case (exclusive).
    fn feat() -> Vec<u8> {
        let mut data = vec![0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0];
        for (kind, flags) in [(1_u16, 0_u16), (3, EXCLUSIVE)] {
            for word in [kind, 4, 0, 0, flags, 260] {
                data.extend_from_slice(&word.to_be_bytes());
            }
        }
        data
    }

    fn user(tag: [u8; 4], value: u32, start: usize, end: usize) -> UserFeature {
        UserFeature {
            tag: Tag::new(&tag),
            value,
            start,
            end,
        }
    }

    #[test]
    fn features_map_to_named_types() {
        let feat = feat();
        let ranges = aat_features(
            Some(&feat),
            &[
                user(*b"liga", 0, 0, usize::MAX),
                user(*b"smcp", 1, 2, 5),
                user(*b"zero", 1, 0, usize::MAX),
                user(*b"kern", 0, 0, usize::MAX),
            ],
        );
        assert_eq!(ranges.len(), 2);
        assert_eq!((ranges[0].feature.kind, ranges[0].feature.setting), (1, 3));
        // Small capitals fall back to the letter-case type's presence but
        // keep the lower-case type.
        assert_eq!((ranges[1].feature.kind, ranges[1].feature.setting), (37, 1));
        assert!(ranges[1].feature.exclusive);
        assert_eq!(aat_features(None, &[user(*b"liga", 0, 0, 1)]), Vec::new());
    }

    #[test]
    fn ranges_split_where_features_change() {
        let feat = feat();
        let features = aat_features(
            Some(&feat),
            &[user(*b"liga", 0, 3, 6), user(*b"dlig", 1, 0, usize::MAX)],
        );
        let chains = compile(&features, |active| {
            vec![u32::from(active.contains(1, 3)) | (u32::from(active.contains(1, 4)) << 1)]
        });
        let ranges: Vec<(u32, usize, usize)> = chains[0]
            .iter()
            .map(|range| (range.flags, range.first, range.last))
            .collect();
        assert_eq!(ranges, [(2, 0, 2), (3, 3, 5), (2, 6, usize::MAX)]);
        let plain = compile(&[], |_| vec![7, 8]);
        assert_eq!(plain.len(), 2);
        assert_eq!(
            plain[1],
            [RangeFlags {
                flags: 8,
                first: 0,
                last: usize::MAX
            }]
        );
    }

    #[test]
    fn languages_match_by_prefix() {
        assert!(language_matches("en", "en-us"));
        assert!(language_matches("en", "en"));
        assert!(!language_matches("en", "eng"));
        assert!(!language_matches("en-us", "en"));
        let mut ltag = vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 16, 0, 2];
        ltag.extend_from_slice(b"TR");
        assert_eq!(ltag_language(&ltag, 0).as_deref(), Some("tr"));
        assert_eq!(ltag_language(&ltag, 1), None);
    }
}
