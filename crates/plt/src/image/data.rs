//! The pixels of an image: colour-mapped scalars or explicit colours
//! (matplotlib's image array `A` after `_normalize_image_array`).

use super::ImageError;
use crate::artist::{format_mapped, format_values};
use crate::colors::ScalarMappable;
use crate::numeric::Grid;

/// What kind of numbers a scalar image holds; it decides the precision
/// of data-stage resampling, as matplotlib's dtype does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScalarKind {
    /// Floating-point data (`float64`).
    #[default]
    Float,
    /// Integer data.
    Integer,
    /// Boolean data (`false` = 0, `true` = 1).
    Boolean,
}

/// Scalar data to be colour-mapped, with its mask (a numpy masked array).
///
/// Non-finite values are masked, as matplotlib's `safe_masked_invalid`
/// does; the values under the mask are kept.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarField {
    values: Grid<f64>,
    masked: Grid<bool>,
    kind: ScalarKind,
}

impl ScalarField {
    /// Floating-point data; NaN and infinite values are masked.
    pub fn new(values: Grid<f64>) -> Self {
        Self::with_kind(values, ScalarKind::Float)
    }

    /// Data with explicitly masked cells (`true` = masked), in addition
    /// to the non-finite ones.
    ///
    /// # Errors
    ///
    /// When the mask's shape differs from the data's.
    pub fn masked(values: Grid<f64>, mask: &Grid<bool>) -> Result<Self, ImageError> {
        if mask.shape() != values.shape() {
            return Err(ImageError::ShapeMismatch {
                what: "mask",
                expected: values.shape(),
                actual: mask.shape(),
            });
        }
        let mut field = Self::new(values);
        for (masked, &extra) in field.masked.as_mut_slice().iter_mut().zip(mask.as_slice()) {
            *masked |= extra;
        }
        Ok(field)
    }

    /// Integer data.
    pub fn integers(values: &Grid<i64>) -> Self {
        Self::with_kind(values.map(|&v| v as f64), ScalarKind::Integer)
    }

    /// Boolean data.
    pub fn booleans(values: &Grid<bool>) -> Self {
        Self::with_kind(values.map(|&v| f64::from(u8::from(v))), ScalarKind::Boolean)
    }

    fn with_kind(values: Grid<f64>, kind: ScalarKind) -> Self {
        let masked = values.map(|v| !v.is_finite());
        Self {
            values,
            masked,
            kind,
        }
    }

    /// The values, including those under the mask.
    pub fn values(&self) -> &Grid<f64> {
        &self.values
    }

    /// Which cells are masked.
    pub fn mask(&self) -> &Grid<bool> {
        &self.masked
    }

    /// The kind of numbers held.
    pub fn kind(&self) -> ScalarKind {
        self.kind
    }

    /// The values with masked cells as `None`, row-major.
    pub fn masked_values(&self) -> Vec<Option<f64>> {
        self.values
            .as_slice()
            .iter()
            .zip(self.masked.as_slice())
            .map(|(&v, &masked)| (!masked).then_some(v))
            .collect()
    }

    /// Whether data-stage resampling runs in single precision: matplotlib
    /// resamples integer data as `float32` unless its range exceeds 1e8.
    pub(crate) fn resamples_in_f32(&self) -> bool {
        if self.kind == ScalarKind::Float {
            return false;
        }
        let values = self.values.as_slice();
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        max - min <= 1e8
    }
}

/// The pixels of an image.
///
/// Float colour channels are in `[0, 1]` and are clipped into that range
/// (unless the data holds NaN, which matplotlib leaves alone); NaN colour
/// pixels are drawn transparent.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageData {
    /// Scalars mapped to colours through the image's norm and colormap.
    Scalar(ScalarField),
    /// RGB colours with float channels.
    Rgb(Grid<[f64; 3]>),
    /// RGBA colours with float channels.
    Rgba(Grid<[f64; 4]>),
    /// RGB colours with 8-bit channels.
    RgbBytes(Grid<[u8; 3]>),
    /// RGBA colours with 8-bit channels.
    RgbaBytes(Grid<[u8; 4]>),
}

impl ImageData {
    /// `(rows, columns)`.
    pub fn shape(&self) -> (usize, usize) {
        match self {
            Self::Scalar(field) => field.values.shape(),
            Self::Rgb(grid) => grid.shape(),
            Self::Rgba(grid) => grid.shape(),
            Self::RgbBytes(grid) => grid.shape(),
            Self::RgbaBytes(grid) => grid.shape(),
        }
    }

    /// Returns `true` for an image without pixels.
    pub fn is_empty(&self) -> bool {
        let (rows, cols) = self.shape();
        rows == 0 || cols == 0
    }

    /// The scalar field, if the image is colour-mapped.
    pub fn scalars(&self) -> Option<&ScalarField> {
        match self {
            Self::Scalar(field) => Some(field),
            _ => None,
        }
    }

