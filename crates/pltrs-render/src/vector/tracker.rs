//! matplotlib's `CharacterTracker`: which glyphs of which fonts a PDF or
//! PostScript document uses, grouped into subsets small enough for the
//! embedded font format (256 codes for Type 3 fonts).

use std::collections::HashMap;

use pltrs_font::GlyphId;
use pltrs_text::FontId;

/// One subset of a font: character codes mapped to glyphs, in the order
/// they were first used.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Subset {
    entries: Vec<(u32, GlyphId)>,
}

impl Subset {
    /// The subset of the `(code, glyph)` pairs `entries`.
    pub(crate) const fn from_entries(entries: Vec<(u32, GlyphId)>) -> Self {
        Self { entries }
    }

    /// `(code, glyph)` pairs in first-use order.
    pub(crate) fn entries(&self) -> &[(u32, GlyphId)] {
        &self.entries
    }

    /// The glyph mapped to `code`.
    pub(crate) fn glyph(&self, code: u32) -> Option<GlyphId> {
        self.entries
            .iter()
            .find(|(c, _)| *c == code)
            .map(|&(_, glyph)| glyph)
    }

    fn contains(&self, code: u32) -> bool {
        self.glyph(code).is_some()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no glyph is used from this subset.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The glyphs used from one font.
#[derive(Clone, Debug, Default)]
struct FontUsage {
    subsets: Vec<Subset>,
    /// `(characters, glyph)` → `(subset, code)`.
    forward: HashMap<(String, GlyphId), (usize, u32)>,
    /// `(subset, code)` → characters.
    inverse: HashMap<(usize, u32), String>,
}

/// Tracks glyph usage per font, assigning each glyph a subset and a
/// character code within it.
#[derive(Clone, Debug)]
pub(crate) struct CharacterTracker {
    subset_size: u32,
    /// Fonts in first-use order.
    fonts: Vec<(FontId, FontUsage)>,
}

impl CharacterTracker {
    /// A tracker grouping codes into subsets of `subset_size` (256 for
    /// Type 3 fonts, 65536 for Type 42; 0 keeps every code as is).
    pub(crate) const fn new(subset_size: u32) -> Self {
        Self {
            subset_size,
            fonts: Vec::new(),
        }
    }

    /// `track_glyph`: records that `glyph` of `font` renders `chars`, and
    /// returns its `(subset, code)`. Codes below the subset size keep their
    /// value in subset 0; other glyphs fill the following subsets in order.
    pub(crate) fn track(&mut self, font: FontId, chars: &str, glyph: GlyphId) -> (usize, u32) {
        let known = self.fonts.iter().position(|(id, _)| *id == font);
        let index = known.unwrap_or_else(|| {
            let usage = FontUsage {
                subsets: vec![Subset::default()],
                ..FontUsage::default()
            };
            self.fonts.push((font, usage));
            self.fonts.len() - 1
        });
        let subset_size = self.subset_size;
        let usage = &mut self.fonts[index].1;
        let key = (chars.to_owned(), glyph);
        if let Some(&result) = usage.forward.get(&key) {
            return result;
        }
        let charcode = chars.chars().next().map_or(0, u32::from);
        let next_subset = chars.chars().count() > 1
            || (subset_size != 0
                && (charcode >= subset_size || usage.subsets[0].contains(charcode)));
        let (subset, code) = if next_subset {
            let last_full = usage
                .subsets
                .last()
                .is_some_and(|s| s.len() == subset_size as usize);
            if usage.subsets.len() == 1 || last_full {
                usage.subsets.push(Subset::default());
            }
            let subset = usage.subsets.len() - 1;
            (subset, usage.subsets[subset].len() as u32)
        } else {
            (0, charcode)
        };
        usage.subsets[subset].entries.push((code, glyph));
        usage.forward.insert(key.clone(), (subset, code));
        usage.inverse.insert((subset, code), key.0);
        (subset, code)
    }

    /// Every used font with its subsets, in first-use order.
    pub(crate) fn fonts(&self) -> impl Iterator<Item = (FontId, &[Subset])> {
        self.fonts
            .iter()
            .map(|(id, usage)| (*id, usage.subsets.as_slice()))
    }

    /// The subset `subset` of `font` (empty when unused).
    pub(crate) fn subset(&self, font: FontId, subset: usize) -> Option<&Subset> {
        self.fonts
            .iter()
            .find(|(id, _)| *id == font)
            .and_then(|(_, usage)| usage.subsets.get(subset))
    }

    /// `subset_to_unicode`: the characters behind a subset code.
    pub(crate) fn chars(&self, font: FontId, subset: usize, code: u32) -> Option<&str> {
        self.fonts
            .iter()
            .find(|(id, _)| *id == font)
            .and_then(|(_, usage)| usage.inverse.get(&(subset, code)))
            .map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_text::{FontQuery, FontRequest, TextEngine};

    fn font() -> FontId {
        TextEngine::global()
            .layout_line(
                "a",
                &FontRequest::new(FontQuery::families(["DejaVu Sans"])),
                72.0,
            )
            .expect("bundled font")
            .glyphs[0]
            .font
    }

    #[test]
    fn type3_subsets_keep_latin1_codes() {
        let font = font();
        let mut tracker = CharacterTracker::new(256);
        assert_eq!(tracker.track(font, "1", GlyphId(20)), (0, 0x31));
        assert_eq!(tracker.track(font, "1", GlyphId(20)), (0, 0x31));
        assert_eq!(tracker.track(font, "\u{2212}", GlyphId(3228)), (1, 0));
        assert_eq!(
            tracker.track(font, "1", GlyphId(999)),
            (1, 1),
            "an alternate glyph for a used code"
        );
        assert_eq!(tracker.track(font, "fi", GlyphId(500)), (1, 2));
        assert_eq!(tracker.chars(font, 1, 0), Some("\u{2212}"));
        let subsets: Vec<_> = tracker.fonts().collect();
        assert_eq!(subsets.len(), 1);
        assert_eq!(subsets[0].1[0].entries(), &[(0x31, GlyphId(20))]);
        assert_eq!(tracker.subset(font, 1).map(Subset::len), Some(3));
    }

    #[test]
    fn unlimited_tracker_keeps_codes() {
        let font = font();
        let mut tracker = CharacterTracker::new(0);
        assert_eq!(tracker.track(font, "\u{2212}", GlyphId(3228)), (0, 0x2212));
    }
}
