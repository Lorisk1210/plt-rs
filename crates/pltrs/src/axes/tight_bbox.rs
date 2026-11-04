//! The extent of an axes with its decorations (matplotlib's
//! `Axes.get_tightbbox`), as layout engines measure it.

use pltrs_core::Bbox;

use super::aspect::Placement;
use super::axis_layout::{AxisPainter, PlacedText};
use super::{Axes, AxesState, Axis};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ClipPathSpec, LayoutContext, tight_extent_of,
};
use crate::axes_grid1::parasites_of;
use crate::figure::FigureState;
use crate::spines::SpineArtist;
use crate::text::TextArtist;
use crate::ticker::AxisName;
use crate::transforms::TransformContext;

impl Axes {
    /// The extent of the axes with its decorations in display pixels, the
    /// figure laid out first (matplotlib's
    /// `get_tightbbox(for_layout_only=...)`): its box united with the tick
    /// labels, axis labels, offset texts, titles, spines with their ticks,
    /// inset axes and unclipped artists. `for_layout_only` leaves out the
    /// length of the axis labels and titles. `None` for invisible axes.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::with_size(4.0, 3.0, 100.0);
    /// let ax = fig.subplot();
    /// let tight = ax.tight_bbox(false).unwrap();
    /// let frame = ax.window_extent().unwrap();
    /// assert!(tight.x0() < frame.x0() && tight.y0() < frame.y0());
    /// ```
    pub fn tight_bbox(&self, for_layout_only: bool) -> Option<Bbox> {
        let mut state = self.raw().figure_state().borrow_mut();
        state.prepare();
        state.axes_tight_bbox(self.id(), for_layout_only)
    }

    /// [`Axes::tight_bbox`] without the axes' children (its spines, inset
    /// axes and other artists): the box with the tick labels, axis labels,
    /// offset texts and titles only (matplotlib's
    /// `get_tightbbox(bbox_extra_artists=[])`). `None` for invisible axes
    /// and for 3-D axes.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::with_size(4.0, 3.0, 100.0).subplot();
    /// ax.set_xlim(0.0, 1.0)?;
    /// ax.text(1.0, 0.5, "This dangles over the end");
    /// let decorated = ax.decorations_tight_bbox(false).unwrap();
    /// assert!(decorated.x1() < ax.tight_bbox(false).unwrap().x1());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn decorations_tight_bbox(&self, for_layout_only: bool) -> Option<Bbox> {
        let mut state = self.raw().figure_state().borrow_mut();
        state.prepare();
        if crate::mplot3d::is_axes3d(&state.store, self.id()) {
            return None;
        }
        state.axes_tight_bbox_at(self.id(), for_layout_only, Placement::Located, Extras::None)
    }
}

/// Which children of an axes its tight box takes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Extras {
    /// Its inset axes, spines and the artists not clipped to it
    /// (matplotlib's `get_default_bbox_extra_artists`).
    Children,
    /// None of them.
    None,
}

impl Axis {
    /// The extent of the axis' tick labels, label and offset text in
    /// display pixels, the figure laid out first (matplotlib's
    /// `Axis.get_tightbbox`); `None` for a hidden axis or one showing no
    /// text.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.xaxis().tight_bbox().is_some());
    /// ax.set_xticks(Vec::<f64>::new());
    /// assert_eq!(ax.xaxis().tight_bbox(), None);
    /// ```
    pub fn tight_bbox(&self) -> Option<Bbox> {
        self.tight_bbox_with(false)
    }

    /// [`Axis::tight_bbox`], the axis label collapsed along the axis when
    /// `for_layout_only` (matplotlib's `get_tightbbox(for_layout_only=
    /// True)`, which lets layout engines ignore labels longer than their
    /// axis).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::with_size(4.0, 3.0, 100.0).subplot();
    /// ax.set_xlabel("a label much, much, much, much longer than its axis");
    /// let full = ax.xaxis().tight_bbox_with(false).unwrap();
    /// let layout = ax.xaxis().tight_bbox_with(true).unwrap();
    /// assert!(layout.width() < full.width());
    /// assert_eq!(layout.y0(), full.y0());
    /// ```
    pub fn tight_bbox_with(&self, for_layout_only: bool) -> Option<Bbox> {
        let mut state = self.axes.raw().figure_state().borrow_mut();
        state.prepare();
        state.tight_bbox_of_axis(self.axes.id(), self.name, for_layout_only)
    }
}

