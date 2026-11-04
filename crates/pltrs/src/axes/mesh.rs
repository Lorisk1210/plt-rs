//! The data of pseudocolour plots: colour-mapped values with optional x
//! and y coordinates and the shading, prepared into full vertex grids as
//! matplotlib's `_pcolorargs` prepares them.

use std::fmt;
use std::str::FromStr;

use pltrs_core::Point;

use crate::collections::MeshShading;
use crate::image::{ImageError, MultiField, ScalarField};
use crate::numeric::{Grid, cell_buffer};
use crate::rc::UnknownName;

/// Where the colour values of a mesh sit (matplotlib's `shading`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Shading {
    /// `Flat` when the grid is one larger than the values, else `Nearest`.
    #[default]
    Auto,
    /// Values fill the cells between the grid vertices.
    Flat,
    /// Values sit at the grid points; cells are centred on them.
    Nearest,
    /// Values sit at the grid points and are interpolated across cells.
    Gouraud,
}

pltrs_core::archive_enum!(Shading {
    Auto,
    Flat,
    Nearest,
    Gouraud,
});

impl Shading {
    const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Flat => "flat",
            Self::Nearest => "nearest",
            Self::Gouraud => "gouraud",
        }
    }
}

impl fmt::Display for Shading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Shading {
    type Err = UnknownName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        [Self::Auto, Self::Flat, Self::Nearest, Self::Gouraud]
            .into_iter()
            .find(|shading| shading.name() == s.to_ascii_lowercase())
            .ok_or_else(|| UnknownName {
                kind: "shading",
                name: s.to_owned(),
            })
    }
}

/// The coordinates of a mesh along one direction.
#[derive(Clone, Debug, PartialEq)]
pub enum MeshCoords {
    /// One value per column (for x) or per row (for y).
    Vector(Vec<f64>),
    /// One value per grid point.
    Grid(Grid<f64>),
}

impl From<Vec<f64>> for MeshCoords {
    fn from(values: Vec<f64>) -> Self {
        Self::Vector(values)
    }
}

impl From<&[f64]> for MeshCoords {
    fn from(values: &[f64]) -> Self {
        Self::Vector(values.to_vec())
    }
}

impl<const N: usize> From<[f64; N]> for MeshCoords {
    fn from(values: [f64; N]) -> Self {
        Self::Vector(values.to_vec())
    }
}

impl From<Grid<f64>> for MeshCoords {
    fn from(values: Grid<f64>) -> Self {
        Self::Grid(values)
    }
}

/// The colour-mapped values of a pseudocolour plot (matplotlib's `C`).
#[derive(Clone, Debug, PartialEq)]
pub enum MeshValues {
    /// Scalars, mapped through a norm and a colormap.
    Scalar(ScalarField),
    /// Several variates, mapped through one norm each and a bivariate or
    /// multivariate colormap (`pcolormesh` only, as in matplotlib 3.11).
    Multivariate(MultiField),
}

impl MeshValues {
    /// `(rows, columns)`.
    pub fn shape(&self) -> (usize, usize) {
        match self {
            Self::Scalar(field) => field.values().shape(),
            Self::Multivariate(field) => field.shape(),
        }
    }
}

impl From<ScalarField> for MeshValues {
    fn from(field: ScalarField) -> Self {
        Self::Scalar(field)
    }
}

impl From<Grid<f64>> for MeshValues {
    fn from(values: Grid<f64>) -> Self {
        Self::Scalar(values.into())
    }
}

impl From<MultiField> for MeshValues {
    fn from(field: MultiField) -> Self {
        Self::Multivariate(field)
    }
}

/// The data of a pseudocolour plot: values and, optionally, their x and
/// y coordinates (matplotlib's `[X, Y,] C`).
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// The coordinates; `None` uses the column and row indices.
    pub coords: Option<(MeshCoords, MeshCoords)>,
    /// The colour-mapped values, `(rows, columns)`.
    pub values: MeshValues,
}

impl Mesh {
    /// Values on the grid of column and row indices.
    pub fn new(values: impl Into<MeshValues>) -> Self {
        Self {
            coords: None,
            values: values.into(),
        }
    }

    /// Values at explicit coordinates.
    pub fn from_coords(
        x: impl Into<MeshCoords>,
        y: impl Into<MeshCoords>,
        values: impl Into<MeshValues>,
    ) -> Self {
        Self {
            coords: Some((x.into(), y.into())),
            values: values.into(),
        }
    }
}

/// A mesh after matplotlib's `_pcolorargs`: full vertex grids and the
/// resolved shading.
pub(super) struct PreparedMesh {
    pub(super) x: Grid<f64>,
    pub(super) y: Grid<f64>,
    pub(super) values: MeshValues,
    pub(super) shading: MeshShading,
}

