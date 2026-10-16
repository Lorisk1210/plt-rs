//! Colour specifications, named colours, colour sequences, colormaps and
//! normalisation (matplotlib.colors, matplotlib.cm).
//!
//! * [`Color`] — any colour matplotlib accepts (`"tab:blue"`, `"#ff8800"`,
//!   `"0.5"`, `"C2"`, `(0.1, 0.2, 0.3)`), resolved with [`Color::resolve`].
//! * [`to_rgba`], [`to_hex`], [`is_color_like`] — matplotlib's conversion
//!   functions.
//! * [`ColorTable`] / [`named_colors`] — the base, Tableau, CSS4 and xkcd
//!   colour names.
//! * [`color_sequence`] — named colour lists such as `"tab10"`.

mod named;
mod named_data;
mod sequence_data;
mod sequences;
mod spec;

pub use named::{ColorTable, named_colors};
pub use sequences::{color_sequence, color_sequence_names};
pub use spec::{Color, ColorError, is_color_like, is_cycle_reference, to_hex, to_rgba};
