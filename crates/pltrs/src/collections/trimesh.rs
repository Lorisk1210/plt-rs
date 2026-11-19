//! Gouraud-shaded triangle meshes (matplotlib's `TriMesh`): collections
//! whose items are the unmasked triangles of a triangulation, coloured by
//! interpolating the colours of its points.

use pltrs_core::{Path, Point, Rgba};

use super::style::resolve_colors;
use super::{Collection, CollectionArtist, CollectionKind, CollectionSpec, ColorMapping, Mapped};
use crate::artist::{Artist, ArtistHandle, DrawContext, DrawErrorKind, RawHandle, TypedHandle};
use crate::colors::{Colormap, Norm};
use crate::rc::RcParams;
use crate::tri::Triangulation;

/// Why a triangle mesh cannot be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriMeshError {
    /// Number of points of the triangulation.
    pub points: usize,
    /// Number of values given.
    pub values: usize,
}

impl std::fmt::Display for TriMeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a triangle mesh needs one value per point: {} points, {} values",
            self.points, self.values
        )
    }
}

impl std::error::Error for TriMeshError {}

/// The triangles a triangle mesh draws: the points of its triangulation
/// and the indices of its unmasked triangles.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TriangleMesh {
    pub(crate) points: Vec<Point>,
    pub(crate) triangles: Vec<[usize; 3]>,
}

pltrs_core::archive_struct!(TriangleMesh { points, triangles });

impl TriangleMesh {
    /// The points and the unmasked triangles of `triangulation`.
    fn of(triangulation: &Triangulation) -> Self {
        Self {
            points: (0..triangulation.npoints())
                .map(|i| triangulation.point(i))
                .collect(),
            triangles: triangulation.drawn_triangles(),
        }
    }

    /// The outline of each triangle, its corners in their order
    /// (matplotlib's `TriMesh.convert_mesh_to_paths`).
    pub(crate) fn paths(&self) -> Vec<Path> {
        self.triangles
            .iter()
            .map(|t| Path::polyline(t.map(|i| self.points[i]).to_vec()))
            .collect()
    }

    /// The colours of the corners of every triangle, taken from `colors`
    /// by point; `Err` with numpy's message when a point has no colour
    /// (matplotlib indexes its face colours by the triangles' points, so
    /// that a mesh without one colour per point fails to draw).
    fn corner_colors(&self, colors: &[Rgba]) -> Result<Vec<[Rgba; 3]>, String> {
        let beyond = self
            .triangles
            .iter()
            .flatten()
            .find(|&&index| index >= colors.len());
        if let Some(index) = beyond {
            return Err(format!(
                "index {index} is out of bounds for axis 0 with size {}",
                colors.len()
            ));
        }
        Ok(self
            .triangles
            .iter()
            .map(|t| t.map(|i| colors[i]))
            .collect())
    }
}

impl CollectionArtist {
    /// A triangle mesh over the unmasked triangles of `triangulation`
    /// (matplotlib's `TriMesh(triangulation)`), uncoloured until it is
    /// given values.
    pub(crate) fn tri_mesh(triangulation: &Triangulation, rc: std::rc::Rc<RcParams>) -> Self {
        let mut mesh = Self::new(CollectionKind::TriMesh, Vec::new(), rc);
        mesh.triangle_mesh = Some(TriangleMesh::of(triangulation));
        mesh
    }
}

