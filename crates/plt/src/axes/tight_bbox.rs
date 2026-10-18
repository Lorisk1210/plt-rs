//! The extent of an axes with its decorations (matplotlib's
//! `Axes.get_tightbbox`), as layout engines measure it.

use plt_core::Bbox;

use super::AxesState;
use super::aspect::Placement;
use super::axis_layout::{AxisPainter, PlacedText};
use crate::artist::{Artist, ArtistBase, ArtistId, ClipPathSpec, LayoutContext};
use crate::axes_grid1::parasites_of;
use crate::figure::FigureState;
use crate::spines::SpineArtist;
use crate::text::TextArtist;
use crate::ticker::AxisName;
use crate::transforms::TransformContext;

impl FigureState {
    /// matplotlib's `ax.get_tightbbox(for_layout_only=...)` in display
    /// coordinates, or `None` for an invisible axes: brings the axes up to
    /// date (autoscaling, aspect, ticks), then unites its box with the
    /// tick labels, axis labels and offset texts, the titles, and the
    /// children not clipped to the axes (spines with their ticks, inset
    /// axes, unclipped artists). `for_layout_only` collapses the axis
    /// labels along their axis and the titles horizontally, so that their
    /// length does not count. Axes with parasites measure their full
    /// decorations, as matplotlib's `HostAxes.get_tightbbox` has no
    /// `for_layout_only`; their parasites count like insets.
    pub(crate) fn axes_tight_bbox(
        &mut self,
        axes: ArtistId,
        for_layout_only: bool,
    ) -> Option<Bbox> {
        self.axes_tight_bbox_at(axes, for_layout_only, Placement::Located)
    }

    /// The box of `axes` and its tight box (with full decorations) at its
    /// original position, without calling its locators (matplotlib's
    /// `ax.bbox` after `get_tightbbox(call_axes_locator=False)`), or
    /// `None` for an invisible axes. The axes keeps its placement and
    /// ticks: matplotlib measures axes for dividers while drawing, after
    /// the axes measured were drawn.
    pub(crate) fn unlocated_extents(&mut self, axes: ArtistId) -> Option<(Bbox, Bbox)> {
        let state = self.store.get_as::<AxesState>(axes)?;
        let placed = (
            state.aspect,
            state.axis(AxisName::X).ticks.clone(),
            state.axis(AxisName::Y).ticks.clone(),
        );
        let tight = self.axes_tight_bbox_at(axes, false, Placement::Original);
        let extent = self.axes_bbox(axes);
        let state = self.store.get_as_mut::<AxesState>(axes)?;
        let (aspect, x_ticks, y_ticks) = placed;
        state.aspect = aspect;
        state.axis_mut(AxisName::X).ticks = x_ticks;
        state.axis_mut(AxisName::Y).ticks = y_ticks;
        Some((extent?, tight?))
    }

    /// [`axes_tight_bbox`](Self::axes_tight_bbox) with the axes placed by
    /// `placement`.
    fn axes_tight_bbox_at(
        &mut self,
        axes: ArtistId,
        for_layout_only: bool,
        placement: Placement,
    ) -> Option<Bbox> {
        if !self.store.get_as::<AxesState>(axes)?.base().visible {
            return None;
        }
        let for_layout_only = for_layout_only && parasites_of(&self.store, axes).is_empty();
        self.lay_out_axes_at(axes, placement);
        let extent = self.axes_bbox(axes)?;
        let dpi = self.dpi();
        let shared = self.shared_tick_spaces(axes);
        let state = self.store.get_as_mut::<AxesState>(axes)?;
        state.update_ticks(&extent, dpi, shared);
        let children = state.children.clone();
        self.update_title_position(axes);
        let insets: Vec<ArtistId> = children
            .into_iter()
            .filter(|&child| self.store.get_as::<AxesState>(child).is_some())
            .collect();
        let mut boxes: Vec<Bbox> = insets
            .into_iter()
            .filter_map(|child| self.axes_tight_bbox(child, false))
            .filter(has_area)
            .collect();
        let state = self.store.get_as::<AxesState>(axes)?;
        let layout = LayoutContext::new(self);
        if state.decorations.axis_on {
            for name in [AxisName::X, AxisName::Y] {
                if state.axis(name).visible {
                    let painter = AxisPainter::new(axes, state, name);
                    boxes.extend(axis_tight_bbox(layout, &painter, for_layout_only));
                }
            }
        }
        boxes.push(extent);
        boxes.extend(state.titles.iter().filter_map(|&title| {
            let text = layout.store().get_as::<TextArtist>(title)?;
            let bbox = text.base().visible.then(|| text.window_extent(&layout))??;
            Some(if for_layout_only && bbox.width() > 0.0 {
                collapse_x(&bbox)
            } else {
                bbox
            })
        }));
        boxes.extend(
            state
                .children
                .iter()
                .filter(|child| !state.titles.contains(child))
                .filter_map(|&child| child_tight_bbox(layout, state, axes, child))
                .filter(has_area),
        );
        boxes
            .into_iter()
            .filter(|b| b.width() != 0.0 || b.height() != 0.0)
            .reduce(|a, b| a.union(&b))
    }
}

