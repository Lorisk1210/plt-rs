//! HarfBuzz's Unicode normalization for shaping ("composed diacritics"
//! mode): characters missing from the font are decomposed, marks are put in
//! canonical order (by HarfBuzz's modified combining classes, with a
//! shaper hook), and base + mark sequences are recomposed when the font
//! has the precomposed character.

use crate::layout::buffer::{self, CharProps, Ignorable, Mask};
use crate::shape::spaces::{SpaceWidth, fallback_width};
use crate::shape::unicode::char_props;
use crate::shape::unicode_data::{COMPOSITIONS, DECOMPOSITIONS};

/// Longest run of marks HarfBuzz reorders.
const MAX_COMBINING_MARKS: usize = 32;

/// COMBINING GRAPHEME JOINER.
const CGJ: char = '\u{034F}';

/// A character with the byte offset of its cluster, its feature mask and
/// its shaping properties.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ClusterChar {
    pub(crate) c: char,
    pub(crate) cluster: usize,
    pub(crate) mask: Mask,
    pub(crate) props: CharProps,
    /// Set when a space the font lacks is drawn with the space glyph.
    pub(crate) space: Option<SpaceWidth>,
}

impl ClusterChar {
    pub(crate) fn new(c: char, cluster: usize, mask: Mask) -> Self {
        Self {
            c,
            cluster,
            mask,
            props: char_props(c),
            space: None,
        }
    }

    /// The same cluster and mask with another character.
    fn with_char(self, c: char) -> Self {
        Self::new(c, self.cluster, self.mask)
    }

    fn combining_class(&self) -> u8 {
        self.props.combining_class
    }
}

/// Merges the clusters of `chars[start..end]` like HarfBuzz.
pub(crate) fn merge_clusters(chars: &mut [ClusterChar], start: usize, end: usize) {
    buffer::merge_clusters(chars, start, end, |item| &mut item.cluster);
}

/// Script shaper hooks into normalization.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Hooks {
    /// Adjusts each canonically ordered run of marks `start..end` of the
    /// characters (HarfBuzz's shaper `reorder_marks`).
    pub(crate) reorder_marks: Option<fn(&mut [ClusterChar], usize, usize)>,
    /// Composes pairs canonical composition leaves alone (HarfBuzz's
    /// shaper `compose`).
    pub(crate) compose: Option<fn(char, char) -> Option<char>>,
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

/// HarfBuzz's recursive `decompose`: appends to `out` a decomposition of
/// `c` into characters the font has. `shortest` stops at the first level
/// the font supports; otherwise the deepest supported decomposition wins.
fn decompose_into(
    c: char,
    shortest: bool,
    has_glyph: &dyn Fn(char) -> bool,
    out: &mut Vec<char>,
) -> bool {
    let Some((a, b)) = decompose(c) else {
        return false;
    };
    if b.is_some_and(|b| !has_glyph(b)) {
        return false;
    }
    let has_a = has_glyph(a);
    if (shortest && has_a) || !decompose_into(a, shortest, has_glyph, out) {
        if !has_a {
            return false;
        }
        out.push(a);
    }
    out.extend(b);
    true
}

/// HarfBuzz's multi-character clusters: each run of marks together with
/// the character before it. Returns which characters belong to one.
fn multi_char_clusters(chars: &[ClusterChar]) -> Vec<bool> {
    let is_mark = |index: usize| chars[index].props.is_mark();
    let mut clustered = vec![false; chars.len()];
    let mut start = 0;
    while let Some(mark) = (start + 1..chars.len()).find(|&i| is_mark(i)) {
        let end = (mark..chars.len())
            .find(|&i| !is_mark(i))
            .unwrap_or(chars.len());
        clustered[mark - 1..end].fill(true);
        start = end;
    }
    clustered
}

/// HarfBuzz's `decompose_current_character`: characters of simple
/// clusters (`shortest`) keep their glyph when the font has one; in
/// clusters with marks the full decomposition is preferred.
fn decompose_char(
    item: ClusterChar,
    shortest: bool,
    has_glyph: &dyn Fn(char) -> bool,
    out: &mut Vec<ClusterChar>,
) {
    let mut parts = Vec::new();
    if shortest && has_glyph(item.c) {
        out.push(item);
    } else if decompose_into(item.c, shortest, has_glyph, &mut parts) {
        out.extend(parts.iter().map(|&c| item.with_char(c)));
    } else if has_glyph(item.c) {
        out.push(item);
    } else if let Some(width) = fallback_width(item.c).filter(|_| has_glyph(' ')) {
        out.push(ClusterChar {
            space: Some(width),
            ..item.with_char(' ')
        });
    } else if item.c == '\u{2011}' && has_glyph('\u{2010}') {
        // The non-breaking hyphen falls back to the hyphen.
        out.push(item.with_char('\u{2010}'));
    } else {
        out.push(item);
    }
}

