//! A figure's state worked on outside its cell, and lent back to the
//! program's code the work runs.
//!
//! Laying a figure out runs the program's code: a tick formatter or
//! locator of a host language, which matplotlib lets read and change the
//! figure while it is laid out (a formatter labelling ticks with the
//! axes' limits, or setting a title). A layout borrowing the state from
//! its cell would leave that code a figure in use. So the layout takes
//! the state out of the cell ([`FigureCell::work_on`]) and works on it as
//! a value; where it calls the program, [`FigureState::lend`] puts the
//! state, as the layout left it, back into the cell for the length of the
//! call, and takes it out again once the call returns, with whatever the
//! program changed. The program finds the figure free and consistent,
//! changes it at once, and the rest of the layout sees the changes, as in
//! matplotlib.
//!
//! While the state is out, the cell holds an empty placeholder, which
//! every handle finds in use (as a borrowed state: see
//! [`in_use::reached`](super::in_use)). While it is lent, only the thread
//! working on it finds it free: another thread's code (a host's thread
//! switching in the middle of the call) finds it in use, as it would find
//! it borrowed.

use std::cell::RefCell;
use std::rc::Rc;
use std::thread::{self, ThreadId};

use super::cell::FigureCell;
use super::state::FigureState;

/// Where a figure's state is, for its handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lending {
    /// In its cell, free to every thread (but for the cell's own borrow).
    Home,
    /// Worked on outside its cell, which holds a placeholder.
    Away,
    /// Back in its cell for a call of the program's code made by the work
    /// on it, on the thread named, the only one that finds it free.
    Lent(ThreadId),
}

impl Lending {
    /// Whether the state can be borrowed from the cell by the current
    /// thread.
    pub(crate) fn reachable(self) -> bool {
        match self {
            Self::Home => true,
            Self::Away => false,
            Self::Lent(owner) => owner == current_thread(),
        }
    }
}

/// The id of the current thread, read from the thread's own storage (a
/// draw asks it at every borrow of the figure it paints).
pub(super) fn current_thread() -> ThreadId {
    thread_local! {
        static ID: ThreadId = thread::current().id();
    }
    ID.with(|id| *id)
}

thread_local! {
    /// The states this thread works on outside their cells, innermost
    /// last: the cell of each and the address of the state worked on.
    static WORKED_ON: RefCell<Vec<(Rc<FigureCell>, usize)>> = const { RefCell::new(Vec::new()) };
}

/// The address of `state`, which tells the state a work runs on.
fn address(state: &FigureState) -> usize {
    std::ptr::from_ref(state).addr()
}

/// A state taken out of its cell, put back when dropped (also when the
/// work on it unwinds).
struct Away {
    cell: Rc<FigureCell>,
    state: FigureState,
    /// Where the state was before it was taken out: home, or lent to the
    /// program's code a work further up the stack runs.
    before: Lending,
}

impl Drop for Away {
    fn drop(&mut self) {
        WORKED_ON.with_borrow_mut(|worked_on| {
            let ours = address(&self.state);
            if let Some(index) = worked_on.iter().rposition(|&(_, at)| at == ours) {
                worked_on.remove(index);
            }
        });
        self.cell.swap_state(&mut self.state);
        self.cell.set_lending(self.before);
    }
}

impl FigureCell {
    /// Runs `work` on the state taken out of the cell, so that the
    /// program's code `work` calls through [`FigureState::lend`] reaches
    /// the figure free; the state is back in the cell when this returns.
    /// A state in use is reached as a borrow for change would reach it.
    pub(crate) fn work_on<R>(self: &Rc<Self>, work: impl FnOnce(&mut FigureState) -> R) -> R {
        let state = std::mem::replace(&mut *self.borrow_mut(), FigureState::placeholder());
        let mut away = Away {
            cell: Rc::clone(self),
            state,
            before: self.lending(),
        };
        self.set_lending(Lending::Away);
        WORKED_ON.with_borrow_mut(|worked_on| {
            worked_on.push((Rc::clone(self), address(&away.state)));
        });
        work(&mut away.state)
    }
}

impl FigureCell {
    /// Write access to the state brought up to date
    /// ([`FigureState::prepare`]) by a layout worked on outside the cell,
    /// so that the program's code it calls finds the figure lent.
    pub(crate) fn prepared(self: &Rc<Self>) -> std::cell::RefMut<'_, FigureState> {
        self.work_on(FigureState::prepare);
        self.borrow_mut()
    }

    /// Write access to the state as it is between draws
    /// ([`FigureState::prepare_as_is`]: no layout engine, no aspect
    /// applied), for what matplotlib reads without laying the figure out:
    /// whether a point hits an artist (`contains`, picking).
    pub(crate) fn as_is(self: &Rc<Self>) -> std::cell::RefMut<'_, FigureState> {
        self.work_on(FigureState::prepare_as_is);
        self.borrow_mut()
    }
}

