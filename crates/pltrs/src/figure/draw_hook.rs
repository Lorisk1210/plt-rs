//! The program's work at every draw: before the figure is laid out
//! ([`Figure::set_layout_hook`]), and once it is laid out, before its
//! artists are drawn ([`Figure::set_draw_hook`]).

use std::fmt;
use std::rc::Rc;

use super::Figure;

/// A function run whenever a figure is drawn, before its layout (see
/// [`Figure::set_layout_hook`]) or after it, before its artists (see
/// [`Figure::set_draw_hook`]).
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
        self.run_hook(hook);
    }

    /// Runs `hook`, if any, on the figure.
    fn run_hook(&self, hook: Option<DrawHook>) {
        if let Some(DrawHook(hook)) = hook {
            hook(self);
        }
    }

    /// Runs `hook` whenever the figure is laid out to be drawn (drawn,
    /// saved, or laid out without drawing), before its layout; `None`
    /// removes the hook. This is where a program brings into its figure
    /// what the layout depends on and only the program computes, as
    /// matplotlib calls the axes locators of the program's when it draws
    /// (pltrs's Python package places there the axes such locators
    /// place). The hook may change the figure; it lives only while the
    /// figure does, and is not archived.
    ///
    /// ```
    /// use pltrs::figure::{DrawHook, Figure};
    /// use pltrs::transforms::Bbox;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let placed = ax.clone();
    /// fig.set_layout_hook(Some(DrawHook::new(move |_| {
    ///     placed.set_position([0.25, 0.25, 0.5, 0.5]);
    /// })));
    /// fig.layout();
    /// assert_eq!(ax.position(), Bbox::from_bounds(0.25, 0.25, 0.5, 0.5));
    /// ```
    pub fn set_layout_hook(&self, hook: Option<DrawHook>) {
        self.state_mut().layout_hook = hook;
    }

    /// Runs the layout hook, if any, with no borrow of the figure held.
    pub(crate) fn run_layout_hook(&self) {
        let hook = self.state().layout_hook.clone();
        self.run_hook(hook);
    }
}
