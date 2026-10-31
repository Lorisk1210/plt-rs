//! Text metrics cached per renderer (matplotlib's
//! `text._get_text_metrics_function`): matplotlib measures every line of
//! text through a least-recently-used cache of its renderer, keyed by the
//! line, its font, how it is typeset and the resolution. pltrs (Python)
//! keeps such a cache with each renderer it hands out and installs it on
//! the thread while pltrs lays out or draws a figure with that renderer,
//! so that the lines pltrs measures meanwhile go through it, and the
//! program sees the cache matplotlib's would have filled.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use pltrs_text::{LineMetrics, TextError};

use super::overrides::LineKind;
use crate::fonts::FontProperties;

/// How many lines a cache holds before it forgets the least recently used
/// (matplotlib's `lru_cache(4096)`).
pub const METRICS_CACHE_SIZE: usize = 4096;

/// What a line is measured by: the line, its font (by its index among the
/// fonts of the cache), how it is typeset and the resolution.
#[derive(Debug)]
struct Key {
    line: String,
    font: usize,
    kind: LineKind,
    dpi: u64,
}

impl Key {
    /// Whether the key is that of `line` in font `font` typeset as `kind`
    /// at the resolution whose bits are `dpi`.
    fn is(&self, line: &str, font: usize, kind: LineKind, dpi: u64) -> bool {
        self.line == line && self.font == font && self.kind == kind && self.dpi == dpi
    }
}

/// A fingerprint of `line` and the rest of its key, under which keys are
/// filed.
fn fingerprint(line: &str, font: usize, kind: LineKind, dpi: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    line.hash(&mut hasher);
    (font, kind as u8, dpi).hash(&mut hasher);
    hasher.finish()
}

/// A cached line: its key, metrics and when it was last used.
#[derive(Debug)]
struct Entry {
    key: Key,
    metrics: LineMetrics,
    used: u64,
}

/// The lines of a cache, the fonts they are set in and its statistics.
#[derive(Debug, Default)]
struct Lines {
    /// The fonts of the lines, each once, which keys refer to by index.
    fonts: Vec<FontProperties>,
    by_fingerprint: HashMap<u64, Vec<Entry>>,
    len: usize,
    hits: u64,
    misses: u64,
    clock: u64,
}

/// What a cache has done: matplotlib's `cache_info()`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MetricsCacheInfo {
    /// Lines found in the cache.
    pub hits: u64,
    /// Lines measured because they were not.
    pub misses: u64,
    /// Lines held.
    pub size: usize,
}

/// The metrics of the lines measured with one renderer.
#[derive(Debug, Default)]
pub struct MetricsCache {
    lines: RefCell<Lines>,
}

