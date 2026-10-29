//! Comparing the images of the examples: pixel by pixel, between the Rust
//! and the Python version of an example, and byte for byte, between a
//! fresh rendering and the committed image.

use std::fs;
use std::path::Path;

use pltrs::image::{ImageData, imread};
use pltrs::numeric::Grid;
use pltrs::testing::calculate_rms;

/// Converts a float channel in `[0, 1]` to a byte.
fn byte(channel: f32) -> u8 {
    (channel * 255.0).round() as u8
}

/// The colours of the image at `path` (a PNG, or the first frame of a
/// GIF), as bytes. The figures are opaque, so alpha is dropped.
fn colours(path: &Path) -> Result<Grid<[u8; 3]>, String> {
    let image = imread(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    match image {
        ImageData::RgbBytes(grid) => Ok(grid),
        ImageData::RgbaBytes(grid) => Ok(grid.map(|&[r, g, b, _]| [r, g, b])),
        ImageData::RgbF32(grid) => Ok(grid.map(|&[r, g, b]| [byte(r), byte(g), byte(b)])),
        ImageData::RgbaF32(grid) => Ok(grid.map(|&[r, g, b, _]| [byte(r), byte(g), byte(b)])),
        _ => Err(format!("{} is not a colour image", path.display())),
    }
}

/// The root mean square of the colour differences between the images at
/// `a` and `b`, on the 0–255 scale (matplotlib's `calculate_rms`, the
/// measure of the spec's pixel comparisons). Animations are compared by
/// their first frames.
pub(super) fn rms(a: &Path, b: &Path) -> Result<f64, String> {
    calculate_rms(&colours(a)?, &colours(b)?).map_err(|err| err.to_string())
}

/// Whether the files at `a` and `b` hold the same bytes.
pub(super) fn same_bytes(a: &Path, b: &Path) -> Result<bool, String> {
    let read = |path: &Path| {
        fs::read(path).map_err(|err| format!("cannot read {}: {err}", path.display()))
    };
    Ok(read(a)? == read(b)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_channels_round_to_the_nearest_byte() {
        assert_eq!(byte(0.0), 0);
        assert_eq!(byte(1.0), 255);
        assert_eq!(byte(128.0 / 255.0), 128);
    }
}
