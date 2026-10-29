//! HarfBuzz's `hb_ot_tags_from_script_and_language` (`hb-ot-tag.cc`): the
//! OpenType script tags and language system tags a run looks for in a
//! font, in preference order.
//!
//! Language tags come from the BCP 47 language: a private-use override
//! (`x-hbotTRK`, `x-hbot-54524b20`), else the multi-subtag rules of
//! [`complex`], else HarfBuzz's tables for the primary (or extended
//! language) subtag, else the upper-cased three-letter subtag. Script
//! tags are the script's new and old OpenType tags, unless the language
//! overrides them (`x-hbsccyrl`).

mod complex;
mod tables;

use crate::shape::{LanguageTag, Script};
use crate::types::Tag;
use complex::complex_language_tags;
use tables::{LANGUAGES2, LANGUAGES3, LANGUAGES3_BLOCKED, LANGUAGES3_MULTI};

/// Most tags looked for per script and per language
/// (`HB_OT_MAX_TAGS_PER_SCRIPT`, `HB_OT_MAX_TAGS_PER_LANGUAGE`).
const MAX_TAGS: usize = 3;

/// The default script tag (`HB_OT_TAG_DEFAULT_SCRIPT`), which private-use
/// overrides spell in either case.
const DEFAULT_SCRIPT: Tag = Tag::new(b"DFLT");

/// The bits clearing ASCII case in each byte of a tag.
const CASE_BITS: u32 = 0x2020_2020;

/// Scripts with "new" Indic OpenType tags (`hb_ot_new_tag_from_script`),
/// tried before the old tag; all but Myanmar also have a version 3 tag.
const NEW_SCRIPT_TAGS: [(&str, &[u8; 4]); 10] = [
    ("Beng", b"bng2"),
    ("Deva", b"dev2"),
    ("Gujr", b"gjr2"),
    ("Guru", b"gur2"),
    ("Knda", b"knd2"),
    ("Mlym", b"mlm2"),
    ("Orya", b"ory2"),
    ("Taml", b"tml2"),
    ("Telu", b"tel2"),
    ("Mymr", b"mym2"),
];

/// The tags a run of one script and language looks for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OtTags {
    /// Script tags, most preferred first.
    pub(crate) scripts: Vec<Tag>,
    /// Language system tags, most preferred first (empty: the default
    /// language system).
    pub(crate) languages: Vec<Tag>,
}

impl OtTags {
    /// The tags of `script` in `language` (`None`: no language, as for
    /// HarfBuzz's invalid language).
    pub(crate) fn new(script: Script, language: Option<&LanguageTag>) -> Self {
        let Some(language) = language else {
            return Self {
                scripts: script_tags(script),
                languages: Vec::new(),
            };
        };
        let language = language.canonical().as_bytes();
        let (private_use, limit) = split_private_use(language);
        let script_override =
            private_use.and_then(|subtag| private_use_tag(subtag, b"-hbsc", Case::Lower));
        let language_override =
            private_use.and_then(|subtag| private_use_tag(subtag, b"-hbot", Case::Upper));
        Self {
            scripts: script_override.map_or_else(|| script_tags(script), |tag| vec![tag]),
            languages: language_override.map_or_else(
                || limit.map_or_else(Vec::new, |limit| language_tags(language, limit)),
                |tag| vec![tag],
            ),
        }
    }
}

/// HarfBuzz's `hb_ot_all_tags_from_script`: the new Indic tags (version 3,
/// then 2) and the old tag; Common, Inherited and Unknown have none a font
/// would list.
fn script_tags(script: Script) -> Vec<Tag> {
    let mut tags = Vec::with_capacity(MAX_TAGS);
    if let Some(&(_, new)) = NEW_SCRIPT_TAGS
        .iter()
        .find(|(code, _)| *code == script.iso_code())
    {
        if new != b"mym2" {
            tags.push(Tag::new(&[new[0], new[1], new[2], b'3']));
        }
        tags.push(Tag::new(new));
    }
    tags.extend(script.opentype_tag());
    tags
}

