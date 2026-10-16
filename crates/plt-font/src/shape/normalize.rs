//! HarfBuzz's Unicode normalization for shaping ("composed diacritics"
//! mode): characters missing from the font are decomposed, marks are put in
//! canonical order, and base + mark sequences are recomposed when the font
//! has the precomposed character.

use crate::shape::script::mark_kind;
use crate::shape::spaces::{SpaceWidth, fallback_width};
use crate::shape::unicode_data::{COMBINING_CLASSES, COMPOSITIONS, DECOMPOSITIONS};

/// Longest run of marks HarfBuzz reorders.
const MAX_COMBINING_MARKS: usize = 32;

/// A character with the byte offset of its cluster.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ClusterChar {
    pub(crate) c: char,
    pub(crate) cluster: usize,
    /// Set when a space the font lacks is drawn with the space glyph.
    pub(crate) space: Option<SpaceWidth>,
}

impl ClusterChar {
    pub(crate) const fn new(c: char, cluster: usize) -> Self {
        Self {
            c,
            cluster,
            space: None,
        }
    }
}

/// Canonical combining class of `c` (0 for starters).
pub(crate) fn combining_class(c: char) -> u8 {
    let code = u32::from(c);
    let index = COMBINING_CLASSES.partition_point(|&(start, _, _)| start <= code);
    match COMBINING_CLASSES.get(index.wrapping_sub(1)) {
        Some(&(_, end, class)) if code <= end => class,
        _ => 0,
    }
}

/// One-level canonical decomposition of `c`.
fn decompose(c: char) -> Option<(char, Option<char>)> {
    let code = u32::from(c);
    let index = DECOMPOSITIONS
        .binary_search_by_key(&code, |&(composed, _, _)| composed)
        .ok()?;
    let (_, first, second) = DECOMPOSITIONS[index];
    Some((
        char::from_u32(first)?,
        char::from_u32(second).filter(|&c| c != '\0'),
    ))
}

/// Primary composite of `a` followed by `b`.
fn compose(a: char, b: char) -> Option<char> {
    let key = (u32::from(a), u32::from(b));
    let index = COMPOSITIONS
        .binary_search_by_key(&key, |&(x, y, _)| (x, y))
        .ok()?;
    char::from_u32(COMPOSITIONS[index].2)
}

/// HarfBuzz's recursive `decompose` with short-circuiting: returns the
/// decomposition of `c` into characters the font has, if any.
fn decompose_supported(c: char, has_glyph: &dyn Fn(char) -> bool, out: &mut Vec<char>) -> bool {
    let Some((a, b)) = decompose(c) else {
        return false;
    };
    if b.is_some_and(|b| !has_glyph(b)) {
        return false;
    }
    let start = out.len();
    if has_glyph(a) || decompose_supported(a, has_glyph, out) {
        if out.len() == start {
            out.push(a);
        }
        out.extend(b);
        return true;
    }
    false
}

/// Normalizes `chars` for a font whose character coverage is `has_glyph`.
pub(crate) fn normalize(
    chars: &[ClusterChar],
    has_glyph: &dyn Fn(char) -> bool,
) -> Vec<ClusterChar> {
    // Round 1: decompose characters the font lacks.
    let mut out: Vec<ClusterChar> = Vec::with_capacity(chars.len());
    let mut parts = Vec::new();
    for &item in chars {
        parts.clear();
        if has_glyph(item.c) {
            out.push(item);
        } else if decompose_supported(item.c, has_glyph, &mut parts) {
            out.extend(parts.iter().map(|&c| ClusterChar::new(c, item.cluster)));
        } else if let Some(width) = fallback_width(item.c).filter(|_| has_glyph(' ')) {
            out.push(ClusterChar {
                c: ' ',
                space: Some(width),
                ..item
            });
        } else if item.c == '\u{2011}' && has_glyph('\u{2010}') {
            // The non-breaking hyphen falls back to the hyphen.
            out.push(ClusterChar {
                c: '\u{2010}',
                ..item
            });
        } else {
            out.push(item);
        }
    }
    if !out.iter().any(|item| mark_kind(item.c).is_some()) {
        return out;
    }
    reorder_marks(&mut out);
    recompose(&out, has_glyph)
}

