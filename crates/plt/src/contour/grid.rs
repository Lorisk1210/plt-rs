//! Fields on quadrilateral grids: z with its mask and coordinates
//! (matplotlib's `QuadContourSet._contour_args`, `_check_xyz` and
//! `_initialize_x_y`).

use plt_core::Point;

use super::set::{ContourSource, ContourWarning};
use super::{ContourError, QuadContourGenerator};
use crate::numeric::{Grid, linspace, meshgrid};
use crate::rc::RcParams;

/// Where the pixel centres of an image-like grid start (matplotlib's
/// `origin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Row 0 at the bottom.
    Lower,
    /// Row 0 at the top.
    Upper,
    /// `image.origin`.
    Image,
}

/// Where the values of a [`QuadField`] sit.
#[derive(Clone, Debug, PartialEq)]
pub enum QuadCoords {
    /// Without `origin`: at `(column, row)`, or spread evenly over
    /// `extent = [x0, x1, y0, y1]` (first to last point). With `origin`:
    /// at the pixel centres of an image covering `extent` (default
    /// `[0, columns, 0, rows]`), rows reversed for [`Origin::Upper`].
    Implicit {
        /// Image-like placement.
        origin: Option<Origin>,
        /// The covered rectangle `[x0, x1, y0, y1]`.
        extent: Option<[f64; 4]>,
    },
    /// One x per column and one y per row.
    Axes {
        /// Column coordinates.
        x: Vec<f64>,
        /// Row coordinates.
        y: Vec<f64>,
    },
    /// The x and y of every point, shaped like z.
    Mesh {
        /// x of every point.
        x: Grid<f64>,
        /// y of every point.
        y: Grid<f64>,
    },
}

/// Values on a quadrilateral grid to contour.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadField {
    /// The values, one row per y index.
    pub z: Grid<f64>,
    /// Masked points (`true` = left out); NaN and infinite z are masked as
    /// well.
    pub mask: Option<Grid<bool>>,
    /// Where the values sit.
    pub coords: QuadCoords,
    /// Whether z holds booleans (0 or 1), which changes the default levels.
    pub boolean: bool,
    /// Whether a quad with one masked point keeps the triangle opposite it
    /// (default `contour.corner_mask`).
    pub corner_mask: Option<bool>,
    /// Chunk size in quads in both directions (0 = no chunking).
    pub nchunk: usize,
}

impl QuadField {
    /// Values at index positions.
    pub fn new(z: Grid<f64>) -> Self {
        Self {
            z,
            mask: None,
            coords: QuadCoords::Implicit {
                origin: None,
                extent: None,
            },
            boolean: false,
            corner_mask: None,
            nchunk: 0,
        }
    }

    /// Boolean values (as 0 and 1).
    pub fn from_bools(z: &Grid<bool>) -> Self {
        Self {
            boolean: true,
            ..Self::new(z.map(|&b| f64::from(u8::from(b))))
        }
    }

    /// The same values at the given coordinates.
    #[must_use]
    pub fn with_coords(mut self, coords: QuadCoords) -> Self {
        self.coords = coords;
        self
    }

    /// The same values with a point mask.
    #[must_use]
    pub fn with_mask(mut self, mask: Grid<bool>) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Checks the shapes and builds the contour generator, masking invalid
    /// (and, when log-scaled, non-positive) values.
    pub(crate) fn source(
        &self,
        logscale: bool,
        rc: &RcParams,
    ) -> Result<ContourSource<QuadContourGenerator>, ContourError> {
        let (rows, cols) = self.z.shape();
        if rows < 2 || cols < 2 {
            return Err(ContourError::GridTooSmall { rows, cols });
        }
        let (x, y) = self.coordinates(rc)?;
        let mut mask: Vec<bool> = self.z.as_slice().iter().map(|v| !v.is_finite()).collect();
        if let Some(user) = &self.mask {
            if user.shape() != (rows, cols) {
                return Err(ContourError::ShapeMismatch {
                    expected: (rows, cols),
                    actual: user.shape(),
                });
            }
            mask.iter_mut()
                .zip(user.as_slice())
                .for_each(|(m, &u)| *m |= u);
        }
        let zmax = extreme(self.z.as_slice(), &mask, f64::max);
        let mut zmin = extreme(self.z.as_slice(), &mask, f64::min);
        let mut warnings = Vec::new();
        if logscale && zmin.is_some_and(|z| z <= 0.0) {
            mask.iter_mut()
                .zip(self.z.as_slice())
                .for_each(|(m, &z)| *m |= z <= 0.0);
            warnings.push(ContourWarning::NonPositiveMasked);
            zmin = extreme(self.z.as_slice(), &mask, f64::min);
        }
        let mask = Grid::from_vec(rows, cols, mask).expect("the mask has the shape of z");
        let corner_mask = self.corner_mask.unwrap_or(rc.contour_corner_mask);
        let chunk = (self.nchunk, self.nchunk);
        let generator =
            QuadContourGenerator::new(&x, &y, &self.z, Some(&mask), corner_mask, chunk)?;
        Ok(ContourSource {
            generator,
            zmin,
            zmax,
            data_lim: (
                Point::new(np_min(x.as_slice()), np_min(y.as_slice())),
                Point::new(np_max(x.as_slice()), np_max(y.as_slice())),
            ),
            boolean: self.boolean,
            warnings,
        })
    }

