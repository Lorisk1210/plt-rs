//! VP8L, WebP's lossless bitstream: entropy-coded ARGB pixels with
//! optional prediction, colour decorrelation, green subtraction and
//! palette transforms.

mod bit_reader;
mod decode;
mod prefix;
mod transform;

pub(crate) use decode::{decode, decode_stream, read_info};
