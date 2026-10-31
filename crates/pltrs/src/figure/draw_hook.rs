//! The program's work at every draw, once the figure is laid out and
//! before its artists are drawn ([`Figure::set_draw_hook`]).

use std::fmt;
use std::rc::Rc;

use super::Figure;

/// A function run whenever a figure is drawn, after its layout and before
/// its artists (see [`Figure::set_draw_hook`]).
#[derive(Clone)]
pub struct DrawHook(Rc<dyn Fn(&Figure)>);

impl DrawHook {
    /// The hook running `hook`.
    pub fn new(hook: impl Fn(&Figure) + 'static) -> Self {
        Self(Rc::new(hook))
    }
}

impl fmt::Debug for DrawHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DrawHook(..)")
    }
}

impl Figure {
    /// Runs `hook` whenever the figure is drawn, once it is laid out and
    /// before its artists are drawn; `None` removes the hook. This is
    /// where a program brings into its artists what it computes from the
    /// laid-out figure, as matplotlib's artists do in their `draw` (pltrs's
    /// Python package evaluates there the transforms that Python subclasses
    /// compute in `get_transform`). The hook may change the figure's
    /// artists; it lives only while the figure does, and is not archived.
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    /// use pltrs::figure::{DrawHook, Figure};
    /// let fig = Figure::new();
    /// let runs = Rc::new(Cell::new(0));
    /// let counted = Rc::clone(&runs);
    /// fig.set_draw_hook(Some(DrawHook::new(move |_| counted.set(counted.get() + 1))));
    /// fig.canvas().draw()?;
    /// assert_eq!(runs.get(), 1);
    /// # Ok::<(), pltrs::figure::CanvasError>(())
    /// ```
    pub fn set_draw_hook(&self, hook: Option<DrawHook>) {
        self.state_mut().draw_hook = hook;
    }

    /// Runs the draw hook, if any, with no borrow of the figure held.
    pub(crate) fn run_draw_hook(&self) {
        let hook = self.state().draw_hook.clone();
        if let Some(DrawHook(hook)) = hook {
            hook(self);
        }
    }
}