    /// The x and y grids of the points.
    fn coordinates(&self, rc: &RcParams) -> Result<(Grid<f64>, Grid<f64>), ContourError> {
        let (rows, cols) = self.z.shape();
        match &self.coords {
            QuadCoords::Axes { x, y } => {
                if x.len() != cols {
                    return Err(ContourError::XLength { len: x.len(), cols });
                }
                if y.len() != rows {
                    return Err(ContourError::YLength { len: y.len(), rows });
                }
                Ok(meshgrid(x, y))
            }
            QuadCoords::Mesh { x, y } => {
                for shape in [x.shape(), y.shape()] {
                    if shape != (rows, cols) {
                        return Err(ContourError::ShapeMismatch {
                            expected: (rows, cols),
                            actual: shape,
                        });
                    }
                }
                Ok((x.clone(), y.clone()))
            }
            QuadCoords::Implicit { origin, extent } => {
                let origin = origin.map(|o| match o {
                    Origin::Image if rc.image_origin == "lower" => Origin::Lower,
                    Origin::Image => Origin::Upper,
                    other => other,
                });
                Ok(implicit_coordinates(rows, cols, origin, *extent))
            }
        }
    }
}

/// matplotlib's `_initialize_x_y`.
#[expect(clippy::cast_precision_loss, reason = "grid sizes are far below 2^52")]
fn implicit_coordinates(
    rows: usize,
    cols: usize,
    origin: Option<Origin>,
    extent: Option<[f64; 4]>,
) -> (Grid<f64>, Grid<f64>) {
    let Some(origin) = origin else {
        let (x, y) = match extent {
            None => (
                (0..cols).map(|j| j as f64).collect(),
                (0..rows).map(|i| i as f64).collect(),
            ),
            Some([x0, x1, y0, y1]) => (linspace(x0, x1, cols), linspace(y0, y1, rows)),
        };
        return meshgrid(&x, &y);
    };
    let [x0, x1, y0, y1] = extent.unwrap_or([0.0, cols as f64, 0.0, rows as f64]);
    let dx = (x1 - x0) / cols as f64;
    let dy = (y1 - y0) / rows as f64;
    let x: Vec<f64> = (0..cols).map(|j| x0 + (j as f64 + 0.5) * dx).collect();
    let mut y: Vec<f64> = (0..rows).map(|i| y0 + (i as f64 + 0.5) * dy).collect();
    if origin == Origin::Upper {
        y.reverse();
    }
    meshgrid(&x, &y)
}

/// The extreme unmasked value, or `None` when all are masked.
fn extreme(values: &[f64], mask: &[bool], pick: fn(f64, f64) -> f64) -> Option<f64> {
    values
        .iter()
        .zip(mask)
        .filter(|(_, masked)| !**masked)
        .map(|(&v, _)| v)
        .reduce(pick)
}

/// numpy's `min` (NaN propagates).
fn np_min(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .reduce(|a, b| if a.is_nan() || a < b { a } else { b })
        .unwrap_or(f64::NAN)
}

/// numpy's `max` (NaN propagates).
fn np_max(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .reduce(|a, b| if a.is_nan() || a > b { a } else { b })
        .unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(rows: usize, cols: usize) -> QuadField {
        QuadField::new(Grid::from_fn(rows, cols, |i, j| (i * cols + j) as f64))
    }

    #[test]
    fn implicit_coordinates_follow_origin_and_extent() {
        let (x, y) = implicit_coordinates(2, 3, None, None);
        assert_eq!(x.row(1), [0.0, 1.0, 2.0]);
        assert_eq!(y.column(0), [0.0, 1.0]);
        let (x, _) = implicit_coordinates(2, 3, None, Some([10.0, 20.0, 0.0, 1.0]));
        assert_eq!(x.row(0), [10.0, 15.0, 20.0]);
        let (x, y) = implicit_coordinates(2, 4, Some(Origin::Upper), None);
        assert_eq!(x.row(0), [0.5, 1.5, 2.5, 3.5]);
        assert_eq!(y.column(0), [1.5, 0.5]);
    }

    #[test]
    fn source_masks_invalid_and_non_positive_values() {
        let mut z = field(3, 3).z;
        z[(0, 0)] = f64::NAN;
        let rc = RcParams::default();
        let source = QuadField::new(z.clone()).source(false, &rc).unwrap();
        assert_eq!((source.zmin, source.zmax), (Some(1.0), Some(8.0)));
        z[(0, 1)] = -2.0;
        let source = QuadField::new(z).source(true, &rc).unwrap();
        assert_eq!(source.zmin, Some(2.0));
        assert_eq!(source.warnings, [ContourWarning::NonPositiveMasked]);
    }

    #[test]
    fn shape_errors() {
        let rc = RcParams::default();
        assert!(matches!(
            field(1, 3).source(false, &rc),
            Err(ContourError::GridTooSmall { .. })
        ));
        let wrong_x = field(3, 3).with_coords(QuadCoords::Axes {
            x: vec![0.0, 1.0],
            y: vec![0.0, 1.0, 2.0],
        });
        assert!(matches!(
            wrong_x.source(false, &rc),
            Err(ContourError::XLength { len: 2, cols: 3 })
        ));
        let all_nan = QuadField::new(Grid::filled(2, 2, f64::NAN))
            .source(false, &rc)
            .unwrap();
        assert_eq!((all_nan.zmin, all_nan.zmax), (None, None));
    }
}
