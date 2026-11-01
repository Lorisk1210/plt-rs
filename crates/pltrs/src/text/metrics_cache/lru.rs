//! The lines a [`MetricsCache`](super::MetricsCache) holds, as
//! `functools.lru_cache` holds them: at most [`METRICS_CACHE_SIZE`], the
//! least recently used one forgotten for a new one. Lines are filed under
//! a fingerprint of their key and linked in the order they were last used,
//! so that finding a line, using it and forgetting the least recently
//! used one each take the same time however many lines are held. Their
//! text is kept in one buffer, so that caching a line seldom allocates.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use pltrs_text::LineMetrics;

use super::{METRICS_CACHE_SIZE, MetricsCacheInfo};
use crate::fonts::FontProperties;
use crate::text::LineKind;

/// What a line is measured by: the line (where it is in the text of the
/// lines), its font (by its index among the fonts of the cache), how it
/// is typeset and the resolution.
#[derive(Debug)]
struct Key {
    line: Range<usize>,
    font: usize,
    kind: LineKind,
    dpi: u64,
}

impl Key {
    /// Whether the key, whose line is in `text`, is that of `line` in font
    /// `font` typeset as `kind` at the resolution whose bits are `dpi`.
    fn is(&self, text: &str, line: &str, font: usize, kind: LineKind, dpi: u64) -> bool {
        self.font == font
            && self.kind == kind
            && self.dpi == dpi
            && text[self.line.clone()] == *line
    }
}

/// How a key — its line, font index, kind and resolution bits — is
/// fingerprinted to be filed.
type Fingerprint = fn(&str, usize, LineKind, u64) -> u64;

/// A fingerprint of `line` and the rest of its key, under which keys are
/// filed.
fn fingerprint(line: &str, font: usize, kind: LineKind, dpi: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    line.hash(&mut hasher);
    (font, kind as u8, dpi).hash(&mut hasher);
    hasher.finish()
}

/// A cached line: its key and metrics, the next line filed under the same
/// fingerprint, and the lines used just before and after it.
#[derive(Debug)]
struct Entry {
    key: Key,
    fingerprint: u64,
    metrics: LineMetrics,
    /// The next line filed under the same fingerprint.
    collision: Option<usize>,
    /// The line used just before this one.
    older: Option<usize>,
    /// The line used just after this one.
    newer: Option<usize>,
}

/// The lines of a cache, the fonts they are set in and its statistics.
#[derive(Debug)]
pub(super) struct Lines {
    /// How keys are fingerprinted: [`fingerprint`], or in tests a function
    /// filing many keys under one fingerprint.
    fingerprint: Fingerprint,
    /// The fonts of the lines, each once, which keys refer to by index.
    fonts: Vec<FontProperties>,
    /// The index of the font last looked up, tried first: runs of lines
    /// share a font.
    last_font: usize,
    /// The lines, in no particular order.
    entries: Vec<Entry>,
    /// The text of the lines, one after another, with that of forgotten
    /// lines until it is squeezed out.
    text: String,
    /// How many bytes of `text` forgotten lines take.
    forgotten: usize,
    /// The first line filed under each fingerprint.
    by_fingerprint: HashMap<u64, usize>,
    /// The line used least recently.
    oldest: Option<usize>,
    /// The line used most recently.
    newest: Option<usize>,
    hits: u64,
    misses: u64,
}

impl Default for Lines {
    fn default() -> Self {
        Self::new(fingerprint)
    }
}

impl Lines {
    /// No lines, their keys to be fingerprinted by `fingerprint`.
    fn new(fingerprint: Fingerprint) -> Self {
        Self {
            fingerprint,
            fonts: Vec::new(),
            last_font: 0,
            entries: Vec::new(),
            text: String::new(),
            forgotten: 0,
            by_fingerprint: HashMap::new(),
            oldest: None,
            newest: None,
            hits: 0,
            misses: 0,
        }
    }

    /// What the cache has done so far.
    pub(super) fn info(&self) -> MetricsCacheInfo {
        MetricsCacheInfo {
            hits: self.hits,
            misses: self.misses,
            size: self.entries.len(),
        }
    }