/// Normalizes `chars` for a font whose character coverage is `has_glyph`,
/// with the script shaper's `hooks`.
pub(crate) fn normalize(
    chars: &[ClusterChar],
    has_glyph: &dyn Fn(char) -> bool,
    hooks: Hooks,
) -> Vec<ClusterChar> {
    // Round 1: decompose.
    let clustered = multi_char_clusters(chars);
    let mut out: Vec<ClusterChar> = Vec::with_capacity(chars.len());
    for (&item, &in_cluster) in chars.iter().zip(&clustered) {
        decompose_char(item, !in_cluster, has_glyph, &mut out);
    }
    if !clustered.contains(&true) {
        return out;
    }
    reorder_marks(&mut out, hooks.reorder_marks);
    unhide_idle_joiners(&mut out);
    recompose(&out, has_glyph, hooks.compose)
}

/// HarfBuzz's `hb_buffer_t::sort` by combining class: an insertion sort
/// that merges the clusters of every character it moves.
fn sort_marks(chars: &mut [ClusterChar], start: usize, end: usize) {
    for i in start + 1..end {
        let class = chars[i].combining_class();
        let j = (start..i)
            .rev()
            .take_while(|&k| chars[k].combining_class() > class)
            .last()
            .unwrap_or(i);
        if j < i {
            merge_clusters(chars, j, i + 1);
            chars[j..=i].rotate_right(1);
        }
    }
}

/// Round 2: sorts runs of non-starters by combining class, then lets the
/// shaper adjust them.
fn reorder_marks(chars: &mut [ClusterChar], reorder: Option<fn(&mut [ClusterChar], usize, usize)>) {
    let mut start = 0;
    while start < chars.len() {
        if chars[start].combining_class() == 0 {
            start += 1;
            continue;
        }
        let end = (start..chars.len())
            .find(|&i| chars[i].combining_class() == 0)
            .unwrap_or(chars.len());
        if end - start <= MAX_COMBINING_MARKS {
            sort_marks(chars, start, end);
            if let Some(reorder) = reorder {
                reorder(chars, start, end);
            }
        }
        start = end;
    }
}

/// A combining grapheme joiner that didn't block any reordering may be
/// skipped like other default ignorables.
fn unhide_idle_joiners(chars: &mut [ClusterChar]) {
    for i in 1..chars.len().saturating_sub(1) {
        let (before, after) = (
            chars[i - 1].combining_class(),
            chars[i + 1].combining_class(),
        );
        if chars[i].c == CGJ && (after == 0 || before <= after) {
            chars[i].props.ignorable = Some(Ignorable::Other);
        }
    }
}

