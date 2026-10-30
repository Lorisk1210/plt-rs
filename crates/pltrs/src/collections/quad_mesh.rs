//! Quadrilateral meshes (matplotlib's `QuadMesh`): a grid of cells drawn
//! flat (one colour per cell) or Gouraud-shaded (colours interpolated
//! from the vertices).

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_render::QuadMesh;

use super::style::resolve_colors;
use super::{CollectionArtist, CollectionKind};
use crate::artist::{Artist, DrawContext};
use crate::numeric::Grid;
use crate::rc::RcParams;

/// How the cells of a mesh are coloured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MeshShading {
    /// One colour per cell; the vertex grid is one larger than the colours.
    #[default]
    Flat,
    /// Colours at the vertices, interpolated across each cell.
    Gouraud,
}

pltrs_core::archive_enum!(MeshShading { Flat, Gouraud });

/// The vertex grid of a quad mesh.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeshGeometry {
    /// Vertex `(row, col)` of the grid.
    pub(crate) coordinates: Grid<Point>,
    /// Flat or Gouraud shading.
    pub(crate) shading: MeshShading,
}

pltrs_core::archive_struct!(MeshGeometry {
    coordinates,
    shading
});

impl CollectionArtist {
    /// A quad mesh over `coordinates` (matplotlib's `QuadMesh`): the
    /// cells become the paths, ordered row by row.
    pub(crate) fn quad_mesh(
        coordinates: Grid<Point>,
        shading: MeshShading,
        rc: std::rc::Rc<RcParams>,
    ) -> Self {
        let mut mesh = Self::new(CollectionKind::QuadMesh, cell_paths(&coordinates), rc);
        // Picks fall inside the cells.
        mesh.pickradius = 0.0;
        mesh.mesh = Some(MeshGeometry {
            coordinates,
            shading,
        });
        mesh
    }
}

impl CollectionArtist {
    /// How many values a quad mesh maps to colours: one per cell, or one
    /// per vertex when Gouraud-shaded (`None` for other collections).
    pub(super) fn mesh_value_count(&self) -> Option<usize> {
        let mesh = self.mesh.as_ref()?;
        let (rows, cols) = mesh.coordinates.shape();
        Some(match mesh.shading {
            MeshShading::Flat => rows.saturating_sub(1) * cols.saturating_sub(1),
            MeshShading::Gouraud => rows * cols,
        })
    }
}

/// The outline of every cell, row by row, as matplotlib's
/// `QuadMesh.get_paths` gives them.
fn cell_paths(coordinates: &Grid<Point>) -> Vec<Path> {
    QuadMesh {
        master_transform: Affine2D::IDENTITY,
        columns: coordinates.shape().1.saturating_sub(1),
        coordinates: coordinates.as_slice(),
        facecolors: &[],
        edgecolors: &[],
        antialiased: false,
    }
    .cell_paths()
}

/// matplotlib's `QuadMesh.draw`: non-affine transforms are applied to the
/// vertices up front; flat meshes go through the path-collection
/// machinery with one line width and anti-aliasing flag
/// (`draw_quad_mesh`), Gouraud meshes as triangles.
pub(super) fn draw_quad_mesh(
    c: &CollectionArtist,
    mesh: &MeshGeometry,
    ctx: &mut DrawContext<'_>,
) -> Result<(), String> {
    let resolved = ctx
        .resolve(&c.transform())
        .ok_or("its coordinate system no longer exists")?;
    let transformed;
    let (coordinates, master) = if resolved.is_affine() {
        (&mesh.coordinates, resolved.affine_part())
    } else {
        transformed = mesh.coordinates.map(|&p| resolved.transform_point(p));
        (&transformed, Affine2D::IDENTITY)
    };
    let (faces, edges) = resolve_colors(c)?;
    let mut gc = ctx.base_gc(c.base());
    gc.alpha = None;
    gc.linewidth = c
        .linewidths
        .as_ref()
        .and_then(|widths| widths.first().copied())
        .unwrap_or(c.rc().patch_linewidth);
    let antialiased = c
        .antialiased
        .as_ref()
        .and_then(|flags| flags.first().copied())
        .unwrap_or(c.rc().patch_antialiased);
    ctx.renderer()
        .open_group("QuadMesh", c.base().gid.as_deref());
    match mesh.shading {
        MeshShading::Flat => {
            let mesh = QuadMesh {
                master_transform: master,
                columns: coordinates.shape().1.saturating_sub(1),
                coordinates: coordinates.as_slice(),
                facecolors: &faces,
                edgecolors: &edges,
                antialiased,
            };
            ctx.renderer().draw_quad_mesh(&gc, &mesh);
        }
        MeshShading::Gouraud => {
            let masked = c.mapping.as_ref().map(|m| {
                m.source
                    .values
                    .iter()
                    .map(Option::is_none)
                    .collect::<Vec<_>>()
            });
            let (triangles, colors) = triangles(coordinates, &faces, masked.as_deref());
            ctx.renderer()
                .draw_gouraud_triangles(&gc, &triangles, &colors, &master);
        }
    }
    ctx.renderer().close_group("QuadMesh");
    Ok(())
}

