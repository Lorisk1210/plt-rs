//! Per-area operation tables binding golden op names to plt-rs.
//!
//! Each module exposes `OPS`; an empty table means the area is not
//! implemented yet and every probe in it reports as missing.

pub(crate) mod animation;
pub(crate) mod axes3d;
pub(crate) mod colormap_luts;
pub(crate) mod colormaps;
pub(crate) mod colors;
pub(crate) mod contour;
pub(crate) mod dates;
pub(crate) mod imsave;
pub(crate) mod lightsource;
pub(crate) mod markers;
pub(crate) mod norms;
pub(crate) mod path;
pub(crate) mod scales;
pub(crate) mod stats;
pub(crate) mod text_metrics;
pub(crate) mod ticker;
pub(crate) mod transforms;
pub(crate) mod tri;
