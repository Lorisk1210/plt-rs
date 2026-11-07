//! The fonts the spec sees. Its goldens must not depend on the fonts of
//! the host that recorded them: macOS has Helvetica, Times, Comic Sans MS
//! or Apple Chancery where a Linux host falls back to `DejaVu Sans`. So the
//! reference installs only matplotlib's bundled fonts wherever a golden
//! depends on font lookup (`spec_support.fonts.bundled_fonts_only`), and
//! pltrs sees only its bundled fonts there too: for the probes of some
//! areas (`probes::fonts`) and for every figure case.

use std::sync::Once;

use pltrs_font::FontSource;
use pltrs_text::TextEngine;

/// Removes every entry of `engine`'s catalogue for a font that is not
/// bundled.
pub(crate) fn remove_installed_fonts(engine: &TextEngine) {
    let entries = engine.database().entries().clone();
    let installed = entries.iter().filter(|entry| {
        !matches!(
            engine.database().source(entry.font),
            Some(FontSource::Bundled(_))
        )
    });
    for entry in installed {
        engine.remove_font_entry(entry);
    }
}

/// Leaves only the bundled fonts in the shared catalogue, which text is
/// drawn with, for the rest of the process: the figure cases' fonts, as
/// the reference's `FigureCase.render` installs only matplotlib's. The
/// first call does it; fonts added afterwards (a case's own font files)
/// stay.
pub(crate) fn bundled_fonts_for_figures() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| remove_installed_fonts(TextEngine::global()));
}

#[cfg(test)]
mod tests {
    use pltrs_font::{FontDatabase, bundled_font_data};

    use super::*;

    #[test]
    fn only_bundled_fonts_remain() {
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let bundled = engine.database().entries().len();
        let path = std::env::temp_dir().join(format!(
            "pltrs-spec-removed-font-{}.ttf",
            std::process::id()
        ));
        std::fs::write(&path, bundled_font_data("DejaVuSans.ttf").unwrap()).unwrap();
        engine.add_font_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(engine.database().entries().len() > bundled);
        remove_installed_fonts(&engine);
        assert_eq!(engine.database().entries().len(), bundled);
    }
}
