//! Surfaces and wireframes over a grid (matplotlib's `plot_surface` and
//! `plot_wireframe`).

use super::art3d::ZSort;
use super::collections3d::{FaceColors, NewPolys, PolyFaces, resolve_all};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::columns;
use super::shade::LightSource;
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;
use crate::numeric::Grid;

/// How densely a grid is sampled (matplotlib's `rstride`/`cstride` and
/// `rcount`/`ccount`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sampling {
    /// At most this many samples along the rows and the columns.
    Counts {
        /// Samples down the rows.
        rows: usize,
        /// Samples across the columns.
        cols: usize,
    },
    /// Every `rows`-th row and `cols`-th column (plus the last ones).
    Strides {
        /// Row step.
        rows: usize,
        /// Column step.
        cols: usize,
    },
}

impl Default for Sampling {
    /// At most 50 samples each way.
    fn default() -> Self {
        Self::Counts { rows: 50, cols: 50 }
    }
}

impl Sampling {
    /// The row and column strides for a grid of `rows × cols`; a count of
    /// zero gives a stride of zero (no lines in that direction).
    fn strides(self, rows: usize, cols: usize) -> (usize, usize) {
        match self {
            Self::Strides { rows, cols } => (rows, cols),
            Self::Counts {
                rows: row_count,
                cols: col_count,
            } => (count_stride(rows, row_count), count_stride(cols, col_count)),
        }
    }
}

/// The stride sampling `len` values at most `count` times.
fn count_stride(len: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        len.div_ceil(count).max(1)
    }
}

/// Options of [`Axes3D::plot_surface`].
#[derive(Clone, Debug, Default)]
pub struct SurfaceOptions {
    /// Which grid points become polygon corners.
    pub sampling: Sampling,
    /// How the faces are coloured.
    pub colors: FaceColors,
    /// One colour per grid cell, overriding `colors`; the edges take the
    /// same colours (matplotlib's `facecolors`).
    pub cell_colors: Option<Grid<Color>>,
    /// Whether colours are shaded by the light; by default unless
    /// colour-mapped.
    pub shade: Option<bool>,
    /// The light shading the faces.
    pub lightsource: Option<LightSource>,
    /// Further collection properties (alpha, edge colour, width, ...).
    pub props: CollectionProps,
    /// Whether the points outside the view limits are hidden, breaking the
    /// faces reaching them (matplotlib's `axlim_clip`).
    pub axlim_clip: bool,
}

/// Options of [`Axes3D::plot_wireframe`].
#[derive(Clone, Debug, Default)]
pub struct WireframeOptions {
    /// Which rows and columns are drawn; a zero stride or count draws no
    /// lines in that direction.
    pub sampling: Sampling,
    /// Line properties.
    pub props: CollectionProps,
    /// Whether the points outside the view limits are hidden, breaking the
    /// lines there (matplotlib's `axlim_clip`).
    pub axlim_clip: bool,
}

/// The x, y and z grids of a surface, checked to share one shape.
pub(crate) struct SurfaceGrid<'a> {
    x: &'a Grid<f64>,
    y: &'a Grid<f64>,
    z: &'a Grid<f64>,
}

impl<'a> SurfaceGrid<'a> {
    pub(crate) fn new(
        x: &'a Grid<f64>,
        y: &'a Grid<f64>,
        z: &'a Grid<f64>,
    ) -> Result<Self, Axes3DError> {
        let len = z.rows() * z.cols();
        check_len("x", len, x.rows() * x.cols())?;
        check_len("y", len, y.rows() * y.cols())?;
        check_len("x columns", z.cols(), x.cols())?;
        check_len("y columns", z.cols(), y.cols())?;
        Ok(Self { x, y, z })
    }

    fn point(&self, (row, col): (usize, usize)) -> Vec3 {
        [self.x[(row, col)], self.y[(row, col)], self.z[(row, col)]]
    }

    /// The grids reordered so that `zdir` becomes z (matplotlib's
    /// `rotate_axes(X, Y, Z, zdir)`).
    pub(crate) fn rotated(&self, zdir: Axis3) -> [Grid<f64>; 3] {
        let (rows, cols) = self.z.shape();
        [0, 1, 2].map(|k| Grid::from_fn(rows, cols, |i, j| zdir.rotate(self.point((i, j)))[k]))
    }

