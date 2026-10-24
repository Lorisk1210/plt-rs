//! Handles: the user-facing references to artists inside a figure.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use plt_core::{Bbox, Point};
use plt_render::Sketch;

use super::{
    AggFilter, Artist, ArtistBase, ArtistId, ArtistStore, ClipPathSpec, LayoutContext, Picker,
};
use crate::axes::Axes;
use crate::figure::{Figure, FigureState};
use crate::patheffects::PathEffect;
use crate::transforms::Transform;

/// An untyped reference to one artist of a figure.
///
/// Typed handles (`Line2D`, `Axes`, ...) wrap a `RawHandle` and implement
/// [`ArtistHandle`]. Using a handle after its artist was removed from the
/// figure panics: like a closed file, it no longer refers to anything.
#[derive(Clone)]
pub struct RawHandle {
    figure: Rc<RefCell<FigureState>>,
    id: ArtistId,
}

impl fmt::Debug for RawHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawHandle")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl PartialEq for RawHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.figure, &other.figure) && self.id == other.id
    }
}

impl Eq for RawHandle {}

impl RawHandle {
    pub(crate) fn new(figure: Rc<RefCell<FigureState>>, id: ArtistId) -> Self {
        Self { figure, id }
    }

    /// The artist's id within its figure.
    pub fn id(&self) -> ArtistId {
        self.id
    }

    pub(crate) fn figure_state(&self) -> &Rc<RefCell<FigureState>> {
        &self.figure
    }

    /// Whether the artist still exists.
    pub fn is_alive(&self) -> bool {
        self.figure.borrow().store.contains(self.id)
    }

    /// The typed handle of the artist, if it is of type `H` (matplotlib
    /// code tests `isinstance`, as in `fig.findobj(Line2D)`).
    ///
    /// ```
    /// use plt::lines::Line2D;
    /// use plt::prelude::*;
    /// use plt::text::Text;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// let lines: Vec<Line2D> = fig.findobj(|_| true).iter().filter_map(|a| a.downcast()).collect();
    /// assert_eq!(lines.len(), 1);
    /// assert!(lines[0].raw().downcast::<Text>().is_none());
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn downcast<H: TypedHandle>(&self) -> Option<H> {
        H::from_raw_handle(self)
    }

    /// Whether the artist exists and is a `T`.
    pub(crate) fn is<T: Artist>(&self) -> bool {
        self.figure.borrow().store.get_as::<T>(self.id).is_some()
    }

