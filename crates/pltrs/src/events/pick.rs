//! Pick events (matplotlib's `Figure.pick` and `PickEvent`): mouse presses
//! and scrolls over artists that have a [`Picker`] fire [`Event::Pick`], as
//! do those near an axis given one ([`Axis::set_picker`]).

use std::rc::Rc;

use pltrs_core::Point;

use super::{Event, MouseEvent};
use crate::artist::{ArtistHandle, ArtistId, LayoutContext, PickFn, Picker, RawHandle};
use crate::axes::{Axes, AxesState, Axis};
use crate::figure::{Figure, FigureState};
use crate::ticker::AxisName;

/// An artist picked by a mouse press or scroll (matplotlib's `PickEvent`).
#[derive(Clone, Debug, PartialEq)]
pub struct PickEvent {
    /// The press or scroll that picked the artist.
    pub mouse: MouseEvent,
    /// The artist picked, or the axes whose [`axis`](Self::axis) was.
    pub artist: RawHandle,
    /// The items of the artist under the pointer: point or segment indices
    /// of a line, item indices of a collection; empty for artists picked
    /// as a whole.
    pub ind: Vec<usize>,
    /// The axis of the axes `artist` picked, when an axis was
    /// ([`Axis::set_picker`]; matplotlib's `event.artist` is the axis).
    pub axis: Option<AxisName>,
}

impl PickEvent {
    /// Whether `artist` is the artist picked; an axes is not when one of
    /// its axes was.
    pub fn is(&self, artist: &impl ArtistHandle) -> bool {
        self.axis.is_none() && self.artist == *artist.raw()
    }

    /// The axis picked, if an axis was rather than an artist.
    pub fn picked_axis(&self) -> Option<Axis> {
        self.axis
            .map(|name| Axes::from_raw(self.artist.clone()).axis_of(name))
    }
}

/// What a press may pick: an artist, or an axis of an axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Candidate {
    /// An artist with a picker.
    Artist(ArtistId),
    /// An axis with a picker, of the axes with this id.
    Axis(ArtistId, AxisName),
}

impl Candidate {
    /// The artist a pick reports: the artist, or the axes of the axis.
    const fn artist(self) -> ArtistId {
        match self {
            Self::Artist(id) | Self::Axis(id, _) => id,
        }
    }

    /// The axis a pick reports, if any.
    const fn axis(self) -> Option<AxisName> {
        match self {
            Self::Artist(_) => None,
            Self::Axis(_, name) => Some(name),
        }
    }
}

/// How a pickable artist answers a pick.
enum Answer {
    /// Its own hit test found these items.
    Hit(Vec<usize>),
    /// A custom picker decides.
    Ask(Rc<PickFn>),
}

