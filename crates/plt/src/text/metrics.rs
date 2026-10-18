//! How text is measured: matplotlib asks the renderer for the extent of
//! every line, and its SVG backend measures text with `TextToPath`, laid
//! out at 100 points and 72 dpi and scaled to the text size, which rounds
//! differently from laying it out at that size.

use std::cell::Cell;

use plt_text::{TextEngine, TextError, TextRun};

/// The size text paths are measured at (`TextToPath.FONT_SCALE`).
const FONT_SCALE: f64 = 100.0;

/// The resolution text paths are measured at (`TextToPath.DPI`).
const DPI: f64 = 72.0;

thread_local! {
    /// Whether text is measured as matplotlib's SVG backend measures it.
    static TEXT_PATH_METRICS: Cell<bool> = const { Cell::new(false) };
}

/// Restores the previous measuring mode when dropped (also on panics).
struct Restore(bool);

impl Drop for Restore {
    fn drop(&mut self) {
        TEXT_PATH_METRICS.set(self.0);
    }
}

/// Runs `f` with text measured as matplotlib's SVG backend measures it
/// (`TextToPath.get_text_width_height_descent`).
pub(crate) fn with_text_path_metrics<R>(f: impl FnOnce() -> R) -> R {
    let _restore = Restore(TEXT_PATH_METRICS.replace(true));
    f()
}

/// `run` (laid out at `dpi`) with the width, height and descent the
/// current measuring mode reports.
pub(super) fn measured(mut run: TextRun, dpi: f64) -> Result<TextRun, TextError> {
    if !TEXT_PATH_METRICS.get() {
        return Ok(run);
    }
    let Some(source) = &run.source else {
        return Ok(run);
    };
    let scale = source.size() / FONT_SCALE * dpi / DPI;
    let path = source.layout(TextEngine::global(), FONT_SCALE, DPI)?;
    run.width = path.width * scale;
    run.height = path.height * scale;
    run.descent = path.descent * scale;
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_text::{FontQuery, FontRequest};

    #[test]
    fn text_path_metrics_scale_the_100_point_layout() {
        let font = FontRequest::new(FontQuery {
            size: 10.0,
            ..FontQuery::families(["DejaVu Sans"])
        });
        let run = TextEngine::global()
            .layout_line("AVA To", &font, 72.0)
            .expect("bundled font");
        let native = measured(run.clone(), 72.0).expect("bundled font");
        assert_eq!(native.width, run.width);
        let svg = with_text_path_metrics(|| measured(run.clone(), 72.0)).expect("bundled font");
        let at_100 = TextEngine::global()
            .layout_line(
                "AVA To",
                &FontRequest::new(FontQuery {
                    size: 100.0,
                    ..FontQuery::families(["DejaVu Sans"])
                }),
                72.0,
            )
            .expect("bundled font");
        assert_eq!(svg.width, at_100.width / 10.0);
        assert!(!TEXT_PATH_METRICS.get(), "the mode is restored");
    }
}
