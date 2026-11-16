//! Placing a legend: the size of its box, where it goes and where each of
//! its parts lands (matplotlib's `Legend.draw`, `_findoffset` and
//! `_get_anchored_bbox`). The figure's layout pass runs this for every
//! legend once the axes are laid out.

use std::borrow::Cow;
use std::fmt;

use pltrs_core::{Bbox, Point};

use super::artist::{HandleRef, LegendArtist, LegendParent};
use super::best::best_position;
use super::settings::Location;
use super::{LegendError, LegendMode, LegendShadowStyle};
use crate::artist::{Artist, ArtistId, ArtistStore, DrawError, LayoutContext};
use crate::figure::FigureState;
use crate::offsetbox::{
    OffsetBox, OffsetBoxError, Placement, anchored_bbox, apply_placements, transformed_bbox,
};
use crate::patches::{FancyBboxPatch, PatchArtist, PatchShape, PatchStyle, Shadow, ShadowError};
use crate::transforms::Transform;

/// The shadow of a frame of `shape` painted with `style`, offset and
/// shaded as `shadow` says (matplotlib's `Shadow(legendPatch,
/// **_shadow_props)`; the other properties are applied by
/// [`restyle_shadow`]).
pub(super) fn frame_shadow(
    shape: FancyBboxPatch,
    style: &PatchStyle,
    shadow: &LegendShadowStyle,
) -> Result<Shadow<FancyBboxPatch>, ShadowError> {
    Shadow::new(
        shape,
        style,
        Some(Transform::Display),
        (shadow.ox, shadow.oy),
        shadow.shade,
    )
}

/// Gives the shadow patch the outline `outline` and the look matplotlib
/// gives a fresh `Shadow`: the frame's darkened colours at half opacity,
/// overridden by the properties of `shadow`, shown when the frame is.
pub(super) fn restyle_shadow(
    patch: &mut PatchArtist<Shadow<FancyBboxPatch>>,
    outline: Shadow<FancyBboxPatch>,
    shadow: &LegendShadowStyle,
    visible: bool,
) {
    let mut base = patch.base().clone();
    outline.init_style(&mut patch.style, &mut base);
    base.visible = visible;
    *patch.base_mut() = base;
    patch.shape = outline;
    shadow.props.clone().apply(patch);
}

/// Where a legend and its parts go.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LegendLayout {
    /// The legend box in display pixels, which the frame outlines.
    pub(crate) extent: Bbox,
    /// The legend font size in pixels, the frame's mutation scale.
    pub(crate) fontsize_px: f64,
    /// Where every text and handle box goes.
    pub(crate) placements: Vec<Placement<HandleRef>>,
    /// The width the draw gave the legend box in the `"expand"` mode,
    /// which it keeps (matplotlib's `_legend_box.set_width` in `draw`).
    pub(crate) expanded_width: Option<f64>,
}

/// The legend box as a draw sizes it ([`LegendArtist::layout`]).
struct SizedBox<'a> {
    /// The box, as wide as its anchor in the `"expand"` mode.
    legend_box: Cow<'a, OffsetBox<HandleRef>>,
    /// The legend font size in pixels.
    fontsize_px: f64,
    /// The width of the box in the `"expand"` mode.
    expanded_width: Option<f64>,
}

/// Why a legend cannot be laid out.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LayoutError {
    /// A text of the legend cannot be measured.
    Boxes(OffsetBoxError),
    /// The parent or the anchor's coordinate system no longer exists.
    Detached,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boxes(err) => write!(f, "cannot lay out the legend: {err}"),
            Self::Detached => f.write_str("the legend's parent no longer exists"),
        }
    }
}

impl LayoutError {
    /// The problem a draw of the legend `id` reports: a text of the
    /// legend that cannot be measured is the text's own problem, of its
    /// kind and with its message, as matplotlib's `Legend.draw` raises it
    /// measuring the text.
    fn draw_error(&self, id: ArtistId) -> DrawError {
        match self {
            Self::Boxes(err @ OffsetBoxError::Text(_)) => DrawError {
                artist: id,
                message: err.to_string(),
                kind: err.draw_kind(),
            },
            Self::Boxes(_) | Self::Detached => DrawError::new(id, self.to_string()),
        }
    }
}

