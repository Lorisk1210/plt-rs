//! Renderer abstraction with raster (PNG), SVG, PDF and PostScript backends.
//!
//! Artists in the `plt` crate describe what to draw through the
//! [`Renderer`] trait, using a [`GraphicsContext`] for stroke and clipping
//! state. Backends translate those calls into pixels or vector documents.
#![forbid(unsafe_code)]

mod collection;
mod gc;
mod image;
mod renderer;

pub use collection::{CollectionItem, PathCollection};
pub use gc::{CapStyle, ClipPath, Dashes, GraphicsContext, Hatch, JoinStyle, Sketch, SnapMode};
pub use image::{ImageSizeError, RgbaImage};
pub use renderer::Renderer;