/// The extent of an axis' texts (matplotlib's `Axis.get_tightbbox`).
pub(super) fn axis_tight_bbox(
    layout: LayoutContext<'_>,
    painter: &AxisPainter<'_>,
    for_layout_only: bool,
) -> Option<Bbox> {
    let texts = painter.texts(layout, &painter.axes.rc);
    let label = texts.label.as_ref().map(|label| {
        let bbox = label.bbox();
        match painter.name {
            AxisName::X if for_layout_only && bbox.width() > 0.0 => collapse_x(&bbox),
            AxisName::Y if for_layout_only && bbox.height() > 0.0 => collapse_y(&bbox),
            _ => bbox,
        }
    });
    texts
        .offset
        .iter()
        .map(PlacedText::bbox)
        .chain(texts.tick_labels.iter().map(|label| label.text.bbox()))
        .chain(label)
        .filter(has_area)
        .reduce(|a, b| a.union(&b))
}

/// The tight extent of a child artist that is not fully clipped to the
/// axes, or `None` (matplotlib's `get_default_bbox_extra_artists` and
/// `Artist.get_tightbbox`).
fn child_tight_bbox(
    layout: LayoutContext<'_>,
    state: &AxesState,
    axes: ArtistId,
    child: ArtistId,
) -> Option<Bbox> {
    let artist = layout.store().get(child)?;
    let base = artist.base();
    if !(base.visible && base.in_layout) || layout.store().get_as::<AxesState>(child).is_some() {
        return None;
    }
    if let Some(spine) = layout.store().get_as::<SpineArtist>(child) {
        return state
            .decorations
            .framed()
            .then(|| spine_tight_bbox(layout, state, axes, spine))?;
    }
    if fully_clipped(base, axes) {
        return None;
    }
    clipped(layout, base, artist.window_extent(&layout)?)
}

/// A spine's extent, widened by the ticks of a visible axis drawn on it
/// (matplotlib's `Spine.get_window_extent`).
fn spine_tight_bbox(
    layout: LayoutContext<'_>,
    state: &AxesState,
    axes: ArtistId,
    spine: &SpineArtist,
) -> Option<Bbox> {
    let name = spine.side.axis();
    if state.axis(name).visible {
        AxisPainter::new(axes, state, name).spine_extent(layout, spine.side)
    } else {
        spine.window_extent(&layout)
    }
}

/// Whether an artist is clipped to exactly its axes (matplotlib's
/// `_fully_clipped_to_axes`).
fn fully_clipped(base: &ArtistBase, axes: ArtistId) -> bool {
    base.clip_on && base.clip_path.is_none() && base.clip_box == Some(axes)
}

/// `bbox` reduced by the artist's clip box and clip path.
fn clipped(layout: LayoutContext<'_>, base: &ArtistBase, bbox: Bbox) -> Option<Bbox> {
    if !base.clip_on {
        return Some(bbox);
    }
    let bbox = match layout.clip_rect(base) {
        Some(clip) => bbox.intersection(&clip)?,
        None => bbox,
    };
    let clip_path = match base.clip_path.as_ref() {
        Some(ClipPathSpec::Path { path, transform }) => {
            Some(layout.resolve(transform)?.transform_path(path).extents())
        }
        Some(ClipPathSpec::Artist(id)) => {
            let outline = layout.store().get(*id)?.outline(&layout)?;
            Some(outline.path.transformed(&outline.transform).extents())
        }
        Some(ClipPathSpec::BoundingBox { .. }) | None => None,
    };
    match clip_path {
        Some(clip) => bbox.intersection(&clip),
        None => Some(bbox),
    }
}

/// Whether a box has a positive, finite width and height.
fn has_area(bbox: &Bbox) -> bool {
    let (w, h) = (bbox.width(), bbox.height());
    0.0 < w && w < f64::INFINITY && 0.0 < h && h < f64::INFINITY
}

/// `bbox` narrowed to one pixel around its horizontal centre.
fn collapse_x(bbox: &Bbox) -> Bbox {
    let x0 = f64::midpoint(bbox.x0(), bbox.x1()) - 0.5;
    Bbox::from_extents(x0, bbox.y0(), x0 + 1.0, bbox.y1())
}

/// `bbox` narrowed to one pixel around its vertical centre.
fn collapse_y(bbox: &Bbox) -> Bbox {
    let y0 = f64::midpoint(bbox.y0(), bbox.y1()) - 0.5;
    Bbox::from_extents(bbox.x0(), y0, bbox.x1(), y0 + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn decorations_reach_beyond_the_axes() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        ax.set_xlabel("a long x label that does not count");
        let mut state = ax.raw().figure_state().borrow_mut();
        let extent = state.axes_bbox(ax.id()).unwrap();
        let tight = state.axes_tight_bbox(ax.id(), true).unwrap();
        assert!(tight.x0() < extent.x0(), "y tick labels on the left");
        assert!(
            tight.y0() < extent.y0() - 20.0,
            "x tick labels and label below"
        );
        assert!(
            (tight.x1() - extent.x1()).abs() < 10.0,
            "nothing on the right"
        );
        let full = state.axes_tight_bbox(ax.id(), false).unwrap();
        assert!(full.width() >= tight.width());
    }

    #[test]
    fn collapsing_keeps_the_centre() {
        let bbox = Bbox::from_extents(10.0, 0.0, 30.0, 4.0);
        assert_eq!(collapse_x(&bbox), Bbox::from_extents(19.5, 0.0, 20.5, 4.0));
        assert_eq!(collapse_y(&bbox), Bbox::from_extents(10.0, 1.5, 30.0, 2.5));
        assert!(!has_area(&Bbox::from_extents(0.0, 0.0, 0.0, 1.0)));
    }
}