impl From<OffsetBoxError> for LayoutError {
    fn from(err: OffsetBoxError) -> Self {
        Self::Boxes(err)
    }
}

impl LegendArtist {
    /// The display box `loc` refers to (matplotlib's `get_bbox_to_anchor`):
    /// without `bbox_to_anchor`, the parent's `bbox`, which for the root
    /// figure is the saved window while part of it is printed
    /// (matplotlib's `adjust_bbox` replaces the figure's `bbox`).
    pub(crate) fn anchor_box(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        match &self.anchor {
            None => match self.parent {
                LegendParent::Axes(id) => ctx.axes_extent(id),
                LegendParent::Figure(id) => ctx.frame_patch_extent(id),
            },
            Some((bbox, transform)) => Some(transformed_bbox(bbox, &ctx.resolve(transform)?)),
        }
    }

    /// The legend box in display pixels (matplotlib's
    /// `_legend_box.get_window_extent`, which locates the legend) as it
    /// measures between draws: in the `"expand"` mode as wide as its last
    /// draw made it, as narrow as its entries before the first, since
    /// matplotlib sets that width only when it draws the legend.
    pub(crate) fn extent(&self, ctx: LayoutContext<'_>) -> Result<Bbox, LayoutError> {
        let fontsize_px = ctx.measuring_points_to_pixels(self.settings.fontsize);
        self.located(&self.legend_box, ctx, fontsize_px)
    }

    /// `legend_box` located as the legend, in display pixels.
    fn located(
        &self,
        legend_box: &OffsetBox<HandleRef>,
        ctx: LayoutContext<'_>,
        fontsize_px: f64,
    ) -> Result<Bbox, LayoutError> {
        let bbox = legend_box.bbox(ctx)?;
        let offset = self.offset(ctx, &bbox, fontsize_px)?;
        Ok(bbox.translated(offset.x, offset.y))
    }

    /// Lays the legend out for the current figure geometry, as wide as
    /// its anchor in the `"expand"` mode. As matplotlib's `Legend.draw`
    /// does, the legend is located twice: for its frame
    /// (`_legend_box.get_window_extent`) and for its parts
    /// (`_legend_box.draw`).
    pub(crate) fn layout(&self, ctx: LayoutContext<'_>) -> Result<LegendLayout, LayoutError> {
        let SizedBox {
            legend_box,
            fontsize_px,
            expanded_width,
        } = self.sized_box(ctx)?;
        let extent = self.located(&legend_box, ctx, fontsize_px)?;
        // matplotlib's `_legend_box.draw` measures the box as it places its
        // children, which `place` does.
        let bbox = crate::text::font_notes::unnoted(|| legend_box.bbox(ctx))?;
        let offset = self.offset(ctx, &bbox, fontsize_px)?;
        Ok(LegendLayout {
            extent,
            fontsize_px,
            placements: legend_box.place(ctx, offset)?,
            expanded_width,
        })
    }

