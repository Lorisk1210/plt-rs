//! Fields on quadrilateral grids: z with its mask and coordinates
//! (matplotlib's `QuadContourSet._contour_args`, `_check_xyz` and
//! `_initialize_x_y`).

use pltrs_core::Point;

use super::quad::GridCoordinates;
use super::set::{ContourSource, ContourWarning};
use super::{ContourAlgorithm, ContourError, QuadContourGenerator};
use crate::image::Origin;
use crate::numeric::{Grid, linspace, meshgrid};
use crate::rc::RcParams;
use crate::transforms::ResolvedTransform;

/// Where the pixel centres of an image-like grid start (matplotlib's
/// `origin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContourOrigin {
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
    /// `[0, columns, 0, rows]`), rows reversed for [`ContourOrigin::Upper`].
    Implicit {
        /// Image-like placement.
        origin: Option<ContourOrigin>,
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
    /// The contouring algorithm (default `contour.algorithm`).
    pub algorithm: Option<ContourAlgorithm>,
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
            algorithm: None,
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

    /// The same values contoured by `algorithm`.
    #[must_use]
    pub fn with_algorithm(mut self, algorithm: ContourAlgorithm) -> Self {
        self.algorithm = Some(algorithm);
        self
    }

    /// The algorithm: as asked, else `contour.algorithm`.
    fn effective_algorithm(&self, rc: &RcParams) -> ContourAlgorithm {
        self.algorithm.unwrap_or(rc.contour_algorithm)
    }

    /// Whether corners are masked: as asked, else `contour.corner_mask`,
    /// except that the `mpl2005` algorithm cannot mask corners, so
    /// matplotlib turns them off for it unless asked.
    fn effective_corner_mask(&self, algorithm: ContourAlgorithm, rc: &RcParams) -> bool {
        self.corner_mask
            .unwrap_or(rc.contour_corner_mask && algorithm.supports_corner_mask())
    }

    /// Checks the shapes and builds the contour generator, masking invalid
    /// (and, when log-scaled, non-positive) values. The data limits cover
    /// the points mapped by `to_data` when the grid is not in data
    /// coordinates.
    pub(crate) fn source(
        &self,
        logscale: bool,
        rc: &RcParams,
        to_data: Option<&ResolvedTransform>,
    ) -> Result<ContourSource<QuadContourGenerator>, ContourError> {
        let (rows, cols) = self.z.shape();
        if rows < 2 || cols < 2 {
            return Err(ContourError::GridTooSmall { rows, cols });
        }
        let coords = self.grid_coordinates(rc)?;
        let algorithm = self.effective_algorithm(rc);
        let corner_mask = self.effective_corner_mask(algorithm, rc);
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
        let (mut zmin, zmax) = extremes(self.z.as_slice(), &mask).unzip();
        let mut warnings = Vec::new();
        if logscale && zmin.is_some_and(|z| z <= 0.0) {
            mask.iter_mut()
                .zip(self.z.as_slice())
                .for_each(|(m, &z)| *m |= z <= 0.0);
            warnings.push(ContourWarning::NonPositiveMasked);
            zmin = extremes(self.z.as_slice(), &mask).map(|(min, _)| min);
        }
        let mask = Grid::from_vec(rows, cols, mask).expect("the mask has the shape of z");
        let chunk = (self.nchunk, self.nchunk);
        let data_lim = data_lim(coords.points(cols, rows), to_data);
        let generator = QuadContourGenerator::with_coordinates(
            algorithm,
            coords,
            &self.z,
            Some(&mask),
            corner_mask,
            chunk,
        )?;
        Ok(ContourSource {
            generator,
            zmin,
            zmax,
            data_lim,
            boolean: self.boolean,
            warnings,
        })
    }

    /// What the field's contour generator is made from, the defaults
    /// resolved with `rc` (matplotlib's arguments to
    /// `contourpy.contour_generator`).
    ///
    /// ```
    /// use pltrs::contour::{ContourAlgorithm, QuadField};
    /// use pltrs::numeric::Grid;
    /// use pltrs::rc::RcParams;
    /// let field = QuadField::new(Grid::from_vec(2, 3, vec![0.0; 6]).unwrap())
    ///     .with_algorithm(ContourAlgorithm::Mpl2005);
    /// let arguments = field.generator_arguments(&RcParams::default())?;
    /// assert_eq!(arguments.x.row(1), [0.0, 1.0, 2.0]);
    /// assert!(!arguments.corner_mask);
    /// # Ok::<(), pltrs::contour::ContourError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates do not fit z.
    pub fn generator_arguments(&self, rc: &RcParams) -> Result<GeneratorArguments, ContourError> {
        let (rows, cols) = self.z.shape();
        let (x, y) = match self.grid_coordinates(rc)? {
            GridCoordinates::Axes { x, y } => meshgrid(&x, &y),
            GridCoordinates::Points { x, y } => (
                Grid::from_vec(rows, cols, x).expect("per-point coordinates fit z"),
                Grid::from_vec(rows, cols, y).expect("per-point coordinates fit z"),
            ),
        };
        let algorithm = self.effective_algorithm(rc);
        Ok(GeneratorArguments {
            x,
            y,
            algorithm,
            corner_mask: self.effective_corner_mask(algorithm, rc),
        })
    }

    /// Where the points sit, checked against z.
    fn grid_coordinates(&self, rc: &RcParams) -> Result<GridCoordinates, ContourError> {
        let (rows, cols) = self.z.shape();
        match &self.coords {
            QuadCoords::Axes { x, y } => {
                if x.len() != cols {
                    return Err(ContourError::XLength { len: x.len(), cols });
                }
                if y.len() != rows {
                    return Err(ContourError::YLength { len: y.len(), rows });
                }
                Ok(GridCoordinates::Axes {
                    x: x.clone(),
                    y: y.clone(),
                })
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
                Ok(GridCoordinates::Points {
                    x: x.as_slice().to_vec(),
                    y: y.as_slice().to_vec(),
                })
            }
            QuadCoords::Implicit { origin, extent } => {
                let origin = origin.map(|o| match (o, rc.image_origin) {
                    (ContourOrigin::Image, Origin::Lower) => ContourOrigin::Lower,
                    (ContourOrigin::Image, Origin::Upper) => ContourOrigin::Upper,
                    (other, _) => other,
                });
                Ok(implicit_coordinates(rows, cols, origin, *extent))
            }
        }
    }
}

