//! Handles of lines and line collections (matplotlib's `HandlerLine2D`
//! and `HandlerLineCollection`).

use pltrs_render::Dashes;

use super::{HandleBox, HandlerArtist, HandlerContext, copy_line};
use crate::collections::{CollectionArtist, CollectionKind};
use crate::colors::Color;
use crate::legend::LegendError;
use crate::lines::{LineArtist, LineStyle, MarkEvery};
use crate::numeric::linspace;

/// The vertical middle of a handle box, where line handles run.
pub(super) fn middle(area: &HandleBox) -> f64 {
    (area.height - area.ydescent) / 2.0
}

/// A line across the handle box in the style of `source`, with
/// `numpoints` markers, or one marker in the middle (matplotlib's
/// `HandlerLine2D`).
pub(super) fn line2d(
    ctx: &HandlerContext<'_>,
    source: &LineArtist,
    area: &HandleBox,
) -> Vec<HandlerArtist> {
    let numpoints = ctx.settings.numpoints;
    let (mut xdata, _) = ctx.xdata(area, numpoints);
    let mut markevery = None;
    if numpoints == 1 {
        // A three-point line marked only in the middle.
        xdata = linspace(xdata[0], xdata[xdata.len() - 1], 3);
        markevery = Some(MarkEvery::Indices(vec![1]));
    }
    let ydata = vec![middle(area); xdata.len()];
    let mut line = ctx.line(xdata, ydata);
    line.markevery = markevery;
    copy_line(&mut line, source);
    scale_markers(ctx, &mut line);
    vec![HandlerArtist::placed(line)]
}

/// Applies the legend's `markerscale`.
pub(super) fn scale_markers(ctx: &HandlerContext<'_>, line: &mut LineArtist) {
    line.markersize *= ctx.settings.markerscale;
}

/// A line in the colour, width and dash pattern of the first line of
/// `source`, with `scatterpoints` points (matplotlib's
/// `HandlerLineCollection`). matplotlib hands the pattern over as an
/// `(offset, dashes)` tuple, which makes the handle a dashed line: it
/// takes the dash cap and join styles even when the pattern is solid.
pub(super) fn line_collection(
    ctx: &HandlerContext<'_>,
    source: &CollectionArtist,
    area: &HandleBox,
) -> Result<Vec<HandlerArtist>, LegendError> {
    let (xdata, _) = ctx.xdata(area, ctx.settings.scatterpoints);
    let ydata = vec![middle(area); xdata.len()];
    let mut line = ctx.line(xdata, ydata);
    let (_, edges) = source
        .resolved_colors()
        .map_err(LegendError::HandleColors)?;
    let Some(&color) = edges.first() else {
        return Ok(Vec::new());
    };
    line.color = Color::from(color);
    let style = source.linestyles.first().unwrap_or(&LineStyle::Solid);
    let Dashes { offset, pattern } = style.dash_pattern(source.rc());
    line.linestyle = LineStyle::Dashes { offset, pattern };
    line.linewidth = first_width(source);
    Ok(vec![HandlerArtist::placed(line)])
}

/// The first edge width of a collection, with its kind's default.
pub(super) fn first_width(source: &CollectionArtist) -> f64 {
    source
        .linewidths
        .as_ref()
        .and_then(|widths| widths.first().copied())
        .unwrap_or_else(|| {
            let rc = source.rc();
            if source.kind == CollectionKind::Line {
                rc.lines_linewidth
            } else {
                rc.patch_linewidth
            }
        })
}
