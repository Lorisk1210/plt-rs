//! Pseudocolour meshes of colours given directly instead of colour-mapped
//! values (matplotlib's `pcolormesh` of an `(M, N, 3)` or `(M, N, 4)`
//! array).

use plt_core::Rgba;

use super::Axes;
use super::mesh::{Mesh, MeshCoords, pcolor_args};
use super::pcolor::{PcolorOptions, default_shading};
use crate::collections::Collection;
use crate::colors::Color;
use crate::image::{ImageError, ScalarField};
use crate::numeric::Grid;

/// One colour per cell or grid point, at optional coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct RgbaMesh {
    /// The coordinates; `None` uses the column and row indices.
    pub coords: Option<(MeshCoords, MeshCoords)>,
    /// The colours, `(rows, columns)`.
    pub colors: Grid<Rgba>,
}

impl RgbaMesh {
    /// Colours on the grid of column and row indices.
    pub fn new(colors: Grid<Rgba>) -> Self {
        Self {
            coords: None,
            colors,
        }
    }

    /// Colours at explicit coordinates.
    pub fn from_coords(
        x: impl Into<MeshCoords>,
        y: impl Into<MeshCoords>,
        colors: Grid<Rgba>,
    ) -> Self {
        Self {
            coords: Some((x.into(), y.into())),
            colors,
        }
    }
}

impl Axes {
    /// A quadrilateral mesh painted in the given colours (matplotlib's
    /// `pcolormesh` of an RGB or RGBA array): one per cell, or with
    /// `nearest` and Gouraud shading one per grid point. The colours
    /// replace `facecolors` and the colormap options; an `alpha` in the
    /// props replaces their own, as matplotlib's does for RGB arrays.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{PcolorOptions, RgbaMesh};
    /// use plt::numeric::Grid;
    /// use plt::colors::Rgba;
    /// let ax = Figure::new().subplot();
    /// let colors = Grid::from_fn(2, 3, |r, c| Rgba::new(0.2 * c as f64, 0.5, r as f64, 1.0));
    /// ax.pcolormesh_rgba(RgbaMesh::new(colors), PcolorOptions::default())?;
    /// assert_eq!(ax.xlim(), (0.0, 3.0));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates are not finite or do not fit the colours.
    pub fn pcolormesh_rgba(
        &self,
        mesh: RgbaMesh,
        options: PcolorOptions,
    ) -> Result<Collection, ImageError> {
        let (rows, cols) = mesh.colors.shape();
        let shape_only = Mesh {
            coords: mesh.coords,
            values: ScalarField::new(Grid::from_fn(rows, cols, |_, _| 0.0)).into(),
        };
        let prepared = pcolor_args(shape_only, default_shading(options.shading), true)?;
        let mut props = options.props;
        props.facecolors = Some(
            mesh.colors
                .as_slice()
                .iter()
                .copied()
                .map(Color::from)
                .collect(),
        );
        Ok(self.add_pcolormesh(&prepared, props, options.snap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::Shading;
    use crate::figure::Figure;

    fn gray(rows: usize, cols: usize) -> Grid<Rgba> {
        Grid::from_fn(rows, cols, |_, _| Rgba::new(0.5, 0.5, 0.5, 0.5))
    }

    #[test]
    fn colours_fill_the_cells_of_the_index_grid() {
        let ax = Figure::new().subplot();
        let mesh = ax
            .pcolormesh_rgba(RgbaMesh::new(gray(5, 6)), PcolorOptions::default())
            .unwrap();
        assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 6.0), (0.0, 5.0)));
        assert_eq!(mesh.paths().len(), 30);
        assert_eq!(mesh.sticky_edges().0, vec![0.0, 6.0]);
    }

    #[test]
    fn nearest_shading_centres_cells_on_the_points() {
        let ax = Figure::new().subplot();
        let options = PcolorOptions {
            shading: Some(Shading::Nearest),
            ..PcolorOptions::default()
        };
        let mesh = RgbaMesh::from_coords([0.0, 1.0], [0.0, 2.0], gray(2, 2));
        ax.pcolormesh_rgba(mesh, options).unwrap();
        assert_eq!(ax.xlim(), (-0.5, 1.5));
    }

    #[test]
    fn coordinates_must_fit_the_colours() {
        let ax = Figure::new().subplot();
        let mesh = RgbaMesh::from_coords([0.0, 1.0, 2.0, 3.0], [0.0, 1.0], gray(1, 2));
        assert!(matches!(
            ax.pcolormesh_rgba(mesh, PcolorOptions::default()),
            Err(ImageError::MeshShape { .. })
        ));
    }
}