/// Where the private-use subtags of `language` start (at the `x`), and
/// HarfBuzz's limit: the `-` before the first singleton subtag, the end
/// without one, `None` for a language that is all private use (`x-...`).
fn split_private_use(language: &[u8]) -> (Option<&[u8]>, Option<usize>) {
    if language.starts_with(b"x-") {
        return (Some(language), None);
    }
    let mut limit = None;
    for index in 1..language.len() {
        let singleton = language[index - 1] == b'-' && language.get(index + 1) == Some(&b'-');
        if singleton {
            limit.get_or_insert(index - 1);
            if language[index] == b'x' {
                return (Some(&language[index..]), limit);
            }
        }
    }
    (None, Some(limit.unwrap_or(language.len())))
}

/// How a private-use override is normalised.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Case {
    Lower,
    Upper,
}

/// HarfBuzz's `parse_private_use_subtag`: the tag following `prefix` in
/// the private-use subtags, as 8 hex digits after a `-` or as up to 4
/// letters and digits.
fn private_use_tag(private_use: &[u8], prefix: &[u8], case: Case) -> Option<Tag> {
    let at = private_use
        .windows(prefix.len())
        .position(|window| window == prefix)?;
    let rest = &private_use[at + prefix.len()..];
    let mut bytes = [b' '; 4];
    if let Some(hex) = rest.strip_prefix(b"-") {
        let digits = hex
            .get(..8)
            .filter(|d| d.iter().all(u8::is_ascii_hexdigit))?;
        for (byte, pair) in bytes.iter_mut().zip(digits.chunks(2)) {
            let text = std::str::from_utf8(pair).ok()?;
            *byte = u8::from_str_radix(text, 16).ok()?;
        }
    } else {
        let len = rest
            .iter()
            .take(4)
            .take_while(|c| c.is_ascii_alphanumeric())
            .count();
        if len == 0 {
            return None;
        }
        for (byte, &c) in bytes.iter_mut().zip(&rest[..len]) {
            *byte = match case {
                Case::Lower => c.to_ascii_lowercase(),
                Case::Upper => c.to_ascii_uppercase(),
            };
        }
    }
    let tag = Tag::new(&bytes).to_u32();
    Some(Tag::from_u32(
        if tag & !CASE_BITS == DEFAULT_SCRIPT.to_u32() {
            tag ^ CASE_BITS
        } else {
            tag
        },
    ))
}

/// HarfBuzz's `hb_ot_tags_from_language` for a canonical `language`
/// considered up to `limit`.
fn language_tags(language: &[u8], limit: usize) -> Vec<Tag> {
    if let Some(tags) = complex_language_tags(language, limit) {
        return tags.iter().copied().take(MAX_TAGS).collect();
    }
    let mut start = 0;
    if let Some(dash) = language.iter().position(|&c| c == b'-')
        && limit >= 6
    {
        // An extended language subtag (`zh-yue`) stands for the language.
        let extended = &language[dash + 1..];
        let len = extended
            .iter()
            .position(|&c| c == b'-')
            .unwrap_or(extended.len());
        if len == 3 && extended[0].is_ascii_alphabetic() {
            start = dash + 1;
        }
    }
    let primary = &language[start..];
    let len = primary
        .iter()
        .position(|&c| c == b'-')
        .unwrap_or_else(|| limit.saturating_sub(start));
    match *primary {
        [a, b, ..] if len == 2 => {
            let first = LANGUAGES2.partition_point(|(key, _)| *key < [a, b]);
            LANGUAGES2[first..]
                .iter()
                .take_while(|(key, _)| *key == [a, b])
                .take(MAX_TAGS)
                .map(|&(_, tag)| tag)
                .collect()
        }
        [a, b, c, ..] if len == 3 => three_letter_tags([a, b, c]),
        _ => Vec::new(),
    }
}