impl FigureState {
    /// [`Axis::tight_bbox_with`] of axis `name` of `axes`, its ticks
    /// brought up to date.
    fn tight_bbox_of_axis(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        for_layout_only: bool,
    ) -> Option<Bbox> {
        self.lay_out_axes(axes);
        let extent = self.axes_bbox(axes)?;
        let dpi = self.dpi();
        let shared = self.shared_tick_spaces(axes);
        self.store
            .get_as_mut::<AxesState>(axes)?
            .update_ticks(&extent, dpi, shared);
        let state = self.store.get_as::<AxesState>(axes)?;
        if !state.axis(name).visible {
            return None;
        }
        let painter = AxisPainter::new(axes, state, name);
        axis_tight_bbox(LayoutContext::new(self), &painter, for_layout_only)
    }

    /// matplotlib's `ax.get_tightbbox(for_layout_only=...)` in display
    /// coordinates, or `None` for an invisible axes: brings the axes up to
    /// date (autoscaling, aspect, ticks), then unites its box with the
    /// tick labels, axis labels and offset texts, the titles, and the
    /// children not clipped to the axes (spines with their ticks, inset
    /// axes, unclipped artists). `for_layout_only` collapses the axis
    /// labels along their axis and the titles horizontally, so that their
    /// length does not count. Axes with parasites measure their full
    /// decorations, as matplotlib's `HostAxes.get_tightbbox` has no
    /// `for_layout_only`; their parasites count like insets. 3-D axes
    /// measure their box, axes and titles, `for_layout_only` leaving out
    /// the axis labels.
    pub(crate) fn axes_tight_bbox(
        &mut self,
        axes: ArtistId,
        for_layout_only: bool,
    ) -> Option<Bbox> {
        if crate::mplot3d::is_axes3d(&self.store, axes) {
            return crate::mplot3d::axes3d_fresh_tight_bbox(self, axes, for_layout_only);
        }
        self.axes_tight_bbox_at(axes, for_layout_only, Placement::Located, Extras::Children)
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
        let tight = self.axes_tight_bbox_at(axes, false, Placement::Original, Extras::Children);
        let extent = self.axes_bbox(axes);
        let state = self.store.get_as_mut::<AxesState>(axes)?;
        let (aspect, x_ticks, y_ticks) = placed;
        state.aspect = aspect;
        state.axis_mut(AxisName::X).ticks = x_ticks;
        state.axis_mut(AxisName::Y).ticks = y_ticks;
        Some((extent?, tight?))
    }

