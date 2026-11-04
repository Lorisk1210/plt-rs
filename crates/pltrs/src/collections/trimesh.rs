//! Gouraud-shaded triangle meshes (matplotlib's `TriMesh`).

use std::rc::Rc;

use pltrs_core::path::DataLimits;
use pltrs_core::{Bbox, Point, Rgba};

use super::{ColorMapping, Mapped};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, DrawContext, LayoutContext,
    LimitPoints, RawHandle, TypedHandle,
};
use crate::colorizer::{self, ColorMapped, MappingChange, change_mapping};
use crate::colors::{Colormap, Norm, NormError, ScalarMappable};
use crate::rc::RcParams;
use crate::transforms::Transform;
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

/// State of a triangle mesh.
#[derive(Clone, Debug)]
pub(crate) struct TriMeshArtist {
    base: ArtistBase,
    points: Vec<Point>,
    triangles: Vec<[usize; 3]>,
    mapping: Mapped,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(TriMeshArtist {
    base,
    points,
    triangles,
    mapping,
    rc
});

impl TriMeshArtist {
    /// The norm and colormap.
    pub(crate) fn mappable(&self) -> &ScalarMappable {
        &self.mapping.source.mappable
    }

    fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    /// The extent of the drawn triangles (matplotlib's `get_datalim` of
    /// its triangle paths).
    pub(crate) fn data_limits(&self) -> DataLimits {
        let used: Vec<Point> = self
            .triangles
            .iter()
            .flat_map(|t| t.map(|i| self.points[i]))
            .collect();
        let mut limits = DataLimits::NULL;
        limits.update_from_points(&used, true, true);
        limits
    }

    fn draw_mesh(&self, ctx: &mut DrawContext<'_>) -> Result<(), String> {
        let resolved = ctx
            .resolve(&self.transform())
            .ok_or("its coordinate system no longer exists")?;
        let colors = self.mapping.colors(&self.base.alpha.into())?;
        let corners: Vec<[Point; 3]> = self
            .triangles
            .iter()
            .map(|t| t.map(|i| resolved.transform_point_non_affine(self.points[i])))
            .collect();
        let shades: Vec<[Rgba; 3]> = self
            .triangles
            .iter()
            .map(|t| t.map(|i| colors[i]))
            .collect();
        let mut gc = ctx.base_gc(&self.base);
        gc.alpha = None;
        gc.linewidth = self.rc.patch_linewidth;
        ctx.renderer()
            .open_group("TriMesh", self.base.gid.as_deref());
        ctx.renderer()
            .draw_gouraud_triangles(&gc, &corners, &shades, &resolved.affine_part());
        ctx.renderer().close_group("TriMesh");
        Ok(())
    }
}

impl ColorMapped for TriMeshArtist {
    fn mapping_mut(&mut self) -> Option<&mut ScalarMappable> {
        Some(&mut self.mapping.source.mappable)
    }

    /// Problems normalising the values are reported when drawing.
    fn remap(&mut self, _others: &mut ArtistStore) -> Result<(), NormError> {
        self.mapping.refresh();
        Ok(())
    }
}

impl Artist for TriMeshArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        if let Err(problem) = self.draw_mesh(ctx) {
            ctx.report(self.base.id(), problem);
        }
    }

    /// matplotlib's `_update_collection_limits`: the vertices of the drawn
    /// triangles.
    fn limit_points(&self, _axes: ArtistId) -> Option<LimitPoints> {
        LimitPoints::from_limits(self.data_limits(), (true, true))
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let resolved = ctx.resolve(&self.transform())?;
        Some(Bbox::from_points(
            self.points.iter().map(|&p| resolved.transform_point(p)),
        ))
    }
}

/// A triangle mesh ready for
/// [`Axes::add_trimesh`](crate::axes::Axes::add_trimesh): the unmasked
/// triangles of a triangulation, shaded by interpolating the colours of a
/// value at each point.
#[derive(Debug)]
pub struct TriMeshSpec {
    pub(crate) artist: TriMeshArtist,
}

impl TriMeshSpec {
    /// The mesh of `triangulation` coloured by `mapping` (one value per
    /// point).
    pub fn new(triangulation: &Triangulation, mapping: ColorMapping) -> Result<Self, TriMeshError> {
        let points = triangulation.npoints();
        if mapping.values.len() != points {
            return Err(TriMeshError {
                points,
                values: mapping.values.len(),
            });
        }
        let rc = crate::rc::params();
        Ok(Self {
            artist: TriMeshArtist {
                base: ArtistBase::with_zorder(1.0),
                points: (0..points).map(|i| triangulation.point(i)).collect(),
                triangles: triangulation.drawn_triangles(),
                mapping: Mapped::new(mapping),
                rc,
            },
        })
    }
}

/// A handle to a triangle mesh (matplotlib's `TriMesh`).
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
        raw.is::<TriMeshArtist>()
            .then(|| Self::from_raw(raw.clone()))
    }
}

impl TriMesh {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Recolours the mesh (one value per point).
    pub fn set_mapping(&self, mapping: ColorMapping) -> Result<&Self, TriMeshError> {
        let points = self.raw.with::<TriMeshArtist, _>(|m| m.points.len());
        if mapping.values.len() != points {
            return Err(TriMeshError {
                points,
                values: mapping.values.len(),
            });
        }
        let mapped = Mapped::new(mapping);
        self.raw
            .with_mut::<TriMeshArtist, _>(|m| m.mapping = mapped);
        colorizer::notify(&self.raw, true);
        Ok(self)
    }

    /// The colormap.
    pub fn cmap(&self) -> Colormap {
        self.raw
            .with::<TriMeshArtist, _>(|m| m.mappable().cmap().clone())
    }

    /// The data values mapped to the ends of the colormap.
    pub fn clim(&self) -> (Option<f64>, Option<f64>) {
        self.raw.with::<TriMeshArtist, _>(|m| m.mappable().clim())
    }

    /// Replaces the colormap (matplotlib's `set_cmap`); the mesh's
    /// colorbars follow.
    pub fn set_cmap(&self, cmap: Colormap) -> &Self {
        change_mapping::<TriMeshArtist>(&self.raw, MappingChange::Cmap(Box::new(cmap)));
        self
    }

    /// Replaces the norm (matplotlib's `set_norm`); unset limits come
    /// from the values and the mesh's colorbars follow.
    pub fn set_norm(&self, norm: Box<dyn Norm>) -> &Self {
        change_mapping::<TriMeshArtist>(&self.raw, MappingChange::Norm(norm));
        self
    }

    /// Sets the data values mapped to the ends of the colormap
    /// (matplotlib's `set_clim`); `None` leaves a limit unchanged. The
    /// mesh's colorbars follow.
    pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
        change_mapping::<TriMeshArtist>(&self.raw, MappingChange::Clim(vmin, vmax));
        self
    }
}
