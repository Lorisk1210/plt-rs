//! Rendering a collection (matplotlib's `Collection.draw`).

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{Dashes, GraphicsContext, Hatch, PathCollection};

use super::style::{broadcast_widths_and_dashes, resolve_colors};
use super::{CollectionArtist, CollectionKind, cycled};
use crate::artist::{Artist, DrawContext};

/// A collection resolved for one draw: shapes after the non-affine part
/// of the transform, offsets in display space and per-item styles.
struct Batch {
    paths: Vec<Path>,
    master: Affine2D,
    transforms: Vec<Affine2D>,
    offsets: Vec<Point>,
    faces: Vec<Rgba>,
    edges: Vec<Rgba>,
    widths: Vec<f64>,
    dashes: Vec<Dashes>,
    antialiased: Vec<bool>,
}

impl Batch {
    fn new(c: &CollectionArtist, ctx: &DrawContext<'_>) -> Result<Self, String> {
        let resolved = ctx
            .resolve(&c.transform())
            .ok_or("its coordinate system no longer exists")?;
        let paths = c
            .paths
            .iter()
            .map(|p| resolved.split_path(p).0.into_owned())
            .collect();
        let dpi = ctx.layout().dpi();
        let transforms = match &c.sizes {
            Some(sizes) => sizes
                .iter()
                .map(|area| {
                    let scale = area.sqrt() * dpi / 72.0;
                    Affine2D::scaling(scale, scale)
                })
                .collect(),
            None => c.transforms.clone(),
        };
        let offsets = match &c.offset_transform {
            Some(t) => {
                let r = ctx
                    .resolve(t)
                    .ok_or("its offset coordinate system no longer exists")?;
                c.offsets.iter().map(|&p| r.transform_point(p)).collect()
            }
            None => vec![Point::ORIGIN],
        };
        let (faces, edges) = resolve_colors(c)?;
        let rc = c.rc();
        let line_defaults = c.kind == CollectionKind::Line;
        let widths = c.linewidths.clone().unwrap_or_else(|| {
            vec![if line_defaults {
                rc.lines_linewidth
            } else {
                rc.patch_linewidth
            }]
        });
        let (widths, dashes) = broadcast_widths_and_dashes(&widths, &c.linestyles, rc);
        let antialiased = c.antialiased.clone().unwrap_or_else(|| {
            vec![if line_defaults {
                rc.lines_antialiased
            } else {
                rc.patch_antialiased
            }]
        });
        Ok(Self {
            paths,
            master: resolved.affine_part(),
            transforms,
            offsets,
            faces,
            edges,
            widths,
            dashes,
            antialiased,
        })
    }

    /// The shape transform for drawing every item as the same marker, when
    /// matplotlib's single-path optimisation applies: one shape, style and
    /// colour pair, solid, unhatched and smaller than the canvas.
    fn single_marker(&self, hatched: bool, canvas: (f64, f64)) -> Option<Affine2D> {
        let uniform = self.paths.len() == 1
            && self.transforms.len() <= 1
            && self.faces.len() == 1
            && self.edges.len() == 1
            && self.widths.len() == 1
            && self.dashes.iter().all(Dashes::is_solid)
            && self.antialiased.len() == 1
            && !hatched;
        if !uniform {
            return None;
        }
        let combined = cycled(&self.transforms, 0)
            .copied()
            .unwrap_or_default()
            .then(self.master);
        let extents = self.paths[0].transformed(&combined).extents();
        (extents.width() < canvas.0 && extents.height() < canvas.1).then_some(combined)
    }
}

/// Draws the collection, using `draw_markers` when every item shares one
/// shape and style (matplotlib's single-path optimisation).
pub(super) fn draw_collection(
    c: &CollectionArtist,
    ctx: &mut DrawContext<'_>,
) -> Result<(), String> {
    if c.paths.is_empty() {
        return Ok(());
    }
    let batch = Batch::new(c, ctx)?;
    let mut gc = graphics_context(c, ctx, &batch.edges)?;
    let name = match c.kind {
        CollectionKind::Generic => "Collection",
        CollectionKind::Path => "PathCollection",
        CollectionKind::Poly => "PolyCollection",
        CollectionKind::Line => "LineCollection",
        CollectionKind::QuadMesh => "QuadMesh",
    };
    ctx.renderer().open_group(name, c.base().gid.as_deref());
    let canvas = ctx.renderer().canvas_size();
    if let Some(combined) = batch.single_marker(c.hatch.is_some(), canvas) {
        gc.foreground = batch.edges[0];
        gc.linewidth = batch.widths[0];
        gc.dashes.clone_from(&batch.dashes[0]);
        gc.antialiased = batch.antialiased[0];
        let positions = Path::polyline(batch.offsets);
        let fill = Some(batch.faces[0]);
        ctx.renderer().draw_markers(
            &gc,
            &batch.paths[0],
            &combined,
            &positions,
            &Affine2D::IDENTITY,
            fill,
        );
    } else {
        let collection = PathCollection {
            master_transform: batch.master,
            paths: &batch.paths,
            path_transforms: &batch.transforms,
            offsets: &batch.offsets,
            facecolors: &batch.faces,
            edgecolors: &batch.edges,
            hatchcolors: &[],
            linewidths: &batch.widths,
            dashes: &batch.dashes,
            antialiased: &batch.antialiased,
            urls: &[],
        };
        ctx.renderer().draw_path_collection(&gc, &collection);
    }
    ctx.renderer().close_group(name);
    Ok(())
}

/// The shared graphics state: clipping, hatch, caps and joins.
fn graphics_context(
    c: &CollectionArtist,
    ctx: &DrawContext<'_>,
    edges: &[Rgba],
) -> Result<GraphicsContext, String> {
    let rc = c.rc();
    let mut gc = ctx.base_gc(c.base());
    gc.alpha = None;
    if let Some(pattern) = &c.hatch {
        let color = match edges.first() {
            Some(&edge) => edge,
            None => rc.patch_edgecolor.resolve().map_err(|e| e.to_string())?,
        };
        gc.hatch = Some(Hatch {
            pattern: pattern.clone(),
            color,
            linewidth: rc.hatch_linewidth,
        });
    }
    if let Some(cap) = c.capstyle {
        gc.cap = cap;
    }
    if let Some(join) = c.joinstyle {
        gc.join = join;
    }
    Ok(gc)
}
