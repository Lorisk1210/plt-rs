//! JPEG encoding and decoding, both mirroring libjpeg-turbo's arithmetic as
//! Pillow uses it.
//!
//! * [`encode_rgb8`] writes baseline JPEG files byte for byte as Pillow
//!   saves them with its defaults.
//! * [`decode`] reads baseline, extended and progressive Huffman-coded
//!   JPEG files of one (greyscale), three (YCbCr or RGB) or four (CMYK or
//!   YCCK) components with any sampling factors and restart intervals.
//!   Its pixels are bit-identical to what Pillow returns: the accurate
//!   integer IDCT, libjpeg-turbo's "fancy" chroma upsampling and its
//!   fixed-point colour conversion are ported exactly.
//!
//! ```
//! use plt_codec::jpeg::{self, JpegOptions};
//! use plt_codec::raster::Pixels;
//!
//! let file = jpeg::encode_rgb8(2, 1, &[200, 100, 50, 200, 100, 50], &JpegOptions::default())?;
//! let image = jpeg::decode(&file)?;
//! assert_eq!((image.width(), image.height()), (2, 1));
//! assert!(matches!(image.pixels(), Pixels::Rgb(_)));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod decode;
mod encode;
mod marker;
mod tables;

pub use decode::{DecodeError, TableKind, decode};
pub use encode::{JpegError, JpegOptions, encode_rgb8};
