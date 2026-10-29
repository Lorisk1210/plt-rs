//! VP8L, WebP's lossless bitstream: entropy-coded ARGB pixels with
//! optional prediction, colour decorrelation, green subtraction and
//! palette transforms, decoded and encoded.

mod bit_reader;
mod decode;
mod encode;
mod format;
mod prefix;
mod transform;

pub(crate) use decode::{decode, decode_stream, read_info};
pub(crate) use encode::encode;
