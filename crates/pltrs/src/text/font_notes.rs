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

use std::cell::Cell;

use pltrs_text::{TextEngine, TextError, TextRun, TextSource};

use super::layout::Typesetting;
use super::math::Line;
use super::metrics;
use crate::fonts::FontProperties;

thread_local! {
    /// Whether the texts laid out now are measured again for what
    /// matplotlib keeps from an earlier measure, noting nothing.
    static UNNOTED: Cell<bool> = const { Cell::new(false) };
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
    TextEngine::global().note_font_lookup(&font.drawn_request())
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
    engine.note_font_lookup(&request)?;
    engine.note_missing_glyphs(run, text.as_ref(), &request)
}

/// Notes the lookup drawing the line `run` makes in matplotlib (its
/// renderers' `draw_text` prepares the fonts of plain lines).
///
/// # Errors
///
/// When the fonts cannot be looked up.
pub(super) fn note_drawn(run: &TextRun) -> Result<(), TextError> {
    match &run.source {
        Some(TextSource::Plain { font, .. }) => TextEngine::global().note_font_lookup(font),
        _ => Ok(()),
    }
}
