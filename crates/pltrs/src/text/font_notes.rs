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
//! ([`pltrs_text::take_font_notes`], [`pltrs_text::take_missing_glyphs`]).
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

use pltrs_text::{AfmDirectory, TextEngine, TextError, TextRun, TextSource};

use super::layout::Typesetting;
use super::math::Line;
use super::math_parses;
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
    /// Whether the lookups noted now stand for one of the draws of such a
    /// print only: its layout engine laying it out (matplotlib runs it in
    /// the first draw only), or what pltrs measured before the print.
    static SINGLE: Cell<bool> = const { Cell::new(false) };
    /// Whether the figure drawn now is drawn without rendering, its
    /// renderer drawing no text.
    static NOT_RENDERING: Cell<bool> = const { Cell::new(false) };
    /// Whether texts are drawn through path effects now.
    static THROUGH_EFFECTS: Cell<bool> = const { Cell::new(false) };
    /// Whether the texts laid out now are laid out again where they are
    /// drawn, which notes their layout then.
    static PLACED_BEFORE_DRAWN: Cell<bool> = const { Cell::new(false) };
    /// How many of matplotlib's `Text._get_layout` each layout made now
    /// stands for (see [`layouts_counted`]).
    static LAYOUT_TIMES: Cell<usize> = const { Cell::new(1) };
    /// Whether the texts laid out now are those of text areas measured
    /// (see [`text_area_layouts`]).
    static IN_TEXT_AREA: Cell<bool> = const { Cell::new(false) };
    /// Whether the texts laid out now are laid out for their outlines
    /// (see [`outlined`]).
    static OUTLINED: Cell<bool> = const { Cell::new(false) };
}

/// Runs `outline`, which lays text out for its outline (matplotlib's
/// `TextToPath.get_text_path`, as `TextPath` makes it): the lookups of its
/// fonts are noted, but not the characters no font has, as matplotlib's
/// layout of a text path (`_text_helpers.layout`) warns about none.
pub(crate) fn outlined<R>(outline: impl FnOnce() -> R) -> R {
    let before = OUTLINED.replace(true);
    let result = outline();
    OUTLINED.set(before);
    result
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

/// How many of matplotlib's draws the draw running now stands for: two
/// for a print matplotlib draws once before (see [`predrawn`]), else one.
pub(crate) fn draws_of_print() -> usize {
    if PRE_DRAW.get().is_some() { 2 } else { 1 }
}

/// Whether the program's code of the draw before the print running now
/// ran already as the print began (a print cut to a window, see
/// `Figure::predraw_program`), the draw running now calling it once.
pub(crate) fn program_predrawn() -> bool {
    PRE_DRAW.get() == Some(PreDraw::Measured)
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
    let before = SINGLE.replace(true);
    let laid_out = if pre_draw == PreDraw::OwnRenderers {
        with_renderer_of_its_own(lay_out)
    } else {
        lay_out()
    };
    SINGLE.set(before);
    laid_out
}

/// Runs `measure`, a part of the draw of a print matplotlib draws twice
/// that pltrs measured before the print (the titles of a print cut to its
/// tight box, placed in the layout that measured it): its lookups stand
/// for the print's draw only.
pub(crate) fn measured_before<R>(measure: impl FnOnce() -> R) -> R {
    if PRE_DRAW.get() != Some(PreDraw::Measured) {
        return measure();
    }
    let before = SINGLE.replace(true);
    let measured = measure();
    SINGLE.set(before);
    measured
}

/// How many draws a lookup noted now stands for: two for a lookup of the
/// draw of a print matplotlib draws twice, outside its engine, where
/// `measured` (a line measured, not found in the cache) counts only with
/// renderers of the first draw's own (whose cache misses it too).
fn draws_noted(measured: bool) -> usize {
    match PRE_DRAW.get() {
        Some(pre_draw) if !SINGLE.get() && (!measured || pre_draw == PreDraw::OwnRenderers) => 2,
        _ => 1,
    }
}

/// Runs `measure`, which measures texts again for what matplotlib keeps
/// from its last measure of them (a text area's baseline, the box a
/// legend's draw located), noting nothing and looking no line up in the
/// renderer's text metrics cache: matplotlib does not look their fonts up
/// again there, nor ask its renderer.
pub(crate) fn unnoted<R>(measure: impl FnOnce() -> R) -> R {
    let before = UNNOTED.replace(true);
    let result = measure();
    UNNOTED.set(before);
    result
}

/// Runs `place`, which lays texts out that their draw lays out again,
/// noting the layout then: matplotlib's draw of the texts of an axis
/// artist measures them (noted), then lays each out once, as it draws it
/// (`Text.draw`). Measuring them is noted as before.
pub(crate) fn placed_before_drawn<R>(place: impl FnOnce() -> R) -> R {
    let before = PLACED_BEFORE_DRAWN.replace(true);
    let result = place();
    PLACED_BEFORE_DRAWN.set(before);
    result
}

/// Runs `lay_out`, each layout of a text in which stands for `times` of
/// matplotlib's `Text._get_layout` of it: none where the caller notes
/// them itself (an offset box's tree, which matplotlib measures a number
/// of times that depends on what it does with the box), two where
/// matplotlib measures the texts twice and pltrs once (the tight layout
/// engine's two passes). Nested, the counts multiply. The lines measured
/// (uncached) are noted as ever.
pub(crate) fn layouts_counted<R>(times: usize, lay_out: impl FnOnce() -> R) -> R {
    let before = LAYOUT_TIMES.get();
    LAYOUT_TIMES.set(before * times);
    let result = lay_out();
    LAYOUT_TIMES.set(before);
    result
}

/// [`layouts_counted`] for the texts of text areas, which matplotlib lays
/// out even when they are empty: `TextArea.get_bbox` calls
/// `Text._get_layout` whatever the text, where a text's own draw and
/// measures skip an empty one.
pub(crate) fn text_area_layouts<R>(times: usize, lay_out: impl FnOnce() -> R) -> R {
    let before = IN_TEXT_AREA.replace(true);
    let result = layouts_counted(times, lay_out);
    IN_TEXT_AREA.set(before);
    result
}

/// Whether texts measured now note what matplotlib's measures note.
pub(super) fn noting() -> bool {
    !UNNOTED.get()
}

/// Whether laying `text` out now stands for matplotlib's
/// `Text._get_layout` of it: not for an empty text (which matplotlib lays
/// out only in a text area, see [`text_area_layouts`]), nor for layouts
/// matplotlib keeps from an earlier one (see [`unnoted`]) or makes only
/// where it draws (see [`placed_before_drawn`]).
fn lays_out_as_matplotlib(text: &str) -> bool {
    noting() && !PLACED_BEFORE_DRAWN.get() && (!text.is_empty() || IN_TEXT_AREA.get())
}

/// How many of matplotlib's `Text._get_layout` of `text` laying it out now
/// stands for: none where matplotlib does not lay it out (see
/// [`lays_out_as_matplotlib`]), else one per draw (two in the draw of a
/// print matplotlib draws once before, see [`predrawn`]), as many times
/// as [`layouts_counted`] says.
pub(super) fn layouts_of(text: &str) -> usize {
    if lays_out_as_matplotlib(text) {
        draws_noted(false) * LAYOUT_TIMES.get()
    } else {
        0
    }
}

/// Notes the lookup matplotlib's `Text._get_layout` makes for the height
/// of the font of `text`: none where it does not lay the text out (see
/// [`layouts_of`]), for TeX, or for the AFM fonts the PDF and PostScript backends
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
    if typesetting == Typesetting::Tex {
        return Ok(());
    }
    let request = font.drawn_request();
    if let Some(directory) = metrics::afm_directory() {
        // The renderer measures the text in an AFM font it looks up.
        TextEngine::global().note_afm_lookup(&request, directory);
        return Ok(());
    }
    for _ in 0..layouts_of(text) {
        TextEngine::global().note_font_lookup(&request)?;
    }
    Ok(())
}

