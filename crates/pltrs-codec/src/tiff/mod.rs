//! TIFF files.
//!
//! * [`encode_rgba8`] writes uncompressed RGBA files laid out as Pillow
//!   writes them.
//! * [`decode`] reads the first image of a file as Pillow opens it:
//!   little- and big-endian (and `BigTIFF`) files, strips or tiles,
//!   uncompressed, `PackBits`, LZW or Deflate data (with horizontal
//!   differencing), in the layouts Pillow knows — bilevel, 2-, 4-, 8- and
//!   16-bit grey, grey with alpha, RGB and RGBA of 8 or 16 bits (alpha
//!   associated or not), palettes and CMYK — oriented by the
//!   `Orientation` tag.

mod decode;
mod encode;

pub use decode::{DecodeError, decode};
pub use encode::{TiffError, encode_rgba8};
