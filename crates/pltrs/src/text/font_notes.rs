//! What matplotlib's renderers note about the fonts of the texts they lay
//! out, measure and draw, at the moments they note it: the font manager
//! looks the fonts of a text up (`_find_fonts_by_props`, which logs about
//! a family no font has or a face of another weight) when `Text._get_layout`
//! takes the height of its font, when a renderer measures one of its lines
//! that the cache of line metrics does not hold, and when a renderer draws
//! one of its lines (`_prepare_font` in `draw_text`); `FreeType`'s layout
//! warns about the characters no font has only when a line is measured.
//! pltrs (Rust) lays texts out from memoised runs, so it notes these
//! moments itself, for pltrs (Python) to report
//! ([`pltrs_text::take_font_lookups`], [`pltrs_text::take_missing_glyphs`]).
//!
//! matplotlib prints a figure with a layout engine (the placeholder
//! `tight_layout()` leaves included), or the tight box of a figure, twice:
//! it draws it once with rendering disabled, its engine laying it out,
//! then prints it with the engine off. pltrs draws such a print once
//! ([`predrawn`]), so the lookups of the first draw are noted with those
//! of the print: each one a draw notes but `draw_text`'s twice, but those
//! of the engine ([`layout_engine`]), which runs in the first draw only.

use std::cell::Cell;
use std::rc::Rc;

use pltrs_text::{TextEngine, TextError, TextRun, TextSource};

use super::layout::Typesetting;
use super::math::Line;
use super::metrics;
use super::metrics_cache::{self, MetricsCache};
use crate::fonts::FontProperties;

thread_local! {
    /// Whether the texts laid out now are measured again for what
    /// matplotlib keeps from an earlier measure, noting nothing.
    static UNNOTED: Cell<bool> = const { Cell::new(false) };
    /// The draw matplotlib makes before the print drawn now, if it makes
    /// one.
    static PRE_DRAW: Cell<Option<PreDraw>> = const { Cell::new(None) };
    /// Whether the layout engine of such a print is laying it out.
    static ENGINE: Cell<bool> = const { Cell::new(false) };
    /// Whether the figure drawn now is drawn without rendering, its
    /// renderer drawing no text.
    static NOT_RENDERING: Cell<bool> = const { Cell::new(false) };
}

/// Runs `draw`, a draw without rendering (matplotlib's
/// `draw_without_rendering`, whose renderer's `draw_text` does nothing):
/// the lookups of drawing texts are not noted.
pub(crate) fn without_rendering<R>(draw: impl FnOnce() -> R) -> R {
    let before = NOT_RENDERING.replace(true);
    let drawn = draw();
    NOT_RENDERING.set(before);
    drawn
}

/// The draw matplotlib makes before a print, and its renderers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreDraw {
    /// The first draw of a raster print of the whole figure, with the
    /// renderer printing (the canvas' renderer), whose text metrics cache
    /// it fills.
    SameRenderer,
    /// The first draw of a vector print of the whole figure, with
    /// renderers of their own for the draw and for its engine, whose
    /// caches start empty (`_get_renderer` makes one from the print
    /// method, and the engine another).
    OwnRenderers,
    /// The first draw of a print of a part of the figure (its tight box,
    /// or a box of the figure with a layout engine), which pltrs made
    /// already measuring the part, but for the texts its draw lays out
    /// again: the print draws with a renderer of its own, the size of the
    /// part.
    Measured,
}

/// Runs `draw`, the draw of a print that matplotlib draws once more
/// before (`pre_draw`, if it does), noting the lookups of both draws.
pub(crate) fn predrawn<R>(pre_draw: Option<PreDraw>, draw: impl FnOnce() -> R) -> R {
    let before = PRE_DRAW.replace(pre_draw);
    let drawn = if pre_draw == Some(PreDraw::Measured) {
        with_renderer_of_its_own(draw)
    } else {
        draw()
    };
    PRE_DRAW.set(before);
    drawn
}

