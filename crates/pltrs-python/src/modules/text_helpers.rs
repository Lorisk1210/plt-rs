//! matplotlib's `_text_helpers` on the native side: the warning matplotlib
//! issues for each character it lays out that no font has, the font
//! manager's log messages about the fonts of the texts it lays out (a
//! family no font has, a face of another weight) and mathtext's about the
//! formulas it typesets, delivered at the end of the native call during
//! which pltrs (Rust) laid them out.

use pltrs_text::{AfmDirectory, FontNote, FontSource, LookupKind, LookupRun};

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

/// Hands what pltrs (Rust) noted of fonts on this thread to the
/// package, in order: each font lookup matplotlib's font manager logs
/// about to `report_font_lookup(kind, family, style, variant, weight,
/// stretch, size, times)`, which looks the fonts up with the program's
/// font manager as many times as pltrs (Rust) made the lookup in a row,
/// logging as matplotlib's does; each font mathtext opened while the
/// deprecated `text.kerning_factor` is set to `report_font_open(name,
/// path, parsing)`, which opens it with the program's `get_font`, warning as
/// matplotlib's mathtext does (both registered by `_text_helpers`).
pub(super) fn report_font_notes(py: Python<'_>) -> PyResult<()> {
    for note in pltrs_text::take_font_notes() {
        match note {
            FontNote::Lookup(run) => report_lookup(py, &run)?,
            FontNote::Opened { source, parsing } => report_open(py, &source, parsing)?,
        }
    }
    Ok(())
}

/// Hands the lookup `run` to `report_font_lookup`.
fn report_lookup(py: Python<'_>, run: &LookupRun) -> PyResult<()> {
    let font = &run.lookup;
    let query = &font.query;
    let kind = match font.kind {
        LookupKind::Fonts => "fonts",
        LookupKind::Font => "font",
        LookupKind::Afm(AfmDirectory::PostScript) => "afm",
        LookupKind::Afm(AfmDirectory::PdfCore) => "core fonts",
    };
    let weight = match font.weight_keyword {
        Some(keyword) => keyword.into_py(py)?,
        None => i64::from(query.weight.0).into_py(py)?,
    };
    lookup(py, "report_font_lookup")?.call(
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
    Ok(())
}

/// Hands the font `source` mathtext opened (`parsing` a formula, or
/// before) to `report_font_open(name, path, parsing)`: the file name of a
/// font pltrs bundles, or the path of one on disk.
fn report_open(py: Python<'_>, source: &FontSource, parsing: bool) -> PyResult<()> {
    let (name, path) = match source {
        FontSource::Bundled(name) => (Some(*name), None),
        FontSource::File(path) => (None, Some(path.to_string_lossy().into_owned())),
    };
    lookup(py, "report_font_open")?.call(
        &[name.into_py(py)?, path.into_py(py)?, parsing.into_py(py)?],
        &[],
    )?;
    Ok(())
}

/// Hands each message mathtext logs about a formula pltrs (Rust) typeset
/// on this thread (a list packed to a size its glue cannot reach,
/// "Underful Hlist: ...", a symbol no font has) to the package's
/// `log_mathtext_warning(message)` (registered by `_mathtext`), which logs
/// it as matplotlib's mathtext does.
pub(super) fn log_mathtext_messages(py: Python<'_>) -> PyResult<()> {
    let messages = pltrs_text::take_mathtext_log();
    if messages.is_empty() {
        return Ok(());
    }
    let log = lookup(py, "log_mathtext_warning")?;
    for message in messages {
        log.call(&[message.into_py(py)?], &[])?;
    }
    Ok(())
}
