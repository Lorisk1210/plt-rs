//! Handles of patches, bars, step patches and polygon collections
//! (matplotlib's `HandlerPatch`, `HandlerStepPatch` and
//! `HandlerPolyCollection`).

use std::rc::Rc;

use plt_core::Rgba;

use super::line::first_width;
use super::{HandleArtist, HandleArtists, HandleBox, HandlerContext, copy_base};
use crate::artist::{Artist, ArtistBase};
use crate::collections::CollectionArtist;
use crate::colors::Color;
use crate::legend::LegendError;
use crate::lines::LineStyle;
use crate::markers::Marker;
use crate::patches::{PatchArtist, PatchView, Rectangle};
use crate::rc::{ColorOrEdge, RcParams};

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
) -> HandleArtists {
    let mut rect = rectangle(ctx, area);
    if let Some((base, view)) = source {
        copy_patch(&mut rect, base, view, ctx.rc);
    }
    HandleArtists::unmapped(vec![HandleArtist::placed(rect)])
}

/// Copies the look of a patch (matplotlib's `Patch.update_from`), with
/// default colours resolved as the source's class resolves them.
fn copy_patch(
    dst: &mut PatchArtist<Rectangle>,
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
) -> Result<HandleArtists, LegendError> {
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
    Ok(HandleArtists::unmapped(vec![HandleArtist::placed(line)]))
}

/// A rectangle filling the handle box in the first face, edge and hatch
/// colour, width and line style of a polygon collection (matplotlib's
/// `HandlerPolyCollection`).
pub(super) fn poly_collection(
    ctx: &HandlerContext<'_>,
    source: &CollectionArtist,
    area: &HandleBox,
) -> Result<HandleArtists, LegendError> {
    let (faces, edges) = source
        .resolved_colors()
        .map_err(LegendError::HandleColors)?;
    let first = |colors: &[Rgba]| colors.first().copied().unwrap_or(Rgba::TRANSPARENT);
    let hatch_color = match edges.first() {
        Some(&edge) => edge,
        None => source
            .rc()
            .patch_edgecolor
            .resolve_alpha(source.base().alpha)?,
    };
    let mut rect = rectangle(ctx, area);
    let style = &mut rect.style;
    style.facecolor = Some(first(&faces).into());
    style.edgecolor = Some(first(&edges).into());
    style.hatchcolor = ColorOrEdge::Color(hatch_color.into());
    style.fill =
        !matches!(&source.facecolors, Some(colors) if colors.len() == 1 && colors[0].is_none());
    style.hatch.clone_from(&source.hatch);
    style.linewidth = Some(first_width(source));
    style.linestyle = source
        .linestyles
        .first()
        .cloned()
        .unwrap_or(LineStyle::Solid);
    Ok(HandleArtists::unmapped(vec![HandleArtist::placed(rect)]))
}