    /// [`axes_tight_bbox`](Self::axes_tight_bbox) with the axes placed by
    /// `placement`, taking in the children `extras` says.
    fn axes_tight_bbox_at(
        &mut self,
        axes: ArtistId,
        for_layout_only: bool,
        placement: Placement,
        extras: Extras,
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
            .filter(|_| extras == Extras::Children)
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
                let axis = state.axis(name);
                if axis.visible && (axis.in_layout || !for_layout_only) {
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
                .filter(|_| extras == Extras::Children)
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

/// The extent of an axis' texts (matplotlib's `Axis.get_tightbbox`,
/// which places its ticks).
pub(super) fn axis_tight_bbox(
    layout: LayoutContext<'_>,
    painter: &AxisPainter<'_>,
    for_layout_only: bool,
) -> Option<Bbox> {
    painter.axis.ticks_placed.set(true);
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
        .chain(
            (texts.tick_labels.iter())
                .filter(|label| label.in_layout)
                .map(|label| label.text.bbox()),
        )
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
            .then(|| spine.window_extent(&layout))?;
    }
    // A spine of no side (a colorbar's outline) counts while the axes is
    // framed, as the side spines do.
    if state.spines.contains(&child) && !state.decorations.framed() {
        return None;
    }
    if fully_clipped(base, axes) {
        return None;
    }
    if let Some(text) = layout.store().get_as::<TextArtist>(child)
        && !text.in_data_domain(layout.store())
    {
        return None;
    }
    clipped_extent(layout, base, tight_extent_of(artist, &layout)?)
}

/// A spine's extent widened by the ticks of the visible axis drawn on
/// it (matplotlib's `Spine.get_window_extent`); `None` for spines of
/// non-rectilinear axes, of hidden axes and outside any axes.
pub(crate) fn spine_extent_with_ticks(
    layout: LayoutContext<'_>,
    spine: &SpineArtist,
) -> Option<Bbox> {
    let axes = spine.base().parent?;
    let state = layout.store().get_as::<AxesState>(axes)?;
    let name = spine.side.axis();
    state
        .axis(name)
        .visible
        .then(|| AxisPainter::new(axes, state, name).spine_extent(layout, spine.side))?
}

/// Whether an artist is clipped to exactly its axes (matplotlib's
/// `_fully_clipped_to_axes`).
pub(crate) fn fully_clipped(base: &ArtistBase, axes: ArtistId) -> bool {
    base.clip_on && base.clip_path.is_none() && base.clip_box == Some(axes)
}

/// `bbox` reduced by the artist's clip box and clip path (the clipping
/// part of matplotlib's `Artist.get_tightbbox`).
pub(crate) fn clipped_extent(
    layout: LayoutContext<'_>,
    base: &ArtistBase,
    bbox: Bbox,
) -> Option<Bbox> {
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
pub(crate) fn collapse_x(bbox: &Bbox) -> Bbox {
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
    use crate::axes::{TickDirection, TickParams};
    use crate::figure::Figure;
    use crate::spines::SpineSide;

    #[test]
    fn spine_extents_take_in_their_ticks() {
        // matplotlib's test_tickdirs: 72 dpi, so ticks are as long in
        // pixels as in points.
        let fig = Figure::with_size(4.0, 3.0, 72.0);
        let ax = fig
            .add_axes([0.25, 0.25, 0.5, 0.5])
            .expect("a valid rectangle");
        ax.tick_params(TickParams {
            direction: Some(TickDirection::Out),
            length: Some(5.0),
            ..TickParams::default()
        });
        let frame = ax.window_extent().expect("an axes box");
        let bottom = ax.spines().get(SpineSide::Bottom).window_extent();
        let bottom = bottom.expect("a spine extent");
        assert_eq!((bottom.y0(), bottom.y1()), (frame.y0() - 5.0, frame.y0()));
        ax.xaxis().set_visible(false);
        let bare = ax.spines().get(SpineSide::Bottom).window_extent();
        assert_eq!(bare.map(|b| b.y0()), Some(frame.y0()));
    }

    #[test]
    fn axis_tight_boxes_hold_the_axis_texts() {
        let fig = Figure::with_size(4.0, 3.0, 72.0);
        let ax = fig
            .add_axes([0.25, 0.25, 0.5, 0.5])
            .expect("a valid rectangle");
        let frame = ax.window_extent().expect("an axes box");
        let labels = ax.xaxis().tight_bbox().expect("tick labels");
        assert!(labels.y1() < frame.y0());
        ax.set_xlabel("x");
        let labelled = ax.xaxis().tight_bbox().expect("tick labels and a label");
        assert!(labelled.y0() < labels.y0());
        ax.xaxis().set_visible(false);
        assert_eq!(ax.xaxis().tight_bbox(), None);
    }

    #[test]
    fn decorations_reach_beyond_the_axes() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
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
    fn the_longer_ticks_pad_a_spine_without_labels() {
        use crate::ticker::{MultipleLocator, NullFormatter, NullLocator};
        let fig = Figure::with_size(4.0, 3.0, 72.0);
        let ax = fig.subplot();
        ax.xaxis()
            .set_major_formatter(NullFormatter)
            .set_minor_locator(MultipleLocator::new(0.1).unwrap());
        ax.yaxis().set_major_locator(NullLocator);
        let mut state = ax.raw().figure_state().borrow_mut();
        let extent = state.axes_bbox(ax.id()).unwrap();
        let tight = state.axes_tight_bbox(ax.id(), true).unwrap();
        // The 3.5 pt major ticks, not added to the 2 pt minor ones.
        assert!((extent.y0() - tight.y0() - 3.5).abs() < 1e-9);
    }

    #[test]
    fn texts_outside_the_scale_domain_do_not_count() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_yscale("log").unwrap();
        ax.set_ylim(1.0, 100.0).expect("finite limits");
        let tight = |ax: &crate::axes::Axes| {
            let mut state = ax.raw().figure_state().borrow_mut();
            state.axes_tight_bbox(ax.id(), false).unwrap()
        };
        let without = tight(&ax);
        for (x, y) in [(0.5, -5.0), (0.5, 0.0), (f64::NAN, 10.0)] {
            ax.text(x, y, "outside").set_clip_on(false);
        }
        assert_eq!(tight(&ax), without);
        ax.text(0.5, 1e-3, "below").set_clip_on(false);
        assert!(tight(&ax).y0() < without.y0() - 100.0, "in the domain");
    }

    #[test]
    fn collapsing_keeps_the_centre() {
        let bbox = Bbox::from_extents(10.0, 0.0, 30.0, 4.0);
        assert_eq!(collapse_x(&bbox), Bbox::from_extents(19.5, 0.0, 20.5, 4.0));
        assert_eq!(collapse_y(&bbox), Bbox::from_extents(10.0, 1.5, 30.0, 2.5));
        assert!(!has_area(&Bbox::from_extents(0.0, 0.0, 0.0, 1.0)));
    }
}