impl MetricsCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// The metrics of `line` in `font` typeset as `kind` at `dpi`: the
    /// cached ones (a hit), else those `measure` gives, which are cached
    /// (a miss; failures are not).
    ///
    /// # Errors
    ///
    /// When `measure` fails.
    pub fn get_or_measure(
        &self,
        line: &str,
        font: &FontProperties,
        kind: LineKind,
        dpi: f64,
        measure: impl FnOnce() -> Result<LineMetrics, TextError>,
    ) -> Result<LineMetrics, TextError> {
        let dpi = dpi.to_bits();
        if let Some(metrics) = self.find(line, font, kind, dpi) {
            return Ok(metrics);
        }
        let metrics = measure()?;
        self.insert(line, font, kind, dpi, metrics);
        Ok(metrics)
    }

    /// The cached metrics of `line` in `font` as `kind` at the resolution
    /// whose bits are `dpi`, counting a hit or a miss.
    fn find(
        &self,
        line: &str,
        font: &FontProperties,
        kind: LineKind,
        dpi: u64,
    ) -> Option<LineMetrics> {
        let mut lines = self.lines.borrow_mut();
        lines.clock += 1;
        let clock = lines.clock;
        let found = lines.font_index(font).and_then(|font| {
            let entry = lines
                .by_fingerprint
                .get_mut(&fingerprint(line, font, kind, dpi))?
                .iter_mut()
                .find(|entry| entry.key.is(line, font, kind, dpi))?;
            entry.used = clock;
            Some(entry.metrics)
        });
        if found.is_some() {
            lines.hits += 1;
        } else {
            lines.misses += 1;
        }
        found
    }

    /// Caches `metrics` for `line` in `font` as `kind` at `dpi`,
    /// forgetting the least recently used line when the cache is full.
    fn insert(
        &self,
        line: &str,
        font: &FontProperties,
        kind: LineKind,
        dpi: u64,
        metrics: LineMetrics,
    ) {
        let mut lines = self.lines.borrow_mut();
        if lines.len >= METRICS_CACHE_SIZE {
            lines.forget_least_recent();
        }
        let font = lines.intern(font);
        let used = lines.clock;
        let key = Key {
            line: line.to_owned(),
            font,
            kind,
            dpi,
        };
        lines
            .by_fingerprint
            .entry(fingerprint(line, font, kind, dpi))
            .or_default()
            .push(Entry { key, metrics, used });
        lines.len += 1;
    }

    /// What the cache has done so far.
    pub fn info(&self) -> MetricsCacheInfo {
        let lines = self.lines.borrow();
        MetricsCacheInfo {
            hits: lines.hits,
            misses: lines.misses,
            size: lines.len,
        }
    }

    /// Forgets every line and the statistics (`cache_clear()`).
    pub fn clear(&self) {
        *self.lines.borrow_mut() = Lines::default();
    }
}

impl Lines {
    /// The index of `font` among the fonts of the lines, if it is one.
    fn font_index(&self, font: &FontProperties) -> Option<usize> {
        self.fonts.iter().position(|known| known == font)
    }

    /// The index of `font` among the fonts of the lines, added when it is
    /// new. A cache whose lines were set in as many fonts as it holds
    /// lines starts afresh.
    fn intern(&mut self, font: &FontProperties) -> usize {
        if let Some(index) = self.font_index(font) {
            return index;
        }
        if self.fonts.len() >= METRICS_CACHE_SIZE {
            self.fonts.clear();
            self.by_fingerprint.clear();
            self.len = 0;
        }
        self.fonts.push(font.clone());
        self.fonts.len() - 1
    }

    /// Forgets the line used least recently.
    fn forget_least_recent(&mut self) {
        let oldest = self
            .by_fingerprint
            .iter()
            .flat_map(|(fingerprint, bucket)| {
                bucket
                    .iter()
                    .enumerate()
                    .map(move |(index, entry)| (entry.used, *fingerprint, index))
            })
            .min();
        let Some((_, fingerprint, index)) = oldest else {
            return;
        };
        if let Some(bucket) = self.by_fingerprint.get_mut(&fingerprint) {
            bucket.swap_remove(index);
            if bucket.is_empty() {
                self.by_fingerprint.remove(&fingerprint);
            }
            self.len -= 1;
        }
    }
}

thread_local! {
    /// The cache of the renderer the thread lays out and draws with, if
    /// the program installed one.
    static CURRENT: RefCell<Option<Rc<MetricsCache>>> = const { RefCell::new(None) };
}

/// Installs `cache` (or, with `None`, no cache) as the cache of the lines
/// this thread measures; returns the cache installed before.
pub fn set_metrics_cache(cache: Option<Rc<MetricsCache>>) -> Option<Rc<MetricsCache>> {
    CURRENT.replace(cache)
}

/// Whether a cache is installed on the thread.
pub(crate) fn installed() -> bool {
    CURRENT.with_borrow(Option::is_some)
}

