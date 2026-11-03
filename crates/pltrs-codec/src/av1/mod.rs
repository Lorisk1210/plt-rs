//! AV1 still pictures: an intra-only encoder that writes the sequence
//! header and the key frame of a still picture, following the AV1
//! Bitstream & Decoding Process Specification.
//!
//! The encoder codes 8-bit monochrome, 4:2:0 and 4:4:4 pictures in 64×64
//! superblocks, partitioned down to 8×8 blocks by a rate-distortion
//! search. Each block is predicted from its decoded neighbours (DC,
//! smooth, Paeth and the eight directional modes with their angle
//! deltas) or from a palette of up to eight colours, and its residual is
//! transformed by the largest transform that fits (a DCT, an ADST or the
//! identity), quantized and coded by the multi-symbol arithmetic coder
//! with the specification's default CDFs and their adaptation.
//! Superblocks of low variance get finer quantizers, as libaom's image
//! tuning gives them ("Variance Boost"). Quantizer index 0 codes
//! losslessly, with 4×4 Walsh-Hadamard transforms. The encoder
//! reconstructs every block exactly as a decoder does, its inverse
//! transforms being the specification's.
//!
//! ```
//! use pltrs_codec::av1::{self, ChromaSampling, Picture};
//! let luma = [128u8; 16 * 8];
//! let picture = Picture::new(16, 8, ChromaSampling::Monochrome, &[&luma])?;
//! let bitstream = av1::encode(&picture, 0);
//! assert_eq!(bitstream.data[0], 0x0A); // the sequence header OBU
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod bit_writer;
mod block;
mod cdf;
mod coeffs;
mod context;
mod delta_q;
mod entropy;
mod headers;
mod mode_info;
mod palette;
mod plane;
mod predict;
mod quant;
mod search;
mod syntax;
mod tables;
mod tile;
mod transform;

use std::fmt;

use self::coeffs::Scans;
use self::context::Contexts;
use self::headers::{OBU_FRAME, OBU_SEQUENCE_HEADER, Sequence, TILE_SIZE_BYTES, TileLayout};
use self::plane::Plane;
use self::quant::Quantizer;
use self::tile::{Frame, TileBounds, encode_tile};
use self::transform::Bases;

/// The largest side of an AV1 frame.
pub const MAX_SIDE: u32 = 1 << 16;

/// How the colours of a picture are signalled: the colour description of
/// the sequence header, with the code points of ITU-T H.273, as an AVIF
/// `colr` box of type `nclx` repeats them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorDescription {
    /// `color_primaries`: 1 for BT.709 and sRGB.
    pub primaries: u8,
    /// `transfer_characteristics`: 13 for sRGB.
    pub transfer: u8,
    /// `matrix_coefficients`: 6 for BT.601, 1 for BT.709, 0 for the
    /// identity (planes G, B and R).
    pub matrix: u8,
}

impl ColorDescription {
    /// sRGB colours in BT.601 luma and chroma, as Pillow saves AVIF.
    pub const SRGB_BT601: Self = Self {
        primaries: 1,
        transfer: 13,
        matrix: 6,
    };

    /// sRGB colours stored as they are, green, blue and red.
    pub const SRGB_IDENTITY: Self = Self {
        primaries: 1,
        transfer: 13,
        matrix: 0,
    };

    /// Whether these are sRGB colours stored as they are, which AV1
    /// allows only in 4:4:4 and full range.
    pub(crate) fn is_srgb_identity(self) -> bool {
        self == Self::SRGB_IDENTITY
    }
}

/// The planes of a picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromaSampling {
    /// Luma only.
    Monochrome,
    /// Chroma at half the resolution in both directions.
    Yuv420,
    /// Chroma at full resolution.
    Yuv444,
}

impl ChromaSampling {
    /// The halvings of the chroma planes, horizontal and vertical.
    pub const fn subsampling(self) -> (u32, u32) {
        match self {
            Self::Monochrome | Self::Yuv420 => (1, 1),
            Self::Yuv444 => (0, 0),
        }
    }

    /// The number of planes.
    pub const fn planes(self) -> usize {
        match self {
            Self::Monochrome => 1,
            Self::Yuv420 | Self::Yuv444 => 3,
        }
    }

    /// `seq_profile`: Main (0) for monochrome and 4:2:0, High (1) for
    /// 4:4:4.
    pub const fn profile(self) -> u8 {
        match self {
            Self::Monochrome | Self::Yuv420 => 0,
            Self::Yuv444 => 1,
        }
    }

    /// The size of each plane of a `width × height` picture.
    fn plane_size(self, plane: usize, width: u32, height: u32) -> (usize, usize) {
        let (ss_x, ss_y) = if plane == 0 {
            (0, 0)
        } else {
            self.subsampling()
        };
        (
            width.div_ceil(1 << ss_x) as usize,
            height.div_ceil(1 << ss_y) as usize,
        )
    }
}

/// Why a picture cannot be encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Av1Error {
    /// A side is zero or larger than [`MAX_SIDE`].
    Size {
        /// The width given.
        width: u32,
        /// The height given.
        height: u32,
    },
    /// The planes do not match the sampling: the wrong number of planes,
    /// or a plane of the wrong length.
    Planes,
    /// sRGB colours stored as they are need 4:4:4 sampling.
    IdentityNeeds444,
}

impl fmt::Display for Av1Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size { width, height } => {
                write!(f, "a {width}×{height} picture cannot be coded in AV1")
            }
            Self::Planes => f.write_str("the planes do not match the picture's size and sampling"),
            Self::IdentityNeeds444 => {
                f.write_str("the identity matrix needs 4:4:4 chroma sampling")
            }
        }
    }
}