/// Runs `measure` with a renderer of its own, whose text metrics cache
/// starts empty, where the program measures through a cache.
pub(crate) fn with_renderer_of_its_own<R>(measure: impl FnOnce() -> R) -> R {
    if metrics_cache::installed() {
        metrics_cache::with_cache(Rc::new(MetricsCache::new()), measure)
    } else {
        measure()
    }
}

/// Runs the layout engine (`lay_out`) of the figure drawn now, which
/// matplotlib runs in the first draw of a print only: its lookups are
/// noted once, with a text metrics cache of its own where that draw has
/// renderers of its own.
pub(crate) fn layout_engine<R>(lay_out: impl FnOnce() -> R) -> R {
    let Some(pre_draw) = PRE_DRAW.get() else {
        return lay_out();
    };
    let before = ENGINE.replace(true);
    let laid_out = if pre_draw == PreDraw::OwnRenderers {
        with_renderer_of_its_own(lay_out)
    } else {
        lay_out()
    };
    ENGINE.set(before);
    laid_out
}

/// How many draws a lookup noted now stands for: two for a lookup of the
/// draw of a print matplotlib draws twice, outside its engine, where
/// `measured` (a line measured, not found in the cache) counts only with
/// renderers of the first draw's own (whose cache misses it too).
fn draws_noted(measured: bool) -> usize {
    match PRE_DRAW.get() {
        Some(pre_draw) if !ENGINE.get() && (!measured || pre_draw == PreDraw::OwnRenderers) => 2,
        _ => 1,
    }
}

/// Runs `measure`, which measures texts again for what matplotlib keeps
/// from its last measure of them (a text area's baseline, the box a
/// legend's draw located), noting nothing: matplotlib does not look their
/// fonts up again there.
pub(crate) fn unnoted<R>(measure: impl FnOnce() -> R) -> R {
    let before = UNNOTED.replace(true);
    let result = measure();
    UNNOTED.set(before);
    result
}

/// Whether texts measured now note what matplotlib's measures note.
fn noting() -> bool {
    !UNNOTED.get()
}

/// Notes the lookup matplotlib's `Text._get_layout` makes for the height
/// of the font of `text`: none for an empty text (which matplotlib never
/// lays out), for TeX, or for the AFM fonts the PDF and PostScript backends
/// measure with (`_get_font_height_metrics`).
///
/// # Errors
///
/// When the fonts cannot be looked up.
pub(super) fn note_layout(
    text: &str,
    font: &FontProperties,
    typesetting: Typesetting,
) -> Result<(), TextError> {
    if !noting() || text.is_empty() || typesetting == Typesetting::Tex || metrics::measures_afm() {
        return Ok(());
    }
    let request = font.drawn_request();
    for _ in 0..draws_noted(false) {
        TextEngine::global().note_font_lookup(&request)?;
    }
    Ok(())
}

/// Notes what measuring the plain line `line` of `run`, uncached, notes in
/// matplotlib (`get_text_width_height_descent`): the lookup of its fonts
/// and the characters none has. Math lines are measured by mathtext, which
/// notes neither.
///
/// # Errors
///
/// When the fonts cannot be looked up.
pub(super) fn note_measured(
    line: &Line<'_>,
    font: &FontProperties,
    run: &TextRun,
) -> Result<(), TextError> {
    let Line::Plain(text) = line else {
        return Ok(());
    };
    if !noting() {
        return Ok(());
    }
    let engine = TextEngine::global();
    let request = font.drawn_request();
    for _ in 0..draws_noted(true) {
        engine.note_font_lookup(&request)?;
        engine.note_missing_glyphs(run, text.as_ref(), &request)?;
    }
    Ok(())
}

/// Notes the lookup drawing the line `run` makes in matplotlib (its
/// renderers' `draw_text` prepares the fonts of plain lines).
///
/// # Errors
///
/// When the fonts cannot be looked up.
pub(super) fn note_drawn(run: &TextRun) -> Result<(), TextError> {
    if NOT_RENDERING.get() {
        return Ok(());
    }
    match &run.source {
        Some(TextSource::Plain { font, .. }) => TextEngine::global().note_font_lookup(font),
        _ => Ok(()),
    }
}