/// matplotlib's `TriMesh.draw`: the triangles through the collection's
/// transform, each corner in the face colour of its point, as Gouraud
/// triangles with the first line width.
pub(super) fn draw_tri_mesh(
    c: &CollectionArtist,
    mesh: &TriangleMesh,
    ctx: &mut DrawContext<'_>,
) -> Result<(), String> {
    let resolved = ctx
        .resolve(&c.transform())
        .ok_or("its coordinate system no longer exists")?;
    let (faces, _) = resolve_colors(c)?;
    let shades = match mesh.corner_colors(&faces) {
        Ok(shades) => shades,
        Err(problem) => {
            ctx.report_kind(c.base().id(), DrawErrorKind::OutOfRange, problem);
            return Ok(());
        }
    };
    let corners: Vec<[Point; 3]> = mesh
        .triangles
        .iter()
        .map(|t| t.map(|i| resolved.transform_point_non_affine(mesh.points[i])))
        .collect();
    let mut gc = ctx.base_gc(c.base());
    gc.alpha = None;
    gc.linewidth = c
        .linewidths
        .as_ref()
        .and_then(|widths| widths.first().copied())
        .unwrap_or(c.rc().patch_linewidth);
    ctx.renderer()
        .open_group("TriMesh", c.base().gid.as_deref());
    ctx.renderer()
        .draw_gouraud_triangles(&gc, &corners, &shades, &resolved.affine_part());
    ctx.renderer().close_group("TriMesh");
    Ok(())
}

/// A triangle mesh ready for
/// [`Axes::add_trimesh`](crate::axes::Axes::add_trimesh): the unmasked
/// triangles of a triangulation, shaded by interpolating the colours of a
/// value at each point.
#[derive(Debug)]
pub struct TriMeshSpec {
    pub(crate) artist: CollectionArtist,
}

impl TriMeshSpec {
    /// The mesh of `triangulation` coloured by `mapping` (one value per
    /// point).
    ///
    /// # Errors
    ///
    /// When `mapping` does not have one value per point.
    pub fn new(triangulation: &Triangulation, mapping: ColorMapping) -> Result<Self, TriMeshError> {
        let points = triangulation.npoints();
        if mapping.values.len() != points {
            return Err(TriMeshError {
                points,
                values: mapping.values.len(),
            });
        }
        let mut artist = CollectionArtist::tri_mesh(triangulation, crate::rc::params());
        artist.mapping = Some(Mapped::new(mapping));
        Ok(Self { artist })
    }
}

impl CollectionSpec {
    /// A collection of the unmasked triangles of `triangulation`, coloured
    /// by interpolating the colours of its points (matplotlib's
    /// `TriMesh(triangulation)`): the face colour of every point, or the
    /// colours of the values [`Collection::set_array`] maps, one per point.
    ///
    /// ```
    /// use pltrs::collections::{Collection, CollectionSpec, TriMesh};
    /// use pltrs::artist::{ArtistHandle, TypedHandle};
    /// use pltrs::tri::Triangulation;
    /// let tri = Triangulation::delaunay(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0])?;
    /// let mesh = Collection::loose(CollectionSpec::tri_mesh(&tri));
    /// assert!(TriMesh::from_raw_handle(mesh.raw()).is_some());
    /// assert_eq!(mesh.paths().len(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn tri_mesh(triangulation: &Triangulation) -> Self {
        Self {
            artist: CollectionArtist::tri_mesh(triangulation, crate::rc::params()),
            autolim: true,
        }
    }
}

/// A handle to a triangle mesh (matplotlib's `TriMesh`), a collection
/// whose colour mapping has one value per point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriMesh {
    raw: RawHandle,
}

impl ArtistHandle for TriMesh {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for TriMesh {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        let is_mesh = raw.is::<CollectionArtist>()
            && raw.with::<CollectionArtist, _>(|c| c.kind == CollectionKind::TriMesh);
        is_mesh.then(|| Self::from_raw(raw.clone()))
    }
}

impl TriMesh {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The mesh as the collection it is.
    pub fn as_collection(&self) -> Collection {
        Collection::from_raw(self.raw.clone())
    }