    /// Forgets every line and the statistics.
    pub(super) fn clear(&mut self) {
        *self = Self::new(self.fingerprint);
    }

    /// The cached metrics of `line` in `font` as `kind` at the resolution
    /// whose bits are `dpi`, counting a hit, which makes the line the most
    /// recently used, or a miss.
    pub(super) fn find(
        &mut self,
        line: &str,
        font: &FontProperties,
        kind: LineKind,
        dpi: u64,
    ) -> Option<LineMetrics> {
        let found = self
            .font_index(font)
            .and_then(|font| self.position(line, font, kind, dpi));
        let Some(index) = found else {
            self.misses += 1;
            return None;
        };
        self.hits += 1;
        self.unlink(index);
        self.link_newest(index);
        Some(self.entries[index].metrics)
    }

    /// Caches `metrics` for `line` in `font` as `kind` at the resolution
    /// whose bits are `dpi`, as the most recently used line, in the place
    /// of the least recently used one when the cache is full.
    pub(super) fn insert(
        &mut self,
        line: &str,
        font: &FontProperties,
        kind: LineKind,
        dpi: u64,
        metrics: LineMetrics,
    ) {
        let font = self.intern(font);
        let fingerprint = (self.fingerprint)(line, font, kind, dpi);
        let reused = match self.oldest {
            Some(oldest) if self.entries.len() >= METRICS_CACHE_SIZE => {
                self.forget(oldest);
                Some(oldest)
            }
            _ => None,
        };
        let index = reused.unwrap_or(self.entries.len());
        let start = self.text.len();
        self.text.push_str(line);
        let entry = Entry {
            key: Key {
                line: start..self.text.len(),
                font,
                kind,
                dpi,
            },
            fingerprint,
            metrics,
            collision: self.by_fingerprint.insert(fingerprint, index),
            older: None,
            newer: None,
        };
        match reused {
            Some(index) => self.entries[index] = entry,
            None => self.entries.push(entry),
        }
        self.link_newest(index);
        if self.forgotten > self.text.len() / 2 {
            self.squeeze();
        }
    }

    /// Squeezes the text of forgotten lines out of the text of the lines.
    fn squeeze(&mut self) {
        let mut text = String::with_capacity(self.text.len() - self.forgotten);
        for entry in &mut self.entries {
            let start = text.len();
            text.push_str(&self.text[entry.key.line.clone()]);
            entry.key.line = start..text.len();
        }
        self.text = text;
        self.forgotten = 0;
    }

    /// Where the line `line` in font `font` as `kind` at the resolution
    /// whose bits are `dpi` is kept, if it is cached.
    fn position(&self, line: &str, font: usize, kind: LineKind, dpi: u64) -> Option<usize> {
        let first = self
            .by_fingerprint
            .get(&(self.fingerprint)(line, font, kind, dpi))
            .copied();
        std::iter::successors(first, |&index| self.entries[index].collision).find(|&index| {
            self.entries[index]
                .key
                .is(&self.text, line, font, kind, dpi)
        })
    }

    /// Takes the line at `index` out of the order of use.
    fn unlink(&mut self, index: usize) {
        let Entry { older, newer, .. } = self.entries[index];
        match older {
            Some(older) => self.entries[older].newer = newer,
            None => self.oldest = newer,
        }
        match newer {
            Some(newer) => self.entries[newer].older = older,
            None => self.newest = older,
        }
    }

    /// Puts the line at `index` last in the order of use.
    fn link_newest(&mut self, index: usize) {
        let entry = &mut self.entries[index];
        entry.older = self.newest;
        entry.newer = None;
        match self.newest {
            Some(newest) => self.entries[newest].newer = Some(index),
            None => self.oldest = Some(index),
        }
        self.newest = Some(index);
    }

