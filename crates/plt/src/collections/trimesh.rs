//! Gouraud-shaded triangle meshes (matplotlib's `TriMesh`).

use std::rc::Rc;

use plt_core::path::DataLimits;
use plt_core::{Bbox, Point, Rgba};

use super::{ColorMapping, Mapped};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, LayoutContext, RawHandle};
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
#[derive(Debug)]
pub(crate) struct TriMeshArtist {
    base: ArtistBase,
    points: Vec<Point>,
    triangles: Vec<[usize; 3]>,
    mapping: Mapped,
    rc: Rc<RcParams>,
}

impl TriMeshArtist {
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
        let colors = self.mapping.colors(self.base.alpha)?;
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
                triangles: triangulation.masked_triangles(),
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
        Ok(self)
    }
}