    /// What a status bar shows of the element at `(row, column)`: the
    /// colour-mapped value (`[]` when masked) or the colour channels
    /// (matplotlib's `format_cursor_data` of `get_cursor_data`).
    pub(crate) fn cursor_text(&self, at: (usize, usize), mappable: &ScalarMappable) -> String {
        match self {
            Self::Scalar(field) => {
                let value = (!field.mask()[at]).then(|| field.values()[at]);
                format_mapped(mappable, value)
            }
            Self::Rgb(grid) => format_values(grid[at]),
            Self::Rgba(grid) => format_values(grid[at]),
            Self::RgbBytes(grid) => format_values(grid[at].map(f64::from)),
            Self::RgbaBytes(grid) => format_values(grid[at].map(f64::from)),
        }
    }

    /// Clips float colours into `[0, 1]` (matplotlib's
    /// `_normalize_image_array`, which skips the clip when NaN is present).
    pub(crate) fn normalized(self) -> Self {
        match self {
            Self::Rgb(grid) => Self::Rgb(clip_unit(grid)),
            Self::Rgba(grid) => Self::Rgba(clip_unit(grid)),
            other => other,
        }
    }
}

/// `np.clip(A, 0, 1)` unless the range check fails on NaN.
fn clip_unit<const N: usize>(grid: Grid<[f64; N]>) -> Grid<[f64; N]> {
    if grid.as_slice().iter().flatten().any(|v| v.is_nan()) {
        return grid;
    }
    grid.map(|pixel| pixel.map(|v| v.clamp(0.0, 1.0)))
}

impl From<Grid<f64>> for ScalarField {
    fn from(values: Grid<f64>) -> Self {
        Self::new(values)
    }
}

impl From<ScalarField> for ImageData {
    fn from(field: ScalarField) -> Self {
        Self::Scalar(field)
    }
}

impl From<Grid<f64>> for ImageData {
    fn from(values: Grid<f64>) -> Self {
        Self::Scalar(ScalarField::new(values))
    }
}

impl From<Grid<[f64; 3]>> for ImageData {
    fn from(grid: Grid<[f64; 3]>) -> Self {
        Self::Rgb(grid)
    }
}

impl From<Grid<[f64; 4]>> for ImageData {
    fn from(grid: Grid<[f64; 4]>) -> Self {
        Self::Rgba(grid)
    }
}

impl From<Grid<[u8; 3]>> for ImageData {
    fn from(grid: Grid<[u8; 3]>) -> Self {
        Self::RgbBytes(grid)
    }
}

impl From<Grid<[u8; 4]>> for ImageData {
    fn from(grid: Grid<[u8; 4]>) -> Self {
        Self::RgbaBytes(grid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_values_are_masked() {
        let grid = Grid::from_vec(1, 3, vec![1.0, f64::NAN, f64::INFINITY]).unwrap();
        let field = ScalarField::new(grid);
        assert_eq!(field.masked_values(), vec![Some(1.0), None, None]);
        assert!(
            field.values().as_slice()[1].is_nan(),
            "data under the mask stays"
        );
    }

    #[test]
    fn explicit_masks_add_to_invalid_values() {
        let grid = Grid::from_vec(1, 2, vec![1.0, 2.0]).unwrap();
        let mask = Grid::from_vec(1, 2, vec![true, false]).unwrap();
        let field = ScalarField::masked(grid.clone(), &mask).unwrap();
        assert_eq!(field.masked_values(), vec![None, Some(2.0)]);
        let wrong = Grid::filled(2, 1, false);
        assert!(matches!(
            ScalarField::masked(grid, &wrong),
            Err(ImageError::ShapeMismatch { what: "mask", .. })
        ));
    }

    #[test]
    fn integer_ranges_choose_the_precision() {
        let small = ScalarField::integers(&Grid::from_vec(1, 2, vec![0, 5]).unwrap());
        assert!(small.resamples_in_f32());
        let wide = ScalarField::integers(&Grid::from_vec(1, 2, vec![0, 1_000_000_000]).unwrap());
        assert!(!wide.resamples_in_f32());
        assert!(!ScalarField::new(Grid::filled(1, 1, 0.0)).resamples_in_f32());
        let flags = ScalarField::booleans(&Grid::from_vec(1, 2, vec![false, true]).unwrap());
        assert_eq!(flags.kind(), ScalarKind::Boolean);
        assert_eq!(flags.values().as_slice(), &[0.0, 1.0]);
    }

    #[test]
    fn float_colours_are_clipped_unless_nan() {
        let rgb = Grid::from_vec(1, 1, vec![[1.5, -0.5, 0.5]]).unwrap();
        let ImageData::Rgb(clipped) = ImageData::from(rgb).normalized() else {
            unreachable!("stays RGB")
        };
        assert_eq!(clipped.as_slice(), &[[1.0, 0.0, 0.5]]);
        let with_nan = Grid::from_vec(1, 1, vec![[2.0, f64::NAN, 0.5, 1.0]]).unwrap();
        let ImageData::Rgba(kept) = ImageData::from(with_nan).normalized() else {
            unreachable!("stays RGBA")
        };
        assert_eq!(kept.as_slice()[0][0], 2.0);
        assert_eq!(
            ImageData::from(Grid::<f64>::filled(0, 3, 0.0)).shape(),
            (0, 3)
        );
        assert!(ImageData::from(Grid::<f64>::filled(0, 3, 0.0)).is_empty());
    }
}
