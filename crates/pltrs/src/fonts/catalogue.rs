//! The catalogue of the fonts installed on the system, kept in the cache
//! directory between processes, as matplotlib keeps its font list there
//! (`fontlist-v390.json`): a process starting up reads it instead of
//! every font file, and only the fonts added or changed since are read.
//!
//! [`keep_catalogue`] makes the shared text engine keep it; pltrs calls
//! it where a program starts using it (when a thread first needs its
//! [rc parameters](crate::rc::params), and in [`FontManager::global`]),
//! and pltrs (Python) when its extension loads. The catalogue is read
//! when the engine is first used, which is when matplotlib's `findfont`
//! would first be called. When it cannot be written, the fonts are used
//! all the same and the font manager logs matplotlib's warning.
//!
//! [`FontManager::global`]: super::FontManager::global

use std::io;
use std::path::Path;
use std::sync::OnceLock;

use pltrs_text::{CatalogueReport, FontDatabase, TextEngine, catalogue_file_name};

/// What became of the catalogue, once the engine added the system's fonts
/// with it.
static REPORT: OnceLock<CatalogueReport> = OnceLock::new();

/// Makes the shared text engine keep the catalogue of the system's fonts
/// in the cache directory ([`config::cache_dir`](crate::config::cache_dir)),
/// if the engine is not made yet (see the module documentation).
pub fn keep_catalogue() {
    TextEngine::add_system_fonts_with(add_system_fonts);
}

/// What became of the catalogue when the shared text engine added the
/// system's fonts with it: whether it was used, brought up to date or
/// rebuilt, and why it could not be written. `None` before the engine was
/// made, and when it read every font (without a cache directory, or
/// before [`keep_catalogue`]).
pub fn catalogue_report() -> Option<&'static CatalogueReport> {
    REPORT.get()
}

/// Adds the system's fonts to `db` with the catalogue in the cache
/// directory, logging why it could not be written as matplotlib's
/// `json_dump` does.
fn add_system_fonts(db: &FontDatabase) {
    let Ok(dir) = crate::config::cache_dir() else {
        db.add_system_fonts();
        return;
    };
    let report = db.add_system_fonts_catalogued(&dir.join(catalogue_file_name()));
    if let Some(error) = &report.save_error {
        crate::warnings::log(
            "font_manager",
            &format!(
                "Could not save font_manager cache {}",
                os_error(error, &report.file)
            ),
        );
    }
    let _ = REPORT.set(report);
}

/// `error` about `path` as Python words the `OSError` matplotlib reports:
/// `[Errno 13] Permission denied: '/path'`.
fn os_error(error: &io::Error, path: &Path) -> String {
    let text = error.to_string();
    match error.raw_os_error() {
        Some(code) => {
            let reason = text
                .strip_suffix(&format!(" (os error {code})"))
                .unwrap_or(&text);
            format!("[Errno {code}] {reason}: '{}'", path.display())
        }
        None => format!("{text}: '{}'", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_read_as_python_words_them() {
        let path = Path::new("/cache/fontlist-v1.pltrs");
        let denied = io::Error::from_raw_os_error(13);
        let text = os_error(&denied, path);
        assert!(text.starts_with("[Errno 13] "), "{text}");
        assert!(text.ends_with(": '/cache/fontlist-v1.pltrs'"), "{text}");
        assert!(!text.contains("os error"), "{text}");
        let other = io::Error::new(io::ErrorKind::InvalidData, "bad");
        assert_eq!(os_error(&other, path), "bad: '/cache/fontlist-v1.pltrs'");
    }
}
