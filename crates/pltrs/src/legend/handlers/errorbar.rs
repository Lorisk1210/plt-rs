//! Handles of error bar plots (matplotlib's `HandlerErrorbar`).

use std::rc::Rc;

use pltrs_core::{Path, Point};

use super::line::{middle, scale_markers};
use super::{HandleBox, HandlerArtist, HandlerContext, copy_collection, copy_line};
use crate::artist::Artist;
use crate::collections::{CollectionArtist, CollectionKind};
use crate::container::ErrorbarIds;
use crate::legend::LegendError;
use crate::lines::{DrawStyle, LineArtist, LineStyle};
use crate::markers::{Marker, MarkerSymbol};

/// Half the length of the legend's error bars, in font sizes.
const ERROR_SIZE: f64 = 0.5;

/// The data line and its markers with error bars and caps at the markers,
/// styled like the plot's own (matplotlib's `HandlerErrorbar`).
pub(super) fn errorbar(
    ctx: &HandlerContext<'_>,
    ids: &ErrorbarIds,
    area: &HandleBox,
) -> Result<Vec<HandlerArtist>, LegendError> {
    let store = ctx.store;
    let (xdata, xmarkers) = ctx.xdata(area, ctx.settings.numpoints);
    let y = middle(area);
    let ydata = vec![y; xdata.len()];
    let mut line = ctx.line(xdata, ydata);
    let ymarkers = vec![y; xmarkers.len()];
    let mut markers = ctx.line(xmarkers.clone(), ymarkers.clone());
    if let Some(data) = ids.data_line.and_then(|id| store.get_as::<LineArtist>(id)) {
        copy_line(&mut line, data);
        line.drawstyle = DrawStyle::Default;
        line.recache();
        line.set_marker_code(Marker::default());
        copy_line(&mut markers, data);
        markers.linestyle = LineStyle::None;
        scale_markers(ctx, &mut markers);
    } else {
        line.base_mut().visible = false;
        markers.base_mut().visible = false;
    }

    let size = ERROR_SIZE * ctx.settings.fontsize;
    let bars = ids
        .barlinecols
        .first()
        .and_then(|&id| store.get_as::<CollectionArtist>(id));
    let cap = ids
        .caplines
        .first()
        .and_then(|&id| store.get_as::<LineArtist>(id));
    let mut bar_artists = Vec::new();
    let mut cap_artists = Vec::new();
    let markers_at: Vec<Point> = xmarkers
        .iter()
        .zip(&ymarkers)
        .map(|(&x, &y)| Point::new(x, y))
        .collect();
    for (present, horizontal) in [(ids.has_xerr, true), (ids.has_yerr, false)] {
        if !present {
            continue;
        }
        let delta = if horizontal {
            Point::new(size, 0.0)
        } else {
            Point::new(0.0, size)
        };
        if let Some(bars) = bars {
            let segments = markers_at
                .iter()
                .map(|&p| Path::polyline(vec![p - delta, p + delta]))
                .collect();
            let mut collection =
                CollectionArtist::new(CollectionKind::Line, segments, Rc::clone(ctx.rc));
            copy_collection(&mut collection, bars)?;
            bar_artists.push(HandlerArtist::placed(collection));
        }
        if let Some(cap) = cap {
            let symbol = if horizontal {
                MarkerSymbol::VLine
            } else {
                MarkerSymbol::HLine
            };
            for sign in [-1.0, 1.0] {
                let ends: Vec<Point> = markers_at.iter().map(|&p| p + delta * sign).collect();
                let mut capline = ctx.line(
                    ends.iter().map(|p| p.x).collect(),
                    ends.iter().map(|p| p.y).collect(),
                );
                copy_line(&mut capline, cap);
                capline.set_marker_code(Marker::Symbol(symbol));
                cap_artists.push(HandlerArtist::placed(capline));
            }
        }
    }
    let mut artists = bar_artists;
    artists.extend(cap_artists);
    artists.push(HandlerArtist::placed(line));
    artists.push(HandlerArtist::placed(markers));
    Ok(artists)
}