    /// Reads the artist as concrete type `T`.
    pub(crate) fn with<T: Artist, R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let state = self.figure.borrow();
        f(state
            .store
            .get_as::<T>(self.id)
            .unwrap_or_else(|| self.gone()))
    }

    /// Mutates the artist as concrete type `T` and marks the figure stale
    /// (see [`RawHandle::mark_changed`]).
    pub(crate) fn with_mut<T: Artist, R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut state = self.figure.borrow_mut();
        let artist = state
            .store
            .get_as_mut::<T>(self.id)
            .unwrap_or_else(|| self.gone());
        let result = f(artist);
        self.mark_changed(&mut state);
        result
    }

    /// Mutates the artist as concrete type `T` together with the rest of
    /// the figure's artists (see [`ArtistStore::with_detached`]) and
    /// marks the figure stale (see [`RawHandle::mark_changed`]).
    pub(crate) fn with_detached<T: Artist, R>(
        &self,
        f: impl FnOnce(&mut T, &mut ArtistStore) -> R,
    ) -> R {
        let mut state = self.figure.borrow_mut();
        let result = state
            .store
            .with_detached(self.id, f)
            .unwrap_or_else(|| self.gone());
        self.mark_changed(&mut state);
        result
    }

    /// Marks the figure stale after a change to the artist, unless the
    /// artist is animated: figures leave animated artists out of their
    /// draws, so a change to one only shows once the program draws it
    /// (matplotlib's `Artist.stale` does not reach the figure from an
    /// animated artist).
    fn mark_changed(&self, state: &mut FigureState) {
        let animated = state.store.get(self.id).is_some_and(|a| a.base().animated);
        if !animated {
            state.stale = true;
        }
    }

    /// Reads the common artist properties.
    pub(crate) fn with_base<R>(&self, f: impl FnOnce(&ArtistBase) -> R) -> R {
        let state = self.figure.borrow();
        f(state
            .store
            .get(self.id)
            .unwrap_or_else(|| self.gone())
            .base())
    }

    /// Mutates the common artist properties and marks the figure stale
    /// (see [`RawHandle::mark_changed`]).
    pub(crate) fn with_base_mut<R>(&self, f: impl FnOnce(&mut ArtistBase) -> R) -> R {
        let mut state = self.figure.borrow_mut();
        let result = f(state
            .store
            .get_mut(self.id)
            .unwrap_or_else(|| self.gone())
            .base_mut());
        self.mark_changed(&mut state);
        result
    }

    /// Whether display point `at` hits the artist in the figure's current
    /// layout, and which of its items (matplotlib's `contains`); `None`
    /// when it misses.
    pub(crate) fn hit_test(&self, at: Point) -> Option<Vec<usize>> {
        let mut state = self.figure.borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        layout.store().get(self.id)?.pick_hit(&layout, at, None)
    }

    fn gone(&self) -> ! {
        panic!(
            "artist {} was removed from its figure or has a different type",
            self.id
        )
    }
}

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
    /// holding it).
    fn figure(&self) -> Figure {
        Figure::root_of(self.raw().figure_state())
    }

    /// The axes the artist is drawn in (matplotlib's `artist.axes`): an
    /// axes is its own, the artists it owns (lines, texts, its legend, ...)
    /// give it, and artists placed on the figure give `None`. So do the
    /// artists of a 3-D axes, whose handle is the
    /// [`Axes3D`](crate::mplot3d::Axes3D) they were made with.
    fn axes(&self) -> Option<Axes> {
        self.raw().owning_axes()
    }

    /// The artist and the artists it owns for which `matches` holds, each
    /// after the artists it owns (matplotlib's `Artist.findobj(match)`).
    ///
    /// ```
    /// use plt::lines::Line2D;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// let lines: Vec<Line2D> = ax.findobj(|_| true).iter().filter_map(|a| a.downcast()).collect();
    /// assert_eq!(lines, [line]);
    /// assert_eq!(fig.findobj(|artist| artist == ax.raw()), [ax.raw().clone()]);
    /// # Ok::<(), plt::axes::PlotError>(())
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
    /// (matplotlib's `set_picker`); `None` makes it unpickable again.
    fn set_picker(&self, picker: Option<Picker>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.picker = picker);
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
        self.raw().with_base_mut(|b| b.clip_box = owner);
        self
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

    /// Requests rasterization of this artist in vector output.
    fn set_rasterized(&self, rasterized: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.rasterized = rasterized);
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
    /// it ([`Axes::draw_artist`](crate::axes::Axes::draw_artist)).
    ///
    /// [`Figure::is_stale`]: crate::figure::Figure::is_stale
    fn set_animated(&self, animated: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.animated = animated);
        self
    }

    /// Whether the artist is animated (matplotlib's `get_animated`).
    fn animated(&self) -> bool {
        self.raw().with_base(|b| b.animated)
    }

    /// Includes or excludes the artist from layout computations.
    fn set_in_layout(&self, in_layout: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.in_layout = in_layout);
        self
    }

    /// Whether layout computations account for the artist (matplotlib's
    /// `get_in_layout`).
    fn in_layout(&self) -> bool {
        self.raw().with_base(|b| b.in_layout)
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

    /// Sets the group id used in vector output.
    fn set_gid(&self, gid: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let gid = gid.into();
        self.raw().with_base_mut(|b| b.gid = Some(gid));
        self
    }

    /// The group id used in vector output, if set.
    fn gid(&self) -> Option<String> {
        self.raw().with_base(|b| b.gid.clone())
    }

    /// Sets the hyperlink used in vector output.
    fn set_url(&self, url: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let url = url.into();
        self.raw().with_base_mut(|b| b.url = Some(url));
        self
    }

    /// The hyperlink used in vector output, if set (matplotlib's
    /// `get_url`).
    fn url(&self) -> Option<String> {
        self.raw().with_base(|b| b.url.clone())
    }

    /// The artist's extent in display coordinates in the figure's current
    /// layout (laid out first), if it occupies space (matplotlib's
    /// `get_window_extent`).
    fn window_extent(&self) -> Option<Bbox> {
        let mut state = self.raw().figure_state().borrow_mut();
        state.prepare();
        let ctx = LayoutContext::new(&state);
        state.store.get(self.id())?.window_extent(&ctx)
    }

    /// Whether display point `at` hits the artist in the figure's current
    /// layout (laid out first), as a mouse event there would (matplotlib's
    /// `contains`): the indices of the items hit (empty for artists hit as
    /// a whole), or `None` when the point misses.
    fn contains(&self, at: Point) -> Option<Vec<usize>> {
        self.raw().hit_test(at)
    }

    /// Removes the artist (and everything it owns) from the figure.
    fn remove(&self) {
        self.raw()
            .figure_state()
            .borrow_mut()
            .remove_artist(self.id());
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
