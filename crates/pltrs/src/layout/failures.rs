//! The failures of the program's code that a layout engine's pass runs: a
//! host language's exceptions, which its binding keeps until pltrs
//! returns to it.
//!
//! matplotlib's engines are Python. An exception the program's code
//! raises while they measure the figure (a formatter's, as they measure
//! the tick labels) ends their pass there, before it places anything, and
//! `Figure.draw` passes over the `ValueError`s its engine raises (resizing
//! a window to nothing raises them), drawing the figure as the pass left
//! it. pltrs asks the thread's [`Failures`] (the threads sharing a
//! [state](pltrs_core::state)), which a program installs with
//! [`set_failures`] (pltrs (Python) reads the exceptions its callbacks
//! raised), once it measured the figure; without one the program's code
//! never fails.
//!
//! ```
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use pltrs::layout::{LayoutEngine, failures};
//! use pltrs::prelude::*;
//! use pltrs::ticker::FuncFormatter;
//!
//! /// Whether the program's code raised an exception not raised yet.
//! static RAISED: AtomicBool = AtomicBool::new(false);
//! /// Whether the program's formatter is still to fail.
//! static TO_FAIL: AtomicBool = AtomicBool::new(true);
//!
//! /// A program whose failing code raises a `ValueError`.
//! struct Program;
//! impl failures::Failures for Program {
//!     fn failed(&self) -> bool {
//!         RAISED.load(Ordering::Relaxed)
//!     }
//!     fn pass_over(&self, class: failures::ErrorClass) -> bool {
//!         class == failures::ErrorClass::ValueError && RAISED.swap(false, Ordering::Relaxed)
//!     }
//! }
//!
//! failures::set_failures(Program);
//! let fig = Figure::new();
//! fig.set_layout_engine(LayoutEngine::constrained())?;
//! let ax = fig.subplot();
//! // The formatter fails the first time it labels the ticks: in the
//! // layout engine's pass of the first draw.
//! ax.xaxis().set_major_formatter(FuncFormatter::new(|x, _| {
//!     if TO_FAIL.swap(false, Ordering::Relaxed) {
//!         RAISED.store(true, Ordering::Relaxed);
//!     }
//!     format!("{x}")
//! }));
//! let before = ax.position();
//! fig.canvas().draw()?;
//! assert_eq!(ax.position(), before, "drawn without the layout");
//! assert!(!RAISED.load(Ordering::Relaxed), "the draw passed over the failure");
//! fig.canvas().draw()?;
//! assert_ne!(ax.position(), before, "laid out once nothing fails");
//! failures::clear_failures();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::cell::RefCell;
use std::rc::Rc;

/// The classes of the program's errors matplotlib's code passes over, as
/// Python names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorClass {
    /// `ValueError`: `Figure.draw` ignores those of its layout engine, and
    /// draws without the rest of the layout.
    ValueError,
    /// `TypeError`: measuring an axis for a layout engine
    /// (`_get_tightbbox_for_layout_only`) takes one for a `get_tightbbox`
    /// without `for_layout_only`, and measures the axis again without it.
    TypeError,
}

/// What the program tells pltrs of its code's failures.
pub trait Failures {
    /// Whether the program's code failed in the work pltrs does now (its
    /// error waits to be raised once that work is done).
    fn failed(&self) -> bool;

    /// Forgets the failure the program's code met if its error is of the
    /// class `class` (or of a subclass), which matplotlib's code passes
    /// over there, returning whether it did.
    fn pass_over(&self, class: ErrorClass) -> bool;
}

pltrs_core::thread_state! {
    /// The thread's program; `None` never fails.
    static FAILURES: RefCell<Option<Rc<dyn Failures>>> = const { RefCell::new(None) };
}

/// Asks `failures` of the failures of the program's code from now on.
pub fn set_failures(failures: impl Failures + 'static) {
    FAILURES.set(Some(Rc::new(failures)));
}

/// Takes the program's code as never failing again.
pub fn clear_failures() {
    FAILURES.set(None);
}

/// The thread's program, on a copy, so that its answer may install
/// another or ask again without borrowing the slot twice.
fn program() -> Option<Rc<dyn Failures>> {
    FAILURES.with(|slot| slot.borrow().clone())
}

/// Whether the program's code failed in the work done now, which a layout
/// engine's pass stops at.
pub(crate) fn failed() -> bool {
    program().is_some_and(|program| program.failed())
}

/// Forgets a failure of the program's code of the class `class`, which
/// matplotlib passes over where pltrs asks (see [`Failures::pass_over`]),
/// returning whether there was one.
pub(crate) fn pass_over(class: ErrorClass) -> bool {
    program().is_some_and(|program| program.pass_over(class))
}
