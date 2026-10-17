//! Image encoding helpers for embedded raster images.

use plt_codec::png::SIGNATURE;

use crate::image::RgbaImage;

/// The RGBA bytes of `image` with the rows in reverse order (bottom row
/// first): the layout matplotlib's images have when they reach a renderer.
pub(crate) fn flipped_rows(image: &RgbaImage) -> Vec<u8> {
    let row = image.width() * 4;
    if row == 0 {
        return Vec::new();
    }
    image
        .data()
        .chunks_exact(row)
        .rev()
        .flatten()
        .copied()
        .collect()
}

/// The concatenated `IDAT` payload of a PNG file: zlib-compressed,
/// PNG-filtered rows, which PDF reads with `/Predictor 10`
/// (matplotlib's `_writePng`).
pub(crate) fn png_idat(file: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    let mut rest = file.strip_prefix(&SIGNATURE[..]).unwrap_or(file);
    while rest.len() >= 12 {
        let length = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        let kind = &rest[4..8];
        let Some(body) = rest.get(8..8 + length) else {
            break;
        };
        if kind == b"IDAT" {
            data.extend_from_slice(body);
        }
        rest = rest.get(12 + length..).unwrap_or_default();
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_codec::png::{self, ImageRef};

    #[test]
    fn flips_and_extracts() {
        let image = RgbaImage::new(1, 2, vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        assert_eq!(flipped_rows(&image), [5, 6, 7, 8, 1, 2, 3, 4]);
        let rgb = [10, 20, 30, 40, 50, 60];
        let file = png::encode(ImageRef::rgb8(2, 1, &rgb).unwrap()).unwrap();
        let idat = png_idat(&file);
        let rows = plt_codec::zlib::decompress(&idat).unwrap();
        assert_eq!(rows.len(), 1 + rgb.len(), "one filter byte per row");
        assert_eq!(
            flipped_rows(&RgbaImage::transparent(0, 3)),
            Vec::<u8>::new()
        );
    }
}
