//! Measuring and laying out the lines of a text artist.

use plt_text::{
    BlockLayout, BlockStyle, LineMetrics, TextEngine, TextError, TextRun, layout_block,
};

use crate::fonts::FontProperties;
use crate::rc::RcParams;

/// A text block laid out for drawing: per-line runs plus their placement.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LaidOutText {
    /// Placement of every line relative to the anchor (pixels).
    pub(crate) block: BlockLayout,
    /// The glyph run of every line, in the same order.
    pub(crate) runs: Vec<TextRun>,
}

/// Lays out `text` (lines separated by `\n`) at `dpi` with matplotlib's
/// line metrics, stacking and alignment rules.
pub(crate) fn lay_out(
    text: &str,
    font: &FontProperties,
    style: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
) -> Result<LaidOutText, TextError> {
    let engine = TextEngine::global();
    let request = font.request(rc);
    let runs = text
        .split('\n')
        .map(|line| {
            if line.is_empty() {
                Ok(TextRun {
                    dpi,
                    ..TextRun::default()
                })
            } else {
                engine.layout_line(line, &request, dpi)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let metrics: Vec<LineMetrics> = runs
        .iter()
        .map(|run| LineMetrics {
            width: run.width,
            height: run.height,
            descent: run.descent,
        })
        .collect();
    let font_metrics = engine.font_height_metrics(&request, dpi)?;
    Ok(LaidOutText {
        block: layout_block(&metrics, font_metrics, style),
        runs,
    })
}