impl PreparedMesh {
    /// Vertex `(r, c)`.
    pub(super) fn point(&self, r: usize, c: usize) -> Point {
        Point::new(self.x[(r, c)], self.y[(r, c)])
    }

    /// The vertices as a grid.
    pub(super) fn vertex_grid(&self) -> Grid<Point> {
        let (rows, cols) = self.x.shape();
        Grid::from_fn(rows, cols, |r, c| self.point(r, c))
    }

    /// Every finite vertex, row by row.
    pub(super) fn finite_points(&self) -> Vec<Point> {
        let (rows, cols) = self.x.shape();
        let mut points = cell_buffer(rows, cols);
        for r in 0..rows {
            points.extend(
                (0..cols)
                    .map(|c| self.point(r, c))
                    .filter(|p| p.is_finite()),
            );
        }
        points
    }
}

/// The plotting function a mesh is prepared for, whose name matplotlib's
/// messages give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MeshFunction {
    /// `pcolor`, which takes non-finite coordinates.
    Pcolor,
    /// `pcolormesh`, which rejects them.
    Pcolormesh,
}

impl MeshFunction {
    fn name(self) -> &'static str {
        match self {
            Self::Pcolor => "pcolor",
            Self::Pcolormesh => "pcolormesh",
        }
    }
}

/// matplotlib's `_pcolorargs`: builds the vertex grids, resolves `auto`
/// shading and turns `nearest` into flat cells around the points, warning
/// when the centres are not monotonic along their axis; `pcolormesh`
/// rejects non-finite coordinates.
pub(super) fn pcolor_args(
    mesh: Mesh,
    shading: Shading,
    function: MeshFunction,
) -> Result<PreparedMesh, ImageError> {
    let (nrows, ncols) = mesh.values.shape();
    let (x, y, shading) = match mesh.coords {
        None => {
            let (w, h, shading) = match shading {
                Shading::Gouraud | Shading::Nearest => (ncols, nrows, shading),
                Shading::Auto | Shading::Flat => (ncols + 1, nrows + 1, Shading::Flat),
            };
            let x = Grid::from_fn(h, w, |_, c| c as f64);
            let y = Grid::from_fn(h, w, |r, _| r as f64);
            (x, y, shading)
        }
        Some((x, y)) => {
            let finite = |coords: &MeshCoords| match coords {
                MeshCoords::Vector(v) => v.iter().all(|v| v.is_finite()),
                MeshCoords::Grid(g) => g.as_slice().iter().all(|v| v.is_finite()),
            };
            if function == MeshFunction::Pcolormesh && !(finite(&x) && finite(&y)) {
                return Err(ImageError::InvalidCoordinates);
            }
            let (x, y) = full_grids(x, y)?;
            (x, y, shading)
        }
    };
    let (ny, nx) = x.shape();
    let shading = match shading {
        Shading::Auto if ncols == nx && nrows == ny => Shading::Nearest,
        Shading::Auto => Shading::Flat,
        other => other,
    };
    let mismatch = |shading: Shading| ImageError::MeshShape {
        shading: shading.name(),
        values: (nrows, ncols),
        nx,
        ny,
    };
    let (x, y, shading) = match shading {
        Shading::Flat => {
            if (nx, ny) != (ncols + 1, nrows + 1) {
                return Err(mismatch(shading));
            }
            (x, y, MeshShading::Flat)
        }
        Shading::Nearest | Shading::Gouraud | Shading::Auto => {
            if (nx, ny) != (ncols, nrows) {
                return Err(mismatch(shading));
            }
            if shading == Shading::Gouraud {
                (x, y, MeshShading::Gouraud)
            } else {
                let monotonic = Some(function);
                let (x, y) = (interp_columns(&x, monotonic), interp_columns(&y, None));
                let x = interp_columns(&x.transposed(), None).transposed();
                let y = interp_columns(&y.transposed(), monotonic).transposed();
                (x, y, MeshShading::Flat)
            }
        }
    };
    Ok(PreparedMesh {
        x,
        y,
        values: mesh.values,
        shading,
    })
}