/// Round 2: stable-sorts runs of non-starters by combining class.
fn reorder_marks(chars: &mut [ClusterChar]) {
    let mut start = 0;
    while start < chars.len() {
        if combining_class(chars[start].c) == 0 {
            start += 1;
            continue;
        }
        let end = (start..chars.len())
            .find(|&i| combining_class(chars[i].c) == 0)
            .unwrap_or(chars.len());
        if end - start <= MAX_COMBINING_MARKS {
            chars[start..end].sort_by_key(|item| combining_class(item.c));
        }
        start = end;
    }
}

/// Round 3: composes marks with their starter when not blocked and the
/// font has the composite.
fn recompose(chars: &[ClusterChar], has_glyph: &dyn Fn(char) -> bool) -> Vec<ClusterChar> {
    let mut out: Vec<ClusterChar> = Vec::with_capacity(chars.len());
    let mut starter: Option<usize> = None;
    for &item in chars {
        if let (Some(s), true) = (starter, mark_kind(item.c).is_some()) {
            let previous = out.last().map_or(0, |p| combining_class(p.c));
            let unblocked = s + 1 == out.len() || previous < combining_class(item.c);
            if let Some(composed) = compose(out[s].c, item.c).filter(|&c| unblocked && has_glyph(c))
            {
                out[s].c = composed;
                out[s].cluster = out[s].cluster.min(item.cluster);
                continue;
            }
        }
        out.push(item);
        if combining_class(item.c) == 0 {
            starter = Some(out.len() - 1);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<ClusterChar> {
        text.char_indices()
            .map(|(cluster, c)| ClusterChar::new(c, cluster))
            .collect()
    }

    fn text(items: &[ClusterChar]) -> String {
        items.iter().map(|item| item.c).collect()
    }

    #[test]
    fn unicode_properties() {
        assert_eq!(combining_class('\u{301}'), 230);
        assert_eq!(combining_class('\u{323}'), 220);
        assert_eq!(combining_class('a'), 0);
        assert_eq!(decompose('é'), Some(('e', Some('\u{301}'))));
        assert_eq!(decompose('\u{212B}'), Some(('Å', None)));
        assert_eq!(compose('e', '\u{301}'), Some('é'));
        assert_eq!(compose('x', '\u{301}'), None);
    }

    #[test]
    fn composes_when_the_font_has_the_composite() {
        let all = |_: char| true;
        assert_eq!(
            text(&normalize(&chars("e\u{301}x\u{301}"), &all)),
            "éx\u{301}"
        );
        let no_e_acute = |c: char| c != 'é';
        assert_eq!(
            text(&normalize(&chars("e\u{301}"), &no_e_acute)),
            "e\u{301}"
        );
    }

    #[test]
    fn decomposes_missing_characters() {
        let without_precomposed = |c: char| c.is_ascii() || mark_kind(c).is_some();
        let result = normalize(&chars("ǘ"), &without_precomposed);
        assert_eq!(text(&result), "u\u{308}\u{301}");
        assert!(result.iter().all(|item| item.cluster == 0));
        let nothing = |_: char| false;
        assert_eq!(text(&normalize(&chars("é"), &nothing)), "é");
    }

    #[test]
    fn missing_spaces_fall_back_to_the_space_glyph() {
        let ascii = |c: char| c.is_ascii();
        let result = normalize(&chars("a\u{2009}b\u{2011}"), &ascii);
        assert_eq!(text(&result), "a b\u{2011}");
        assert_eq!(result[1].space, Some(SpaceWidth::EmFraction(5)));
        let with_hyphen = |c: char| c.is_ascii() || c == '\u{2010}';
        assert_eq!(
            text(&normalize(&chars("\u{2011}"), &with_hyphen)),
            "\u{2010}"
        );
    }

    #[test]
    fn reorders_marks_canonically() {
        let all = |_: char| true;
        // Dot below (220) sorts before acute (230); then both compose.
        assert_eq!(
            text(&normalize(&chars("a\u{301}\u{323}"), &all)),
            "ạ\u{301}"
        );
    }
}
