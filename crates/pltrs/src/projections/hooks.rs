//! How the axes machinery consults a projection defined by the program:
//! its transforms, tick placement, outline and spines, evaluated at the
//! current view of an axes.

use pltrs_core::{Bbox, Path, Point};
use pltrs_text::{HAlign, VAlign};

use super::{AxesOutline, ProjectionView};
use crate::axes::{AxesState, Sides};
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, unit_to_bbox};

impl AxesState {
    /// What the hooks of a projection see of this axes.
    pub(crate) fn projection_view(&self) -> ProjectionView<'_> {
        ProjectionView::new(self.view_lim, &self.xaxis, &self.yaxis)
    }

    /// The data → display transform of a projection defined by the
    /// program, for the axes placed at `display`.
    pub(crate) fn custom_data_transform(&self, display: &Bbox) -> Option<ResolvedTransform> {
        let custom = self.projection.custom()?;
        let to_axes = custom.data_to_axes(&self.projection_view());
        Some(to_axes.then(ResolvedTransform::affine(unit_to_bbox(display))))
    }

    /// The `(data, fraction)` → display transform of the gridlines, ticks
    /// and spines of axis `name` when a projection defined by the program
    /// gives one, for the axes placed at `display`.
    pub(crate) fn custom_axis_transform(
        &self,
        name: AxisName,
        display: &Bbox,
    ) -> Option<ResolvedTransform> {
        let custom = self.projection.custom()?;
        let to_axes = custom.axis_transform(&self.projection_view(), name)?;
        Some(to_axes.then(ResolvedTransform::affine(unit_to_bbox(display))))
    }

    /// The interval of axis `name` ticks are placed in and drawn for.
    pub(crate) fn tick_interval(&self, name: AxisName) -> (f64, f64) {
        match self.projection.custom() {
            Some(custom) => custom.tick_interval(&self.projection_view(), name),
            None => self.view_interval(name),
        }
    }

    /// The sides on which the tick at `loc` of axis `name` is drawn.
    pub(crate) fn drawn_tick_sides(&self, name: AxisName, loc: f64) -> Sides {
        match self.projection.custom() {
            Some(custom) => custom.tick_sides(&self.projection_view(), name, loc),
            None => Sides::BOTH,
        }
    }

    /// Where a projection defined by the program puts the label of the
    /// tick at `loc` on `side`, in display coordinates for the axes placed
    /// at `display`, with its alignment.
    pub(crate) fn custom_tick_label(
        &self,
        side: SpineSide,
        loc: f64,
        display: &Bbox,
    ) -> Option<(Point, (HAlign, VAlign))> {
        let custom = self.projection.custom()?;
        let placement = custom.tick_label(&self.projection_view(), side, loc)?;
        let anchor = unit_to_bbox(display).transform_point(placement.anchor) + placement.offset;
        Some((anchor, (placement.halign, placement.valign)))
    }

    /// The outline of a projection defined by the program, in axes
    /// fractions, unless it is rectangular.
    pub(crate) fn custom_outline(&self) -> Option<Path> {
        match self.projection.custom()?.outline() {
            AxesOutline::Rectangle => None,
            AxesOutline::Path(path) => Some(path),
        }
    }

    /// The data interval the spine on `side` spans when it is not bounded:
    /// the one a projection defined by the program gives, or the view
    /// interval of its axis.
    pub(crate) fn spine_interval(&self, side: SpineSide) -> (f64, f64) {
        self.projection
            .custom()
            .and_then(|custom| custom.spine_interval(&self.projection_view(), side))
            .unwrap_or_else(|| self.view_interval(side.axis()))
    }
}
