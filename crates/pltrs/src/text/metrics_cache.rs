//! Text metrics cached per renderer (matplotlib's
//! `text._get_text_metrics_function`): matplotlib measures every line of
//! text through a least-recently-used cache of its renderer, keyed by the
//! line, its font, how it is typeset and the resolution. pltrs (Python)
//! keeps such a cache with each renderer it hands out and installs it on
//! the thread while pltrs lays out or draws a figure with that renderer,
//! so that the lines pltrs measures meanwhile go through it, and the
//! program sees the cache matplotlib's would have filled. Where matplotlib
//! lays a text out again only to draw it, pltrs looks its lines up again
//! without laying it out ([`LaidOutText::measure_again`]).

mod lru;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use pltrs_text::{LineMetrics, TextError};

use self::lru::Lines;
use super::font_notes;
use super::layout::{LaidOutText, Typesetting, measure_uncached, measured_as};
use super::math::Line;
use super::overrides::LineKind;
use super::tex::{measure_tex_again, tex_height_metrics};
use crate::fonts::FontProperties;
use crate::rc::RcParams;

/// How many lines a cache holds before it forgets the least recently used
/// (matplotlib's `lru_cache(4096)`).
pub const METRICS_CACHE_SIZE: usize = 4096;

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
        if let Some(metrics) = self.lines.borrow_mut().find(line, font, kind, dpi) {
            return Ok(metrics);
        }
        let metrics = measure()?;
        self.lines
            .borrow_mut()
            .insert(line, font, kind, dpi, metrics);
        Ok(metrics)
    }

    /// What the cache has done so far.
    pub fn info(&self) -> MetricsCacheInfo {
        self.lines.borrow().info()
    }

    /// Forgets every line and the statistics (`cache_clear()`).
    pub fn clear(&self) {
        self.lines.borrow_mut().clear();
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

/// The cache installed on the thread, held weakly: what tells the
/// renderers measuring and drawing apart (see
/// [`math_parses`](super::math_parses)).
pub(super) fn current_weak() -> Option<Weak<MetricsCache>> {
    CURRENT.with_borrow(|current| current.as_ref().map(Rc::downgrade))
}

/// Runs `measure` with `cache` installed in place of the thread's.
pub(super) fn with_cache<R>(cache: Rc<MetricsCache>, measure: impl FnOnce() -> R) -> R {
    let before = set_metrics_cache(Some(cache));
    let measured = measure();
    set_metrics_cache(before);
    measured
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

impl LaidOutText {
    /// Looks the lines up again in the thread's cache, if one is
    /// installed, as laying the text out again at `dpi` would, without
    /// laying them out: a line still cached counts a hit, a line no longer
    /// cached is measured again, and a text typeset by TeX has TeX's `"lp"`
    /// looked up after its lines, as its layout measures it. matplotlib's
    /// `Text.draw` lays out the text it draws, so the renderer's cache sees
    /// every line once more; the layout it makes is this one.
    ///
    /// # Errors
    ///
    /// When a line no longer cached cannot be measured.
    pub(crate) fn measure_again(&self, dpi: f64, rc: &RcParams) -> Result<(), TextError> {
        if !installed() {
            return Ok(());
        }
        for line in &self.lines {
            match self.typesetting {
                Typesetting::Tex => measure_tex_again(line, &self.font, dpi, rc)?,
                Typesetting::Mathtext { .. } if line.is_empty() => {}
                Typesetting::Mathtext { parse_math } => {
                    let line = Line::classify(line, parse_math);
                    let (text, kind) = measured_as(&line);
                    cached(text, &self.font, kind, dpi, || {
                        measure_uncached(&line, &self.font, dpi, rc)
                    })?;
                }
            }
        }
        if self.typesetting == Typesetting::Tex {
            tex_height_metrics(&self.font, dpi, rc)?;
        }
        let text = self.lines.join("\n");
        for _ in 0..font_notes::layouts_of(&text) {
            self.warn_laying_out();
        }
        font_notes::note_layout(&text, &self.font, self.typesetting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Measuring `text`, laid out with `typesetting`, again looks the same
    /// lines up as laying it out again, and measures those no longer cached
    /// as laying it out measured them.
    fn assert_measured_again_as_laid_out_again(text: &str, typesetting: Typesetting) {
        let rc = RcParams::matplotlib();
        let font = font();
        let style = pltrs_text::BlockStyle::default();
        let lay_out = || {
            super::super::lay_out_with(text, &font, &style, 100.0, &rc, typesetting)
                .expect("laid out")
        };
        let cache = Rc::new(MetricsCache::new());
        let before = set_metrics_cache(Some(Rc::clone(&cache)));
        lay_out();
        lay_out();
        let laid_out_again = cache.info();
        cache.clear();
        let laid_out = lay_out();
        laid_out.measure_again(100.0, &rc).expect("measured");
        assert_eq!(cache.info(), laid_out_again);
        cache.clear();
        laid_out.measure_again(100.0, &rc).expect("measured");
        set_metrics_cache(before);
        assert_eq!(cache.info().size, laid_out_again.size);
        for (line, run) in laid_out.lines.iter().zip(&laid_out.runs) {
            let kind = match typesetting {
                _ if line.is_empty() => continue,
                Typesetting::Tex => LineKind::Tex,
                Typesetting::Mathtext { .. } if line.contains('$') => LineKind::Math,
                Typesetting::Mathtext { .. } => LineKind::Plain,
            };
            let found = cache
                .get_or_measure(line, &font, kind, 100.0, || {
                    Err(TextError::Font("not cached".to_owned()))
                })
                .expect("cached");
            assert_eq!(
                (found.width, found.height, found.descent),
                (run.width, run.height, run.descent),
                "{line}"
            );
        }
    }

    #[test]
    fn measuring_again_looks_lines_up_as_laying_out_again_does() {
        assert_measured_again_as_laid_out_again(
            "ab\n$x^2$\n\ncd",
            Typesetting::Mathtext { parse_math: true },
        );
        if super::super::check_usetex().is_ok() {
            assert_measured_again_as_laid_out_again("ab\n\ncd", Typesetting::Tex);
        }
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
