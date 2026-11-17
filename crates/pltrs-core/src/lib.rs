//! Geometry, paths, affine transforms and colours shared by every pltrs crate.
//!
//! This crate is the vocabulary of pltrs. It deliberately mirrors the
//! semantics of matplotlib's `transforms.Affine2D`, `transforms.Bbox` and
//! `path.Path` so that higher layers can follow matplotlib's algorithms
//! faithfully:
//!
//! * [`Point`] — a 2-D point / vector in `f64`.
//! * [`Affine2D`] — an affine map with matplotlib's "append" composition
//!   order (`Affine2D::identity().scale(2.0, 2.0).rotate_deg(90.0)` scales
//!   first, then rotates).
//! * [`Bbox`] — a possibly-inverted axis-aligned box (`x0` may exceed `x1`).
//! * [`Path`] — vertices plus optional [`PathCode`]s, iterated as
//!   [`Segment`]s; see [`path`] for matplotlib's unit shapes, containment,
//!   intersection, clipping and extents queries, and [`path::convert`] for
//!   the conditioning pipeline every backend runs before drawing
//!   (NaN removal, clipping, snapping, simplification, curve flattening,
//!   sketching).
//! * [`alloc`] — allocations that fail as errors ([`alloc::OutOfMemory`],
//!   matplotlib's `MemoryError`) instead of ending the process.
//! * [`archive`] — values written to bytes and read back exactly (what
//!   pickling a figure rests on).
//! * [`bezier`] — evaluation, subdivision and extrema of Bézier curves.
//! * `fuzz` (with the feature `fuzzing`) — the harness of the workspace's
//!   robustness tests.
//! * [`hatch`] — matplotlib's hatch patterns.
//! * [`interned`] — run-time strings kept as `&'static str`.
//! * [`json`] — JSON values, read as Python's `json` module writes them
//!   (the spec's goldens) or strictly by RFC 8259 (untrusted messages),
//!   and written back.
//! * [`numeric`] — Python's and numpy's float semantics (`divmod`, `%`,
//!   `min`/`max`, `minimum`/`maximum`, `sign`, `linspace`) that
//!   matplotlib's results depend on, as the binaries pltrs reproduces
//!   compute them: the copy of numpy's loops a processor runs
//!   ([`numeric::CpuLevel`]), the products their compiler fuses
//!   ([`numeric::Contraction`]), and numpy's own ufuncs where a last bit
//!   decides something discrete ([`numeric::Ufunc`]).
//! * [`os`] — the conventions that differ between Linux, macOS and
//!   Windows (programs on `PATH`, resolved paths), as functions of the
//!   system so that each is tested on any.
//! * [`pyfmt`] — Python's float formatting (`repr`, `%`-formatting and the
//!   `str.format` mini-language), which matplotlib's labels and documents
//!   are written with.
//! * [`rng`] — the `SplitMix64` generator behind pltrs's reproducible
//!   pseudo-random streams.
//! * [`Rgba`] — a straight-alpha colour with components in `[0, 1]`.
//! * [`stack`] — room on the thread's stack for work that goes deep: on a
//!   thread of its own where the caller's stack is short.
//! * [`state`] — the state pltrs keeps where matplotlib keeps
//!   process-wide variables: per thread ([`thread_state!`]), unless
//!   threads work with one [`state::State`] together.
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