impl std::error::Error for Av1Error {}

/// A picture to encode: 8-bit planes, row by row without padding, the
/// chroma planes rounded up in size where they are subsampled.
#[derive(Clone, Debug)]
pub struct Picture<'a> {
    width: u32,
    height: u32,
    sampling: ChromaSampling,
    planes: Vec<&'a [u8]>,
    color: Option<ColorDescription>,
    full_range: bool,
}

impl<'a> Picture<'a> {
    /// A `width × height` picture of `planes` (Y, then U and V), in full
    /// range and without a colour description.
    ///
    /// # Errors
    ///
    /// When a side is zero or above [`MAX_SIDE`], or the planes do not
    /// match the size and sampling.
    pub fn new(
        width: u32,
        height: u32,
        sampling: ChromaSampling,
        planes: &[&'a [u8]],
    ) -> Result<Self, Av1Error> {
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return Err(Av1Error::Size { width, height });
        }
        let lengths_match = planes.iter().enumerate().all(|(plane, samples)| {
            let (w, h) = sampling.plane_size(plane, width, height);
            samples.len() == w * h
        });
        if planes.len() != sampling.planes() || !lengths_match {
            return Err(Av1Error::Planes);
        }
        Ok(Self {
            width,
            height,
            sampling,
            planes: planes.to_vec(),
            color: None,
            full_range: true,
        })
    }

    /// The picture with its colours described, in full range or in the
    /// limited range of video (16 to 235 for luma).
    ///
    /// # Errors
    ///
    /// When `color` is [`ColorDescription::SRGB_IDENTITY`] but the
    /// picture is not 4:4:4 and in full range.
    pub fn with_color(
        mut self,
        color: ColorDescription,
        full_range: bool,
    ) -> Result<Self, Av1Error> {
        if color.is_srgb_identity() && (self.sampling != ChromaSampling::Yuv444 || !full_range) {
            return Err(Av1Error::IdentityNeeds444);
        }
        self.color = Some(color);
        self.full_range = full_range;
        Ok(self)
    }
}

/// A coded picture: its OBUs and the parameters an `av1C` box records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitstream {
    /// The sequence header OBU and the frame OBU.
    pub data: Vec<u8>,
    /// `seq_profile`.
    pub profile: u8,
    /// `seq_level_idx[0]`.
    pub level: u8,
    /// The planes coded.
    pub sampling: ChromaSampling,
}

/// Encodes `picture` as the key frame of a still picture with base
/// quantizer index `qindex`: 0 is lossless, 1 to 255 ever coarser (each
/// index's quantizers are `Dc_Qlookup` and `Ac_Qlookup`).
pub fn encode(picture: &Picture<'_>, qindex: u8) -> Bitstream {
    let (width, height) = (picture.width, picture.height);
    let sampling = picture.sampling;
    let ss = sampling.subsampling();
    let mi = (
        2 * width.div_ceil(8) as usize,
        2 * height.div_ceil(8) as usize,
    );
    let superblocks = (mi.0.div_ceil(16), mi.1.div_ceil(16));
    let source: Vec<Plane> = picture
        .planes
        .iter()
        .enumerate()
        .map(|(plane, samples)| {
            let (ss_x, ss_y) = if plane == 0 { (0, 0) } else { ss };
            let padded = ((superblocks.0 * 64) >> ss_x, (superblocks.1 * 64) >> ss_y);
            Plane::padded(samples, sampling.plane_size(plane, width, height), padded)
        })
        .collect();
    let recon = source.iter().map(Plane::blank_like).collect();
    let quantizer = Quantizer::new(qindex, ss);
    let delta_q_res = (qindex > 0).then(|| delta_q::resolution_log2(qindex));
    let mut frame = Frame {
        source,
        recon,
        ss,
        mi,
        quantizer,
        delta_q_res,
        bases: Bases::new(),
        scans: Scans::new(),
    };
    let mut contexts = Contexts::new((superblocks.0 * 16, superblocks.1 * 16), ss);
    let layout = TileLayout::new(mi.0, mi.1);
    let chroma_delta = (sampling.planes() > 1).then(|| quantizer.chroma_delta());
    let mut payload = headers::frame_header(qindex, chroma_delta, delta_q_res, &layout);
    let count = layout.count();
    let mut index = 0;
    for rows in layout.row_starts.windows(2) {
        for cols in layout.col_starts.windows(2) {
            let bounds = TileBounds {
                rows: (rows[0], rows[1]),
                cols: (cols[0], cols[1]),
            };
            let tile = encode_tile(&mut frame, &mut contexts, bounds);
            index += 1;
            if index < count {
                let size = u32::try_from(tile.len() - 1)
                    .expect("a tile of at most 4096 × 2304 samples codes in less than 4 GiB");
                payload.extend_from_slice(&size.to_le_bytes()[..TILE_SIZE_BYTES]);
            }
            payload.extend(tile);
        }
    }
    let sequence = Sequence {
        profile: sampling.profile(),
        level: headers::level(width, height),
        size: (width, height),
        monochrome: sampling == ChromaSampling::Monochrome,
        color: picture.color,
        full_range: picture.full_range,
    };
    let mut data = headers::obu(OBU_SEQUENCE_HEADER, &sequence.header());
    data.extend(headers::obu(OBU_FRAME, &payload));
    Bitstream {
        data,
        profile: sequence.profile,
        level: sequence.level,
        sampling,
    }
}

#[cfg(test)]
mod tests;
