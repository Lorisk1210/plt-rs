//! matplotlib's `_text_helpers` on the native side: the warning matplotlib
//! issues for each character it lays out that no font has, the font
//! manager's log messages about the fonts of the texts it lays out (a
//! family no font has, a face of another weight) and mathtext's about the
//! lists it packs to sizes their glue cannot reach, delivered at the end
//! of the native call during which pltrs (Rust) laid them out.

use pltrs_text::LookupKind;

use crate::binding::lookup;
use crate::binding::prelude::*;

/// Issues `_text_helpers.warn_on_missing_glyph(codepoint, fontnames)`
/// (registered by the package as `warn_on_missing_glyph`) for each
/// character pltrs (Rust) laid out on this thread that no font of its text
/// has.
pub(super) fn warn_missing_glyphs(py: Python<'_>) -> PyResult<()> {
    let missing = pltrs_text::take_missing_glyphs();
    if missing.is_empty() {
        return Ok(());
    }
    let warn = lookup(py, "warn_on_missing_glyph")?;
    for glyph in missing {
        let codepoint = u32::from(glyph.character).into_py(py)?;
        let families = glyph.families.join(", ").into_py(py)?;
        warn.call(&[codepoint, families], &[])?;
    }
    Ok(())
}

/// Hands each font lookup pltrs (Rust) made on this thread that
/// matplotlib's font manager logs about to the package's
/// `report_font_lookup(kind, family, style, variant, weight, stretch,
/// size, times)` (registered by `_text_helpers`), which looks the fonts
/// up with the program's font manager as many times as pltrs (Rust) made
/// the lookup in a row, logging as matplotlib's does.
pub(super) fn report_font_lookups(py: Python<'_>) -> PyResult<()> {
    let runs = pltrs_text::take_font_lookups();
    if runs.is_empty() {
        return Ok(());
    }
    let report = lookup(py, "report_font_lookup")?;
    for run in runs {
        let font = &run.lookup;
        let query = &font.query;
        let kind = match font.kind {
            LookupKind::Fonts => "fonts",
            LookupKind::Font => "font",
        };
        let weight = match font.weight_keyword {
            Some(keyword) => keyword.into_py(py)?,
            None => i64::from(query.weight.0).into_py(py)?,
        };
        report.call(
            &[
                kind.into_py(py)?,
                query.families.clone().into_py(py)?,
                query.style.name().into_py(py)?,
                query.variant.name().into_py(py)?,
                weight,
                i64::from(query.stretch.0).into_py(py)?,
                query.size.into_py(py)?,
                run.times.into_py(py)?,
            ],
            &[],
        )?;
    }
    Ok(())
}

/// Hands each message mathtext logs about a list pltrs (Rust) packed on
/// this thread to a size its glue cannot reach ("Underful Hlist: ...") to
/// the package's `log_mathtext_warning(message)` (registered by
/// `_mathtext`), which logs it as matplotlib's `List._set_glue` does.
pub(super) fn log_glue_warnings(py: Python<'_>) -> PyResult<()> {
    let messages = pltrs_text::take_glue_warnings();
    if messages.is_empty() {
        return Ok(());
    }
    let log = lookup(py, "log_mathtext_warning")?;
    for message in messages {
        log.call(&[message.into_py(py)?], &[])?;
    }
    Ok(())
}