/// Round 3: composes marks with their starter (the last character of
/// class 0, or the first character) when not blocked and the font has the
/// composite; `extra` composes pairs canonical composition leaves alone.
fn recompose(
    chars: &[ClusterChar],
    has_glyph: &dyn Fn(char) -> bool,
    extra: Option<fn(char, char) -> Option<char>>,
) -> Vec<ClusterChar> {
    let mut out: Vec<ClusterChar> = Vec::with_capacity(chars.len());
    let Some((&first, rest)) = chars.split_first() else {
        return out;
    };
    out.push(first);
    let mut starter = 0;
    for &item in rest {
        let previous = out.last().map_or(0, ClusterChar::combining_class);
        let unblocked = starter + 1 == out.len() || previous < item.combining_class();
        if item.props.is_mark() && unblocked {
            let (a, b) = (out[starter].c, item.c);
            let composed = compose(a, b).or_else(|| extra.and_then(|extra| extra(a, b)));
            if let Some(composed) = composed.filter(|&c| has_glyph(c)) {
                let cluster = out[starter].cluster.min(item.cluster);
                out[starter] = ClusterChar {
                    cluster,
                    ..out[starter].with_char(composed)
                };
                continue;
            }
        }
        out.push(item);
        if item.combining_class() == 0 {
            starter = out.len() - 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<ClusterChar> {
        text.char_indices()
            .map(|(cluster, c)| ClusterChar::new(c, cluster, 1))
            .collect()
    }

    fn text(items: &[ClusterChar]) -> String {
        items.iter().map(|item| item.c).collect()
    }

    #[test]
    fn unicode_properties() {
        assert_eq!(decompose('é'), Some(('e', Some('\u{301}'))));
        assert_eq!(decompose('\u{212B}'), Some(('Å', None)));
        assert_eq!(compose('e', '\u{301}'), Some('é'));
        assert_eq!(compose('x', '\u{301}'), None);
    }

    #[test]
    fn composes_when_the_font_has_the_composite() {
        let all = |_: char| true;
        assert_eq!(
            text(&normalize(
                &chars("e\u{301}x\u{301}"),
                &all,
                Hooks::default()
            )),
            "éx\u{301}"
        );
        let no_e_acute = |c: char| c != 'é';
        assert_eq!(
            text(&normalize(
                &chars("e\u{301}"),
                &no_e_acute,
                Hooks::default()
            )),
            "e\u{301}"
        );
    }

    #[test]
    fn decomposes_missing_characters() {
        let without_precomposed = |c: char| c.is_ascii() || char_props(c).is_mark();
        let result = normalize(&chars("ǘ"), &without_precomposed, Hooks::default());
        assert_eq!(text(&result), "u\u{308}\u{301}");
        assert!(
            result
                .iter()
                .all(|item| item.cluster == 0 && item.mask == 1)
        );
        let nothing = |_: char| false;
        assert_eq!(
            text(&normalize(&chars("é"), &nothing, Hooks::default())),
            "é"
        );
    }

    #[test]
    fn missing_spaces_fall_back_to_the_space_glyph() {
        let ascii = |c: char| c.is_ascii();
        let result = normalize(&chars("a\u{2009}b\u{2011}"), &ascii, Hooks::default());
        assert_eq!(text(&result), "a b\u{2011}");
        assert_eq!(result[1].space, Some(SpaceWidth::EmFraction(5)));
        let with_hyphen = |c: char| c.is_ascii() || c == '\u{2010}';
        assert_eq!(
            text(&normalize(
                &chars("\u{2011}"),
                &with_hyphen,
                Hooks::default()
            )),
            "\u{2010}"
        );
    }

    #[test]
    fn reorders_marks_canonically() {
        let all = |_: char| true;
        // Dot below (220) sorts before acute (230); then both compose.
        assert_eq!(
            text(&normalize(
                &chars("a\u{301}\u{323}"),
                &all,
                Hooks::default()
            )),
            "ạ\u{301}"
        );
        // Arabic: shadda sorts before fatha.
        assert_eq!(
            text(&normalize(
                &chars("\u{628}\u{64E}\u{651}"),
                &all,
                Hooks::default()
            )),
            "\u{628}\u{651}\u{64E}"
        );
        let hooks = Hooks {
            reorder_marks: Some(|chars, start, end| chars[start..end].reverse()),
            compose: None,
        };
        let marks_only = |c: char| c == 'x' || c > '\u{300}';
        assert_eq!(
            text(&normalize(&chars("x\u{323}\u{301}"), &marks_only, hooks)),
            "x\u{301}\u{323}"
        );
    }

    #[test]
    fn shaper_compositions() {
        let all = |_: char| true;
        let hooks = Hooks {
            reorder_marks: None,
            compose: Some(|a, b| (a == 'x' && b == '\u{301}').then_some('y')),
        };
        assert_eq!(text(&normalize(&chars("x\u{301}"), &all, hooks)), "y");
        assert_eq!(
            text(&normalize(&chars("x\u{301}"), &all, Hooks::default())),
            "x\u{301}"
        );
        // A leading mark is the starter of the marks after it.
        let leading = normalize(&chars("\u{301}\u{301}"), &all, hooks);
        assert_eq!(leading.len(), 2);
    }

    #[test]
    fn grapheme_joiners() {
        let all = |_: char| true;
        let idle = normalize(&chars("a\u{301}\u{34F}\u{301}"), &all, Hooks::default());
        assert_eq!(idle[1].props.ignorable, Some(Ignorable::Other));
        let blocking = normalize(
            &chars("a\u{301}\u{34F}\u{323}"),
            &|c| c != 'á',
            Hooks::default(),
        );
        assert_eq!(blocking[2].props.ignorable, Some(Ignorable::Hidden));
    }
}
