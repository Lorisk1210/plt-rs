//! The fonts the probes of an area see. The reference modules of some
//! areas install only matplotlib's bundled fonts for all their probes
//! (`spec_support.fonts.bundled_fonts_only`), so that their goldens do not
//! depend on the fonts of the host that recorded them; pltrs evaluates
//! those areas with only its bundled fonts too, or a host with other fonts
//! (macOS' Times, Comic Sans MS, ...) would find them where matplotlib did
//! not.

use pltrs_font::{FontEntry, FontSource};
use pltrs_text::TextEngine;

/// The areas whose reference module sees only the bundled fonts.
const BUNDLED_FONT_AREAS: [&str; 6] = [
    "afm",
    "mathtext",
    "mpl_font_manager",
    "mpl_textpath",
    "text_metrics",
    "text_shaping",
];

/// An engine's font catalogue holding only the bundled fonts while this
/// value lives, then what it held before (matplotlib's
/// `bundled_fonts_only`, which saves and restores `fontManager.ttflist`).
#[derive(Debug)]
pub(crate) struct BundledFontsOnly<'a> {
    /// The engine whose catalogue is restricted.
    engine: &'a TextEngine,
    /// The catalogue's entries before, in order.
    saved: Vec<FontEntry>,
}

impl BundledFontsOnly<'static> {
    /// Leaves only the bundled fonts in the shared catalogue, which text
    /// is drawn with, when `area` is one whose reference module does so;
    /// `None` for the other areas.
    pub(crate) fn for_area(area: &str) -> Option<Self> {
        BUNDLED_FONT_AREAS
            .contains(&area)
            .then(|| Self::install(TextEngine::global()))
    }
}

impl<'a> BundledFontsOnly<'a> {
    /// Removes every entry of `engine` for a font that is not bundled.
    fn install(engine: &'a TextEngine) -> Self {
        let saved = engine.database().entries().clone();
        let installed = saved.iter().filter(|entry| {
            !matches!(
                engine.database().source(entry.font),
                Some(FontSource::Bundled(_))
            )
        });
        for entry in installed {
            engine.remove_font_entry(entry);
        }
        Self { engine, saved }
    }
}

impl Drop for BundledFontsOnly<'_> {
    /// Puts back the entries saved, in their order, dropping those the
    /// probes added meanwhile.
    fn drop(&mut self) {
        let current = self.engine.database().entries().clone();
        for entry in &current {
            self.engine.remove_font_entry(entry);
        }
        for entry in self.saved.drain(..) {
            self.engine.add_font_entry(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use pltrs_font::{FontDatabase, bundled_font_data};

    use super::*;

    fn is_bundled(engine: &TextEngine, entry: &FontEntry) -> bool {
        matches!(
            engine.database().source(entry.font),
            Some(FontSource::Bundled(_))
        )
    }

    #[test]
    fn only_the_reference_modules_areas_are_restricted() {
        assert!(BundledFontsOnly::for_area("colors").is_none());
        assert!(BundledFontsOnly::for_area("text").is_none());
    }

    #[test]
    fn installed_fonts_are_hidden_until_dropped() {
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let path = std::env::temp_dir().join(format!(
            "pltrs-spec-installed-font-{}.ttf",
            std::process::id()
        ));
        std::fs::write(&path, bundled_font_data("DejaVuSans.ttf").unwrap()).unwrap();
        engine.add_font_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let before = engine.database().entries().clone();
        assert!(before.iter().any(|entry| !is_bundled(&engine, entry)));
        {
            let _only = BundledFontsOnly::install(&engine);
            let entries = engine.database().entries().clone();
            assert!(entries.iter().all(|entry| is_bundled(&engine, entry)));
            // An entry the probes add is gone afterwards.
            engine.add_font_entry(entries[0].clone());
        }
        assert_eq!(*engine.database().entries(), before);
    }
}
