//! Image `XObject`s (`writeImages`, `_unpack`, `_writeImg`): RGB or
//! grey data with a separate alpha soft mask, PNG-predicted and
//! Flate-compressed, and indexed when the image has at most 256 colours.

use pltrs_codec::png::{self, BitDepth, ColorType, ImageRef};

use super::object::Obj;
use super::writer::PdfWriter;
use crate::image::RgbaImage;
use crate::vector::{VectorError, png_idat};

/// Pixel data of one channel layout.
struct Samples {
    width: usize,
    height: usize,
    /// 1 (grey) or 3 (RGB).
    channels: usize,
    data: Vec<u8>,
}

/// `_unpack`: the RGB samples and, unless fully opaque, the alpha samples
/// (rows top to bottom, as PDF expects).
fn unpack(image: &RgbaImage) -> (Samples, Option<Samples>) {
    let (width, height) = (image.width(), image.height());
    let pixels = image.data().as_chunks::<4>().0;
    let rgb = pixels.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let alpha: Vec<u8> = pixels.iter().map(|p| p[3]).collect();
    let alpha = alpha.iter().any(|&a| a != 255).then_some(Samples {
        width,
        height,
        channels: 1,
        data: alpha,
    });
    (
        Samples {
            width,
            height,
            channels: 3,
            data: rgb,
        },
        alpha,
    )
}

/// An indexed version of RGB samples: the palette (sorted by colour) and
/// one index per pixel, when there are at most 256 colours.
fn palette(samples: &Samples) -> Option<(Vec<[u8; 3]>, Vec<u8>)> {
    let pixels = samples.data.as_chunks::<3>().0;
    let mut colors: Vec<[u8; 3]> = pixels.to_vec();
    colors.sort_unstable();
    colors.dedup();
    if colors.len() > 256 {
        return None;
    }
    let indices = pixels
        .iter()
        .map(|p| colors.binary_search(p).map_or(0, |i| i as u8))
        .collect();
    Some((colors, indices))
}

/// The bit depth Pillow saves a palette image of `colors` entries with.
fn index_depth(colors: usize) -> (u8, BitDepth) {
    match colors {
        0..=2 => (1, BitDepth::One),
        3..=4 => (2, BitDepth::Two),
        5..=16 => (4, BitDepth::Four),
        _ => (8, BitDepth::Eight),
    }
}

/// Packs one index per pixel into rows of `bits`-bit samples.
fn pack(indices: &[u8], width: usize, bits: u8) -> Vec<u8> {
    let per_byte = usize::from(8 / bits);
    let mut out = Vec::new();
    for row in indices.chunks(width.max(1)) {
        for group in row.chunks(per_byte) {
            let mut byte = 0u8;
            for (i, &index) in group.iter().enumerate() {
                byte |= index << (8 - bits as usize * (i + 1));
            }
            out.push(byte);
        }
    }
    out
}

/// PNG-predicted, compressed data of `samples` (the `IDAT` payload).
fn predicted(
    width: usize,
    height: usize,
    color: ColorType,
    depth: BitDepth,
    data: &[u8],
) -> Result<Vec<u8>, VectorError> {
    let image = ImageRef::new(width as u32, height as u32, color, depth, data)?;
    Ok(png_idat(&png::encode(image)?))
}

/// `_writeImg`: writes `samples` as image object `id` with the optional
/// soft mask object `smask`.
fn write_samples(
    writer: &mut PdfWriter,
    samples: &Samples,
    id: u32,
    smask: Option<u32>,
    compressed: bool,
) -> Result<(), VectorError> {
    let Samples {
        width,
        height,
        channels,
        ..
    } = *samples;
    let space = if channels == 1 {
        "DeviceGray"
    } else {
        "DeviceRGB"
    };
    let mut dict = vec![
        ("Type".to_owned(), Obj::name("XObject")),
        ("Subtype".to_owned(), Obj::name("Image")),
        ("Width".to_owned(), Obj::Int(width as i64)),
        ("Height".to_owned(), Obj::Int(height as i64)),
        ("ColorSpace".to_owned(), Obj::name(space)),
        ("BitsPerComponent".to_owned(), Obj::Int(8)),
    ];
    if let Some(smask) = smask {
        dict.push(("SMask".to_owned(), Obj::Ref(smask)));
    }
    let length = writer.reserve();
    if !compressed {
        writer.begin_stream(id, Some(length), dict, None);
        writer.write(&samples.data);
        return writer.end_stream();
    }
    let mut parms = vec![
        ("Predictor".to_owned(), Obj::Int(10)),
        ("Colors".to_owned(), Obj::Int(channels as i64)),
        ("Columns".to_owned(), Obj::Int(width as i64)),
    ];
    let indexed = if channels == 3 {
        palette(samples)
    } else {
        None
    };
    let data = if let Some((colors, indices)) = indexed {
        let (bits, depth) = index_depth(colors.len());
        let table: Vec<u8> = colors.iter().flatten().copied().collect();
        dict[4].1 = Obj::Array(vec![
            Obj::name("Indexed"),
            Obj::name("DeviceRGB"),
            Obj::Int(colors.len() as i64 - 1),
            Obj::Str(table),
        ]);
        dict[5].1 = Obj::Int(i64::from(bits));
        parms[1].1 = Obj::Int(1);
        parms.push(("BitsPerComponent".to_owned(), Obj::Int(i64::from(bits))));
        let packed = pack(&indices, width, bits);
        predicted(width, height, ColorType::Gray, depth, &packed)?
    } else {
        let color = if channels == 1 {
            ColorType::Gray
        } else {
            ColorType::Rgb
        };
        predicted(width, height, color, BitDepth::Eight, &samples.data)?
    };
    writer.begin_stream(id, Some(length), dict, Some(Obj::Dict(parms)));
    writer.write(&data);
    writer.end_stream()
}

/// `writeImages` for one image: its soft mask (when not opaque), then
/// the colour data as object `id`.
pub(super) fn write_image(
    writer: &mut PdfWriter,
    image: &RgbaImage,
    id: u32,
    compressed: bool,
) -> Result<(), VectorError> {
    let (rgb, alpha) = unpack(image);
    let smask = match alpha {
        Some(alpha) => {
            let smask = writer.reserve();
            write_samples(writer, &alpha, smask, None, compressed)?;
            Some(smask)
        }
        None => None,
    };
    write_samples(writer, &rgb, id, smask, compressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_and_packing() {
        let samples = Samples {
            width: 3,
            height: 1,
            channels: 3,
            data: vec![9, 9, 9, 1, 2, 3, 9, 9, 9],
        };
        let (colors, indices) = palette(&samples).unwrap();
        assert_eq!(colors, [[1, 2, 3], [9, 9, 9]]);
        assert_eq!(indices, [1, 0, 1]);
        assert_eq!(index_depth(2).0, 1);
        assert_eq!(index_depth(16).0, 4);
        assert_eq!(index_depth(17).0, 8);
        assert_eq!(pack(&[1, 0, 1], 3, 1), [0b1010_0000]);
        assert_eq!(pack(&[3, 1, 2, 0, 1], 5, 2), [0b1101_1000, 0b0100_0000]);
    }

    #[test]
    fn opaque_images_have_no_soft_mask() {
        let image = RgbaImage::new(1, 1, vec![1, 2, 3, 255]).unwrap();
        let (rgb, alpha) = unpack(&image);
        assert_eq!(rgb.data, [1, 2, 3]);
        assert!(alpha.is_none());
        let translucent = RgbaImage::new(1, 1, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(unpack(&translucent).1.map(|a| a.data), Some(vec![4]));
    }
}