/// What the contour generator of a [`QuadField`] is made from
/// ([`QuadField::generator_arguments`]).
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratorArguments {
    /// The x of every point, shaped like z.
    pub x: Grid<f64>,
    /// The y of every point, shaped like z.
    pub y: Grid<f64>,
    /// The contouring algorithm.
    pub algorithm: ContourAlgorithm,
    /// Whether a quad with one masked point keeps the triangle opposite it.
    pub corner_mask: bool,
}

/// matplotlib's `_initialize_x_y`: the x of each column and the y of each
/// row.
fn implicit_coordinates(
    rows: usize,
    cols: usize,
    origin: Option<ContourOrigin>,
    extent: Option<[f64; 4]>,
) -> GridCoordinates {
    let Some(origin) = origin else {
        let (x, y) = match extent {
            None => (
                (0..cols).map(|j| j as f64).collect(),
                (0..rows).map(|i| i as f64).collect(),
            ),
            Some([x0, x1, y0, y1]) => (linspace(x0, x1, cols), linspace(y0, y1, rows)),
        };
        return GridCoordinates::Axes { x, y };
    };
    let [x0, x1, y0, y1] = extent.unwrap_or([0.0, cols as f64, 0.0, rows as f64]);
    let dx = (x1 - x0) / cols as f64;
    let dy = (y1 - y0) / rows as f64;
    let x: Vec<f64> = (0..cols).map(|j| x0 + (j as f64 + 0.5) * dx).collect();
    let mut y: Vec<f64> = (0..rows).map(|i| y0 + (i as f64 + 0.5) * dy).collect();
    if origin == ContourOrigin::Upper {
        y.reverse();
    }
    GridCoordinates::Axes { x, y }
}

/// The smallest and largest unmasked values (`f64::min` and `f64::max`
/// folded from the first), or `None` when all are masked; one pass.
fn extremes(values: &[f64], mask: &[bool]) -> Option<(f64, f64)> {
    let mut unmasked = values
        .iter()
        .zip(mask)
        .filter(|(_, masked)| !**masked)
        .map(|(&v, _)| v);
    let first = unmasked.next()?;
    Some(unmasked.fold((first, first), |(min, max), v| (min.min(v), max.max(v))))
}

/// numpy's `min` of two values in reduction order (NaN propagates).
fn np_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || a < b { a } else { b }
}

/// numpy's `max` of two values in reduction order (NaN propagates).
fn np_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || a > b { a } else { b }
}