    /// The sampled row (or column) indices: every `stride`-th one plus
    /// the last (matplotlib's `row_inds` in `plot_surface`).
    fn sample(len: usize, stride: usize) -> Vec<usize> {
        if len == 0 {
            return Vec::new();
        }
        let mut indices: Vec<usize> = (0..len - 1).step_by(stride).collect();
        indices.push(len - 1);
        indices
    }

    /// One polygon per block between sampled rows and columns, tracing
    /// every grid point on the block's perimeter (matplotlib's
    /// `_array_perimeter`), with the top-left cell of each block.
    fn blocks(&self, rstride: usize, cstride: usize) -> Vec<(Vec<Vec3>, (usize, usize))> {
        let rows = Self::sample(self.z.rows(), rstride);
        let cols = Self::sample(self.z.cols(), cstride);
        let mut blocks = Vec::new();
        for r in rows.windows(2) {
            for c in cols.windows(2) {
                let (r0, r1, c0, c1) = (r[0], r[1], c[0], c[1]);
                let perimeter = (c0..c1)
                    .map(|c| (r0, c))
                    .chain((r0..r1).map(|r| (r, c1)))
                    .chain((c0 + 1..=c1).rev().map(|c| (r1, c)))
                    .chain((r0 + 1..=r1).rev().map(|r| (r, c0)))
                    .map(|cell| self.point(cell))
                    .collect();
                blocks.push((perimeter, (r0, c0)));
            }
        }
        blocks
    }

    /// The lines along every `stride`-th row (or column, `along_rows =
    /// false`) plus the last (matplotlib's `rii`/`cii` in
    /// `plot_wireframe`).
    fn lines(&self, stride: usize, along_rows: bool) -> Vec<Vec<Vec3>> {
        let (count, len) = if along_rows {
            (self.z.rows(), self.z.cols())
        } else {
            (self.z.cols(), self.z.rows())
        };
        if stride == 0 || count * len == 0 {
            return Vec::new();
        }
        let mut indices: Vec<usize> = (0..count).step_by(stride).collect();
        if (count - 1) % stride != 0 {
            indices.push(count - 1);
        }
        indices
            .into_iter()
            .map(|i| {
                (0..len)
                    .map(|j| self.point(if along_rows { (i, j) } else { (j, i) }))
                    .collect()
            })
            .collect()
    }
}

impl Axes3D {
    /// A surface of polygons through the grid points `(x, y, z)`
    /// (matplotlib's `plot_surface`).
    ///
    /// # Errors
    ///
    /// When the grids differ in shape, a stride is zero, the cell colours
    /// do not cover the grid, or a colour is invalid.
    pub fn plot_surface(
        &self,
        x: &Grid<f64>,
        y: &Grid<f64>,
        z: &Grid<f64>,
        options: SurfaceOptions,
    ) -> Result<Collection, Axes3DError> {
        let grid = SurfaceGrid::new(x, y, z)?;
        let had_data = self.has_data();
        let (rstride, cstride) = options.sampling.strides(z.rows(), z.cols());
        if rstride == 0 || cstride == 0 {
            return Err(Axes3DError::NoStride);
        }
        let (faces, cells): (Vec<Vec<Vec3>>, Vec<(usize, usize)>) = grid
            .blocks(rstride, cstride)
            .into_iter()
            .filter_map(|(face, cell)| {
                let finite: Vec<Vec3> = face
                    .into_iter()
                    .filter(|p| p.iter().all(|c| c.is_finite()))
                    .collect();
                (!finite.is_empty()).then_some((finite, cell))
            })
            .unzip();
        let shade = options
            .shade
            .unwrap_or_else(|| options.cell_colors.is_some() || options.colors.shaded_by_default());
        let light = shade.then(|| options.lightsource.unwrap_or_else(LightSource::mplot3d));
        let (colors, shaded_edges) = match &options.cell_colors {
            Some(cell_colors) => {
                let picked = cells
                    .iter()
                    .map(|&cell| cell_colors.get(cell.0, cell.1).cloned())
                    .collect::<Option<Vec<Color>>>()
                    .ok_or(Axes3DError::Shape {
                        argument: "cell colours",
                        expected: z.rows() * z.cols(),
                        actual: cell_colors.rows() * cell_colors.cols(),
                    })?;
                let rgba = resolve_all(&picked)?;
                (PolyFaces::Colors(rgba.clone()), Some(rgba))
            }
            None => (self.face_colors(&options.colors, &faces)?, None),
        };
        let collection = self.add_polys(NewPolys {
            faces,
            colors,
            shaded_edges,
            light,
            zsort: ZSort::Average,
            props: options.props,
            axlim_clip: options.axlim_clip,
        })?;
        self.auto_scale_xyz(x.as_slice(), y.as_slice(), Some(z.as_slice()), had_data)?;
        Ok(collection)
    }