/// Repeats vector coordinates into grids of the same shape (x along the
/// rows, y along the columns).
fn full_grids(x: MeshCoords, y: MeshCoords) -> Result<(Grid<f64>, Grid<f64>), ImageError> {
    let nx = match &x {
        MeshCoords::Vector(v) => v.len(),
        MeshCoords::Grid(g) => g.cols(),
    };
    let ny = match &y {
        MeshCoords::Vector(v) => v.len(),
        MeshCoords::Grid(g) => g.rows(),
    };
    let x = match x {
        MeshCoords::Grid(g) if g.rows() != 1 => g,
        MeshCoords::Grid(g) => Grid::from_fn(ny, nx, |_, c| g[(0, c)]),
        MeshCoords::Vector(v) => Grid::from_fn(ny, nx, |_, c| v[c]),
    };
    let y = match y {
        MeshCoords::Grid(g) if g.cols() != 1 => g,
        MeshCoords::Grid(g) => Grid::from_fn(ny, nx, |r, _| g[(r, 0)]),
        MeshCoords::Vector(v) => Grid::from_fn(ny, nx, |r, _| v[r]),
    };
    if x.shape() == y.shape() {
        Ok((x, y))
    } else {
        Err(ImageError::IncompatibleCoordinates)
    }
}

/// matplotlib's `_interp_grid`: cell edges halfway between the columns,
/// extrapolated by half a step at both ends (a single column is doubled).
/// With the function the centres are given to, it warns, as matplotlib
/// does, when they neither increase nor decrease along every row (steps
/// from or to a missing centre, NaN, are left out, as matplotlib leaves
/// out masked ones).
fn interp_columns(a: &Grid<f64>, monotonic: Option<MeshFunction>) -> Grid<f64> {
    let (rows, cols) = a.shape();
    if cols <= 1 {
        return Grid::from_fn(rows, 2 * cols, |r, c| a[(r, c % cols.max(1))]);
    }
    if let Some(function) = monotonic {
        let steps: Vec<f64> = (0..rows)
            .flat_map(|r| (1..cols).map(move |c| (r, c)))
            .map(|(r, c)| a[(r, c)] - a[(r, c - 1)])
            .filter(|d| !d.is_nan())
            .collect();
        if !(steps.iter().all(|&d| d >= 0.0) || steps.iter().all(|&d| d <= 0.0)) {
            let name = function.name();
            crate::warnings::warn(&format!(
                "The input coordinates to {name} are interpreted as cell centers, but \
                 are not monotonically increasing or decreasing. This may lead to \
                 incorrectly calculated cell edges, in which case, please supply explicit \
                 cell edges to {name}."
            ));
        }
    }
    Grid::from_fn(rows, cols + 1, |r, c| {
        let half = |c: usize| (a[(r, c + 1)] - a[(r, c)]) * 0.5;
        match c {
            0 => a[(r, 0)] - half(0),
            c if c == cols => a[(r, cols - 1)] + half(cols - 2),
            c => a[(r, c - 1)] + half(c - 1),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::counting_field;

    #[test]
    fn auto_shading_follows_the_grid_size() {
        let flat = pcolor_args(
            Mesh::from_coords([0.0, 1.0, 2.0], [0.0, 1.0], counting_field(1, 2)),
            Shading::Auto,
            MeshFunction::Pcolormesh,
        )
        .unwrap();
        assert_eq!(flat.x.shape(), (2, 3));
        let nearest = pcolor_args(
            Mesh::from_coords([1.0, 2.0, 4.0, 8.0], [0.0, 1.0, 3.0], counting_field(3, 4)),
            Shading::Auto,
            MeshFunction::Pcolormesh,
        )
        .unwrap();
        assert_eq!(nearest.shading, MeshShading::Flat);
        assert_eq!(nearest.x.row(0), &[0.5, 1.5, 3.0, 6.0, 10.0]);
        assert_eq!(nearest.y.column(0), vec![-0.5, 0.5, 2.0, 4.0]);
    }

    #[test]
    fn mismatched_grids_are_rejected() {
        let err = pcolor_args(
            Mesh::from_coords([0.0, 1.0], [0.0, 1.0], counting_field(3, 3)),
            Shading::Flat,
            MeshFunction::Pcolormesh,
        );
        assert!(matches!(
            err,
            Err(ImageError::MeshShape {
                shading: "flat",
                ..
            })
        ));
        let x = Grid::filled(2, 3, 0.0);
        let y = Grid::filled(3, 2, 0.0);
        assert_eq!(
            full_grids(MeshCoords::Grid(x), MeshCoords::Grid(y)).err(),
            Some(ImageError::IncompatibleCoordinates)
        );
        let nan = pcolor_args(
            Mesh::from_coords([0.0, f64::NAN, 2.0], [0.0, 1.0], counting_field(1, 2)),
            Shading::Flat,
            MeshFunction::Pcolormesh,
        );
        assert_eq!(nan.err(), Some(ImageError::InvalidCoordinates));
    }

    #[test]
    fn single_columns_are_doubled() {
        let a = Grid::from_vec(2, 1, vec![3.0, 4.0]).unwrap();
        assert_eq!(interp_columns(&a, None).as_slice(), &[3.0, 3.0, 4.0, 4.0]);
    }
}
