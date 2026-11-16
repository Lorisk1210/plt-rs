//! Handles of patches, bars, step patches and polygon collections
//! (matplotlib's `HandlerPatch`, `HandlerStepPatch` and
//! `HandlerPolyCollection`).

use std::rc::Rc;

use pltrs_core::Rgba;

use super::line::first_width;
use super::{HandleBox, HandlerArtist, HandlerContext, copy_base};
use crate::artist::{Artist, ArtistBase, ArtistStore};
use crate::collections::CollectionArtist;
use crate::colors::Color;
use crate::container::ContainerIds;
use crate::legend::LegendError;
use crate::legend::handle::{Proxy, Source};
use crate::lines::LineStyle;
use crate::markers::Marker;
use crate::patches::{PatchArtist, PatchShape, PatchView, Rectangle};
use crate::rc::{ColorOrEdge, RcParams};

/// The patch a handle stands for, with its artist properties: the patch
/// itself, the first bar of a bar container or a proxy patch.
pub(super) fn original_patch<'a>(
    store: &'a ArtistStore,
    source: &'a Source,
) -> Option<(&'a ArtistBase, PatchView<'a>)> {
    let artist = match source {
        Source::Artist(id) => store.get(*id)?,
        Source::Container(ContainerIds::Bar { patches, .. }) => store.get(*patches.first()?)?,
        Source::Proxy(Proxy::Patch(proxy)) => return Some((proxy.base(), proxy.as_patch()?)),
        _ => return None,
    };
    Some((artist.base(), artist.as_patch()?))
}

/// Gives `dst` the look of the patch `source` stands for, if any
/// (matplotlib's `HandlerPatch.update_prop`).
pub(super) fn copy_original<S: PatchShape>(
    dst: &mut PatchArtist<S>,
    ctx: &HandlerContext<'_>,
    source: &Source,
) {
    if let Some((base, view)) = original_patch(ctx.store, source) {
        copy_patch(dst, base, view, ctx.rc);
    }
}

/// A rectangle filling the handle box.
fn rectangle(ctx: &HandlerContext<'_>, area: &HandleBox) -> PatchArtist<Rectangle> {
    PatchArtist::new(
        Rectangle::new((-area.xdescent, -area.ydescent), area.width, area.height),
        Rc::clone(ctx.rc),
    )
}

/// A rectangle filling the handle box in the style of `source`; for bar
/// containers that of their first bar (matplotlib's `HandlerPatch`).
pub(super) fn patch(
    ctx: &HandlerContext<'_>,
    source: Option<(&ArtistBase, PatchView<'_>)>,
    area: &HandleBox,
) -> Vec<HandlerArtist> {
    let mut rect = rectangle(ctx, area);
    if let Some((base, view)) = source {
        copy_patch(&mut rect, base, view, ctx.rc);
    }
    vec![HandlerArtist::placed(rect)]
}

/// Copies the look of a patch (matplotlib's `Patch.update_from`), with
/// default colours resolved as the source's class resolves them.
fn copy_patch<S: PatchShape>(
    dst: &mut PatchArtist<S>,
    base: &ArtistBase,
    view: PatchView<'_>,
    rc: &RcParams,
) {
    copy_base(dst.base_mut(), base);
    let source = view.style;
    let style = &mut dst.style;
    style.facecolor = Some(
        source
            .facecolor
            .clone()
            .unwrap_or_else(|| rc.patch_facecolor.clone()),
    );
    style.edgecolor = Some(source.edgecolor.clone().unwrap_or_else(|| {
        if rc.patch_force_edgecolor || !source.fill || view.edge_default {
            rc.patch_edgecolor.clone()
        } else {
            Color::none()
        }
    }));
    style.fill = source.fill;
    style.hatch.clone_from(&source.hatch);
    style.hatchcolor = source.hatchcolor.clone();
    style.linestyle = source.linestyle.clone();
    style.edgegapcolor = source.edgegapcolor.clone();
    style.linewidth = source.linewidth;
}

/// A filled or hatched step patch as a rectangle like [`patch`]; an
/// outline-only one as a line through the middle of the box in its edge
/// colour, width and style (matplotlib's `HandlerStepPatch`).
pub(super) fn step_patch(
    ctx: &HandlerContext<'_>,
    base: &ArtistBase,
    view: PatchView<'_>,
    area: &HandleBox,
) -> Result<Vec<HandlerArtist>, LegendError> {
    let style = view.style;
    if style.fill || style.hatch.is_some() {
        return Ok(patch(ctx, Some((base, view)), area));
    }
    let resolved = style.resolve(ctx.rc, base.alpha, view.edge_default)?;
    let middle = area.height / 2.0;
    let mut line = ctx.line(vec![0.0, area.width], vec![middle, middle]);
    line.color = Color::from(resolved.edge);
    line.linestyle = style.linestyle.clone();
    line.linewidth = resolved.linewidth;
    line.marker = Marker::default();
    Ok(vec![HandlerArtist::placed(line)])
}

/// A rectangle filling the handle box in the first face, edge and hatch
/// colour, width and line style of a polygon collection (matplotlib's
/// `HandlerPolyCollection`).
pub(super) fn poly_collection(
    ctx: &HandlerContext<'_>,
    source: &CollectionArtist,
    area: &HandleBox,
) -> Result<Vec<HandlerArtist>, LegendError> {
    // The colours the collection reports now (matplotlib's handler reads
    // `get_facecolor`): those of the last draw's mapping, the ones set
    // before its first draw.
    let (faces, edges) = source
        .reported_colors()
        .map_err(LegendError::HandleColors)?;
    let first = |colors: &[Rgba]| colors.first().copied().unwrap_or(Rgba::TRANSPARENT);
    // The collection's own first hatch colour (its edge's unless given).
    let hatch_color = first(&source.hatch_colors(&edges)?);
    let mut rect = rectangle(ctx, area);
    let style = &mut rect.style;
    style.facecolor = Some(first(&faces).into());
    style.edgecolor = Some(first(&edges).into());
    style.hatchcolor = ColorOrEdge::Color(hatch_color.into());
    style.fill =
        !matches!(&source.facecolors, Some(colors) if colors.len() == 1 && colors[0].is_none());
    style.hatch.clone_from(&source.hatch);
    style.linewidth = Some(first_width(source));
    // The first of the collection's dash patterns as drawn, which the
    // rectangle scales by its own width again, as matplotlib's does.
    style.linestyle = match source.drawn_widths_and_dashes().1.first() {
        Some(dashes) if !dashes.lengths().is_empty() => {
            LineStyle::dashes(dashes.offset, dashes.lengths().to_vec())
        }
        _ => LineStyle::Solid,
    };
    Ok(vec![HandlerArtist::placed(rect)])
}
