//! Colour specifications, named colours, colour sequences, colormaps and
//! normalisation (matplotlib.colors, matplotlib.cm).
//!
//! * [`Color`] — any colour matplotlib accepts (`"tab:blue"`, `"#ff8800"`,
//!   `"0.5"`, `"C2"`, `(0.1, 0.2, 0.3)`), resolved with [`Color::resolve`].
//! * [`to_rgba`], [`to_hex`], [`is_color_like`], [`to_rgba_array`],
//!   [`same_color`], [`rgb_to_hsv`], [`hsv_to_rgb`] — matplotlib's
//!   conversion functions.
//! * [`ColorTable`] / [`named_colors`] — the base, Tableau, CSS4 and xkcd
//!   colour names.
//! * [`color_sequence`] — named colour lists such as `"tab10"`.
//! * [`Colormap`] and the [`colormaps`] registry — lookup tables from
//!   normalised values to colours.
//! * [`Norm`] and its implementations ([`Normalize`], [`LogNorm`],
//!   [`BoundaryNorm`], ...) — data to colormap positions.
//! * [`ScalarMappable`] — a norm and a colormap combined.
//!
//! ```
//! use plt::colors::{LogNorm, ScalarMappable, colormaps};
//! let mut mappable =
//!     ScalarMappable::with_norm(colormaps::get("magma")?, Box::new(LogNorm::new(1.0, 1e3)));
//! let rgba = mappable.to_rgba(&[Some(1.0), Some(1e3), Some(-1.0)], None)?;
//! assert_eq!(rgba[2], mappable.cmap().bad(), "log of a negative value is masked");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod array;
mod colormap;
pub mod colormaps;
mod hsv;
mod light;
mod mappable;
pub mod multivar;
mod named;
mod named_data;
mod norm;
mod npmath;
mod sequence_data;
mod sequences;
mod spec;

pub use array::{Alpha, Colors, same_color, to_rgba_array};
pub use colormap::{
    Channel, Colormap, ColormapError, ColormapKind, Extend, Normalized, Segment, SegmentData,
    SegmentFn, SegmentProblem,
};
pub use hsv::{hsv_to_rgb, rgb_to_hsv};
pub use light::{
    BlendMode, LightSource, Relief, ShadeError, ShadeOptions, blend_overlay, blend_soft_light,
};
pub use mappable::ScalarMappable;
pub use named::{ColorTable, NamedColor, named_color_table, named_colors};
pub use norm::{
    AsinhNorm, BoundaryNorm, CenteredNorm, FuncNorm, LogNorm, NoNorm, Norm, NormClone, NormError,
    NormKind, NormTransform, Normalize, PowerNorm, ScaleNorm, SymLogNorm, TwoSlopeNorm,
};
pub use sequences::{color_sequence, color_sequence_names};
pub use spec::{Color, ColorError, is_color_like, is_cycle_reference, to_hex, to_rgba};
