//! The case from `test_png.py`: PNG files of every colour type and bit
//! depth, read back and shown side by side.
//!
//! The test reads the fifteen basic images of the PNG test suite; the
//! case writes files of the same colour types and bit depths whose pixels
//! are gradients, the pixels `_pngsuite` in `cases_mpl_rest.py` writes.

use plt::axes::ImshowOptions;
use plt::colors::colormaps;
use plt::figure::Figure;
use plt::image::{ColorizerOptions, Extent, ImageData, Interpolation, imread_png};
use plt_codec::checksum::crc32;
use plt_codec::deflate::CompressionLevel;
use plt_codec::zlib;

use crate::figures::{CaseResult, FigureCase};

/// The side of the square images, in pixels.
const SIZE: u32 = 32;

/// The PNG colour types (the `IHDR` codes).
#[derive(Clone, Copy)]
enum ColorType {
    Grey = 0,
    Rgb = 2,
    Palette = 3,
    GreyAlpha = 4,
    Rgba = 6,
}

/// Appends a chunk: its length, type, data and CRC.
fn push_chunk(png: &mut Vec<u8>, tag: [u8; 4], data: &[u8]) {
    let length = u32::try_from(data.len()).expect("the images are small");
    png.extend_from_slice(&length.to_be_bytes());
    let start = png.len();
    png.extend_from_slice(&tag);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

/// The bytes of one row of samples: packed most significant bits first
/// below 8 bits, big-endian pairs at 16.
fn scanline(samples: &[u16], bit_depth: u8) -> Vec<u8> {
    if bit_depth == 16 {
        return samples.iter().flat_map(|s| s.to_be_bytes()).collect();
    }
    samples
        .chunks(usize::from(8 / bit_depth))
        .map(|group| {
            group.iter().zip(1..).fold(0, |byte, (&sample, i)| {
                let sample = u8::try_from(sample).expect("samples of at most 8 bits");
                byte | sample << (8 - bit_depth * i)
            })
        })
        .collect()
}

/// A `SIZE × SIZE` PNG whose pixel `(x, y)` has the samples
/// `pixel(x, y)`, unfiltered and not interlaced; `palette` is written
/// when not empty.
fn png(
    color_type: ColorType,
    bit_depth: u8,
    palette: &[[u8; 3]],
    pixel: impl Fn(u32, u32) -> Vec<u16>,
) -> Vec<u8> {
    let mut raw = Vec::new();
    for y in 0..SIZE {
        let samples: Vec<u16> = (0..SIZE).flat_map(|x| pixel(x, y)).collect();
        raw.push(0);
        raw.extend(scanline(&samples, bit_depth));
    }
    let mut header = [SIZE.to_be_bytes(), SIZE.to_be_bytes()].concat();
    header.extend_from_slice(&[bit_depth, color_type as u8, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    push_chunk(&mut png, *b"IHDR", &header);
    if !palette.is_empty() {
        push_chunk(&mut png, *b"PLTE", palette.as_flattened());
    }
    let compressed = zlib::compress(&raw, CompressionLevel::DEFAULT);
    push_chunk(&mut png, *b"IDAT", &compressed);
    push_chunk(&mut png, *b"IEND", &[]);
    png
}

/// `t` in `[0, 62]` as a sample of `bit_depth` bits.
fn ramp(t: u32, bit_depth: u8) -> u16 {
    let max = (1 << bit_depth) - 1;
    u16::try_from(t * max / 62).expect("samples of at most 16 bits")
}

/// Grey images.
fn grey(depth: u8) -> Vec<u8> {
    png(ColorType::Grey, depth, &[], |x, y| vec![ramp(x + y, depth)])
}

/// RGB images.
fn rgb(depth: u8) -> Vec<u8> {
    png(ColorType::Rgb, depth, &[], |x, y| {
        vec![
            ramp(2 * x, depth),
            ramp(2 * y, depth),
            ramp(62 - x - y, depth),
        ]
    })
}

/// Palette images: every one of the `2^depth` colours is used.
fn palette(depth: u8) -> Vec<u8> {
    let n: u32 = 1 << depth;
    let colors: Vec<[u8; 3]> = (0..n)
        .map(|k| {
            let channel = |value: u32| u8::try_from(value).expect("channels of 8 bits");
            let ramp = k * 255 / (n - 1);
            [channel(ramp), channel(255 - ramp), channel(k * 97 % 256)]
        })
        .collect();
    png(ColorType::Palette, depth, &colors, |x, y| {
        vec![u16::try_from((x + y) * n / 64).expect("palette indices of at most 8 bits")]
    })
}

/// Grey images with alpha.
fn grey_alpha(depth: u8) -> Vec<u8> {
    png(ColorType::GreyAlpha, depth, &[], |x, y| {
        vec![ramp(2 * x, depth), ramp(2 * y, depth)]
    })
}

/// RGBA images.
fn rgba(depth: u8) -> Vec<u8> {
    png(ColorType::Rgba, depth, &[], |x, y| {
        vec![
            ramp(2 * x, depth),
            ramp(2 * y, depth),
            ramp(31, depth),
            ramp(x + y, depth),
        ]
    })
}

/// The stand-ins for the suite's `basn*.png`, in file name order.
fn pngsuite_files() -> Vec<Vec<u8>> {
    let mut files: Vec<Vec<u8>> = [1, 2, 4, 8, 16].map(grey).into();
    files.extend([8, 16].map(rgb));
    files.extend([1, 2, 4, 8].map(palette));
    files.extend([8, 16].map(grey_alpha));
    files.extend([8, 16].map(rgba));
    files
}

/// `test_pngsuite`: each file read and shown in a unit square of its own,
/// grey images in grey, on a light cyan background.
fn pngsuite(fig: &Figure) -> CaseResult {
    let files = pngsuite_files();
    let ax = fig.gca();
    let mut left = 0.0;
    for file in &files {
        let data = imread_png(file)?;
        let mut colors = ColorizerOptions::default();
        if matches!(data, ImageData::Scalar(_)) {
            colors = colors.cmap(colormaps::get("gray")?);
        }
        let options = ImshowOptions {
            colors,
            extent: Some(Extent::new(left, left + 1.0, 0.0, 1.0)),
            interpolation: Some(Interpolation::Nearest),
            ..ImshowOptions::default()
        };
        ax.imshow(data, options)?;
        left += 1.0;
    }
    ax.set_facecolor("#ddffff");
    ax.set_xlim(0.0, left);
    Ok(())
}

/// The group's PNG case.
pub(super) const CASES: &[FigureCase] =
    &[FigureCase::new("mpl_rest/pngsuite", pngsuite).sized(15.0, 2.0, 100.0)];
