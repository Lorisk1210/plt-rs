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
        let position = self.aspect.active_position(&self.position);
        ProjectionView::new(self.view_lim, position, &self.xaxis, &self.yaxis)
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
            // The program's tick updates ask the program's class of the
            // axis for it themselves.
            Some(custom) if !self.axis(name).program_view => {
                custom.tick_interval(&self.projection_view(), name)
            }
            _ => self.view_interval(name),
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
    /// the one a projection defined by the program gives (as it gave it at
    /// the last layout, while the view stays the same: see
    /// [`spine_spans`](Self::spine_spans)), or the view interval of its
    /// axis.
    pub(crate) fn spine_interval(&self, side: SpineSide) -> (f64, f64) {
        let view = self.projection_view();
        self.projection
            .custom()
            .and_then(|custom| {
                match (self.program_spine_spans.as_ref()).filter(|kept| kept.holds_for(&view)) {
                    Some(kept) => kept.span(side),
                    None => custom.spine_interval(&view, side),
                }
            })
            .unwrap_or_else(|| self.view_interval(side.axis()))
    }

    /// Whether `spans` were given for the current view.
    pub(crate) fn spans_hold(&self, spans: &SpineSpans) -> bool {
        spans.holds_for(&self.projection_view())
    }

    /// The spans the projection defined by the program gives every spine
    /// at the current view (matplotlib's `Spine._adjust_location`), which
    /// a layout asks with the figure lent, as the program's code may read
    /// and change the figure; `None` for another projection.
    pub(crate) fn spine_spans(&self) -> Option<SpineSpans> {
        let custom = self.projection.custom()?;
        let view = self.projection_view();
        let spans = (SpineSide::ALL.iter())
            .filter_map(|&side| Some((side, custom.spine_interval(&view, side)?)))
            .collect();
        Some(SpineSpans {
            view: (view.limits(), view.position()),
            spans,
        })
    }
}

/// The spans a projection defined by the program gave the spines of an
/// axes, by side (those it gives a span of their own), with the view
/// (limits and position) they were given for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpineSpans {
    view: (Bbox, Bbox),
    spans: Vec<(SpineSide, (f64, f64))>,
}

impl SpineSpans {
    /// The span the spine on `side` was given, if any.
    fn span(&self, side: SpineSide) -> Option<(f64, f64)> {
        self.spans
            .iter()
            .find(|(at, _)| *at == side)
            .map(|&(_, span)| span)
    }

    /// Whether the spans were given for `view`.
    fn holds_for(&self, view: &ProjectionView<'_>) -> bool {
        self.view == (view.limits(), view.position())
    }
}
