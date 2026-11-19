//! Handles: the user-facing references to artists inside a figure.

use pltrs_core::{Bbox, Point};
use pltrs_render::Sketch;

use super::{AggFilter, ArtistId, BaseProps, ClipPathSpec, LayoutContext, Picker, RawHandle};
use crate::axes::Axes;
use crate::figure::{Figure, FigureState};
use crate::patheffects::PathEffect;
use crate::transforms::Transform;

/// The handles of one type of artist, which [`RawHandle::downcast`] turns
/// untyped handles into.
pub trait TypedHandle: ArtistHandle + Sized {
    /// The handle of `raw`'s artist, if it is of this type.
    fn from_raw_handle(raw: &RawHandle) -> Option<Self>;
}

/// The untyped handle has the properties every artist shares, which is
/// what predicates over a figure's artists look at
/// ([`ArtistHandle::findobj`]).
impl ArtistHandle for RawHandle {
    fn raw(&self) -> &RawHandle {
        self
    }
}

/// Properties and operations shared by every artist handle
/// (matplotlib's `Artist` API).
///
/// Setters return `&Self` so calls chain:
/// `line.set_alpha(Some(0.5)).set_zorder(3.0);`
pub trait ArtistHandle {
    /// The underlying untyped handle.
    fn raw(&self) -> &RawHandle;

    /// The artist's id within its figure.
    fn id(&self) -> ArtistId {
        self.raw().id()
    }

    /// The figure the artist is in (matplotlib's `artist.figure`, as
    /// `get_figure(root=True)`: the artists of a subfigure give the figure
    /// holding it). A removed artist gives the figure it was removed from,
    /// which keeps it, where matplotlib gives `None`.
    fn figure(&self) -> Figure {
        Figure::root_of(self.raw().figure_state())
    }

    /// The figure or subfigure the artist is directly in (matplotlib's
    /// `get_figure(root=False)`): the subfigure holding an axes and the
    /// artists it owns, the (sub)figure holding a subfigure, and the
    /// figure itself for a figure. Compare figures by
    /// [`id`](Self::id): every handle of a figure tree is equal to the
    /// others.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::{Figure, SubfigureOptions};
    /// let fig = Figure::new();
    /// let outer = fig.subfigures(1, 1, SubfigureOptions::default())?.as_slice()[0].clone();
    /// let inner = outer.subfigures(1, 1, SubfigureOptions::default())?.as_slice()[0].clone();
    /// let ax = inner.subplot();
    /// assert_eq!(ax.parent_figure().id(), inner.id());
    /// assert_eq!(ax.figure().id(), fig.id());
    /// assert_eq!(inner.parent_figure().id(), outer.id());
    /// assert_eq!(fig.parent_figure().id(), fig.id());
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    fn parent_figure(&self) -> Figure {
        Figure::parent_of(self.raw())
    }

    /// The axes the artist is drawn in (matplotlib's `artist.axes`): an
    /// axes is its own, the artists it owns (lines, texts, its legend, ...)
    /// give it, and artists placed on the figure give `None`. So do the
    /// artists of a 3-D axes, whose handle is the
    /// [`Axes3D`](crate::mplot3d::Axes3D) they were made with, and a
    /// removed artist; the artists of a removed axes still give it.
    fn axes(&self) -> Option<Axes> {
        self.raw().owning_axes()
    }