/// Notes what measuring the plain line `line` of `run`, uncached, notes in
/// matplotlib (`get_text_width_height_descent`): the lookup of its fonts
/// and the characters none has; in the AFM fonts of the PDF and
/// PostScript backends, the lookup of the AFM font alone (`_get_font_afm`).
/// Math lines are measured by mathtext, which notes neither, but parses
/// them (see [`math_parses`]).
///
/// # Errors
///
/// When the fonts cannot be looked up.
pub(super) fn note_measured(
    line: &Line<'_>,
    font: &FontProperties,
    run: &TextRun,
) -> Result<(), TextError> {
    if !noting() {
        return Ok(());
    }
    let Line::Plain(text) = line else {
        math_parses::parsed(run, crate::rc::params().text_antialiased);
        return Ok(());
    };
    let engine = TextEngine::global();
    let request = font.drawn_request();
    for _ in 0..draws_noted(true) {
        if let Some(directory) = metrics::afm_directory() {
            engine.note_afm_lookup(&request, directory);
            continue;
        }
        engine.note_font_lookup(&request)?;
        if !OUTLINED.get() {
            engine.note_missing_glyphs(run, text.as_ref(), &request)?;
        }
    }
    Ok(())
}

/// Runs `draw`, which draws through path effects: matplotlib's
/// `PathEffectRenderer` draws text as paths (`RendererBase.draw_text`),
/// looking its fonts up even in the draw without rendering before a
/// print, whose renderer's own `draw_text` does nothing.
pub(crate) fn through_effects<R>(draw: impl FnOnce() -> R) -> R {
    let before = THROUGH_EFFECTS.replace(true);
    let drawn = draw();
    THROUGH_EFFECTS.set(before);
    drawn
}

/// Notes the lookup drawing the line `run` makes in matplotlib (its
/// renderers' `draw_text` prepares the fonts of plain lines, the PDF and
/// PostScript backends their AFM font where they draw in it): in each of
/// the draws of a print drawn through path effects (whose text paths are
/// TrueType's), else in the last.
/// Drawing a math line parses it, with `antialiased` glyphs (see
/// [`math_parses`]).
///
/// # Errors
///
/// When the fonts cannot be looked up.
pub(super) fn note_drawn(run: &TextRun, antialiased: bool) -> Result<(), TextError> {
    if NOT_RENDERING.get() {
        return Ok(());
    }
    math_parses::parsed(run, antialiased);
    let Some(TextSource::Plain { font, .. }) = &run.source else {
        return Ok(());
    };
    if THROUGH_EFFECTS.get() {
        for _ in 0..draws_of_print() {
            TextEngine::global().note_font_lookup(font)?;
        }
    } else if let Some(directory) = metrics::afm_directory() {
        TextEngine::global().note_afm_lookup(font, directory);
        if directory == AfmDirectory::PdfCore {
            TextEngine::global().note_pdf_font_name_lookup(font);
        }
    } else {
        TextEngine::global().note_font_lookup(font)?;
    }
    Ok(())
}
