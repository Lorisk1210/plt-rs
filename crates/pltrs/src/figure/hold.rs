//! Artists made before they are added to a plot (matplotlib's
//! `Rectangle(...)` or `Line2D(...)` before `ax.add_patch` or
//! `ax.add_line`).
//!
//! pltrs makes artists inside a figure, so such an artist is *held* by a
//! figure: made there as a removed artist, which the figure never draws,
//! lays out or autoscales over, while its handle works as any other.
//! [`Axes::adopt_held`] then moves it into an axes of any figure, as
//! matplotlib's adding methods do.

use std::rc::Rc;

use super::Figure;
use crate::artist::{Artist, ArtistHandle, AttachError, RawHandle};
use crate::axes::Axes;
use crate::axes::DataUpdate;
use crate::collections::{Collection, CollectionSpec};
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchShape};

/// How an axes adds an artist it adopts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdoptAs {
    /// matplotlib's `add_patch`: the data limits grow to cover it, the
    /// view stays, and `relim` counts it.
    Patch,
    /// matplotlib's `add_line`: as a patch.
    Line,
    /// matplotlib's `add_collection`: with `autolim`, the data limits grow
    /// to cover it and the view is autoscaled, and `relim` counts it.
    Collection {
        /// Whether the limits follow the collection.
        autolim: bool,
    },
    /// matplotlib's `add_artist`: the limits stay as they are.
    Artist,
}

impl Figure {
    /// Holds a new artist of its own: as a removed artist, which the figure
    /// never draws.
    fn hold(&self, artist: Box<dyn Artist>) -> RawHandle {
        let id = self.state.borrow_mut().insert_removed(artist);
        RawHandle::new(Rc::clone(&self.state), id)
    }

    /// A patch of `shape` belonging to no plot yet, styled by the rc
    /// defaults: matplotlib's patch made before [`Axes::adopt_held`] adds
    /// it.
    ///
    /// ```
    /// use pltrs::figure::AdoptAs;
    /// use pltrs::patches::Rectangle;
    /// use pltrs::prelude::*;
    /// let held = Figure::new().hold_patch(Rectangle::new((0.0, 0.0), 2.0, 1.0));
    /// held.set_facecolor("red");
    /// let ax = Figure::new().subplot();
    /// ax.adopt_held(held.raw(), AdoptAs::Patch)?;
    /// assert_eq!(ax.patches().len(), 1);
    /// # Ok::<(), pltrs::artist::AttachError>(())
    /// ```
    pub fn hold_patch<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let patch = PatchArtist::new(shape, crate::rc::params());
        Patch::from_raw(self.hold(Box::new(patch)))
    }

    /// A line through `(x, y)` belonging to no plot yet, styled by `props`
    /// over the rc defaults (matplotlib's `Line2D(x, y, **kwargs)`).
    pub fn hold_line(&self, x: Vec<f64>, y: Vec<f64>, props: LineProps) -> Line2D {
        let mut line = LineArtist::new(x, y, crate::rc::params());
        props.apply(&mut line);
        Line2D::from_raw(self.hold(Box::new(line)))
    }

    /// The collection of `spec` belonging to no plot yet (matplotlib's
    /// `LineCollection(...)` and friends).
    pub fn hold_collection(&self, spec: CollectionSpec) -> Collection {
        Collection::from_raw(self.hold(Box::new(spec.artist)))
    }
}

