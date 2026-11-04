//! The cell a figure's handles share its state through, and the changes
//! of the figures watched from outside.

use std::cell::{BorrowError, BorrowMutError, Cell, Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};

use super::in_use::{self, Access};
use super::{Figure, FigureState};
use crate::artist::{ArtistId, RawHandle};

pltrs_core::thread_state! {
    /// The watched figures borrowed for change since they were last taken
    /// ([`take_changed_figures`]).
    static CHANGED: RefCell<Vec<WeakFigure>> = const { RefCell::new(Vec::new()) };
}

/// A figure's state, shared by its handles. A watched figure
/// ([`Figure::watch_changes`]) records itself among the changed figures
/// of its thread the first time it is borrowed for change after its
/// changes were taken, which lets code following figures from outside
/// (matplotlib's transform trees in pltrs (Python)) look again at only the
/// figures that may have changed.
pub(crate) struct FigureCell {
    state: RefCell<FigureState>,
    /// The figure, while it is watched.
    watched: RefCell<Option<WeakFigure>>,
    /// Whether the figure is among the changed figures not yet taken.
    queued: Cell<bool>,
}

impl FigureCell {
    /// The cell of `state`.
    pub(crate) fn new(state: FigureState) -> Self {
        Self {
            state: RefCell::new(state),
            watched: RefCell::new(None),
            queued: Cell::new(false),
        }
    }

    /// Read access to the state (as [`RefCell::borrow`]); a state that
    /// code further up the call stack changes is reached as
    /// [`in_use::reached`] says.
    pub(crate) fn borrow(&self) -> Ref<'_, FigureState> {
        self.state
            .try_borrow()
            .unwrap_or_else(|_| in_use::reached(Access::Read))
    }

    /// Read access to the state, unless it is borrowed for change.
    pub(crate) fn try_borrow(&self) -> Result<Ref<'_, FigureState>, BorrowError> {
        self.state.try_borrow()
    }

    /// Write access to the state (as [`RefCell::borrow_mut`]), which
    /// counts as a change; a state that code further up the call stack
    /// uses is reached as [`in_use::reached`] says.
    pub(crate) fn borrow_mut(&self) -> RefMut<'_, FigureState> {
        let state = self
            .state
            .try_borrow_mut()
            .unwrap_or_else(|_| in_use::reached(Access::Change));
        self.changed();
        state
    }

    /// Write access to the state, unless it is borrowed; a change when
    /// given.
    pub(crate) fn try_borrow_mut(&self) -> Result<RefMut<'_, FigureState>, BorrowMutError> {
        let state = self.state.try_borrow_mut()?;
        self.changed();
        Ok(state)
    }

    /// Whether the state is borrowed now (for reading or for change).
    pub(crate) fn in_use(&self) -> bool {
        self.state.try_borrow_mut().is_err()
    }

    /// The address of the state (as [`RefCell::as_ptr`]), which tells
    /// whether a context's state is this cell's.
    pub(crate) fn as_ptr(&self) -> *mut FigureState {
        self.state.as_ptr()
    }

    /// Records a watched figure among the changed figures, once until they
    /// are taken.
    fn changed(&self) {
        if self.queued.get() {
            return;
        }
        if let Some(figure) = self.watched.borrow().as_ref() {
            self.queued.set(true);
            CHANGED.with(|changed| changed.borrow_mut().push(figure.clone()));
        }
    }
}

