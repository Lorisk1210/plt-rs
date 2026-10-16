//! Measuring and laying out the lines of a text block, with mathtext for
//! the lines matplotlib treats as math.

use plt_text::{
    BlockLayout, BlockStyle, LineMetrics, TextEngine, TextError, TextRun, layout_block,
};

use super::math::{Line, math_style};
use crate::fonts::FontProperties;
use crate::rc::RcParams;

/// A text block laid out for drawing: per-line runs plus their placement.
#[derive(Clone, Debug, PartialEq)]
pub struct LaidOutText {
    /// The lines of the block as given (before any math processing).
    pub lines: Vec<String>,
    /// Placement of every line relative to the anchor (pixels).
    pub block: BlockLayout,
    /// The glyph run of every line, in the same order; its origin is the
    /// left end of the line's baseline.
    pub runs: Vec<TextRun>,
}

/// matplotlib's `get_text_width_height_descent` for one line: with `math`
/// the string is typeset as mathtext (extents rounded up to whole pixels,
/// malformed math is an error), otherwise it is shaped as is.
pub fn measure_text(
    text: &str,
    font: &FontProperties,
    math: bool,
    dpi: f64,
    rc: &RcParams,
) -> Result<LineMetrics, TextError> {
    let line = if math {
        Line::Math(text)
    } else {
        Line::Plain(text.into())
    };
    line_run(&line, font, dpi, rc).map(|run| metrics(&run))
}

/// The width, height and descent of a run.
fn metrics(run: &TextRun) -> LineMetrics {
    LineMetrics {
        width: run.width,
        height: run.height,
        descent: run.descent,
    }
}

/// Lays out one (non-empty) line.
fn line_run(
    line: &Line<'_>,
    font: &FontProperties,
    dpi: f64,
    rc: &RcParams,
) -> Result<TextRun, TextError> {
    let engine = TextEngine::global();
    match line {
        Line::Plain(text) => engine.layout_line(text, &font.request(rc), dpi),
        Line::Math(source) => engine.layout_math(source, &math_style(font, dpi, rc)),
    }
}

/// Lays out `text` (lines separated by `\n`) at `dpi` with matplotlib's
/// line metrics, stacking and alignment rules; lines with math (see
/// [`Line::classify`]) are typeset as mathtext when `rc` enables
/// `text.parse_math`.
pub(crate) fn lay_out(
    text: &str,
    font: &FontProperties,
    style: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
) -> Result<LaidOutText, TextError> {
    lay_out_with_math(text, font, style, dpi, rc, rc.text_parse_math)
}

/// [`lay_out`] with math parsing switched on or off explicitly (a text's
/// `parse_math` property).
pub(crate) fn lay_out_with_math(
    text: &str,
    font: &FontProperties,
    style: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
    parse_math: bool,
) -> Result<LaidOutText, TextError> {
    let lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let runs = lines
        .iter()
        .map(|line| {
            if line.is_empty() {
                Ok(TextRun {
                    dpi,
                    ..TextRun::default()
                })
            } else {
                line_run(&Line::classify(line, parse_math), font, dpi, rc)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let line_metrics: Vec<LineMetrics> = runs.iter().map(metrics).collect();
    let font_metrics = TextEngine::global().font_height_metrics(&font.request(rc), dpi)?;
    Ok(LaidOutText {
        lines,
        block: layout_block(&line_metrics, font_metrics, style),
        runs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> RcParams {
        let mut rc = RcParams::matplotlib();
        rc.font_family = vec!["DejaVu Sans".to_owned()];
        rc
    }

    #[test]
    fn math_lines_round_up_to_whole_pixels() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        // matplotlib: get_text_width_height_descent('$x^2$', ismath=True)
        // at 100 dpi is (16, 13, 0) with dejavusans.
        let math = measure_text("$x^2$", &font, true, 100.0, &rc).expect("valid math");
        assert_eq!((math.width, math.height, math.descent), (16.0, 13.0, 0.0));
        assert!(measure_text("$x^$", &font, true, 100.0, &rc).is_err());
        let plain = measure_text("$x^$", &font, false, 100.0, &rc).expect("plain text");
        assert!(plain.width.fract() != 0.0);
    }

    #[test]
    fn lines_are_classified_independently() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        let laid_out = lay_out(
            "$x^2$\nprice \\$5\n",
            &font,
            &BlockStyle::default(),
            100.0,
            &rc,
        )
        .expect("valid text");
        assert_eq!(laid_out.lines, ["$x^2$", "price \\$5", ""]);
        assert_eq!(laid_out.runs[0].width, 16.0);
        let dollars = measure_text("price $5", &font, false, 100.0, &rc).expect("plain");
        assert_eq!(laid_out.runs[1].width, dollars.width);
        assert!(laid_out.runs[2].is_empty());
    }
}
