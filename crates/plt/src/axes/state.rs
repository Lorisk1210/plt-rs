//! The axes artist: a data coordinate system placed inside a figure frame.

use std::rc::Rc;

use plt_core::{Bbox, Path, Rgba};

use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, LayoutContext};
use crate::transforms::{AxisMap, ResolvedTransform, Step, bbox_to_unit, unit_to_bbox};

/// State of one axes (matplotlib's `_AxesBase`).
#[derive(Debug)]
pub(crate) struct AxesState {
    base: ArtistBase,
    /// Placement as a fraction of the owning frame.
    pub(crate) position: Bbox,
    /// Visible data range; `x0 > x1` means an inverted axis.
    pub(crate) view_lim: Bbox,
    /// Extent of all data added so far.
    pub(crate) data_lim: Bbox,
    /// Non-linear scale of each axis (`None` = linear).
    pub(crate) x_map: Option<Rc<dyn AxisMap>>,
    /// Non-linear scale of the y axis (`None` = linear).
    pub(crate) y_map: Option<Rc<dyn AxisMap>>,
    /// Background colour.
    pub(crate) facecolor: Rgba,
    /// Whether the background patch is drawn.
    pub(crate) frame_on: bool,
    /// Plotted artists in insertion order.
    pub(crate) children: Vec<ArtistId>,
}

impl AxesState {
    /// Axes at `position` (frame fractions) with unit view limits.
    pub(crate) fn new(position: Bbox) -> Self {
        Self {
            base: ArtistBase::default(),
            position,
            view_lim: Bbox::UNIT,
            data_lim: Bbox::NULL,
            x_map: None,
            y_map: None,
            facecolor: Rgba::WHITE,
            frame_on: true,
            children: Vec::new(),
        }
    }

    /// The full data → display transform for the axes placed at `display`:
    /// scales, then view limits onto the unit square, then onto `display`.
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
        let forward = |map: &Option<Rc<dyn AxisMap>>, v: f64| map.as_ref().map_or(v, |m| m.forward(v));
        let scaled_limits = Bbox::from_extents(
            forward(&self.x_map, self.view_lim.x0()),
            forward(&self.y_map, self.view_lim.y0()),
            forward(&self.x_map, self.view_lim.x1()),
            forward(&self.y_map, self.view_lim.y1()),
        );
        let affine = bbox_to_unit(&scaled_limits).then(unit_to_bbox(display));
        if self.x_map.is_none() && self.y_map.is_none() {
            ResolvedTransform::affine(affine)
        } else {
            ResolvedTransform::with_step(
                Step::Separable(self.x_map.clone(), self.y_map.clone()),
                affine,
            )
        }
    }
}

impl Artist for AxesState {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let Some(extent) = ctx.layout().axes_extent(self.base.id()) else {
            return;
        };
        if self.frame_on {
            let mut gc = ctx.base_gc(&self.base);
            gc.linewidth = 0.0;
            ctx.renderer()
                .draw_path(&gc, &Path::unit_rectangle(), &unit_to_bbox(&extent), Some(self.facecolor));
        }
        ctx.draw_children_sorted(&self.children);
    }

    fn children(&self) -> Vec<ArtistId> {
        self.children.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.children.retain(|&id| id != child);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        ctx.axes_extent(self.base.id())
    }
}
