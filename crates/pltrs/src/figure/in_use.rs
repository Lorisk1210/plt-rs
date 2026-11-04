//! Figures reached again while code further up the call stack uses them.
//!
//! Laying a figure out or drawing it runs the program's code: a tick
//! formatter or locator, a transform, a norm, a path effect. That code may
//! reach the figure again through a handle, which finds it borrowed: for
//! change while it is laid out, for reading while it is drawn. A Rust
//! program doing so has a bug, and the handle panics. A host running
//! another language's code (pltrs (Python), whose programs do this as
//! matplotlib's do) runs that code's calls with [`reenter`], which turns
//! reaching a figure in use into an error the host can act on: apply the
//! change once the figure is free, or report it to the program.

use std::cell::Cell;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

thread_local! {
    /// Whether the code running on this thread runs inside [`reenter`].
    static REENTERING: Cell<bool> = const { Cell::new(false) };
}

/// How a handle reached a figure in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// To read it, while code further up changes it (lays it out, say).
    Read,
    /// To change it, while code further up reads or changes it (draws it,
    /// lays it out).
    Change,
}

/// A figure reached through a handle while code further up the call
/// stack uses it (see [`reenter`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FigureInUse {
    access: Access,
}

impl FigureInUse {
    /// How the handle reached the figure.
    pub fn access(self) -> Access {
        self.access
    }
}

impl fmt::Display for FigureInUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.access {
            Access::Read => f.write_str(
                "the figure cannot be read while it is being changed (laid out, say) \
                 by code that called this code",
            ),
            Access::Change => f.write_str(
                "the figure cannot be changed while it is being drawn or laid out \
                 by code that called this code",
            ),
        }
    }
}

impl std::error::Error for FigureInUse {}

/// Runs `f`, code that the layout or the drawing of a figure runs (a
/// program's callback calling back into pltrs), returning
/// [`FigureInUse`] if `f` reaches, through a handle, a figure that code
/// further up the call stack uses, instead of panicking: `f` stops there,
/// what it did until then stays done (a change to another figure, say),
/// and the figure in use is left as it was.
///
/// Calls nest: an inner call reports what its own `f` reached.
///
/// ```
/// use pltrs::figure::reenter;
/// use pltrs::prelude::*;
///
/// let ax = Figure::new().subplot();
/// // Nothing uses the figure here: `f` runs as it would without.
/// let title = reenter(|| ax.set_title("free"))?;
/// assert_eq!(title.text(), "free");
/// # Ok::<(), pltrs::figure::FigureInUse>(())
/// ```
///
/// # Errors
///
/// [`FigureInUse`] when `f` reaches a figure in use.
pub fn reenter<R>(f: impl FnOnce() -> R) -> Result<R, FigureInUse> {
    /// Restores the flag of the code calling [`reenter`], however `f` ends.
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            REENTERING.set(self.0);
        }
    }
    let _restore = Restore(REENTERING.replace(true));
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => Ok(value),
        Err(payload) => match payload.downcast::<FigureInUse>() {
            Ok(in_use) => Err(*in_use),
            Err(panic) => resume_unwind(panic),
        },
    }
}

/// What a handle does on reaching a figure in use: inside [`reenter`],
/// unwinds to it (without the panic hook: this is no bug of pltrs); else
/// panics, as a `RefCell` borrowed twice does.
pub(crate) fn reached(access: Access) -> ! {
    let in_use = FigureInUse { access };
    if REENTERING.get() {
        resume_unwind(Box::new(in_use))
    }
    panic!("{in_use}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::TitleLocation;
    use crate::figure::Figure;

    #[test]
    fn a_figure_in_use_is_reported_with_the_access_tried() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("plots");
        let reading = fig.state();
        assert_eq!(
            reenter(|| line.set_visible(false)).map(drop),
            Err(FigureInUse {
                access: Access::Change
            })
        );
        assert_eq!(
            reenter(|| line.visible()),
            Ok(true),
            "reads share the borrow"
        );
        drop(reading);
        let changing = fig.state_mut();
        let read = reenter(|| line.visible());
        drop(changing);
        assert_eq!(read.map_err(FigureInUse::access), Err(Access::Read));
        assert_eq!(reenter(|| line.set_visible(false)).map(drop), Ok(()));
        assert!(!line.visible());
    }

    #[test]
    fn other_panics_pass_through_and_calls_nest() {
        let caught = catch_unwind(|| reenter(|| panic!("a bug")));
        assert!(caught.is_err());
        assert!(!REENTERING.get(), "restored when unwinding");
        let fig = Figure::new();
        let ax = fig.subplot();
        let held = fig.state_mut();
        let outer = reenter(|| {
            let inner = reenter(|| ax.title(TitleLocation::Center));
            assert_eq!(
                inner.map(drop).map_err(FigureInUse::access),
                Err(Access::Read)
            );
            ax.title(TitleLocation::Center)
        });
        drop(held);
        assert_eq!(
            outer.map(drop).map_err(FigureInUse::access),
            Err(Access::Read)
        );
    }

    #[test]
    #[should_panic(expected = "the figure cannot be changed")]
    fn outside_reenter_a_figure_in_use_panics() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let _reading = fig.state();
        ax.set_title("now");
    }
}
