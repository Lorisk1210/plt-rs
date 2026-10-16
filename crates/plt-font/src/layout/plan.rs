//! Feature selection: which `GSUB`/`GPOS` lookups HarfBuzz would apply for
//! a script, language and feature list.

use std::fmt;
use std::str::FromStr;

use crate::face::Face;
use crate::layout::common::{LangSys, LayoutTable};
use crate::types::Tag;

/// An OpenType feature setting applied to the whole text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Feature {
    /// Feature tag, e.g. `kern`.
    pub tag: Tag,
    /// Feature value: 0 disables, 1 enables, larger values select
    /// alternates.
    pub value: u32,
}

impl Feature {
    /// A feature with an explicit value.
    pub const fn new(tag: Tag, value: u32) -> Self {
        Self { tag, value }
    }
}

/// Error returned when a feature string cannot be parsed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParseFeatureError;

impl fmt::Display for ParseFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid OpenType feature string")
    }
}

impl std::error::Error for ParseFeatureError {}

impl FromStr for Feature {
    type Err = ParseFeatureError;

    /// Parses HarfBuzz-style global feature strings: `kern`, `+kern`,
    /// `-kern`, `kern=0`, `aalt=2` (ranges are not supported).
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (text, default) = match text.as_bytes().first() {
            Some(b'-') => (&text[1..], 0),
            Some(b'+') => (&text[1..], 1),
            _ => (text, 1),
        };
        let (name, value) = match text.split_once('=') {
            Some((name, value)) => match value.trim() {
                "on" | "true" => (name, 1),
                "off" | "false" => (name, 0),
                number => (name, number.parse().map_err(|_| ParseFeatureError)?),
            },
            None => (text, default),
        };
        let tag = Tag::from_str_padded(name.trim().trim_matches(|c| c == '"' || c == '\''))
            .ok_or(ParseFeatureError)?;
        Ok(Self { tag, value })
    }
}

/// Features HarfBuzz enables by default for horizontal left-to-right text
/// with the default shaper, in stage order (`rvrn` alone in the first
/// `GSUB` stage).
const DEFAULT_FEATURES: [&[u8; 4]; 17] = [
    b"rvrn", b"ltra", b"ltrm", b"abvm", b"blwm", b"ccmp", b"locl", b"mark", b"mkmk", b"rlig",
    b"calt", b"clig", b"curs", b"dist", b"kern", b"liga", b"rclt",
];

const KERN: Tag = Tag::new(b"kern");
const RVRN: Tag = Tag::new(b"rvrn");

/// Lookups to apply for one shaping run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) gsub: Vec<u16>,
    pub(crate) gpos: Vec<u16>,
    /// The font has a `GPOS` table, which is then used for positioning.
    pub(crate) apply_gpos: bool,
    /// The legacy `kern` table is used (no `GPOS` kerning available).
    pub(crate) apply_kern_table: bool,
}

/// HarfBuzz's script selection: the script's own tag, then `DFLT`, `dflt`
/// and `latn`.
fn select_lang_sys<'a>(
    table: &LayoutTable<'a>,
    script: Option<Tag>,
    language: Option<Tag>,
) -> Option<LangSys<'a>> {
    let candidates = script
        .into_iter()
        .chain([b"DFLT", b"dflt", b"latn"].map(Tag::new));
    let tags: Vec<Tag> = table.script_tags().collect();
    let chosen = candidates.into_iter().find(|tag| tags.contains(tag))?;
    table.lang_sys(chosen, language)
}

/// Index of the first feature of `lang_sys` with `tag`.
fn find_feature(table: &LayoutTable<'_>, lang_sys: &LangSys<'_>, tag: Tag) -> Option<u16> {
    lang_sys
        .feature_indices
        .iter()
        .find(|&index| table.feature(index).is_some_and(|(t, _)| t == tag))
}

/// Lookups of feature `tag` for `script` (with HarfBuzz's script
/// fallbacks), or `None` when the feature is absent.
pub(crate) fn feature_lookups(
    table: &LayoutTable<'_>,
    script: Option<Tag>,
    tag: Tag,
) -> Option<Vec<u16>> {
    let lang_sys = select_lang_sys(table, script, None)?;
    let (_, lookups) = table.feature(find_feature(table, &lang_sys, tag)?)?;
    Some(lookups.iter().collect())
}