    /// The face colours for `colors` (the cycle colour taken from the line
    /// cycle, like matplotlib).
    pub(crate) fn face_colors(
        &self,
        colors: &FaceColors,
        faces: &[Vec<Vec3>],
    ) -> Result<PolyFaces, Axes3DError> {
        Ok(match colors {
            FaceColors::Cycle => PolyFaces::Colors(vec![self.plane().next_line_color().resolve()?]),
            FaceColors::Single(color) => PolyFaces::Colors(vec![color.resolve()?]),
            FaceColors::Mapped(cmap) => cmap.mean_z_faces(faces),
        })
    }

    /// Lines along the rows and the columns of the grid points `(x, y,
    /// z)` (matplotlib's `plot_wireframe`).
    ///
    /// # Errors
    ///
    /// When the grids differ in shape or both strides are zero.
    pub fn plot_wireframe(
        &self,
        x: &Grid<f64>,
        y: &Grid<f64>,
        z: &Grid<f64>,
        options: WireframeOptions,
    ) -> Result<Collection, Axes3DError> {
        let grid = SurfaceGrid::new(x, y, z)?;
        let had_data = self.has_data();
        let (rstride, cstride) = options.sampling.strides(z.rows(), z.cols());
        if rstride == 0 && cstride == 0 {
            return Err(Axes3DError::NoStride);
        }
        let row_lines = grid.lines(rstride, true);
        let col_lines = grid.lines(cstride, false);
        for (lines, had_data) in [(&row_lines, had_data), (&col_lines, true)] {
            let points: Vec<Vec3> = lines.iter().flatten().copied().collect();
            let [xs, ys, zs] = columns(&points);
            self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        }
        let lines = row_lines.into_iter().chain(col_lines).collect();
        Ok(self.add_segments(lines, options.props, options.axlim_clip))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: usize, cols: usize) -> Grid<f64> {
        Grid::from_fn(rows, cols, |i, j| (i * 10 + j) as f64)
    }

    #[test]
    fn strides_follow_matplotlib() {
        assert_eq!(Sampling::default().strides(40, 120), (1, 3));
        assert_eq!(
            Sampling::Counts { rows: 0, cols: 50 }.strides(40, 40),
            (0, 1)
        );
        assert_eq!(
            Sampling::Strides { rows: 4, cols: 6 }.strides(40, 40),
            (4, 6)
        );
    }

    #[test]
    fn blocks_trace_their_perimeter() {
        let g = grid(3, 3);
        let surface = SurfaceGrid::new(&g, &g, &g).expect("same shapes");
        let blocks = surface.blocks(2, 2);
        assert_eq!(blocks.len(), 1);
        let zs: Vec<f64> = blocks[0].0.iter().map(|p| p[2]).collect();
        assert_eq!(zs, [0.0, 1.0, 2.0, 12.0, 22.0, 21.0, 20.0, 10.0]);
        // A stride that does not divide the grid keeps the last row.
        let g = grid(4, 2);
        let surface = SurfaceGrid::new(&g, &g, &g).expect("same shapes");
        let corners: Vec<(usize, usize)> =
            surface.blocks(2, 1).into_iter().map(|(_, c)| c).collect();
        assert_eq!(corners, [(0, 0), (2, 0)]);
    }

    #[test]
    fn wireframe_lines_include_the_last_row() {
        let g = grid(5, 3);
        let surface = SurfaceGrid::new(&g, &g, &g).expect("same shapes");
        assert_eq!(surface.lines(3, true).len(), 3);
        assert_eq!(surface.lines(2, true).len(), 3);
        assert_eq!(surface.lines(0, true).len(), 0);
        assert_eq!(surface.lines(1, false).len(), 3);
    }

    #[test]
    fn mismatched_grids_are_rejected() {
        let (a, b) = (grid(2, 3), grid(3, 2));
        assert!(SurfaceGrid::new(&a, &b, &a).is_err());
    }
}
