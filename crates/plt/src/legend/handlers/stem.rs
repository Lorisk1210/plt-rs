//! Handles of stem plots (matplotlib's `HandlerStem`).

use super::line::first_width;
use super::{HandleArtist, HandleArtists, HandleBox, HandlerContext, copy_line};
use crate::artist::ArtistId;
use crate::collections::CollectionArtist;
use crate::colors::Color;
use crate::legend::LegendError;
use crate::lines::{LineArtist, LineStyle};

/// Stems rising from a baseline at the bottom of the handle box to heads
/// at the legend's scatter heights (matplotlib's `HandlerStem`).
pub(super) fn stem(
    ctx: &HandlerContext<'_>,
    [markerline, stemlines, baseline]: [ArtistId; 3],
    area: &HandleBox,
) -> Result<Option<HandleArtists>, LegendError> {
    let store = ctx.store;
    let (Some(heads), Some(stems), Some(base)) = (
        store.get_as::<LineArtist>(markerline),
        store.get_as::<CollectionArtist>(stemlines),
        store.get_as::<LineArtist>(baseline),
    ) else {
        return Ok(None);
    };
    let (xdata, xmarkers) = ctx.xdata(area, ctx.settings.numpoints);
    let ydata: Vec<f64> = ctx
        .settings
        .scatteryoffsets
        .iter()
        .map(|offset| area.height * (0.5 * offset + 0.5))
        .collect();
    let bottom = 0.0;

    let count = xmarkers.len().min(ydata.len());
    let mut head_line = ctx.line(xmarkers[..count].to_vec(), ydata[..count].to_vec());
    copy_line(&mut head_line, heads);

    let (_, edges) = stems.resolved_colors().map_err(LegendError::HandleColors)?;
    // matplotlib copies the collection's dash pattern, already scaled by
    // its width, as an explicit (and therefore "dashed") line style.
    let dashes = stems
        .linestyles
        .first()
        .unwrap_or(&LineStyle::Solid)
        .scaled_dashes(first_width(stems), stems.rc());
    let mut artists = Vec::with_capacity(count + 2);
    for (&x, &y) in xmarkers.iter().zip(&ydata) {
        let mut line = ctx.line(vec![x, x], vec![bottom, y]);
        if let Some(&color) = edges.first() {
            line.color = Color::from(color);
        }
        line.linestyle = LineStyle::dashes(dashes.offset, dashes.pattern.clone());
        artists.push(HandleArtist::placed(line));
    }

    let low = xdata.iter().copied().fold(f64::INFINITY, f64::min);
    let high = xdata.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut base_line = ctx.line(vec![low, high], vec![bottom, bottom]);
    copy_line(&mut base_line, base);
    artists.push(HandleArtist::placed(base_line));
    artists.push(HandleArtist::placed(head_line));
    Ok(Some(HandleArtists::unmapped(artists)))
}
