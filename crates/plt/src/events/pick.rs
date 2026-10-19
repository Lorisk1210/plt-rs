//! Pick events (matplotlib's `Figure.pick` and `PickEvent`): mouse presses
//! and scrolls over artists that have a [`Picker`] fire [`Event::Pick`].

use std::rc::Rc;

use plt_core::Point;

use super::{Event, MouseEvent};
use crate::artist::{ArtistHandle, ArtistId, LayoutContext, PickFn, Picker, RawHandle};
use crate::figure::{Figure, FigureState};

/// An artist picked by a mouse press or scroll (matplotlib's `PickEvent`).
#[derive(Clone, Debug, PartialEq)]
pub struct PickEvent {
    /// The press or scroll that picked the artist.
    pub mouse: MouseEvent,
    /// The artist picked.
    pub artist: RawHandle,
    /// The items of the artist under the pointer: point or segment indices
    /// of a line, item indices of a collection; empty for artists picked
    /// as a whole.
    pub ind: Vec<usize>,
}

impl PickEvent {
    /// Whether `artist` is the artist picked.
    pub fn is(&self, artist: &impl ArtistHandle) -> bool {
        self.artist == *artist.raw()
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
    /// `mouse`, parents before children (matplotlib's `Figure.pick`, which
    /// figures run on every button press and scroll before other
    /// callbacks). Nothing is picked while the widget lock is held.
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use plt::artist::{ArtistHandle, Picker};
    /// use plt::events::{Event, EventKind, EventSource};
    /// use plt::figure::Figure;
    /// use plt::navigation::PointerButton;
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
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn pick(&self, mouse: &MouseEvent) {
        if self.widgets_locked() {
            return;
        }
        let inaxes = mouse.location.inaxes.as_ref().map(ArtistHandle::id);
        let candidates = {
            let mut state = self.state_mut();
            state.prepare();
            pickable_artists(&state, self.raw().id(), inaxes)
        };
        let at = Point::new(mouse.location.x, mouse.location.y);
        for id in candidates {
            // Each answer is asked for afresh: callbacks of earlier picks
            // may have changed the figure.
            let ind = match self.answer(id, at) {
                Some(Answer::Hit(ind)) => ind,
                Some(Answer::Ask(test)) => match test(&self.handle(id), mouse) {
                    Some(ind) => ind,
                    None => continue,
                },
                None => continue,
            };
            self.process_event(&Event::Pick(PickEvent {
                mouse: mouse.clone(),
                artist: self.handle(id),
                ind,
            }));
        }
    }

    /// How artist `id` answers a pick at display point `at`, if it is
    /// still pickable and not missed by its own hit test.
    fn answer(&self, id: ArtistId, at: Point) -> Option<Answer> {
        let mut state = self.state_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        let artist = layout.store().get(id)?;
        match artist.base().picker.as_ref()? {
            Picker::Custom(test) => Some(Answer::Ask(Rc::clone(test))),
            picker => artist
                .pick_hit(&layout, at, picker.radius())
                .map(Answer::Hit),
        }
    }

    fn handle(&self, id: ArtistId) -> RawHandle {
        RawHandle::new(Rc::clone(self.raw().figure_state()), id)
    }
}

/// The pickable artists under `root`, parents before children
/// (matplotlib's `Artist.pick` recursion): children belonging to another
/// axes than `inaxes`, the axes under the pointer, are skipped with their
/// own children.
fn pickable_artists(
    state: &FigureState,
    root: ArtistId,
    inaxes: Option<ArtistId>,
) -> Vec<ArtistId> {
    let layout = LayoutContext::new(state);
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let Some(artist) = layout.store().get(id) else {
            continue;
        };
        if artist.base().picker.is_some() {
            found.push(id);
        }
        let children = artist.children();
        pending.extend(children.into_iter().rev().filter(|&child| {
            inaxes.is_none_or(|inaxes| axes_of(layout, child).is_none_or(|axes| axes == inaxes))
        }));
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
