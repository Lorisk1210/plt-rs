//! Flat-shaded quadrilateral meshes (the arguments of matplotlib's
//! `draw_quad_mesh`).

use plt_core::{Affine2D, Path, Point, Rgba};

use crate::collection::PathCollection;

/// A grid of quadrilaterals, one colour per cell.
#[derive(Clone, Copy, Debug)]
pub struct QuadMesh<'a> {
    /// Maps the vertices to display coordinates.
    pub master_transform: Affine2D,
    /// Cells per row (matplotlib's `meshWidth`).
    pub columns: usize,
    /// The `(rows + 1) × (columns + 1)` vertices, row by row.
    pub coordinates: &'a [Point],
    /// Cell colours, row by row; empty = no fills.
    pub facecolors: &'a [Rgba],
    /// Edge colours; empty = no strokes.
    pub edgecolors: &'a [Rgba],
    /// Whether the cells are anti-aliased.
    pub antialiased: bool,
}

impl QuadMesh<'_> {
    /// Vertex `(row, col)`.
    fn vertex(&self, row: usize, col: usize) -> Point {
        self.coordinates[row * (self.columns + 1) + col]
    }

    /// The cells, row by row, each outlined through the given corners
    /// (offsets of row and column from the cell's first vertex).
    fn cells(&self, corners: [(usize, usize); 4]) -> Vec<Path> {
        let stride = self.columns + 1;
        let rows = (self.coordinates.len() / stride.max(1)).saturating_sub(1);
        (0..rows)
            .flat_map(|r| (0..self.columns).map(move |c| (r, c)))
            .map(|(r, c)| {
                let [a, b, d, e] = corners.map(|(dr, dc)| self.vertex(r + dr, c + dc));
                Path::polyline(vec![a, b, d, e, a])
            })
            .collect()
    }

    /// The cells as matplotlib's `QuadMesh._convert_mesh_to_paths` outlines
    /// them: `(r, c) → (r, c + 1) → (r + 1, c + 1) → (r + 1, c) → (r, c)`.
    pub fn cell_paths(&self) -> Vec<Path> {
        self.cells([(0, 0), (0, 1), (1, 1), (1, 0)])
    }

    /// The cells as Agg's `QuadMeshGenerator` walks them:
    /// `(r, c) → (r + 1, c) → (r + 1, c + 1) → (r, c + 1) → (r, c)`.
    pub(crate) fn agg_cell_paths(&self) -> Vec<Path> {
        self.cells([(0, 0), (1, 0), (1, 1), (0, 1)])
    }

    /// The path collection drawing `paths` with the mesh's colours, one
    /// line width (`linewidths`) and anti-aliasing flag (`antialiased`).
    pub(crate) fn collection<'p>(
        &self,
        paths: &'p [Path],
        linewidths: &'p [f64],
        antialiased: &'p [bool],
    ) -> PathCollection<'p>
    where
        Self: 'p,
    {
        PathCollection {
            master_transform: self.master_transform,
            paths,
            facecolors: self.facecolors,
            edgecolors: self.edgecolors,
            linewidths,
            antialiased,
            ..PathCollection::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_in_matplotlib_and_agg_order() {
        let coordinates: Vec<Point> = (0..2)
            .flat_map(|r| (0..3).map(move |c| Point::new(f64::from(c), f64::from(r))))
            .collect();
        let mesh = QuadMesh {
            master_transform: Affine2D::IDENTITY,
            columns: 2,
            coordinates: &coordinates,
            facecolors: &[],
            edgecolors: &[],
            antialiased: false,
        };
        let numpy = mesh.cell_paths();
        assert_eq!(numpy.len(), 2);
        assert_eq!(
            numpy[1].vertices(),
            &[
                Point::new(1.0, 0.0),
                Point::new(2.0, 0.0),
                Point::new(2.0, 1.0),
                Point::new(1.0, 1.0),
                Point::new(1.0, 0.0)
            ]
        );
        assert_eq!(mesh.agg_cell_paths()[1].vertices()[1], Point::new(1.0, 1.0));
        let empty = QuadMesh {
            coordinates: &coordinates[..3],
            ..mesh
        };
        assert_eq!(empty.cell_paths(), Vec::<Path>::new());
    }
}