impl Axes {
    /// Adds a held artist (one made by
    /// [`Figure::hold_patch`](crate::figure::Figure::hold_patch) and its
    /// siblings) to this axes, as `how` says matplotlib adds it: in data
    /// coordinates unless it has a
    /// transform of its own, clipped to the axes unless it has a clip path.
    /// An artist held by this axes' figure, or removed from it, keeps its
    /// handle; one held by another figure moves here and the returned
    /// handle replaces its old one.
    ///
    /// # Errors
    ///
    /// [`AttachError`] when the artist is in a plot already, was removed
    /// with an artist owning it, owns artists of its own (only artists
    /// without children move between figures), or cannot go into an axes.
    pub fn adopt_held(&self, artist: &RawHandle, how: AdoptAs) -> Result<RawHandle, AttachError> {
        let source = artist.figure_state();
        if Rc::ptr_eq(source, self.raw().figure_state()) {
            self.attach(artist)?;
            return Ok(artist.clone());
        }
        let mut moved = {
            let mut state = source.borrow_mut();
            if state.is_attached(artist.id()) {
                return Err(AttachError::NotRemoved);
            }
            if !state.is_removed(artist.id()) {
                return Err(AttachError::OwnedByRemoved);
            }
            let moved = state
                .detached
                .take(artist.id())
                .ok_or(AttachError::OwnedByRemoved)?;
            if !moved.children().is_empty() || moved.as_any_axes() {
                state.detached.insert(artist.id(), moved);
                return Err(AttachError::Misplaced);
            }
            moved
        };
        let base = moved.base_mut();
        base.clip_box = None;
        base.transform.get_or_insert_with(|| self.trans_data());
        let limits = moved.limit_points(self.id());
        let handle = match how {
            AdoptAs::Patch | AdoptAs::Line => {
                moved.base_mut().in_autoscale = true;
                self.adopt_boxed(moved, &DataUpdate::extending(limits))
            }
            AdoptAs::Collection { autolim: true } => {
                moved.base_mut().in_autoscale = true;
                let handle = self.adopt_boxed(moved, &DataUpdate::default());
                self.unstale_view();
                self.update_data(&DataUpdate::autoscaled(limits));
                handle
            }
            AdoptAs::Collection { autolim: false } | AdoptAs::Artist => {
                self.adopt_boxed(moved, &DataUpdate::default())
            }
        };
        Ok(handle)
    }
}

/// Whether an artist is an axes, which no axes takes.
trait AxesCheck {
    /// Whether the artist is a 2-D or 3-D axes.
    fn as_any_axes(&self) -> bool;
}

impl AxesCheck for Box<dyn Artist> {
    fn as_any_axes(&self) -> bool {
        let any: &dyn std::any::Any = self.as_ref();
        any.is::<crate::axes::AxesState>() || any.is::<crate::mplot3d::Axes3DState>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::Rectangle;

    #[test]
    fn held_artists_move_into_another_figure_with_their_style() {
        let holder = Figure::new();
        let held = holder.hold_patch(Rectangle::new((1.0, 2.0), 3.0, 4.0));
        held.set_facecolor("red");
        let ax = Figure::new().subplot();
        let adopted = ax.adopt_held(held.raw(), AdoptAs::Patch).unwrap();
        let patch: Patch<Rectangle> = adopted.downcast().unwrap();
        assert_eq!(patch.style().facecolor, Some("red".into()));
        assert_eq!(ax.patches(), [adopted]);
        let limits = ax.data_lim().bbox;
        assert_eq!((limits.x0(), limits.y1()), (1.0, 6.0));
        assert_eq!(
            ax.adopt_held(held.raw(), AdoptAs::Patch),
            Err(AttachError::OwnedByRemoved),
            "the held artist left its holder"
        );
    }

    #[test]
    fn collections_autoscale_and_artists_leave_the_limits_alone() {
        let holder = Figure::new();
        let segments = vec![vec![(0.0, 0.0).into(), (10.0, 5.0).into()]];
        let lines = holder.hold_collection(CollectionSpec::lines(segments));
        let ax = Figure::new().subplot();
        ax.adopt_held(lines.raw(), AdoptAs::Collection { autolim: true })
            .unwrap();
        assert_eq!(ax.xlim().1, 10.5, "the view covers the lines, with margins");
        let line = holder.hold_line(vec![0.0, 100.0], vec![0.0, 100.0], LineProps::default());
        ax.adopt_held(line.raw(), AdoptAs::Artist).unwrap();
        assert_eq!(ax.data_lim().bbox.x1(), 10.0);
    }

    #[test]
    fn artists_in_a_plot_are_refused() {
        let ax = Figure::new().subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let other = Figure::new().subplot();
        assert_eq!(
            other.adopt_held(line.raw(), AdoptAs::Line),
            Err(AttachError::NotRemoved)
        );
        assert_eq!(
            ax.adopt_held(line.raw(), AdoptAs::Line),
            Err(AttachError::NotRemoved)
        );
    }
}
