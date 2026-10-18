//! How text is measured: matplotlib asks the renderer for the extent of
//! every line. Its Agg backend lays text out with the hinting
//! `text.hinting` selects, the vector backends never hint, and the SVG
//! backend measures text with `TextToPath`, laid out at 100 points and 72
//! dpi and scaled to the text size, which rounds differently from laying it
//! out at that size.

use std::cell::Cell;
use std::thread::LocalKey;

use plt_text::{Hinting, TextEngine, TextError, TextRun};

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
}

/// Restores a flag's previous value when dropped (also on panics).
struct Restore {
    flag: &'static LocalKey<Cell<bool>>,
    previous: bool,
}

impl Drop for Restore {
    fn drop(&mut self) {
        self.flag.set(self.previous);
    }
}

/// Runs `f` with `flag` set.
fn with_flag<R>(flag: &'static LocalKey<Cell<bool>>, f: impl FnOnce() -> R) -> R {
    let _restore = Restore {
        flag,
        previous: flag.replace(true),
    };
    f()
}

/// Runs `f` with text measured as matplotlib's SVG backend measures it
/// (`TextToPath.get_text_width_height_descent`).
pub(crate) fn with_text_path_metrics<R>(f: impl FnOnce() -> R) -> R {
    with_flag(&TEXT_PATH_METRICS, f)
}

/// Runs `f` with text laid out unhinted, whatever `text.hinting` says, as
/// matplotlib's vector backends and `TextToPath` load glyphs.
pub(crate) fn without_hinting<R>(f: impl FnOnce() -> R) -> R {
    with_flag(&UNHINTED, f)
}

/// The hinting text is laid out and rasterized with: `text.hinting` as
/// matplotlib's Agg backend maps it (`get_hinting_flag`), or none inside
/// [`without_hinting`].
pub(crate) fn hinting(rc: &RcParams) -> Hinting {
    if UNHINTED.get() {
        return Hinting::None;
    }
    match rc.text_hinting.as_str() {
        "default" | "either" => Hinting::Default,
        "no_autohint" | "native" => Hinting::Native,
        "force_autohint" | "auto" => Hinting::Auto,
        _ => Hinting::None,
    }
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
            rc.text_hinting = value.to_owned();
            assert_eq!(hinting(&rc), expected, "{value}");
        }
        rc.text_hinting = "native".to_owned();
        assert_eq!(without_hinting(|| hinting(&rc)), Hinting::None);
        assert_eq!(hinting(&rc), Hinting::Native, "the mode is restored");
    }
}
