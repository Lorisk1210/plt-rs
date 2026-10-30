//! The cells of meshes: how a quad mesh is shaded, and the polygons of a
//! mesh drawn cell by cell (matplotlib's `PolyQuadMesh`, which `pcolor`
//! makes), which follow the mask of the values mapped to colours.

use pltrs_core::{Path, Point};

use super::{Collection, CollectionArtist, MeshShading};
use crate::artist::ArtistHandle;
use crate::numeric::Grid;

/// The polygons of the cells of the vertex grid `grid` whose value in
/// `values` (one per cell, row by row) is set and whose corners are
/// finite, with those values (matplotlib's
/// `PolyQuadMesh._get_unmasked_polys`).
pub(crate) fn unmasked_cells(
    grid: &Grid<Point>,
    values: &[Option<f64>],
) -> (Vec<Path>, Vec<Option<f64>>) {
    let (rows, cols) = grid.shape();
    let (rows, cols) = (rows.saturating_sub(1), cols.saturating_sub(1));
    (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .filter_map(|(r, c)| {
            let corners =
                [(r, c), (r + 1, c), (r + 1, c + 1), (r, c + 1), (r, c)].map(|(r, c)| grid[(r, c)]);
            let value = values[r * cols + c];
            (value.is_some() && corners.iter().all(|p| p.is_finite()))
                .then(|| (Path::polygon(corners), value))
        })
        .unzip()
}

impl CollectionArtist {
    /// `values` for the colour mapping: for a mesh drawn cell by cell and
    /// given one value per cell, the polygons become those of the cells
    /// left unmasked, and the values theirs (matplotlib's
    /// `PolyQuadMesh.set_array`); any other collection keeps its paths and
    /// takes the values as they are.
    pub(super) fn mask_polygon_cells(&mut self, values: Vec<Option<f64>>) -> Vec<Option<f64>> {
        let Some(grid) = &self.polygon_grid else {
            return values;
        };
        let (rows, cols) = grid.shape();
        if values.len() != rows.saturating_sub(1) * cols.saturating_sub(1) {
            return values;
        }
        let (paths, kept) = unmasked_cells(grid, &values);
        self.paths = paths;
        kept
    }
}

impl Collection {
    /// How the cells of a quad mesh are coloured; a mesh drawn cell by
    /// cell is flat; `None` for other collections (matplotlib's
    /// `_MeshData._shading`).
    pub fn mesh_shading(&self) -> Option<MeshShading> {
        self.raw().with::<CollectionArtist, _>(|c| {
            c.mesh
                .as_ref()
                .map(|mesh| mesh.shading)
                .or(c.polygon_grid.as_ref().map(|_| MeshShading::Flat))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid<Point> {
        Grid::from_fn(3, 3, |r, c| Point::new(c as f64, r as f64))
    }

    #[test]
    fn masked_values_and_corners_drop_their_cells() {
        let values = [Some(0.0), None, Some(2.0), Some(3.0)];
        let (paths, kept) = unmasked_cells(&grid(), &values);
        assert_eq!(kept, [Some(0.0), Some(2.0), Some(3.0)]);
        assert_eq!(paths[1].vertices()[0], Point::new(0.0, 1.0));
        let mut holed = grid();
        holed[(0, 0)] = Point::new(f64::NAN, 0.0);
        assert_eq!(unmasked_cells(&holed, &[Some(1.0); 4]).1.len(), 3);
    }
}
