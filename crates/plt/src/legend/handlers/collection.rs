//! Handles of scatter plots (matplotlib's `HandlerPathCollection`).

use std::rc::Rc;

use plt_core::Point;

use super::{Attach, HandleBox, HandlerArtist, HandlerContext, copy_collection};
use crate::artist::Artist;
use crate::collections::{CollectionArtist, CollectionKind};
use crate::legend::LegendError;
use crate::numeric::linspace;
use crate::transforms::Transform;

/// `scatterpoints` markers of the first shape of `source`, at the
/// legend's scatter heights, sized between the smallest and largest of
/// its markers (matplotlib's `HandlerPathCollection`).
pub(super) fn path_collection(
    ctx: &HandlerContext<'_>,
    source: &CollectionArtist,
    area: &HandleBox,
) -> Result<Vec<HandlerArtist>, LegendError> {
    let Some(shape) = source.paths.first() else {
        return Ok(Vec::new());
    };
    let settings = ctx.settings;
    let (_, xdata) = ctx.xdata(area, settings.scatterpoints);
    let offsets = xdata
        .iter()
        .zip(&settings.scatteryoffsets)
        .map(|(&x, offset)| Point::new(x, area.height * offset))
        .collect();
    let mut markers =
        CollectionArtist::new(CollectionKind::Path, vec![shape.clone()], Rc::clone(ctx.rc));
    markers.sizes = Some(sizes(source, settings.scatterpoints, settings.markerscale));
    markers.offsets = offsets;
    markers.offset_transform = Some(Transform::Display);
    copy_collection(&mut markers, source)?;
    markers
        .base_mut()
        .transform
        .clone_from(&source.base().transform);
    Ok(vec![HandlerArtist {
        artist: Box::new(markers),
        attach: Attach::Offsets,
        mapped: source.mapping.is_some(),
    }])
}

/// matplotlib's `HandlerRegularPolyCollection.get_sizes`: the mean, the
/// largest and the smallest marker area for up to three points, evenly
/// spaced areas from the smallest to the largest for more, all scaled by
/// `markerscale²`.
fn sizes(source: &CollectionArtist, numpoints: usize, markerscale: f64) -> Vec<f64> {
    let areas = match source.sizes.as_deref() {
        Some(areas) if !areas.is_empty() => areas,
        _ => &[1.0],
    };
    let scale = markerscale.powi(2);
    let largest = areas.iter().copied().fold(f64::NEG_INFINITY, f64::max) * scale;
    let smallest = areas.iter().copied().fold(f64::INFINITY, f64::min) * scale;
    if numpoints < 4 {
        let mut sizes = vec![f64::midpoint(largest, smallest), largest, smallest];
        sizes.truncate(numpoints);
        sizes
    } else {
        linspace(0.0, 1.0, numpoints)
            .into_iter()
            .map(|t| (largest - smallest) * t + smallest)
            .collect()
    }
}
