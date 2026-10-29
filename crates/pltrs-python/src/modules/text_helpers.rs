//! matplotlib's `_text_helpers` on the native side: the warning matplotlib
//! issues for each character it lays out that no font has, delivered at
//! the end of the native call during which pltrs (Rust) laid it out.

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
