//! The formulas matplotlib's mathtext parser keeps
//! (`MathTextParser._parse_cached`, a least-recently-used cache of 50
//! parses keyed by the formula, the resolution, the font properties, the
//! antialiasing and the glyph loading flags): a formula's messages
//! ("Underful Hlist: ...", see
//! [`log_mathtext`](pltrs_text::log_mathtext)) are logged when it
//! is parsed, so only when the cache misses. matplotlib's renderers parse a
//! formula when they measure a line their metrics cache misses (with the
//! antialiasing of `text.antialiased`) and when Agg draws it (with the
//! text's own, so a text drawn without antialiasing parses again); pltrs
//! typesets formulas wherever it needs them, and notes those two moments
//! here ([`parsed`]).
//!
//! matplotlib's keys hold the parser, one per renderer (each figure's
//! canvas has its own), which pltrs tells apart by the renderer's text
//! metrics cache (see [`metrics_cache`](super::metrics_cache)), and the
//! float objects the program gave: a NaN size measured and then drawn is
//! the same object, so the parses of one text of NaN size share a key,
//! which pltrs keys by NaN.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Weak;

use pltrs_text::mathtext::MathStyle;
use pltrs_text::{TextRun, TextSource};

use super::metrics_cache::{self, MetricsCache};

/// How many parses the cache keeps (matplotlib's `lru_cache(50)`).
const PARSES_KEPT: usize = 50;

/// What a parse is keyed by.
#[derive(Debug)]
struct ParseKey {
    /// The renderer parsing (its metrics cache), if one is installed; held
    /// weakly, so that no other takes its place.
    renderer: Option<Weak<MetricsCache>>,
    /// The formula, `$` signs included.
    text: String,
    /// Its fonts, size, resolution and hinting.
    style: MathStyle,
    /// Whether its glyphs are antialiased.
    antialiased: bool,
}

impl ParseKey {
    /// Whether `self` and `other` key the same parse: equal, sizes and
    /// resolutions that are NaN in both included.
    fn same(&self, other: &Self) -> bool {
        let same_renderer = match (&self.renderer, &other.renderer) {
            (None, None) => true,
            (Some(a), Some(b)) => Weak::ptr_eq(a, b),
            _ => false,
        };
        same_renderer
            && self.text == other.text
            && self.antialiased == other.antialiased
            && same_style(&self.style, &other.style)
    }
}

/// Whether `a` and `b` are equal, or would be if their sizes and their
/// resolutions that are NaN in both were equal.
fn same_style(a: &MathStyle, b: &MathStyle) -> bool {
    if a == b {
        return true;
    }
    let both_nan = |x: f64, y: f64| x.is_nan() && y.is_nan();
    let nan_size = both_nan(a.font.query.size, b.font.query.size);
    let nan_dpi = both_nan(a.dpi, b.dpi);
    if !(nan_size || nan_dpi) {
        return false;
    }
    let settled = |style: &MathStyle| {
        let mut style = style.clone();
        if nan_size {
            style.font.query.size = 0.0;
        }
        if nan_dpi {
            style.dpi = 0.0;
        }
        style
    };
    settled(a) == settled(b)
}

thread_local! {
    /// The parses kept, the least recently used first.
    static PARSES: RefCell<VecDeque<ParseKey>> = const { RefCell::new(VecDeque::new()) };
}

/// Notes that matplotlib parses the formula `run` was typeset from, with
/// `antialiased` glyphs: its messages are logged unless the parser keeps
/// that parse. Runs of plain text or TeX are not parsed by mathtext.
pub(crate) fn parsed(run: &TextRun, antialiased: bool) {
    let Some(TextSource::Math {
        text,
        style,
        logged,
    }) = &run.source
    else {
        return;
    };
    let key = ParseKey {
        renderer: metrics_cache::current_weak(),
        text: text.clone(),
        style: style.clone(),
        antialiased,
    };
    let kept = PARSES.with_borrow_mut(|parses| {
        let found = parses.iter().position(|kept| kept.same(&key));
        let kept = found.is_some();
        if let Some(index) = found {
            parses.remove(index);
        } else if parses.len() == PARSES_KEPT {
            parses.pop_front();
        }
        parses.push_back(key);
        kept
    });
    if !kept {
        pltrs_text::log_mathtext(logged);
    }
}

/// Logs the messages typesetting the formula `run` noted, as a parser of
/// its own (which keeps nothing yet) logs them.
pub(crate) fn log_typesetting(run: &TextRun) {
    if let Some(TextSource::Math { logged, .. }) = &run.source {
        pltrs_text::log_mathtext(logged);
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use pltrs_text::TextEngine;

    /// A run of `text` typeset at `size` points, noting `logged`.
    fn run(text: &str, size: f64, logged: &[&str]) -> TextRun {
        let style = MathStyle::default().with_size(size);
        let mut run = TextEngine::global()
            .layout_math("$x$", &MathStyle::default())
            .expect("typesets");
        run.source = Some(TextSource::Math {
            text: text.to_owned(),
            style,
            logged: logged.iter().map(|&message| message.to_owned()).collect(),
        });
        run
    }

    #[test]
    fn a_kept_parse_logs_nothing() {
        PARSES.with_borrow_mut(VecDeque::clear);
        pltrs_text::take_mathtext_log();
        let nan = run("$a$", f64::NAN, &["Underful Hlist"]);
        parsed(&nan, true);
        parsed(&nan, true);
        assert_eq!(pltrs_text::take_mathtext_log(), ["Underful Hlist"]);
        parsed(&nan, false);
        assert_eq!(pltrs_text::take_mathtext_log(), ["Underful Hlist"]);
    }

    #[test]
    fn the_least_recently_used_parse_is_forgotten() {
        PARSES.with_borrow_mut(VecDeque::clear);
        pltrs_text::take_mathtext_log();
        let first = run("$first$", 10.0, &["first"]);
        parsed(&first, true);
        for index in 0..PARSES_KEPT {
            parsed(&run(&format!("${index}$"), 10.0, &[]), true);
        }
        parsed(&first, true);
        assert_eq!(pltrs_text::take_mathtext_log(), ["first", "first"]);
    }

    #[test]
    fn each_renderer_parses_for_itself() {
        PARSES.with_borrow_mut(VecDeque::clear);
        pltrs_text::take_mathtext_log();
        let nan = run("$a$", f64::NAN, &["Underful Hlist"]);
        for _ in 0..2 {
            let renderer = Rc::new(MetricsCache::new());
            metrics_cache::with_cache(Rc::clone(&renderer), || parsed(&nan, true));
        }
        assert_eq!(pltrs_text::take_mathtext_log(), ["Underful Hlist"; 2]);
    }

    #[test]
    fn styles_differing_only_in_nan_are_the_same() {
        let nan = MathStyle::default().with_size(f64::NAN).with_dpi(f64::NAN);
        assert!(same_style(&nan, &nan.clone()));
        assert!(!same_style(&nan, &nan.clone().with_size(10.0)));
        assert!(!same_style(
            &MathStyle::default(),
            &MathStyle::default().with_dpi(100.0)
        ));
    }

    #[test]
    fn plain_runs_are_not_parsed() {
        pltrs_text::take_mathtext_log();
        log_typesetting(&TextRun::default());
        parsed(&TextRun::default(), true);
        assert_eq!(pltrs_text::take_mathtext_log(), Vec::<String>::new());
    }
}