/// The tags of a three-letter language: its table entries, none when it
/// is blocked, else the code upper-cased (assumed to be ISO 639-3).
fn three_letter_tags(key: [u8; 3]) -> Vec<Tag> {
    if let Ok(index) = LANGUAGES3.binary_search_by_key(&key, |&(language, _)| language) {
        return vec![LANGUAGES3[index].1];
    }
    if let Ok(index) = LANGUAGES3_MULTI.binary_search_by_key(&key, |&(language, _)| language) {
        return LANGUAGES3_MULTI[index]
            .1
            .iter()
            .copied()
            .take(MAX_TAGS)
            .collect();
    }
    if LANGUAGES3_BLOCKED.binary_search(&key).is_ok() {
        return Vec::new();
    }
    // HarfBuzz clears bit 5 of each byte, which upper-cases letters (and
    // also changes digits).
    let [a, b, c] = key.map(|byte| byte & !0x20);
    vec![Tag::new(&[a, b, c, b' '])]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(script: &str, language: &str) -> (Vec<String>, Vec<String>) {
        let script = Script::from_iso_code(script).expect("known script");
        let tags = OtTags::new(script, Some(&LanguageTag::new(language)));
        let names = |tags: Vec<Tag>| tags.iter().map(ToString::to_string).collect();
        (names(tags.scripts), names(tags.languages))
    }

    fn languages(language: &str) -> Vec<String> {
        tags("Latn", language).1
    }

    #[test]
    fn script_tags_list_new_indic_tags_first() {
        assert_eq!(tags("Deva", "").0, ["dev3", "dev2", "deva"]);
        assert_eq!(tags("Mymr", "").0, ["mym2", "mymr"]);
        assert_eq!(tags("Latn", "").0, ["latn"]);
        assert_eq!(tags("Hira", "").0, ["kana"]);
        let common = OtTags::new(Script::COMMON, None);
        assert!(common.scripts.is_empty() && common.languages.is_empty());
    }

    #[test]
    fn two_letter_languages_use_the_table() {
        // hb-ot-tag-table.hh: tr -> TRK; mo -> MOL, ROM; bi -> BIS, CPP.
        assert_eq!(languages("tr"), ["TRK "]);
        assert_eq!(languages("TR_tr"), ["TRK "]);
        assert_eq!(languages("mo"), ["MOL ", "ROM "]);
        assert_eq!(languages("bi"), ["BIS ", "CPP "]);
        assert_eq!(
            languages("qq"),
            Vec::<String>::new(),
            "unknown two-letter code"
        );
        assert_eq!(languages("sr-Latn-RS"), ["SRB "]);
    }

    #[test]
    fn three_letter_languages_tables_blocks_and_fallback() {
        assert_eq!(languages("crh"), ["CRT "]);
        assert_eq!(languages("azb"), ["AZB ", "AZE "]);
        assert_eq!(languages("xsl"), ["SSL ", "SLA ", "ATH "]);
        assert_eq!(languages("trk"), Vec::<String>::new(), "Turkic is blocked");
        assert_eq!(languages("gag"), ["GAG "]);
        let digit = OtTags::new(Script::COMMON, Some(&LanguageTag::new("a1b")));
        assert_eq!(digit.languages, [Tag::new(b"A\x11B ")], "bit 5 cleared");
        assert_eq!(languages("zh-yue"), ["ZHH "], "extended language subtag");
        assert_eq!(languages("toolong"), Vec::<String>::new());
        assert_eq!(languages(""), Vec::<String>::new());
        assert_eq!(languages("@"), Vec::<String>::new());
    }

    #[test]
    fn complex_languages_come_first() {
        assert_eq!(languages("zh-Hant-HK"), ["ZHH "]);
        assert_eq!(languages("ro-MD"), ["MOL ", "ROM "]);
        assert_eq!(languages("el-polyton"), ["PGR "]);
        assert_eq!(languages("art-lojban"), ["JBO "]);
    }

    #[test]
    fn private_use_overrides() {
        assert_eq!(languages("x-hbottrk"), ["TRK "]);
        assert_eq!(languages("en-x-hbotcat"), ["CAT "]);
        assert_eq!(languages("x-hbot-54524b20"), ["TRK "]);
        assert_eq!(
            languages("x-hbot-5452"),
            Vec::<String>::new(),
            "too few hex digits"
        );
        assert_eq!(languages("x-foo"), Vec::<String>::new());
        assert_eq!(languages("tr-x-foo"), ["TRK "]);
        assert_eq!(languages("x-hbotdflt"), ["dflt"]);
        assert_eq!(
            tags("Latn", "sr-x-hbsccyrl"),
            (vec!["cyrl".to_owned()], vec!["SRB ".to_owned()])
        );
        assert_eq!(tags("Latn", "x-hbscdflt").0, ["DFLT"]);
        assert_eq!(tags("Latn", "x-hbsc-6379726c").0, ["cyrl"]);
        assert_eq!(tags("Latn", "en-u-ab-x-hbsc").0, ["latn"], "empty override");
        assert_eq!(
            languages("tr-u-ca-x-hbsclatn"),
            ["TRK "],
            "limit at the first singleton"
        );
    }
}