/// The metrics of `line` through the thread's cache if one is installed,
/// else those `measure` gives.
pub(super) fn cached(
    line: &str,
    font: &FontProperties,
    kind: LineKind,
    dpi: f64,
    measure: impl FnOnce() -> Result<LineMetrics, TextError>,
) -> Result<LineMetrics, TextError> {
    match CURRENT.with_borrow(Clone::clone) {
        Some(cache) => cache.get_or_measure(line, font, kind, dpi, measure),
        None => measure(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rc::RcParams;

    fn metrics(width: f64) -> LineMetrics {
        LineMetrics {
            width,
            height: 10.0,
            descent: 2.0,
        }
    }

    fn font() -> FontProperties {
        FontProperties::from_rc(&RcParams::matplotlib())
    }

    #[test]
    fn lines_are_measured_once_per_key() {
        let cache = MetricsCache::new();
        let font = font();
        let mut measured = 0;
        let mut measure = |line: &str, kind| {
            cache
                .get_or_measure(line, &font, kind, 100.0, || {
                    measured += 1;
                    Ok(metrics(line.len() as f64))
                })
                .expect("measured")
        };
        assert_eq!(measure("foo", LineKind::Plain).width, 3.0);
        assert_eq!(measure("foo", LineKind::Plain).width, 3.0);
        // The same line typeset by TeX is another key.
        measure("foo", LineKind::Tex);
        measure("ab", LineKind::Plain);
        assert_eq!(measured, 3);
        assert_eq!(
            cache.info(),
            MetricsCacheInfo {
                hits: 1,
                misses: 3,
                size: 3
            }
        );
        cache.clear();
        assert_eq!(cache.info(), MetricsCacheInfo::default());
    }

    #[test]
    fn failures_are_not_cached() {
        let cache = MetricsCache::new();
        let font = font();
        let failed = cache.get_or_measure("x", &font, LineKind::Math, 72.0, || {
            Err(TextError::Math("bad".to_owned()))
        });
        assert!(failed.is_err());
        let info = cache.info();
        assert_eq!((info.misses, info.size), (1, 0));
    }

    #[test]
    fn the_least_recently_used_line_is_forgotten_when_full() {
        let cache = MetricsCache::new();
        let font = font();
        let measure = |line: &str| {
            cache
                .get_or_measure(line, &font, LineKind::Plain, 72.0, || Ok(metrics(1.0)))
                .expect("measured");
        };
        for i in 0..METRICS_CACHE_SIZE {
            measure(&i.to_string());
        }
        // "0" is used again, so "1" is the least recent when "new" comes.
        measure("0");
        measure("new");
        let info = cache.info();
        assert_eq!(info.size, METRICS_CACHE_SIZE);
        measure("0");
        assert_eq!(cache.info().hits, info.hits + 1);
        measure("1");
        assert_eq!(cache.info().misses, info.misses + 1);
    }

    #[test]
    fn laying_text_out_measures_its_lines_through_the_installed_cache() {
        let rc = RcParams::matplotlib();
        let font = font();
        let style = pltrs_text::BlockStyle::default();
        let cache = Rc::new(MetricsCache::new());
        let lay_out = || super::super::lay_out("ab\ncd\n", &font, &style, 100.0, &rc);
        let uncached = lay_out().expect("laid out");
        let before = set_metrics_cache(Some(Rc::clone(&cache)));
        let first = lay_out().expect("laid out");
        let second = lay_out().expect("laid out");
        set_metrics_cache(before);
        assert_eq!(first, uncached);
        assert_eq!(second, uncached);
        // Two lines measured, then found; the empty line is not measured.
        let info = cache.info();
        assert_eq!((info.hits, info.misses, info.size), (2, 2, 2));
    }

    #[test]
    fn the_installed_cache_measures_the_thread_s_lines() {
        let font = font();
        let cache = Rc::new(MetricsCache::new());
        let measure = || cached("lp", &font, LineKind::Plain, 72.0, || Ok(metrics(5.0)));
        measure().expect("measured");
        assert_eq!(cache.info(), MetricsCacheInfo::default());
        let before = set_metrics_cache(Some(Rc::clone(&cache)));
        measure().expect("measured");
        measure().expect("measured");
        let restored = set_metrics_cache(before);
        assert!(restored.is_some_and(|restored| Rc::ptr_eq(&restored, &cache)));
        assert_eq!((cache.info().hits, cache.info().misses), (1, 1));
    }
}
