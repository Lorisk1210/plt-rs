//! HarfBuzz's `hb_ot_tags_from_complex_language`: languages recognised
//! from more than their primary subtag (a script, region or variant
//! subtag), as rules tried in order. The rules themselves are data
//! generated from HarfBuzz ([`COMPLEX_LANGUAGES`]); this module evaluates
//! the four kinds of test they make.

use super::tables::COMPLEX_LANGUAGES;
use crate::types::Tag;

/// Shortest language whose variant subtags are looked at.
const MIN_VARIANT_LANGUAGE: usize = 7;
/// Shortest stretch from the first `-` to the limit holding a variant.
const MIN_VARIANT_SPAN: usize = 5;

/// How a rule recognises a language, in HarfBuzz's canonical spelling
/// (lower case, subtags joined by `-`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Pattern {
    /// A subtag after the primary one and before the first singleton
    /// (`-fonipa`), for languages of at least seven characters
    /// (`subtag_matches (p, limit, ...)`).
    Variant(&'static [u8]),
    /// The whole language (`art-lojban`; `strcmp`).
    Exact(&'static [u8]),
    /// The leading subtags (`zh-hant-hk`), followed by the end of the
    /// language or another subtag (`lang_matches`).
    Prefix(&'static [u8]),
    /// A leading primary subtag with its `-` (`zh-`) and another subtag
    /// before the first singleton (`-tw`; `strncmp` then
    /// `subtag_matches (lang_str, limit, ...)`).
    PrefixWithSubtag(&'static [u8], &'static [u8]),
}

impl Pattern {
    /// Whether `language` matches, considering subtags up to `limit` (the
    /// `-` before the first singleton, else the end).
    fn matches(self, language: &[u8], limit: usize) -> bool {
        match self {
            Self::Variant(subtag) => {
                let Some(dash) = language.iter().position(|&c| c == b'-') else {
                    return false;
                };
                limit >= MIN_VARIANT_LANGUAGE
                    && dash < limit
                    && limit - dash >= MIN_VARIANT_SPAN
                    && subtag_matches(language, dash, limit, subtag)
            }
            Self::Exact(whole) => language == whole,
            Self::Prefix(prefix) => {
                limit >= prefix.len()
                    && language.starts_with(prefix)
                    && matches!(language.get(prefix.len()), None | Some(b'-'))
            }
            Self::PrefixWithSubtag(prefix, subtag) => {
                language.starts_with(prefix) && subtag_matches(language, 0, limit, subtag)
            }
        }
    }
}

/// One rule of the matcher: a pattern and the tags it yields, in
/// preference order.
#[derive(Clone, Copy, Debug)]
pub(super) struct ComplexLanguage {
    pattern: Pattern,
    tags: &'static [Tag],
}

impl ComplexLanguage {
    /// A rule yielding `tags` for languages matching `pattern`.
    pub(super) const fn new(pattern: Pattern, tags: &'static [Tag]) -> Self {
        Self { pattern, tags }
    }
}

/// HarfBuzz's `subtag_matches`: whether `subtag` (with its leading `-`)
/// occurs in `language` at or after `from`, starting before `limit` and
/// ending at the end of a subtag.
fn subtag_matches(language: &[u8], from: usize, limit: usize, subtag: &[u8]) -> bool {
    if limit.saturating_sub(from) < subtag.len() {
        return false;
    }
    let mut from = from;
    loop {
        let Some(found) = language[from..]
            .windows(subtag.len())
            .position(|window| window == subtag)
        else {
            return false;
        };
        let start = from + found;
        if start >= limit {
            return false;
        }
        let end = start + subtag.len();
        if !language.get(end).is_some_and(u8::is_ascii_alphanumeric) {
            return true;
        }
        from = end;
    }
}

/// The tags of the first rule `language` (canonical, considered up to
/// `limit`) matches, if any.
pub(super) fn complex_language_tags(language: &[u8], limit: usize) -> Option<&'static [Tag]> {
    COMPLEX_LANGUAGES
        .iter()
        .find(|rule| rule.pattern.matches(language, limit))
        .map(|rule| rule.tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(language: &str) -> Option<Vec<String>> {
        let limit = language.find("-x-").unwrap_or(language.len());
        complex_language_tags(language.as_bytes(), limit)
            .map(|tags| tags.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn variants_need_room_before_the_limit() {
        // hb-ot-tag-table.hh: `-polyton` -> PGR, `-fonipa` -> IPPH.
        assert_eq!(tags("el-polyton"), Some(vec!["PGR ".to_owned()]));
        assert_eq!(tags("und-fonipa"), Some(vec!["IPPH".to_owned()]));
        assert_eq!(tags("und-fonipax"), None, "not at a subtag end");
        assert_eq!(tags("de-1901-fonipa-x-a"), Some(vec!["IPPH".to_owned()]));
        assert_eq!(tags("a-x-fonipa"), None, "after the private-use singleton");
        assert_eq!(tags("ab-geok"), Some(vec!["KGE ".to_owned()]));
        assert_eq!(tags("a-geok"), None, "shorter than seven characters");
    }

    #[test]
    fn prefixes_exact_matches_and_regions() {
        let zh_mo = Some(vec!["ZHTM".to_owned(), "ZHH ".to_owned()]);
        assert_eq!(tags("cmn-hant-mo"), zh_mo);
        assert_eq!(tags("cmn-hant-mo-x-y"), zh_mo);
        assert_eq!(tags("cmn-hant-mox"), Some(vec!["ZHT ".to_owned()]));
        assert_eq!(tags("art-lojban"), Some(vec!["JBO ".to_owned()]));
        assert_eq!(tags("art-lojban-x"), None, "exact matches only");
        assert_eq!(tags("zh-tw"), Some(vec!["ZHT ".to_owned()]));
        assert_eq!(tags("zh-latn-tw"), Some(vec!["ZHT ".to_owned()]));
        assert_eq!(
            tags("ro-md"),
            Some(vec!["MOL ".to_owned(), "ROM ".to_owned()])
        );
        assert_eq!(tags("ro-mda"), None);
        assert_eq!(tags("sr"), None);
        assert_eq!(tags(""), None);
    }

    #[test]
    fn subtags_end_at_dashes_or_the_end() {
        let language = b"zh-hk-tw";
        assert!(subtag_matches(language, 0, language.len(), b"-tw"));
        assert!(subtag_matches(language, 0, language.len(), b"-hk"));
        assert!(
            !subtag_matches(language, 0, 5, b"-tw"),
            "starts past the limit"
        );
        assert!(
            !subtag_matches(language, 4, 2, b"-tw"),
            "limit before start"
        );
        assert!(
            subtag_matches(b"zh-twa-tw", 0, 9, b"-tw"),
            "second occurrence"
        );
    }
}