/// matplotlib's `_convert_mesh_to_triangles`: four triangles per cell
/// meeting at its centre, whose colour is the mean of the corners';
/// triangles touching a masked vertex are dropped.
fn triangles(
    coordinates: &Grid<Point>,
    colors: &[Rgba],
    masked: Option<&[bool]>,
) -> (Vec<[Point; 3]>, Vec<[Rgba; 3]>) {
    let (rows, cols) = coordinates.shape();
    let color = |r: usize, c: usize| -> [f64; 4] {
        let i = r * cols + c;
        let mut rgba = colors
            .get(i)
            .or(colors.first())
            .copied()
            .unwrap_or(Rgba::TRANSPARENT)
            .to_array();
        if masked.is_some_and(|m| m.get(i).copied().unwrap_or(false)) {
            rgba[3] = f64::NAN;
        }
        rgba
    };
    let mut triangles = Vec::new();
    let mut shades = Vec::new();
    for r in 0..rows.saturating_sub(1) {
        for c in 0..cols.saturating_sub(1) {
            let corners = [(r, c), (r, c + 1), (r + 1, c + 1), (r + 1, c)];
            let points = corners.map(|(r, c)| coordinates[(r, c)]);
            let rgba = corners.map(|(r, c)| color(r, c));
            let centre = Point::new(
                (points[0].x + points[1].x + points[2].x + points[3].x) / 4.0,
                (points[0].y + points[1].y + points[2].y + points[3].y) / 4.0,
            );
            let mean: [f64; 4] =
                std::array::from_fn(|k| (rgba[0][k] + rgba[1][k] + rgba[2][k] + rgba[3][k]) / 4.0);
            if mean[3].is_nan() {
                continue;
            }
            let to_rgba = |[r, g, b, a]: [f64; 4]| Rgba::new(r, g, b, a);
            for k in 0..4 {
                let next = (k + 1) % 4;
                triangles.push([points[k], points[next], centre]);
                shades.push([to_rgba(rgba[k]), to_rgba(rgba[next]), to_rgba(mean)]);
            }
        }
    }
    (triangles, shades)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_grid() -> Grid<Point> {
        Grid::from_fn(2, 3, |r, c| Point::new(c as f64, r as f64))
    }

    #[test]
    fn cells_follow_matplotlib_vertex_order() {
        let paths = cell_paths(&unit_grid());
        assert_eq!(paths.len(), 2);
        assert_eq!(
            paths[1].vertices(),
            &[
                Point::new(1.0, 0.0),
                Point::new(2.0, 0.0),
                Point::new(2.0, 1.0),
                Point::new(1.0, 1.0),
                Point::new(1.0, 0.0)
            ]
        );
        assert_eq!(cell_paths(&Grid::filled(1, 4, Point::ORIGIN)).len(), 0);
    }

    #[test]
    fn gouraud_cells_split_into_four_triangles() {
        let colors: Vec<Rgba> = (0..6)
            .map(|i| Rgba::new(f64::from(i) / 5.0, 0.0, 0.0, 1.0))
            .collect();
        let (triangles, shades) = triangles(&unit_grid(), &colors, None);
        assert_eq!(triangles.len(), 8);
        assert_eq!(triangles[0][2], Point::new(0.5, 0.5));
        assert!((shades[0][2].r - (0.0 + 0.2 + 0.8 + 0.6) / 4.0).abs() < 1e-12);
        let masked = [false, false, false, false, true, false];
        let (triangles, _) = super::triangles(&unit_grid(), &colors, Some(&masked));
        assert_eq!(triangles.len(), 0, "vertex 4 is shared by both cells");
    }
}