    /// Whether the artist is clipped to exactly its axes, which tight
    /// boxes then leave it out for (matplotlib's `_fully_clipped_to_axes`):
    /// clipping is on and it is clipped to its axes' box only.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// assert!(line.fully_clipped_to_axes());
    /// line.set_clip_on(false);
    /// assert!(!line.fully_clipped_to_axes());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    fn fully_clipped_to_axes(&self) -> bool {
        self.axes().is_some_and(|axes| {
            let figure = self.raw().figure_state().borrow();
            self.raw()
                .with_base(|base| crate::axes::fully_clipped(base, axes.id(), &figure))
        })
    }

    /// The artist and the artists it owns for which `matches` holds, each
    /// after the artists it owns (matplotlib's `Artist.findobj(match)`).
    ///
    /// ```
    /// use pltrs::lines::Line2D;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// let lines: Vec<Line2D> = ax.findobj(|_| true).iter().filter_map(|a| a.downcast()).collect();
    /// assert_eq!(lines, [line]);
    /// assert_eq!(fig.findobj(|artist| artist == ax.raw()), [ax.raw().clone()]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    fn findobj(&self, matches: impl Fn(&RawHandle) -> bool) -> Vec<RawHandle>
    where
        Self: Sized,
    {
        self.raw().find(&matches)
    }

    /// Shows or hides the artist.
    fn set_visible(&self, visible: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.visible = visible);
        self
    }

    /// Whether the artist is drawn.
    fn visible(&self) -> bool {
        self.raw().with_base(|b| b.visible)
    }

    /// Makes mouse presses and scrolls on the artist fire pick events
    /// (matplotlib's `set_picker`); `None` makes it unpickable again. As
    /// in matplotlib, nothing drawn changes: the artist stays as fresh as
    /// it was.
    fn set_picker(&self, picker: Option<Picker>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut_unmarked(|b| b.picker = picker);
        self
    }

    /// How the artist is picked, if it is pickable.
    fn picker(&self) -> Option<Picker> {
        self.raw().with_base(|b| b.picker.clone())
    }

    /// Whether the artist fires pick events (matplotlib's `pickable`).
    fn pickable(&self) -> bool {
        self.raw().with_base(|b| b.picker.is_some())
    }

    /// Sets whether a status bar shows the data under the pointer when it
    /// is over the artist (matplotlib's `set_mouseover`; on by default for
    /// images).
    fn set_mouseover(&self, mouseover: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.mouseover = mouseover);
        self
    }

    /// Whether a status bar shows the data under the pointer over the
    /// artist.
    fn mouseover(&self) -> bool {
        self.raw().with_base(|b| b.mouseover)
    }

    /// Sets (or clears) the image filter for what the artist draws
    /// (matplotlib's `set_agg_filter`; see [`AggFilter`]).
    fn set_agg_filter(&self, filter: Option<AggFilter>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.agg_filter = filter);
        self
    }

    /// The image filter, if any.
    fn agg_filter(&self) -> Option<AggFilter> {
        self.raw().with_base(|b| b.agg_filter.clone())
    }

    /// Sets (or clears) the opacity override.
    fn set_alpha(&self, alpha: Option<f64>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.alpha = alpha);
        self
    }

    /// The opacity override.
    fn alpha(&self) -> Option<f64> {
        self.raw().with_base(|b| b.alpha)
    }

    /// Sets the drawing order (higher is drawn later, on top).
    fn set_zorder(&self, zorder: f64) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.zorder = zorder);
        self
    }

    /// The drawing order.
    fn zorder(&self) -> f64 {
        self.raw().with_base(|b| b.zorder)
    }

    /// Sets the legend label (labels starting with `_` are hidden).
    fn set_label(&self, label: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let label = label.into();
        self.raw().with_base_mut(|b| b.label = label);
        self
    }

    /// The legend label.
    fn label(&self) -> String {
        self.raw().with_base(|b| b.label.clone())
    }

    /// Data values that autoscaling margins must not cross, per axis
    /// (matplotlib's `sticky_edges`).
    fn sticky_edges(&self) -> (Vec<f64>, Vec<f64>) {
        self.raw().with_base(|b| b.sticky_edges.clone())
    }

    /// Replaces the sticky edges.
    fn set_sticky_edges(&self, x: Vec<f64>, y: Vec<f64>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.sticky_edges = (x, y));
        self
    }

    /// Places the artist in another coordinate system.
    fn set_transform(&self, transform: Transform) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.transform = Some(transform));
        self
    }

    /// Forgets the coordinate system set on the artist, which keeps the
    /// default of what adds it (matplotlib's `_transformSet = False`, after
    /// which `add_patch` gives the patch `transData`).
    fn unset_transform(&self) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.transform = None);
        self
    }

    /// The coordinate system set on the artist, `None` while it keeps the
    /// default of the function that made it (matplotlib's `get_transform`
    /// and `is_transform_set`).
    fn transform(&self) -> Option<Transform> {
        self.raw().with_base(|b| b.transform.clone())
    }

    /// Enables or disables clipping.
    fn set_clip_on(&self, clip_on: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.clip_on = clip_on);
        self
    }

    /// Whether clipping is applied (matplotlib's `get_clip_on`).
    fn clip_on(&self) -> bool {
        self.raw().with_base(|b| b.clip_on)
    }

    /// Clips the artist to the box of the axes or figure `owner` (an
    /// [`ArtistId`] such as `ax.id()`), or to no box (matplotlib's
    /// `set_clip_box(ax.bbox)`); clipping must be on for it to apply.
    fn set_clip_box(&self, owner: Option<ArtistId>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| {
            b.clip_box = owner;
            b.clip_region = None;
        });
        self
    }

    /// Clips the artist to the display bounding box of the
    /// [`ClipPathSpec::BoundingBox`] `region` in place of a clip box of an
    /// axes or figure, or to no box (matplotlib's `set_clip_box` given
    /// `fig.bbox`, a `TransformedBbox` or a `Bbox` in pixels, and
    /// `set_clip_path` given a `Rectangle`); the clip path stays, clipping
    /// with it.
    fn set_clip_region(&self, region: Option<ClipPathSpec>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| {
            b.clip_box = None;
            b.clip_region = region;
        });
        self
    }

    /// The clip box set by [`set_clip_region`](Self::set_clip_region), if
    /// any.
    fn clip_region(&self) -> Option<ClipPathSpec> {
        self.raw().with_base(|b| b.clip_region.clone())
    }

    /// The axes or figure whose box clips the artist, if any (matplotlib's
    /// `get_clip_box`).
    fn clip_box(&self) -> Option<ArtistId> {
        self.raw().with_base(|b| b.clip_box)
    }

    /// Clips the artist to a region besides its clip box (matplotlib's
    /// `set_clip_path`); `None` removes it.
    fn set_clip_path(&self, clip: Option<ClipPathSpec>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.clip_path = clip);
        self
    }

    /// The clip region besides the clip box, if any (matplotlib's
    /// `get_clip_path`).
    fn clip_path(&self) -> Option<ClipPathSpec> {
        self.raw().with_base(|b| b.clip_path.clone())
    }

    /// Sets the effects applied when the artist's paths are drawn
    /// (matplotlib's `set_path_effects`).
    fn set_path_effects(&self, effects: Vec<PathEffect>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.path_effects = effects);
        self
    }

    /// The effects applied when the artist's paths are drawn (matplotlib's
    /// `get_path_effects`).
    fn path_effects(&self) -> Vec<PathEffect> {
        self.raw().with_base(|b| b.path_effects.clone())
    }

    /// Requests rasterization of this artist in vector output, leaving it
    /// as fresh as it was (matplotlib's `set_rasterized` makes nothing
    /// stale).
    fn set_rasterized(&self, rasterized: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw()
            .with_base_mut_unmarked(|b| b.rasterized = rasterized);
        self
    }

    /// Whether the artist is rasterized in vector output (matplotlib's
    /// `get_rasterized`).
    fn rasterized(&self) -> bool {
        self.raw().with_base(|b| b.rasterized)
    }

    /// Marks the artist as animated: left out of normal draws (for
    /// blitting), though an animated artist of an axes is drawn when the
    /// figure is saved, as in matplotlib. Changes to an animated artist
    /// leave the figure as drawn ([`Figure::is_stale`]): the program draws
    /// it ([`Axes::draw_artist`](crate::axes::Axes::draw_artist)). Making
    /// an artist animated, or ordinary again, stales nothing either, as in
    /// matplotlib: an animation that ends (making its artists ordinary
    /// again) leaves its last frame as it was blitted.
    ///
    /// [`Figure::is_stale`]: crate::figure::Figure::is_stale
    fn set_animated(&self, animated: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut_unmarked(|b| b.animated = animated);
        self
    }

    /// Whether the artist is animated (matplotlib's `get_animated`).
    fn animated(&self) -> bool {
        self.raw().with_base(|b| b.animated)
    }

    /// Whether the artist changed since it was last drawn (matplotlib's
    /// `stale`): an artist never drawn is stale, drawing it makes it
    /// fresh, and a change to it, or to an artist it contains, makes it
    /// stale again. For a figure, whether anything in it changed since
    /// it was last drawn.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// fig.canvas().draw()?;
    /// assert!(!ax.stale());
    /// ax.set_title("changed");
    /// assert!(ax.stale() && fig.stale());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn stale(&self) -> bool {
        self.raw().is_stale()
    }

    /// Marks the artist as changed since it was last drawn, together
    /// with the artists containing it and the figure unless it is
    /// animated or removed (`true`), or as drawn as it is (`false`;
    /// matplotlib's `stale` setter).
    fn set_stale(&self, stale: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().set_stale(stale);
        self
    }

    /// Includes or excludes the artist from layout computations, leaving
    /// it as fresh as it was (matplotlib's `set_in_layout` makes nothing
    /// stale).
    fn set_in_layout(&self, in_layout: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw()
            .with_base_mut_unmarked(|b| b.in_layout = in_layout);
        self
    }

    /// Whether layout computations account for the artist (matplotlib's
    /// `get_in_layout`).
    fn in_layout(&self) -> bool {
        self.raw().with_base(|b| b.in_layout)
    }

    /// Makes the artist one in its figure without belonging to it, or
    /// back (see [`ArtistBase::figureless`](super::ArtistBase::figureless)).
    fn set_figureless(&self, figureless: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.figureless = figureless);
        self
    }

    /// Whether the artist is in its figure without belonging to it (see
    /// [`ArtistBase::figureless`](super::ArtistBase::figureless)).
    fn figureless(&self) -> bool {
        self.raw().with_base(|b| b.figureless)
    }

    /// Sets whether the artist's paths snap to the pixel grid: always,
    /// never, or as the renderer decides with `None` (matplotlib's
    /// `set_snap`).
    fn set_snap(&self, snap: Option<bool>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.snap = snap);
        self
    }

    /// The pixel snapping preference (matplotlib's `get_snap`).
    fn snap(&self) -> Option<bool> {
        self.raw().with_base(|b| b.snap)
    }

    /// Draws the artist's lines with a hand-drawn wobble, or straight
    /// with `None` (matplotlib's `set_sketch_params`; a new artist takes
    /// `path.sketch`).
    fn set_sketch_params(&self, sketch: Option<Sketch>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.sketch = sketch);
        self
    }

    /// The hand-drawn wobble, if any (matplotlib's `get_sketch_params`).
    fn sketch_params(&self) -> Option<Sketch> {
        self.raw().with_base(|b| b.sketch)
    }

    /// Sets the group id used in vector output, leaving the artist as
    /// fresh as it was (matplotlib's `set_gid` makes nothing stale).
    fn set_gid(&self, gid: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let gid = gid.into();
        self.raw().with_base_mut_unmarked(|b| b.gid = Some(gid));
        self
    }

    /// The group id used in vector output, if set.
    fn gid(&self) -> Option<String> {
        self.raw().with_base(|b| b.gid.clone())
    }

    /// Removes the group id (matplotlib's `set_gid(None)`).
    fn clear_gid(&self) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut_unmarked(|b| b.gid = None);
        self
    }

    /// Removes the hyperlink (matplotlib's `set_url(None)`).
    fn clear_url(&self) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut_unmarked(|b| b.url = None);
        self
    }

    /// Sets the hyperlink used in vector output, leaving the artist as
    /// fresh as it was (matplotlib's `set_url` makes nothing stale).
    fn set_url(&self, url: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let url = url.into();
        self.raw().with_base_mut_unmarked(|b| b.url = Some(url));
        self
    }

    /// The hyperlink used in vector output, if set (matplotlib's
    /// `get_url`).
    fn url(&self) -> Option<String> {
        self.raw().with_base(|b| b.url.clone())
    }

    /// The artist's extent in display coordinates in the figure's current
    /// layout (laid out first), if it occupies space (matplotlib's
    /// `get_window_extent`); `None` once it was removed.
    fn window_extent(&self) -> Option<Bbox> {
        let figure = self.raw().figure_state();
        // A figure being drawn, which code run meanwhile measures, is laid
        // out already.
        if !figure.is_painted() && figure.try_borrow_mut().is_some() {
            figure.work_on(FigureState::prepare);
        }
        let state = figure.borrow();
        let ctx = LayoutContext::new(&state);
        state.store.get(self.id())?.window_extent(&ctx)
    }

    /// Whether display point `at` hits the artist in the figure's current
    /// layout (laid out first), as a mouse event there would (matplotlib's
    /// `contains`): the indices of the items hit (empty for artists hit as
    /// a whole), or `None` when the point misses or the artist was removed.
    fn contains(&self, at: Point) -> Option<Vec<usize>> {
        self.raw().hit_test(at, None)
    }

    /// [`contains`](Self::contains) within the tolerance `radius` that a
    /// [`Picker::Radius`] gives (in points for lines, in pixels for
    /// collections and patches), or the artist's own with `None`.
    fn contains_within(&self, at: Point, radius: Option<f64>) -> Option<Vec<usize>> {
        self.raw().hit_test(at, radius)
    }

    /// Removes the artist, with everything it owns, from the figure
    /// (matplotlib's `remove`). Its handles stay usable: properties read
    /// and change as before, but the figure no longer draws, lays out,
    /// searches or autoscales over it, and its [`axes`](Self::axes) are
    /// `None`. The artist lives on, apart, as long as the figure, and an
    /// axes or a figure can [`attach`](crate::axes::Axes::attach) it
    /// again. Removing it twice, or removing the figure itself, does
    /// nothing.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// line.remove();
    /// assert!(ax.lines().is_empty() && line.axes().is_none());
    /// line.set_linewidth(3.0);
    /// assert_eq!(line.linewidth(), 3.0);
    /// fig.to_png()?;
    /// ax.attach(&line)?;
    /// assert_eq!(ax.lines(), [line]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn remove(&self) {
        self.raw()
            .figure_state()
            .borrow_mut()
            .remove_artist(self.id());
    }

    /// Sets at once every property `props` sets of those all artists share
    /// (matplotlib's `Artist.set` and `Artist.update` with them, and
    /// `pyplot.setp` on each artist): opacity, drawing order, label,
    /// visibility, clipping, rasterization, ids, links, transform, path
    /// effects and picker. Unset ones stay as they are. A kind's own
    /// properties have their own setters, which chain; the kinds with a
    /// property set also take all of theirs at once (`update_props` of
    /// [`Line2D`](crate::lines::Line2D::update_props),
    /// [`Patch`](crate::patches::Patch::update_props),
    /// [`Text`](crate::text::Text::update_props) and
    /// [`Collection`](crate::collections::Collection::update_props)).
    ///
    /// ```
    /// use pltrs::artist::{ArtistHandle, BaseProps};
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let title = ax.set_title("Results");
    /// let props = BaseProps { alpha: Some(0.5), zorder: Some(4.0), ..BaseProps::default() };
    /// for artist in [ax.raw(), title.raw()] {
    ///     artist.update_base_props(props.clone());
    /// }
    /// assert_eq!((title.alpha(), title.zorder()), (Some(0.5), 4.0));
    /// assert_eq!(ax.alpha(), Some(0.5));
    /// assert!(title.visible(), "unset properties stay as they were");
    /// ```
    fn update_base_props(&self, props: BaseProps) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|base| props.apply(base));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::patheffects::PathEffect;

    #[test]
    fn base_properties_read_back_as_set() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
        assert_eq!(line.transform(), Some(ax.trans_data()));
        assert!(line.clip_on() && !line.rasterized() && !line.animated() && line.in_layout());
        assert_eq!((line.clip_box(), line.clip_path()), (Some(ax.id()), None));
        assert_eq!(
            (line.snap(), line.sketch_params(), line.url()),
            (None, None, None)
        );
        assert_eq!(line.path_effects(), Vec::<PathEffect>::new());
        let sketch = Sketch {
            scale: 1.0,
            length: 100.0,
            randomness: 2.0,
        };
        line.set_transform(Transform::Display)
            .set_clip_on(false)
            .set_clip_box(None)
            .set_rasterized(true)
            .set_animated(true)
            .set_in_layout(false)
            .set_snap(Some(false))
            .set_sketch_params(Some(sketch))
            .set_path_effects(vec![PathEffect::normal()])
            .set_url("https://matplotlib.org");
        assert_eq!(line.transform(), Some(Transform::Display));
        assert!(!line.clip_on() && line.rasterized() && line.animated() && !line.in_layout());
        assert_eq!(line.clip_box(), None);
        assert_eq!(
            (line.snap(), line.sketch_params()),
            (Some(false), Some(sketch))
        );
        assert_eq!(line.path_effects(), [PathEffect::normal()]);
        assert_eq!(line.url().as_deref(), Some("https://matplotlib.org"));
    }

    #[test]
    fn new_artists_take_the_rc_sketch() {
        crate::rc::context(|| {
            crate::rc::set("path.sketch", "(1, 50, 3)").expect("a valid sketch");
            let fig = Figure::new();
            let text = fig.text(0.5, 0.5, "x");
            let sketch = text.sketch_params().expect("the rc sketch");
            assert_eq!(
                (sketch.scale, sketch.length, sketch.randomness),
                (1.0, 50.0, 3.0)
            );
            assert_eq!(text.transform(), Some(Transform::Figure(fig.id())));
        });
    }
}
