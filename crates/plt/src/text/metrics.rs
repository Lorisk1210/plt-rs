//! How text is measured: matplotlib asks the renderer for the extent of
//! every line. Its Agg backend lays text out with the hinting
//! `text.hinting` selects, the vector backends never hint, and the SVG
//! backend measures text with `TextToPath`, laid out at 100 points and 72
//! dpi and scaled to the text size, which rounds differently from laying it
//! out at that size. The PDF and PostScript backends measure text that is
//! not mathtext with Adobe Font Metrics when they draw it in standard
//! PostScript fonts (`pdf.use14corefonts`, `ps.useafm`), line spacing
//! included.

use std::cell::Cell;
use std::thread::LocalKey;

use plt_text::{
    AfmDirectory, FontHeightMetrics, FontRequest, Hinting, TextEngine, TextError, TextRun,
    TextSource,
};

use crate::rc::RcParams;

/// The size text paths are measured at (`TextToPath.FONT_SCALE`).
const FONT_SCALE: f64 = 100.0;

/// The resolution text paths are measured at (`TextToPath.DPI`).
const DPI: f64 = 72.0;

thread_local! {
    /// Whether text is measured as matplotlib's SVG backend measures it.
    static TEXT_PATH_METRICS: Cell<bool> = const { Cell::new(false) };
    /// Whether text is laid out unhinted (for a vector backend or as a
    /// text path), whatever `text.hinting` says.
    static UNHINTED: Cell<bool> = const { Cell::new(false) };
    /// The AFM fonts text that is not mathtext is measured with, if any.
    static AFM_METRICS: Cell<Option<AfmDirectory>> = const { Cell::new(None) };
}

/// Restores a mode's previous value when dropped (also on panics).
struct Restore<T: Copy + 'static> {
    mode: &'static LocalKey<Cell<T>>,
    previous: T,
}

impl<T: Copy + 'static> Drop for Restore<T> {
    fn drop(&mut self) {
        self.mode.set(self.previous);
    }
}

/// Runs `f` with `mode` set to `value`.
fn with_mode<T: Copy + 'static, R>(
    mode: &'static LocalKey<Cell<T>>,
    value: T,
    f: impl FnOnce() -> R,
) -> R {
    let _restore = Restore {
        mode,
        previous: mode.replace(value),
    };
    f()
}

/// Runs `f` with text measured as matplotlib's SVG backend measures it
/// (`TextToPath.get_text_width_height_descent`).
pub(crate) fn with_text_path_metrics<R>(f: impl FnOnce() -> R) -> R {
    with_mode(&TEXT_PATH_METRICS, true, f)
}

/// Runs `f` with text laid out unhinted, whatever `text.hinting` says, as
/// matplotlib's vector backends and `TextToPath` load glyphs.
pub(crate) fn without_hinting<R>(f: impl FnOnce() -> R) -> R {
    with_mode(&UNHINTED, true, f)
}

/// Runs `f` with text that is not mathtext measured with the AFM fonts of
/// `directory`, line spacing included, as matplotlib's PDF and PostScript
/// backends measure the text they draw in standard PostScript fonts
/// (`None` measures the fonts text is laid out with).
pub(crate) fn with_afm_metrics<R>(directory: Option<AfmDirectory>, f: impl FnOnce() -> R) -> R {
    with_mode(&AFM_METRICS, directory, f)
}

/// The hinting text is laid out and rasterized with: `text.hinting` as
/// matplotlib's Agg backend maps it (`get_hinting_flag`), or none inside
/// [`without_hinting`].
pub(crate) fn hinting(rc: &RcParams) -> Hinting {
    if UNHINTED.get() {
        Hinting::None
    } else {
        rc.text_hinting.hinting()
    }
}

/// `run` (laid out at `dpi`) with the width, height and descent the
/// current measuring mode reports.
pub(super) fn measured(mut run: TextRun, dpi: f64) -> Result<TextRun, TextError> {
    if let Some(directory) = AFM_METRICS.get()
        && let Some(TextSource::Plain { text, font, .. }) = &run.source
    {
        let line = font.afm_line_metrics(text, directory, dpi)?;
        (run.width, run.height, run.descent) = (line.width, line.height, line.descent);
        return Ok(run);
    }
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

/// The minimum line ascent, descent and line gap of text in `font` at
/// `dpi` in the current measuring mode (the font height metrics of
/// `Text._get_layout`, which the PDF and PostScript backends give for
/// their AFM fonts).
pub(super) fn font_height_metrics(
    font: &FontRequest,
    dpi: f64,
) -> Result<FontHeightMetrics, TextError> {
    match AFM_METRICS.get() {
        Some(directory) => font.afm_height_metrics(directory, dpi),
        None => TextEngine::global().font_height_metrics(font, dpi),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TextHinting;
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

    #[test]
    fn hinting_follows_the_rc_unless_switched_off() {
        let mut rc = RcParams::matplotlib();
        assert_eq!(hinting(&rc), Hinting::Default, "matplotlib's default");
        for (value, expected) in [
            ("either", Hinting::Default),
            ("native", Hinting::Native),
            ("no_autohint", Hinting::Native),
            ("auto", Hinting::Auto),
            ("force_autohint", Hinting::Auto),
            ("none", Hinting::None),
            ("no_hinting", Hinting::None),
        ] {
            rc.set("text.hinting", value).expect("valid");
            assert_eq!(hinting(&rc), expected, "{value}");
        }
        rc.text_hinting = TextHinting::Native;
        assert_eq!(without_hinting(|| hinting(&rc)), Hinting::None);
        assert_eq!(hinting(&rc), Hinting::Native, "the mode is restored");
    }

    #[test]
    fn afm_metrics_replace_plain_extents_and_line_heights() {
        let font = FontRequest::new(FontQuery {
            size: 12.0,
            ..FontQuery::families(["DejaVu Sans"])
        });
        let run = TextEngine::global()
            .layout_line("AVATAR", &font, 72.0)
            .expect("bundled font");
        let core = Some(AfmDirectory::PdfCore);
        let afm = with_afm_metrics(core, || measured(run.clone(), 72.0)).expect("bundled");
        // matplotlib's PDF backend with pdf.use14corefonts measures Helvetica.
        assert_eq!((afm.width, afm.height, afm.descent), (43.332, 8.616, 0.0));
        assert_eq!(afm.glyphs, run.glyphs, "only the extents change");
        let heights = with_afm_metrics(core, || font_height_metrics(&font, 72.0)).expect("bundled");
        assert_eq!((heights.ascent, heights.descent), (8.616, 2.484));
        assert_ne!(
            font_height_metrics(&font, 72.0).expect("bundled"),
            heights,
            "the mode is restored"
        );
        let math = TextEngine::global()
            .layout_math("$x$", &plt_text::mathtext::MathStyle::default())
            .expect("valid math");
        let unchanged = with_afm_metrics(core, || measured(math.clone(), 72.0)).expect("math");
        assert_eq!(unchanged.width, math.width, "mathtext keeps its metrics");
    }
}