    /// Takes the line at `index` out of the order of use and out of the
    /// lines filed under its fingerprint, so that its place can be reused.
    fn forget(&mut self, index: usize) {
        self.unlink(index);
        self.forgotten += self.entries[index].key.line.len();
        let Entry {
            fingerprint,
            collision,
            ..
        } = self.entries[index];
        let first = self.by_fingerprint.get(&fingerprint).copied();
        if first == Some(index) {
            match collision {
                Some(next) => self.by_fingerprint.insert(fingerprint, next),
                None => self.by_fingerprint.remove(&fingerprint),
            };
            return;
        }
        let before = std::iter::successors(first, |&at| self.entries[at].collision)
            .find(|&at| self.entries[at].collision == Some(index));
        if let Some(before) = before {
            self.entries[before].collision = collision;
        }
    }

    /// The index of `font` among the fonts of the lines, if it is one.
    fn font_index(&mut self, font: &FontProperties) -> Option<usize> {
        if self.fonts.get(self.last_font) == Some(font) {
            return Some(self.last_font);
        }
        let index = self.fonts.iter().position(|known| known == font)?;
        self.last_font = index;
        Some(index)
    }

    /// The index of `font` among the fonts of the lines, added when it is
    /// new. A cache whose lines were set in as many fonts as it holds
    /// lines starts afresh, keeping its statistics.
    fn intern(&mut self, font: &FontProperties) -> usize {
        if let Some(index) = self.font_index(font) {
            return index;
        }
        if self.fonts.len() >= METRICS_CACHE_SIZE {
            *self = Self {
                hits: self.hits,
                misses: self.misses,
                ..Self::new(self.fingerprint)
            };
        }
        self.fonts.push(font.clone());
        self.fonts.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::rc::RcParams;

    /// The keys a least-recently-used cache of [`METRICS_CACHE_SIZE`] keys
    /// holds, as `functools.lru_cache` keeps them.
    #[derive(Default)]
    struct Model {
        by_key: HashMap<String, u64>,
        by_use: BTreeMap<u64, String>,
        clock: u64,
    }

    impl Model {
        /// Whether `key` is held; it is then the most recently used.
        fn look_up(&mut self, key: &str) -> bool {
            self.clock += 1;
            let used = self.by_key.insert(key.to_owned(), self.clock);
            if let Some(used) = used {
                self.by_use.remove(&used);
            } else if self.by_key.len() > METRICS_CACHE_SIZE {
                let (_, oldest) = self.by_use.pop_first().expect("full");
                self.by_key.remove(&oldest);
            }
            self.by_use.insert(self.clock, key.to_owned());
            used.is_some()
        }
    }

    /// Looks up random lines, more than `lines` holds, in `lines` (caching
    /// those it lacks) and in the model: the same lines are found, with
    /// their own metrics.
    fn assert_kept_as_the_model_keeps(lines: &mut Lines) {
        let font = FontProperties::from_rc(&RcParams::matplotlib());
        let mut model = Model::default();
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        for _ in 0..20_000 {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let line = (state % 5_000).to_string();
            let metrics = LineMetrics {
                width: line.parse().expect("a number"),
                height: 10.0,
                descent: 2.0,
            };
            let found = lines.find(&line, &font, LineKind::Plain, 72);
            assert_eq!(found.is_some(), model.look_up(&line), "{line}");
            match found {
                Some(found) => assert_eq!(found, metrics),
                None => lines.insert(&line, &font, LineKind::Plain, 72, metrics),
            }
        }
        let info = lines.info();
        assert_eq!(info.size, METRICS_CACHE_SIZE);
        assert_eq!(info.hits + info.misses, 20_000);
    }

    #[test]
    fn lines_are_forgotten_as_an_lru_cache_forgets_them() {
        assert_kept_as_the_model_keeps(&mut Lines::default());
        // Lines filed under a few fingerprints: most keys collide.
        let mut colliding = Lines::new(|line, _, _, _| u64::from(line.as_bytes()[0]));
        assert_kept_as_the_model_keeps(&mut colliding);
        colliding.clear();
        assert_eq!(colliding.info(), MetricsCacheInfo::default());
        assert_kept_as_the_model_keeps(&mut colliding);
    }
}
