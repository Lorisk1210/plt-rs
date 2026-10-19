//! Per-area operation tables binding golden op names to plt-rs.
//!
//! Each module exposes `OPS`; an empty table means the area is not
//! implemented yet and every probe in it reports as missing.

pub(crate) mod afm;
pub(crate) mod animation;
pub(crate) mod axes3d;
pub(crate) mod axes_behaviour;
pub(crate) mod axisartist;
mod canon;
pub(crate) mod colorbar_behaviour;
pub(crate) mod colormap_luts;
pub(crate) mod colormaps;
pub(crate) mod colors;
pub(crate) mod contour;
pub(crate) mod cursor;
pub(crate) mod dates;
pub(crate) mod fancy;
pub(crate) mod imread;
pub(crate) mod imsave;
pub(crate) mod legend_behaviour;
pub(crate) mod lightsource;
pub(crate) mod markers;
pub(crate) mod mathtext;
pub(crate) mod mosaic;
pub(crate) mod multivar;
pub(crate) mod navigation;
pub(crate) mod norms;
pub(crate) mod path;
pub(crate) mod pick;
pub(crate) mod rcsetup;
pub(crate) mod scales;
mod scenario;
pub(crate) mod stats;
pub(crate) mod text_metrics;
pub(crate) mod text_shaping;
pub(crate) mod ticker;
pub(crate) mod transforms;
pub(crate) mod tri;
pub(crate) mod widgets;