impl Figure {
    /// Has the figure recorded, from now on, among the figures of this
    /// thread that may have changed whenever a handle borrows its state for
    /// change, for [`take_changed_figures`] to hand out: how code following
    /// figures from outside learns which to look at again.
    ///
    /// ```
    /// use pltrs::figure::{Figure, take_changed_figures};
    /// let fig = Figure::new();
    /// fig.set_dpi(50.0);
    /// assert!(take_changed_figures().is_empty(), "not watched");
    /// fig.watch_changes();
    /// fig.size_inches();
    /// assert!(take_changed_figures().is_empty(), "only read");
    /// fig.set_dpi(60.0);
    /// fig.set_dpi(70.0);
    /// assert_eq!(take_changed_figures(), [fig.clone()], "once");
    /// assert!(take_changed_figures().is_empty());
    /// ```
    pub fn watch_changes(&self) {
        let cell = self.state_cell();
        *cell.watched.borrow_mut() = Some(WeakFigure {
            state: Rc::downgrade(cell),
            frame: self.state().root,
        });
    }
}

/// The watched figures ([`Figure::watch_changes`]) of this thread that a
/// handle borrowed for change since they were last taken, each once; those
/// dropped meanwhile are left out. Upgrading them does not touch their
/// state, so this works while a figure is being drawn.
pub fn take_changed_figures() -> Vec<Figure> {
    let changed = CHANGED.with(|changed| std::mem::take(&mut *changed.borrow_mut()));
    changed
        .iter()
        .filter_map(WeakFigure::upgrade)
        .inspect(|figure| figure.state_cell().queued.set(false))
        .collect()
}

/// A handle of a figure (or subfigure) that does not keep it alive (from
/// [`Figure::downgrade`]): code following a figure from outside, such as
/// a registry of the figures a program watches, holds it.
#[derive(Clone, Debug)]
pub struct WeakFigure {
    pub(super) state: Weak<FigureCell>,
    /// The frame of the (sub)figure the handle was of.
    pub(super) frame: ArtistId,
}

impl WeakFigure {
    /// The handle it was made from, while the figure lives; reaching it
    /// does not touch the figure's state, so it works while the figure is
    /// being drawn.
    pub fn upgrade(&self) -> Option<Figure> {
        let state = self.state.upgrade()?;
        let root = RawHandle::new(Rc::clone(&state), self.frame);
        Some(Figure { state, root })
    }

    /// Whether the figure still lives.
    pub fn is_alive(&self) -> bool {
        self.state.strong_count() > 0
    }

    /// Whether this is a handle of `figure` (of any (sub)figure of its
    /// tree), without upgrading it.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let (fig, other) = (Figure::new(), Figure::new());
    /// let weak = fig.downgrade();
    /// assert!(weak.is_of(&fig) && !weak.is_of(&other) && weak.is_alive());
    /// drop(fig);
    /// assert!(!weak.is_alive());
    /// ```
    pub fn is_of(&self, figure: &Figure) -> bool {
        std::ptr::eq(self.state.as_ptr(), Rc::as_ptr(figure.state_cell()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;

    #[test]
    fn only_borrows_for_change_of_watched_figures_are_recorded() {
        let fig = Figure::new();
        let other = Figure::new();
        fig.watch_changes();
        drop(take_changed_figures());
        drop(fig.state_cell().borrow());
        drop(fig.state_cell().try_borrow());
        drop(other.state_cell().borrow_mut());
        assert_eq!(take_changed_figures().len(), 0);
        let held = fig.state_cell().borrow();
        assert!(fig.state_cell().try_borrow_mut().is_err());
        drop(held);
        assert_eq!(
            take_changed_figures().len(),
            0,
            "a refused borrow changes nothing"
        );
        drop(fig.state_cell().try_borrow_mut());
        assert_eq!(take_changed_figures(), std::slice::from_ref(&fig));
        drop(fig.state_cell().borrow_mut());
        drop(fig);
        assert_eq!(take_changed_figures().len(), 0, "dropped");
    }

    #[test]
    fn weak_handles_upgrade_while_the_figure_is_borrowed() {
        let fig = Figure::new();
        let weak = fig.downgrade();
        let held = fig.state_mut();
        let upgraded = weak.upgrade().expect("alive");
        assert_eq!(upgraded, fig);
        drop(held);
        assert_eq!(upgraded.id(), fig.id());
    }
}