impl Figure {
    /// Fires a pick event for every pickable artist under the pointer of
    /// `mouse`, parents before children, and for the pickable axes of an
    /// axes after its children (matplotlib's `Figure.pick`, which figures
    /// run on every button press and scroll before other callbacks).
    /// Nothing is picked while the widget lock is held.
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use pltrs::artist::{ArtistHandle, Picker};
    /// use pltrs::events::{Event, EventKind, EventSource};
    /// use pltrs::figure::Figure;
    /// use pltrs::navigation::PointerButton;
    ///
    /// let fig = Figure::with_size(6.4, 4.8, 100.0);
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// line.set_picker(Some(Picker::Radius(5.0)));
    /// let picked = Rc::new(RefCell::new(Vec::new()));
    /// let log = Rc::clone(&picked);
    /// fig.connect(EventKind::Pick, move |event| {
    ///     if let Event::Pick(pick) = event {
    ///         log.borrow_mut().push(pick.ind.clone());
    ///     }
    /// });
    /// // The diagonal runs through the middle of the laid-out axes.
    /// fig.layout();
    /// let (x0, y0, width, height) = ax.position().bounds();
    /// let (x, y) = ((x0 + width / 2.0) * 640.0, (y0 + height / 2.0) * 480.0);
    /// EventSource::new().button_press(&fig, x, y, PointerButton::Left, false);
    /// assert_eq!(*picked.borrow(), [vec![0]]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn pick(&self, mouse: &MouseEvent) {
        if self.widgets_locked() {
            return;
        }
        let inaxes = mouse.location.inaxes.as_ref().map(ArtistHandle::id);
        let candidates = {
            let mut state = self.state_mut();
            state.prepare();
            pickable_artists(&state, self.raw().id(), inaxes, &|_| false)
        };
        let at = Point::new(mouse.location.x, mouse.location.y);
        for candidate in candidates {
            // Each answer is asked for afresh: callbacks of earlier picks
            // may have changed the figure.
            let artist = self.handle(candidate.artist());
            let ind = match self.answer(candidate, at) {
                Some(Answer::Hit(ind)) => ind,
                Some(Answer::Ask(test)) => match test(&artist, mouse) {
                    Some(ind) => ind,
                    None => continue,
                },
                None => continue,
            };
            self.process_event(&Event::Pick(PickEvent {
                mouse: mouse.clone(),
                artist,
                ind,
                axis: candidate.axis(),
            }));
        }
    }

    /// How `candidate` answers a pick at display point `at`, if it is
    /// still pickable and not missed by its own hit test.
    fn answer(&self, candidate: Candidate, at: Point) -> Option<Answer> {
        let mut state = self.state_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        match candidate {
            Candidate::Artist(id) => {
                let artist = layout.store().get(id)?;
                match artist.base().picker.as_ref()? {
                    Picker::Custom(test) => Some(Answer::Ask(Rc::clone(test))),
                    picker => artist
                        .pick_hit(&layout, at, picker.radius())
                        .map(Answer::Hit),
                }
            }
            Candidate::Axis(axes, name) => {
                let picking = &layout.store().get_as::<AxesState>(axes)?.axis(name).picking;
                match picking.picker.as_ref()? {
                    Picker::Custom(test) => Some(Answer::Ask(Rc::clone(test))),
                    Picker::Default | Picker::Radius(_) => picking
                        .hits(&layout.axes_extent(axes)?, name, at)
                        .then(|| Answer::Hit(Vec::new())),
                }
            }
        }
    }

    /// The artists under `root` (an artist of the figure, or the figure
    /// itself) that a press over `inaxes` (the axes under the pointer)
    /// asks, in the order [`pick`](Self::pick) asks them: those with a
    /// [`Picker`] and those of `also` (artists whose program decides their
    /// picks itself, with a Python callable say), parents before children,
    /// leaving out the children of other axes than `inaxes`.
    ///
    /// ```
    /// use pltrs::artist::{ArtistHandle, Picker};
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let first = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// let second = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// second.set_picker(Some(Picker::Default));
    /// let order = fig.pick_order(fig.raw(), Some(&ax), &[first.raw().clone()]);
    /// assert_eq!(order, [first.raw().clone(), second.raw().clone()]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn pick_order(
        &self,
        root: &RawHandle,
        inaxes: Option<&Axes>,
        also: &[RawHandle],
    ) -> Vec<RawHandle> {
        let ids: Vec<ArtistId> = also.iter().map(RawHandle::id).collect();
        let inaxes = inaxes.map(ArtistHandle::id);
        let candidates = {
            let mut state = self.state_mut();
            state.prepare();
            pickable_artists(&state, root.id(), inaxes, &|id| ids.contains(&id))
        };
        candidates
            .into_iter()
            .filter_map(|candidate| match candidate {
                Candidate::Artist(id) => Some(self.handle(id)),
                Candidate::Axis(..) => None,
            })
            .collect()
    }

    fn handle(&self, id: ArtistId) -> RawHandle {
        RawHandle::new(Rc::clone(self.raw().figure_state()), id)
    }
}

/// The pickable artists under `root` (those with a picker, and those
/// `also` accepts), parents before children, and the pickable axes of each
/// axes after its children (matplotlib's `Artist.pick` recursion):
/// children belonging to another axes than `inaxes`, the axes under the
/// pointer, are skipped with their own children.
fn pickable_artists(
    state: &FigureState,
    root: ArtistId,
    inaxes: Option<ArtistId>,
    also: &dyn Fn(ArtistId) -> bool,
) -> Vec<Candidate> {
    let layout = LayoutContext::new(state);
    let mut found = Vec::new();
    let mut pending = vec![Candidate::Artist(root)];
    while let Some(candidate) = pending.pop() {
        let Candidate::Artist(id) = candidate else {
            found.push(candidate);
            continue;
        };
        let Some(artist) = layout.store().get(id) else {
            continue;
        };
        if artist.base().picker.is_some() || also(id) {
            found.push(candidate);
        }
        // Pushed before the children, the axes' axes are asked after them.
        if let Some(axes) = layout.store().get_as::<AxesState>(id) {
            pending.extend(
                [AxisName::Y, AxisName::X]
                    .into_iter()
                    .filter(|&name| axes.axis(name).picking.picker.is_some())
                    .map(|name| Candidate::Axis(id, name)),
            );
        }
        let children = artist.children();
        pending.extend(
            children
                .into_iter()
                .rev()
                .filter(|&child| {
                    inaxes.is_none_or(|inaxes| {
                        axes_of(layout, child).is_none_or(|axes| axes == inaxes)
                    })
                })
                .map(Candidate::Artist),
        );
    }
    found
}

/// The axes artist `id` belongs to (matplotlib's `artist.axes`): itself
/// or its nearest axes ancestor.
fn axes_of(layout: LayoutContext<'_>, mut id: ArtistId) -> Option<ArtistId> {
    loop {
        if layout.axes_extent(id).is_some() {
            return Some(id);
        }
        id = layout.store().get(id)?.base().parent?;
    }
}

#[cfg(test)]
mod tests;
