//! Matplotlib, reimagined in Rust: figures, axes, artists and a pyplot-style API.
//!
//! plt-rs follows matplotlib's architecture and semantics closely so that
//! anything you can draw with matplotlib can be drawn the same way here.
//! The crate is organised like matplotlib's own modules:
//!
//! * [`numeric`] — numpy-style ranges and the [`numeric::Grid`] 2-D array.
//! * [`colors`] — colour specifications, colormaps and normalisation.
//! * [`ticker`] / [`scale`] / [`dates`] — tick placement, formatting and
//!   axis scales.
//! * [`stats`] / [`mlab`] — statistics and spectral analysis used by
//!   `boxplot`, `violinplot`, `hist`, `psd`, `specgram`, ...
//! * [`contour`] / [`tri`] — contour generation and unstructured triangular
//!   grids.
#![forbid(unsafe_code)]

pub mod artist;
pub mod axes;
pub mod colors;
pub mod contour;
pub mod dates;
pub mod figure;
pub mod markers;
pub mod mlab;
pub mod numeric;
pub mod scale;
pub mod stats;
#[cfg(test)]
mod testing;
pub mod ticker;
pub mod transforms;
pub mod tri;
