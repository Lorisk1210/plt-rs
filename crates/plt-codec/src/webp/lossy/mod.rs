//! VP8, WebP's lossy bitstream: one key frame of intra-predicted
//! macroblocks, decoded to the pixels libwebp produces.

mod bool_decoder;
mod filter;
mod frame;
mod header;
mod predict;
mod tables;
mod tokens;
mod transform;
mod yuv;

pub(crate) use header::read_info;

use crate::webp::DecodeError;

/// Decodes a VP8 bitstream into RGB pixels, row by row.
pub(crate) fn decode(data: &[u8]) -> Result<Vec<[u8; 3]>, DecodeError> {
    Ok(yuv::to_rgb(&frame::decode(data)?))
}
