//! Axes locators: what places an axes inside (part of) its original
//! position before its aspect is applied (matplotlib's `set_axes_locator`
//! and the colorbar's `_ColorbarAxesLocator`).

use std::fmt;
use std::rc::Rc;

use pltrs_core::Bbox;
use pltrs_core::archive::{External, ProgramFn};

use crate::axes::{Axes, AxesState};

/// Places an axes inside part of its original position before the aspect
/// is applied, like the axes locator matplotlib gives colorbars to make
/// room for their extensions (`_ColorbarAxesLocator`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AxesLocator {
    /// The factors the original width and height are scaled by.
    pub(crate) shrink: (f64, f64),
    /// The shift of the scaled box, in fractions of the original width
    /// and height.
    pub(crate) offset: (f64, f64),
    /// The box aspect to use instead of the axes' own.
    pub(crate) box_aspect: Option<f64>,
}

pltrs_core::archive_struct!(AxesLocator {
    shrink,
    offset,
    box_aspect
});

impl AxesLocator {
    /// The part of `original` used, computed exactly as matplotlib's
    /// `pos.shrunk(sx, sy).translated(ox * pos.width, oy * pos.height)`.
    pub(super) fn locate(&self, original: &Bbox) -> Bbox {
        let (x0, y0, width, height) = original.bounds();
        let (tx, ty) = (self.offset.0 * width, self.offset.1 * height);
        Bbox::from_extents(
            x0 + tx,
            y0 + ty,
            x0 + self.shrink.0 * width + tx,
            y0 + self.shrink.1 * height + ty,
        )
    }
}

/// What places an axes at layout time (matplotlib's axes locator, a
/// callable of the axes and the renderer): given the position the axes
/// was given, in fractions of its figure, it gives the position to place
/// the axes at before its aspect is applied.
///
/// A function runs inside the layout pass and gets what it needs as its
/// argument: it is `Send`, so it cannot use the figure's handles. Clones
/// share the function. A figure with one is archived only if the
/// function has an origin ([`AxesLocatorFn::with_origin`]). A locator
/// may also be a fixed position ([`AxesLocatorFn::at`]): where a program
/// that computes the position itself, before the layout, places the axes.
#[derive(Clone)]
pub struct AxesLocatorFn(Locate);

/// How an [`AxesLocatorFn`] places its axes.
#[derive(Clone)]
enum Locate {
    /// Where the function says.
    Function(ProgramFn<dyn Fn(Bbox) -> Bbox>),
    /// At this position, or at the original one for `None`.
    At(Option<Bbox>),
}

pltrs_core::archive_enum!(Locate {
    Function(v0),
    At(v0),
});

pltrs_core::archive_struct!(AxesLocatorFn(v0));

impl AxesLocatorFn {
    /// A locator calling `locate`.
    pub fn new(locate: impl Fn(Bbox) -> Bbox + Send + 'static) -> Self {
        Self(Locate::Function(ProgramFn::new(Rc::new(locate))))
    }

    /// A locator calling `locate`, archived as `origin`, what the program
    /// made it from (see [`ProgramFn`]).
    pub fn with_origin(locate: impl Fn(Bbox) -> Bbox + Send + 'static, origin: External) -> Self {
        Self(Locate::Function(ProgramFn::with_origin(
            Rc::new(locate),
            origin,
        )))
    }

    /// A locator placing its axes at `position`, or at its original
    /// position for `None` (what a locator of matplotlib's returning
    /// `None` asks for).
    ///
    /// ```
    /// use pltrs::axes::AxesLocatorFn;
    /// use pltrs::transforms::Bbox;
    /// let original = Bbox::from_bounds(0.1, 0.1, 0.8, 0.8);
    /// let placed = Bbox::from_bounds(0.5, 0.5, 0.2, 0.2);
    /// assert_eq!(AxesLocatorFn::at(Some(placed)).locate(original), placed);
    /// assert_eq!(AxesLocatorFn::at(None).locate(original), original);
    /// ```
    pub fn at(position: Option<Bbox>) -> Self {
        Self(Locate::At(position))
    }

    /// Where the locator places an axes given `original`.
    pub fn locate(&self, original: Bbox) -> Bbox {
        match &self.0 {
            Locate::Function(locate) => (locate.get())(original),
            Locate::At(position) => position.unwrap_or(original),
        }
    }
}

impl fmt::Debug for AxesLocatorFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Locate::Function(_) => f.debug_struct("AxesLocatorFn").finish_non_exhaustive(),
            Locate::At(position) => f.debug_tuple("AxesLocatorFn::at").field(position).finish(),
        }
    }
}

impl PartialEq for AxesLocatorFn {
    /// Functions are equal when they are shared, positions when they are
    /// the same.
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Locate::Function(a), Locate::Function(b)) => std::ptr::addr_eq(a.get(), b.get()),
            (Locate::At(a), Locate::At(b)) => a == b,
            _ => false,
        }
    }
}

impl Axes {
    /// Places the axes with `locator` at every layout, in place of its
    /// original position, before its aspect is applied; `None` removes it
    /// (matplotlib's `set_axes_locator`). An axes has one locator: this
    /// replaces the placement by a divider, an inset locator or a
    /// colorbar.
    ///
    /// ```
    /// use pltrs::axes::AxesLocatorFn;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Bbox;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// ax.set_axes_locator(Some(AxesLocatorFn::new(|original: Bbox| {
    ///     let (x0, y0, width, height) = original.bounds();
    ///     Bbox::from_bounds(x0, y0, width / 2.0, height)
    /// })));
    /// let (_, _, width, _) = ax.position().bounds();
    /// assert!((width - 0.4).abs() < 1e-12);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_axes_locator(&self, locator: Option<AxesLocatorFn>) -> &Self {
        self.raw.with_mut::<AxesState, _>(|axes| {
            axes.axes_locator = locator;
            axes.aspect.divider = None;
            axes.aspect.inset = None;
            axes.aspect.locator = None;
        });
        self.raw.figure_state().borrow_mut().layout.pending = true;
        self
    }

    /// The locator set with [`set_axes_locator`](Self::set_axes_locator),
    /// if any (matplotlib's `get_axes_locator`).
    pub fn axes_locator(&self) -> Option<AxesLocatorFn> {
        self.raw
            .with::<AxesState, _>(|axes| axes.axes_locator.clone())
    }
}