    /// The legend box as a draw sizes it, as wide as its anchor in the
    /// `"expand"` mode.
    fn sized_box(&self, ctx: LayoutContext<'_>) -> Result<SizedBox<'_>, LayoutError> {
        let settings = &self.settings;
        let fontsize_px = ctx.points_to_pixels(settings.fontsize);
        let mut legend_box = Cow::Borrowed(&self.legend_box);
        let mut expanded_width = None;
        if self.mode == LegendMode::Expand {
            let anchor = self.anchor_box(ctx).ok_or(LayoutError::Detached)?;
            let pad = 2.0 * (settings.borderaxespad + settings.borderpad) * fontsize_px;
            let width = anchor.width() - pad;
            legend_box.to_mut().set_width(Some(width));
            expanded_width = Some(width);
        }
        Ok(SizedBox {
            legend_box,
            fontsize_px,
            expanded_width,
        })
    }

    /// Where the origin of the legend box `bbox` goes (matplotlib's
    /// `_legend_box.get_offset`): the point it is pinned at, else where
    /// the location puts it.
    fn offset(
        &self,
        ctx: LayoutContext<'_>,
        bbox: &Bbox,
        fontsize_px: f64,
    ) -> Result<Point, LayoutError> {
        match self.box_offset {
            Some(offset) => Ok(offset),
            None => self.located_offset(ctx, bbox, fontsize_px),
        }
    }

    /// Where the location of the legend puts the origin of the legend box
    /// `bbox` (matplotlib's `_findoffset`).
    pub(super) fn located_offset(
        &self,
        ctx: LayoutContext<'_>,
        bbox: &Bbox,
        fontsize_px: f64,
    ) -> Result<Point, LayoutError> {
        let anchor = self.anchor_box(ctx).ok_or(LayoutError::Detached)?;
        let (width, height) = (bbox.width(), bbox.height());
        let pad = self.settings.borderaxespad * fontsize_px;
        let corner = match self.location {
            Location::Best => best_position(
                ctx,
                self.parent,
                &anchor,
                pad,
                (width, height),
                self.loc_used_default,
            ),
            Location::Anchored(loc) => anchored_bbox(
                loc,
                &Bbox::from_bounds(0.0, 0.0, width, height),
                &anchor,
                pad,
                pad,
            ),
            Location::Point(fx, fy) => Point::new(
                anchor.x0() + anchor.width() * fx,
                anchor.y0() + anchor.height() * fy,
            ),
        };
        Ok(corner - bbox.p0)
    }
}

/// Lays out every legend of the figure and moves its parts into place;
/// legends that cannot be laid out are reported with the next draw.
pub(crate) fn lay_out_legends(state: &mut FigureState) {
    let legends: Vec<ArtistId> = state
        .store
        .ids()
        .filter(|&id| state.store.get_as::<LegendArtist>(id).is_some())
        .collect();
    // matplotlib measures a legend's texts where it draws or measures the
    // legend (see `offsetbox::text_notes`).
    for id in legends {
        let laid_out = crate::text::font_notes::layouts_counted(0, || lay_out_legend(state, id));
        if let Err(err) = laid_out {
            state.pending_errors.push(err);
        }
    }
}

/// Lays out the legend `id` (if it is one) and moves its parts into place,
/// as matplotlib's `Legend.draw` does before drawing them.
///
/// # Errors
///
/// When the legend cannot be laid out.
pub(crate) fn lay_out_legend(state: &mut FigureState, id: ArtistId) -> Result<(), DrawError> {
    let outcome = {
        let ctx = LayoutContext::new(state);
        state.store.get_as::<LegendArtist>(id).map(|legend| {
            legend
                .layout(ctx)
                .map_err(|err| err.draw_error(id))
                .map(|layout| (layout, legend.frame, legend.shadow.clone()))
        })
    };
    match outcome {
        Some(Ok((layout, frame, shadow))) => {
            if let Some(width) = layout.expanded_width
                && let Some(legend) = state.store.get_as_mut::<LegendArtist>(id)
            {
                legend.legend_box.set_width(Some(width));
            }
            apply(&mut state.store, layout, frame, shadow)
                .map_err(|err| DrawError::new(id, err.to_string()))
        }
        Some(Err(err)) => Err(err),
        None => Ok(()),
    }
}

/// Moves the frame, its shadow, the texts and the handles of a legend to
/// where `layout` puts them.
fn apply(
    store: &mut ArtistStore,
    layout: LegendLayout,
    frame: ArtistId,
    shadow: Option<(ArtistId, LegendShadowStyle)>,
) -> Result<(), LegendError> {
    let extent = layout.extent;
    let framed = store
        .get_as_mut::<PatchArtist<FancyBboxPatch>>(frame)
        .map(|patch| {
            patch.shape.xy = extent.p0;
            patch.shape.width = extent.width();
            patch.shape.height = extent.height();
            patch.shape.mutation_scale = layout.fontsize_px;
            (
                patch.shape.clone(),
                patch.style.clone(),
                patch.base().visible,
            )
        });
    if let (Some((id, shadow)), Some((shape, style, visible))) = (shadow, framed) {
        // matplotlib makes the shadow from the frame at every draw.
        let outline = frame_shadow(shape, &style, &shadow)?;
        if let Some(patch) = store.get_as_mut::<PatchArtist<Shadow<FancyBboxPatch>>>(id) {
            restyle_shadow(patch, outline, &shadow, visible);
        }
    }
    // (A legend's box holds no clipped drawing area.)
    apply_placements(store, layout.placements, &|_| false);
    Ok(())
}
