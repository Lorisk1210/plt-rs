//! Geometry, paths, affine transforms and colors for all pltrs crates.
//!
//! This crate holds the base types of pltrs. They follow the matplotlib
//! types `transforms.Affine2D`, `transforms.Bbox` and `path.Path`, so the
//! higher crates can use the matplotlib algorithms unchanged:
//!
//! * [`Point`]: a 2-D point or vector in `f64`.
//! * [`Affine2D`]: an affine map that composes in the matplotlib "append"
//!   order. `Affine2D::identity().scale(2.0, 2.0).rotate_deg(90.0)` scales
//!   first, then rotates.
//! * [`Bbox`]: an axis-aligned box that can be inverted (`x0` can be
//!   larger than `x1`).
//! * [`Path`]: vertices and optional [`PathCode`]s, iterated as
//!   [`Segment`]s. [`path`] has the matplotlib unit shapes and the queries
//!   for containment, intersection, clipping and extents.
//!   [`path::convert`] is the conditioning pipeline that each backend runs
//!   before it draws: NaN removal, clipping, snapping, simplification,
//!   curve flattening and sketching.
//! * [`alloc`]: allocations that return an error
//!   ([`alloc::OutOfMemory`], the matplotlib `MemoryError`) and do not end
//!   the process.
//! * [`archive`]: values written to bytes and read back exactly. Pickling
//!   of figures uses it.
//! * [`bezier`]: evaluation, subdivision and extrema of Bézier curves.
//! * `fuzz` (with the feature `fuzzing`): the harness of the fuzz tests of
//!   the workspace.
//! * [`hatch`]: the matplotlib hatch patterns.
//! * [`interned`]: strings made at run time and kept as `&'static str`.
//! * [`json`]: JSON values, written back and read in two ways. The lenient
//!   reader accepts the output of the Python `json` module (the spec
//!   goldens). The strict reader follows RFC 8259 (untrusted messages).
//! * [`numeric`]: the float behavior of Python and numpy that matplotlib
//!   results depend on (`divmod`, `%`, `min`/`max`, `minimum`/`maximum`,
//!   `sign`, `linspace`). It computes as the binaries that pltrs copies.
//!   [`numeric::CpuLevel`] selects the numpy loops of a processor level.
//!   [`numeric::Contraction`] gives the products that their compiler
//!   fuses. [`numeric::Ufunc`] gives the numpy ufuncs where the last bit
//!   decides a discrete result.
//! * [`os`]: the conventions that differ between Linux, macOS and Windows
//!   (programs on `PATH`, resolved paths). They are functions of the
//!   system, so tests on one system can check all of them.
//! * [`pyfmt`]: Python float formatting (`repr`, `%`-formatting and the
//!   `str.format` mini-language), which matplotlib uses for labels and
//!   documents.
//! * [`rng`]: the `SplitMix64` generator for the reproducible
//!   pseudo-random streams of pltrs.
//! * [`Rgba`]: a straight-alpha color with components in `[0, 1]`.
//! * [`stack`]: stack space for deep work. If the stack of the caller is
//!   short, the work runs on its own thread.
//! * [`state`]: the state that matplotlib keeps in process-wide variables.
//!   pltrs keeps it per thread ([`thread_state!`]), unless threads share
//!   one [`state::State`].
#![forbid(unsafe_code)]

mod affine;
pub mod alloc;
pub mod archive;
mod bbox;
pub mod bezier;
mod color;
#[cfg(feature = "fuzzing")]
pub mod fuzz;
pub mod hatch;
mod intern;
pub mod json;
pub mod numeric;
pub mod os;
pub mod path;
mod point;
pub mod pyfmt;
pub mod rng;
pub mod stack;
pub mod state;

pub use affine::Affine2D;
pub use bbox::{Anchor, Bbox};
pub use color::Rgba;
pub use intern::interned;
pub use path::{Path, PathBuilder, PathCode, PathError, Segment};
pub use point::Point;
