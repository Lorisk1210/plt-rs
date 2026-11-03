//! Patch styles of the program's: the box, connection and arrow styles a
//! program defines (matplotlib's subclasses of `BoxStyle`,
//! `ConnectionStyle._Base` and `ArrowStyle._Base`, registered with
//! `register` or given as objects, and the callables given as styles),
//! which pltrs calls wherever it draws with a style
//! ([`BoxShape::Program`](super::BoxShape::Program),
//! [`ConnectionStyle::Program`](super::ConnectionStyle::Program),
//! [`ArrowStyle::Program`](super::ArrowStyle::Program)).
//!
//! A style is a function of the program's ([`ProgramFn`]): one made with
//! an origin is archived as it.

use std::fmt;
use std::rc::Rc;

use pltrs_core::archive::{External, ProgramFn};
use pltrs_core::{Path, Point};

/// A box style of the program's: the closed path around the rectangle
/// `x0, y0, width, height` at mutation size `mutation_size` (matplotlib's
/// `BoxStyle.__call__`), its padding its own.
pub type BoxFn = dyn Fn(f64, f64, f64, f64, f64) -> Path;

/// A connection of the program's between two display points (matplotlib's
/// `ConnectionStyle._Base.connect`), which pltrs then clips to the
/// patches at its ends and shrinks as matplotlib's `_Base.__call__` does.
pub type ConnectFn = dyn Fn(Point, Point) -> Path;

/// An arrow of the program's along a path in display pixels, at
/// mutation size, line width (pixels) and mutation aspect: its pieces,
/// each with whether it is filled (matplotlib's `ArrowStyle.__call__`).
pub type ArrowFn = dyn Fn(&Path, f64, f64, f64) -> Vec<(Path, bool)>;

/// Implements what the three styles share: construction from a closure,
/// with or without an origin, equality by identity, archiving through
/// the origin.
macro_rules! program_style {
    ($(#[$doc:meta])* $name:ident($function:ident)) => {
        $(#[$doc])*
        #[derive(Clone)]
        pub struct $name(ProgramFn<$function>);

        pltrs_core::archive_struct!($name(v0));

        impl $name {
            /// The style `function`, which cannot be archived.
            pub fn new(function: Rc<$function>) -> Self {
                Self(ProgramFn::new(function))
            }

            /// The style `function`, archived as `origin`, what the program
            /// made it from.
            pub fn with_origin(function: Rc<$function>, origin: External) -> Self {
                Self(ProgramFn::with_origin(function, origin))
            }
        }

        impl PartialEq for $name {
            /// Styles are equal when they are the same function.
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(self.0.shared(), other.0.shared())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(..)"))
            }
        }
    };
}

program_style!(
    /// A box style of the program's (see [`BoxFn`]).
    ///
    /// ```
    /// use std::rc::Rc;
    /// use pltrs::patches::{BoxShape, BoxStyle, ProgramBox};
    /// use pltrs::path::Path;
    /// use pltrs::transforms::Point;
    /// let diamond = ProgramBox::new(Rc::new(|x0, y0, width, height, size| {
    ///     let (cx, cy) = (x0 + width / 2.0, y0 + height / 2.0);
    ///     let (rx, ry) = (width / 2.0 + size, height / 2.0 + size);
    ///     Path::polygon([(cx - rx, cy), (cx, cy - ry), (cx + rx, cy), (cx, cy + ry)].map(Point::from))
    /// }));
    /// let style = BoxStyle::new(BoxShape::Program(diamond));
    /// assert_eq!(style.path(0.0, 0.0, 4.0, 2.0, 1.0).extents().x1(), 5.0);
    /// ```
    ProgramBox(BoxFn)
);

program_style!(
    /// A connection style of the program's (see [`ConnectFn`]).
    ProgramConnection(ConnectFn)
);

program_style!(
    /// An arrow style of the program's (see [`ArrowFn`]).
    ProgramArrow(ArrowFn)
);

impl ProgramBox {
    /// The path around the rectangle at `(x0, y0)` of `width × height` at
    /// mutation size `mutation_size`.
    pub(crate) fn path(
        &self,
        x0: f64,
        y0: f64,
        width: f64,
        height: f64,
        mutation_size: f64,
    ) -> Path {
        (self.0.get())(x0, y0, width, height, mutation_size)
    }
}

impl ProgramConnection {
    /// The connection from `a` to `b`.
    pub(crate) fn path(&self, a: Point, b: Point) -> Path {
        (self.0.get())(a, b)
    }
}

impl ProgramArrow {
    /// The pieces of the arrow along `path`.
    pub(crate) fn pieces(
        &self,
        path: &Path,
        mutation_size: f64,
        linewidth: f64,
        aspect: f64,
    ) -> Vec<(Path, bool)> {
        (self.0.get())(path, mutation_size, linewidth, aspect)
    }
}
