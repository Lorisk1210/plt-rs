//! Placing a legend: the size of its box, where it goes and where each of
//! its parts lands (matplotlib's `Legend.draw`, `_findoffset` and
//! `_get_anchored_bbox`). The figure's layout pass runs this for every
//! legend once the axes are laid out.

use std::borrow::Cow;
use std::fmt;

use plt_core::{Bbox, Point};

use super::artist::{HandleRef, LegendArtist, LegendParent};
use super::best::best_position;
use super::settings::Location;
use super::{LegendError, LegendMode};
use crate::artist::{Artist, ArtistId, ArtistStore, DrawError, LayoutContext};
use crate::figure::FigureState;
use crate::offsetbox::{
    OffsetBoxError, Placement, anchored_bbox, apply_placements, transformed_bbox,
};
use crate::patches::{FancyBboxPatch, PatchArtist, PatchShape, PatchStyle, Shadow, ShadowError};
use crate::transforms::Transform;

/// Offset of the frame's shadow in points (matplotlib's `_shadow_props`).
const SHADOW_OFFSET: (f64, f64) = (2.0, -2.0);

/// How much darker the shadow is than the frame (matplotlib's `Shadow`
/// default).
const SHADOW_SHADE: f64 = 0.7;

/// The shadow of a frame of `shape` painted with `style` (matplotlib's
/// `Shadow(legendPatch, 2, -2)`).
pub(super) fn frame_shadow(
    shape: FancyBboxPatch,
    style: &PatchStyle,
) -> Result<Shadow<FancyBboxPatch>, ShadowError> {
    Shadow::new(
        shape,
        style,
        Some(Transform::Display),
        SHADOW_OFFSET,
        SHADOW_SHADE,
    )
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

impl From<OffsetBoxError> for LayoutError {
    fn from(err: OffsetBoxError) -> Self {
        Self::Boxes(err)
    }
}

impl LegendArtist {
    /// The display box `loc` refers to (matplotlib's `get_bbox_to_anchor`).
    pub(crate) fn anchor_box(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        match &self.anchor {
            None => match self.parent {
                LegendParent::Axes(id) => ctx.axes_extent(id),
                LegendParent::Figure(id) => ctx.frame_extent(id),
            },
            Some((bbox, transform)) => Some(transformed_bbox(bbox, &ctx.resolve(transform)?)),
        }
    }

    /// Lays the legend out for the current figure geometry.
    pub(crate) fn layout(&self, ctx: LayoutContext<'_>) -> Result<LegendLayout, LayoutError> {
        let settings = &self.settings;
        let fontsize_px = ctx.points_to_pixels(settings.fontsize);
        let anchor = self.anchor_box(ctx).ok_or(LayoutError::Detached)?;
        let mut legend_box = Cow::Borrowed(&self.legend_box);
        if self.mode == LegendMode::Expand {
            let pad = 2.0 * (settings.borderaxespad + settings.borderpad) * fontsize_px;
            legend_box.to_mut().set_width(Some(anchor.width() - pad));
        }
        let bbox = legend_box.bbox(ctx)?;
        let (width, height) = (bbox.width(), bbox.height());
        let pad = settings.borderaxespad * fontsize_px;
        let corner = match self.location {
            Location::Best => best_position(ctx, self.parent, &anchor, pad, (width, height)),
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
        let offset = corner - bbox.p0;
        Ok(LegendLayout {
            extent: bbox.translated(offset.x, offset.y),
            fontsize_px,
            placements: legend_box.place(ctx, offset)?,
        })
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
    for id in legends {
        let outcome = {
            let ctx = LayoutContext::new(state);
            state.store.get_as::<LegendArtist>(id).map(|legend| {
                legend
                    .layout(ctx)
                    .map_err(|err| err.to_string())
                    .map(|layout| (layout, legend.frame, legend.shadow))
            })
        };
        let applied = match outcome {
            Some(Ok((layout, frame, shadow))) => {
                apply(&mut state.store, layout, frame, shadow).map_err(|err| err.to_string())
            }
            Some(Err(message)) => Err(message),
            None => Ok(()),
        };
        if let Err(message) = applied {
            state.pending_errors.push(DrawError {
                artist: id,
                message,
            });
        }
    }
}

/// Moves the frame, its shadow, the texts and the handles of a legend to
/// where `layout` puts them.
fn apply(
    store: &mut ArtistStore,
    layout: LegendLayout,
    frame: ArtistId,
    shadow: Option<ArtistId>,
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
    if let (Some(id), Some((shape, style, visible))) = (shadow, framed) {
        // matplotlib makes the shadow from the frame at every draw.
        let outline = frame_shadow(shape, &style)?;
        if let Some(patch) = store.get_as_mut::<PatchArtist<Shadow<FancyBboxPatch>>>(id) {
            let mut base = patch.base().clone();
            outline.init_style(&mut patch.style, &mut base);
            base.visible = visible;
            *patch.base_mut() = base;
            patch.shape = outline;
        }
    }
    apply_placements(store, layout.placements);
    Ok(())
}