    /// Draws the unmasked triangles of `triangulation` over its points
    /// from here on: matplotlib's `TriMesh.draw` reads the points and the
    /// masked triangles of its triangulation each time it draws, so a mask
    /// set, or points moved, after the mesh was made show. The mesh keeps
    /// its colours, one per point.
    ///
    /// ```
    /// use pltrs::collections::{Collection, CollectionSpec, TriMesh};
    /// use pltrs::artist::{ArtistHandle, TypedHandle};
    /// use pltrs::tri::Triangulation;
    /// let tri = Triangulation::delaunay(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0])?;
    /// let mesh = TriMesh::from_raw_handle(Collection::loose(CollectionSpec::tri_mesh(&tri)).raw())
    ///     .expect("a triangle mesh");
    /// mesh.set_triangulation(&tri.with_mask(Some(vec![true, false]))?);
    /// assert_eq!(mesh.as_collection().paths().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_triangulation(&self, triangulation: &Triangulation) -> &Self {
        let drawn = TriangleMesh::of(triangulation);
        self.raw.with_mut::<CollectionArtist, _>(|c| {
            if let Some(mesh) = &mut c.triangle_mesh {
                *mesh = drawn;
            }
        });
        self
    }

    /// Recolours the mesh (one value per point).
    ///
    /// # Errors
    ///
    /// When `mapping` does not have one value per point.
    pub fn set_mapping(&self, mapping: ColorMapping) -> Result<&Self, TriMeshError> {
        let points = self.raw.with::<CollectionArtist, _>(|c| {
            c.triangle_mesh.as_ref().map_or(0, |mesh| mesh.points.len())
        });
        if mapping.values.len() != points {
            return Err(TriMeshError {
                points,
                values: mapping.values.len(),
            });
        }
        self.as_collection().set_mapping(mapping);
        Ok(self)
    }

    /// The colormap.
    pub fn cmap(&self) -> Colormap {
        self.as_collection()
            .cmap()
            .expect("a triangle mesh is colour-mapped")
    }

    /// The data values mapped to the ends of the colormap.
    pub fn clim(&self) -> (Option<f64>, Option<f64>) {
        self.as_collection().clim()
    }

    /// Replaces the colormap (matplotlib's `set_cmap`); the mesh's
    /// colorbars follow.
    pub fn set_cmap(&self, cmap: Colormap) -> &Self {
        self.as_collection().set_cmap(cmap);
        self
    }

    /// Replaces the norm (matplotlib's `set_norm`); unset limits come
    /// from the values and the mesh's colorbars follow.
    pub fn set_norm(&self, norm: Box<dyn Norm>) -> &Self {
        self.as_collection().set_norm(norm);
        self
    }

    /// Sets the data values mapped to the ends of the colormap
    /// (matplotlib's `set_clim`); `None` leaves a limit unchanged. The
    /// mesh's colorbars follow.
    pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
        self.as_collection().set_clim(vmin, vmax);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Triangulation {
        Triangulation::new(
            &[0.0, 1.0, 0.0, 1.0],
            &[0.0, 0.0, 1.0, 1.0],
            vec![[3, 2, 0], [0, 1, 3]],
        )
        .expect("a triangulation")
    }

    #[test]
    fn triangles_are_its_paths() {
        let mesh = CollectionArtist::tri_mesh(&square(), crate::rc::params());
        let paths = mesh.triangle_mesh.as_ref().expect("a mesh").paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(
            paths[0].vertices(),
            [(1.0, 1.0), (0.0, 1.0), (0.0, 0.0)].map(Point::from)
        );
    }

    #[test]
    fn each_point_needs_a_colour() {
        let mesh = CollectionArtist::tri_mesh(&square(), crate::rc::params());
        let triangles = mesh.triangle_mesh.expect("a mesh");
        assert_eq!(
            triangles.corner_colors(&[Rgba::BLACK]),
            Err("index 3 is out of bounds for axis 0 with size 1".to_owned()),
            "numpy's error for the first point beyond the colours"
        );
        let colors = triangles.corner_colors(&[Rgba::BLACK; 4]).expect("colours");
        assert_eq!(colors.len(), 2);
    }

    #[test]
    fn values_follow_the_points() {
        let mapping = |n: usize| ColorMapping {
            values: vec![Some(1.0); n],
            mappable: crate::colors::ScalarMappable::new(
                crate::colors::colormaps::get("viridis").expect("a colormap"),
            ),
        };
        assert!(TriMeshSpec::new(&square(), mapping(3)).is_err());
        assert!(TriMeshSpec::new(&square(), mapping(4)).is_ok());
    }
}