/// The state lent back to its cell, taken out again when dropped.
struct Lent<'a> {
    cell: Rc<FigureCell>,
    state: &'a mut FigureState,
}

impl Drop for Lent<'_> {
    fn drop(&mut self) {
        self.cell.set_lending(Lending::Away);
        self.cell.swap_state(self.state);
    }
}

impl FigureState {
    /// Runs `program`, code of the program's that the work on this state
    /// calls, with the state back in its cell as the work left it, so that
    /// the program reaches the figure free (from this thread) and changes
    /// it at once; the work goes on with the state the program left.
    /// `program` is handed the cell, for handles of the figure. A state not
    /// worked on outside its cell (borrowed from it) cannot be lent:
    /// `None`, `program` not run.
    pub(crate) fn lend<R>(&mut self, program: impl FnOnce(&Rc<FigureCell>) -> R) -> Option<R> {
        let cell = worked_on_cell(self)?;
        cell.swap_state(self);
        cell.set_lending(Lending::Lent(current_thread()));
        let lent = Lent {
            cell: Rc::clone(&cell),
            state: self,
        };
        let outcome = program(&cell);
        drop(lent);
        Some(outcome)
    }

    /// Whether this state is worked on outside its cell, so that
    /// [`lend`](Self::lend) can lend it to the program's code.
    pub(crate) fn lendable(&self) -> bool {
        worked_on_cell(self).is_some()
    }
}

/// The cell of `state` when this thread works on it outside the cell.
fn worked_on_cell(state: &FigureState) -> Option<Rc<FigureCell>> {
    let ours = address(state);
    WORKED_ON.with_borrow(|worked_on| {
        worked_on
            .iter()
            .rev()
            .find(|&&(_, at)| at == ours)
            .map(|(cell, _)| Rc::clone(cell))
    })
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::{Access, Figure, FigureInUse, reenter};

    #[test]
    fn work_finds_the_figure_in_use_and_lends_it_to_the_program() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let cell = std::rc::Rc::clone(fig.state_cell());
        let (refused, seen) = cell.work_on(|state| {
            let refused = reenter(|| ax.set_visible(false)).map(drop);
            state.stale = true;
            let seen = state.lend(|_| {
                // The program sees what the work did, and changes the
                // figure at once.
                let seen = fig.is_stale();
                ax.set_title("lent");
                seen
            });
            assert_eq!(
                state.lend(|_| ax.title(crate::axes::TitleLocation::Center)),
                Some("lent".to_owned()),
                "the work goes on with the program's change"
            );
            (refused, seen)
        });
        assert_eq!(
            refused.map_err(FigureInUse::access),
            Err(Access::Change),
            "a placeholder stands in the cell"
        );
        assert_eq!(seen, Some(true));
        assert_eq!(ax.title(crate::axes::TitleLocation::Center), "lent");
        assert!(!fig.raw().figure_in_use());
    }

    #[test]
    fn work_nests_and_borrowed_states_are_not_lent() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let cell = std::rc::Rc::clone(fig.state_cell());
        cell.work_on(|state| {
            assert!(state.lendable());
            state.lend(|_| {
                // The program lays the figure out again, inside.
                fig.layout();
                ax.set_xlabel("inner");
            });
        });
        assert_eq!(ax.xaxis().label_text(), "inner");
        let mut borrowed = fig.state_mut();
        assert!(!borrowed.lendable());
        assert!(borrowed.lend(|_| ()).is_none(), "not lent");
        drop(borrowed);
        assert_eq!(
            reenter(|| ax.xaxis().label_text()).map_err(FigureInUse::access),
            Ok("inner".to_owned())
        );
    }

    #[test]
    fn a_lent_state_is_in_use_for_other_threads_only() {
        let fig = Figure::new();
        let cell = std::rc::Rc::clone(fig.state_cell());
        cell.work_on(|state| {
            state.lend(|cell| {
                assert!(cell.lending().reachable());
                assert!(fig.raw().figure_in_use(), "its layout is not done");
            });
        });
        let other = std::thread::spawn(|| super::Lending::Lent(std::thread::current().id()))
            .join()
            .expect("the thread runs");
        assert!(!other.reachable());
    }

    #[test]
    fn an_unwinding_work_puts_the_state_back() {
        let fig = Figure::new();
        fig.subplot().set_title("kept");
        let cell = std::rc::Rc::clone(fig.state_cell());
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cell.work_on(|_| panic!("a bug"));
        }));
        assert!(unwound.is_err());
        assert_eq!(
            fig.axes()[0].title(crate::axes::TitleLocation::Center),
            "kept"
        );
        assert!(!fig.raw().figure_in_use());
    }
}