/// Enabled feature tags, with user settings overriding the defaults.
fn enabled_features(user: &[Feature]) -> Vec<(Tag, u32)> {
    let mut features: Vec<(Tag, u32)> = DEFAULT_FEATURES
        .iter()
        .map(|&tag| (Tag::new(tag), 1))
        .collect();
    for feature in user {
        match features.iter_mut().find(|(tag, _)| *tag == feature.tag) {
            Some(entry) => entry.1 = feature.value,
            None => features.push((feature.tag, feature.value)),
        }
    }
    features.retain(|&(_, value)| value != 0);
    features
}

/// Lookup indices for one table, stage by stage, each stage sorted.
fn collect(
    table: &LayoutTable<'_>,
    lang_sys: Option<&LangSys<'_>>,
    features: &[(Tag, u32)],
    staged: bool,
) -> Vec<u16> {
    let Some(lang_sys) = lang_sys else {
        return Vec::new();
    };
    let mut stages: [Vec<u16>; 2] = [Vec::new(), Vec::new()];
    let mut add = |stage: usize, index: u16| {
        if let Some((_, lookups)) = table.feature(index) {
            stages[stage].extend(lookups.iter());
        }
    };
    if let Some(required) = lang_sys.required_feature {
        add(0, required);
    }
    for &(tag, _) in features {
        if let Some(index) = find_feature(table, lang_sys, tag) {
            add(usize::from(staged && tag != RVRN), index);
        }
    }
    stages
        .into_iter()
        .flat_map(|mut stage| {
            stage.sort_unstable();
            stage.dedup();
            stage
        })
        .collect()
}

/// Builds the plan for `face` shaping text in `script` (OpenType tag).
pub(crate) fn plan(
    face: &Face<'_>,
    script: Option<Tag>,
    language: Option<Tag>,
    user: &[Feature],
) -> Plan {
    let features = enabled_features(user);
    let gsub = face.gsub().map(|table| {
        let lang_sys = select_lang_sys(table, script, language);
        collect(table, lang_sys.as_ref(), &features, true)
    });
    let gpos = face.gpos().map(|table| {
        let lang_sys = select_lang_sys(table, script, language);
        let kern_enabled = features.iter().any(|&(tag, _)| tag == KERN);
        let has_kern = kern_enabled
            && lang_sys
                .as_ref()
                .is_some_and(|ls| find_feature(table, ls, KERN).is_some());
        (
            collect(table, lang_sys.as_ref(), &features, false),
            has_kern,
        )
    });
    let apply_gpos = gpos.is_some();
    let has_gpos_kern = gpos.as_ref().is_some_and(|(_, has_kern)| *has_kern);
    let kern_enabled = features.iter().any(|&(tag, _)| tag == KERN);
    Plan {
        gsub: gsub.unwrap_or_default(),
        gpos: gpos.map(|(lookups, _)| lookups).unwrap_or_default(),
        apply_gpos,
        apply_kern_table: kern_enabled && !has_gpos_kern && face.has_legacy_kerning(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_feature_strings() {
        let kern = Tag::new(b"kern");
        assert_eq!("kern".parse(), Ok(Feature::new(kern, 1)));
        assert_eq!("-kern".parse(), Ok(Feature::new(kern, 0)));
        assert_eq!("+kern".parse(), Ok(Feature::new(kern, 1)));
        assert_eq!("kern=0".parse(), Ok(Feature::new(kern, 0)));
        assert_eq!("aalt=2".parse(), Ok(Feature::new(Tag::new(b"aalt"), 2)));
        assert_eq!("liga=off".parse(), Ok(Feature::new(Tag::new(b"liga"), 0)));
        assert_eq!("ss1".parse(), Ok(Feature::new(Tag::new(b"ss1 "), 1)));
        assert_eq!("".parse::<Feature>(), Err(ParseFeatureError));
        assert_eq!("kern=x".parse::<Feature>(), Err(ParseFeatureError));
        assert_eq!("toolong".parse::<Feature>(), Err(ParseFeatureError));
        assert!(ParseFeatureError.to_string().contains("feature"));
    }

    #[test]
    fn user_features_override_defaults() {
        let features =
            enabled_features(&[Feature::new(KERN, 0), Feature::new(Tag::new(b"smcp"), 1)]);
        assert!(!features.iter().any(|&(tag, _)| tag == KERN));
        assert!(features.iter().any(|&(tag, _)| tag == Tag::new(b"smcp")));
        assert!(features.iter().any(|&(tag, _)| tag == Tag::new(b"liga")));
    }
}