/// The corners of the bounding box of the grid `points`, mapped by
/// `to_data` first, all at once (matplotlib's `_mins` and `_maxs`):
/// numpy's `min` and `max` of each coordinate, reduced in one pass.
fn data_lim(
    points: impl Iterator<Item = Point>,
    to_data: Option<&ResolvedTransform>,
) -> (Point, Point) {
    match to_data {
        Some(to_data) => {
            let mut points: Vec<Point> = points.collect();
            to_data.transform_points(&mut points);
            extent(points.into_iter())
        }
        None => extent(points),
    }
}

/// The coordinate-wise numpy `min` and `max` of `points` (NaN when empty).
fn extent(mut points: impl Iterator<Item = Point>) -> (Point, Point) {
    let Some(first) = points.next() else {
        let nan = Point::new(f64::NAN, f64::NAN);
        return (nan, nan);
    };
    points.fold((first, first), |(min, max), p| {
        (
            Point::new(np_min(min.x, p.x), np_min(min.y, p.y)),
            Point::new(np_max(max.x, p.x), np_max(max.y, p.y)),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(rows: usize, cols: usize) -> QuadField {
        QuadField::new(Grid::from_fn(rows, cols, |i, j| (i * cols + j) as f64))
    }

    #[test]
    fn implicit_coordinates_follow_origin_and_extent() {
        let axes = |x: &[f64], y: &[f64]| GridCoordinates::Axes {
            x: x.to_vec(),
            y: y.to_vec(),
        };
        let implicit = implicit_coordinates(2, 3, None, None);
        assert_eq!(implicit, axes(&[0.0, 1.0, 2.0], &[0.0, 1.0]));
        assert_eq!(implicit.at(4, 3), Point::new(1.0, 1.0));
        let spread = implicit_coordinates(2, 3, None, Some([10.0, 20.0, 0.0, 1.0]));
        assert_eq!(spread, axes(&[10.0, 15.0, 20.0], &[0.0, 1.0]));
        let upper = implicit_coordinates(2, 4, Some(ContourOrigin::Upper), None);
        assert_eq!(upper, axes(&[0.5, 1.5, 2.5, 3.5], &[1.5, 0.5]));
    }

    #[test]
    fn source_masks_invalid_and_non_positive_values() {
        let mut z = field(3, 3).z;
        z[(0, 0)] = f64::NAN;
        let rc = RcParams::default();
        let source = QuadField::new(z.clone()).source(false, &rc, None).unwrap();
        assert_eq!((source.zmin, source.zmax), (Some(1.0), Some(8.0)));
        z[(0, 1)] = -2.0;
        let source = QuadField::new(z).source(true, &rc, None).unwrap();
        assert_eq!(source.zmin, Some(2.0));
        assert_eq!(source.warnings, [ContourWarning::NonPositiveMasked]);
    }

    #[test]
    fn mpl2005_masks_no_corners_unless_asked() {
        let mut rc = RcParams::default();
        let corners = |field: &QuadField, rc: &RcParams| {
            field.effective_corner_mask(field.effective_algorithm(rc), rc)
        };
        assert!(corners(&field(2, 2), &rc));
        rc.set("contour.algorithm", "mpl2005").unwrap();
        assert!(!corners(&field(2, 2), &rc));
        let asked = QuadField {
            corner_mask: Some(true),
            ..field(2, 2)
        };
        assert!(corners(&asked, &rc));
        assert!(
            matches!(
                asked.source(false, &rc, None),
                Err(ContourError::CornerMaskUnsupported(
                    ContourAlgorithm::Mpl2005
                ))
            ),
            "matplotlib's ValueError"
        );
        let serial = field(2, 2).with_algorithm(ContourAlgorithm::Serial);
        assert!(corners(&serial, &rc), "the keyword overrides the rc");
        rc.set("contour.corner_mask", "False").unwrap();
        assert!(!corners(&serial, &rc));
    }

    #[test]
    fn shape_errors() {
        let rc = RcParams::default();
        assert!(matches!(
            field(1, 3).source(false, &rc, None),
            Err(ContourError::GridTooSmall { .. })
        ));
        let wrong_x = field(3, 3).with_coords(QuadCoords::Axes {
            x: vec![0.0, 1.0],
            y: vec![0.0, 1.0, 2.0],
        });
        assert!(matches!(
            wrong_x.source(false, &rc, None),
            Err(ContourError::XLength { len: 2, cols: 3 })
        ));
        let all_nan = QuadField::new(Grid::filled(2, 2, f64::NAN))
            .source(false, &rc, None)
            .unwrap();
        assert_eq!((all_nan.zmin, all_nan.zmax), (None, None));
    }
}
