//! Rendering a collection (matplotlib's `Collection.draw`).

use plt_core::{Affine2D, Path, PathCode, Point, Rgba};
use plt_render::{Dashes, GraphicsContext, Hatch, PathCollection};

use super::spec::{EllipseUnits, Ellipses};
use super::style::{broadcast_widths_and_dashes, resolve_colors, resolve_hatch_colors};
use super::{CollectionArtist, CollectionKind, cycled};
use crate::artist::{Artist, ArtistId, DrawContext, LayoutContext};
use crate::axes::AxesState;
use crate::transforms::Transform;

/// Where a collection's items are for one draw: the shapes after the
/// non-affine part of the transform, its affine rest, the per-item shape
/// transforms and the offsets in display space.
pub(super) struct Geometry {
    paths: Vec<Path>,
    master: Affine2D,
    transforms: Vec<Affine2D>,
    offsets: Vec<Point>,
}

impl Geometry {
    pub(super) fn new(
        c: &CollectionArtist,
        layout: LayoutContext<'_>,
        shapes: &[Path],
        transform: &Transform,
    ) -> Result<Self, String> {
        let resolved = layout
            .resolve(transform)
            .ok_or("its coordinate system no longer exists")?;
        let paths = shapes
            .iter()
            .map(|p| resolved.split_path(p).0.into_owned())
            .collect();
        let dpi = layout.dpi();
        let mut master = resolved.affine_part();
        let transforms = match (&c.ellipses, &c.sizes) {
            (Some(ellipses), _) => {
                let axes = c.base().parent;
                let scales = ellipse_scales(ellipses.units, layout, axes)
                    .ok_or("its axes no longer exist")?;
                if let Some(data) = scales.data {
                    master = data;
                }
                ellipse_transforms(ellipses, scales.scale)
            }
            (None, Some(sizes)) => sizes
                .iter()
                .map(|area| {
                    let scale = area.sqrt() * dpi / 72.0 * c.size_factor;
                    let scaled = Affine2D::scaling(scale, scale);
                    if c.size_rotation == 0.0 {
                        scaled
                    } else {
                        scaled.rotate(-c.size_rotation)
                    }
                })
                .collect(),
            (None, None) => c.transforms.clone(),
        };
        let offsets = match &c.offset_transform {
            Some(t) => {
                let r = layout
                    .resolve(t)
                    .ok_or("its offset coordinate system no longer exists")?;
                c.offsets.iter().map(|&p| r.transform_point(p)).collect()
            }
            None => vec![Point::ORIGIN],
        };
        Ok(Self {
            paths,
            master,
            transforms,
            offsets,
        })
    }

    /// The items within `radius` pixels of display point `at` — on their
    /// outline stroked `2 * radius` wide, or inside them when `filled`
    /// (matplotlib's `point_in_path_collection`).
    pub(super) fn hits(&self, at: Point, radius: f64, filled: bool) -> Vec<usize> {
        if self.paths.is_empty() {
            return Vec::new();
        }
        let count = self.paths.len().max(self.offsets.len());
        (0..count)
            .filter(|&i| {
                let path = &self.paths[i % self.paths.len()];
                let offset = self.offsets[i % self.offsets.len()];
                let transform = cycled(&self.transforms, i)
                    .copied()
                    .unwrap_or_default()
                    .then(self.master)
                    .then(Affine2D::translation(offset.x, offset.y));
                if filled {
                    path.contains_point(at, Some(&transform), radius)
                } else {
                    path.stroke_contains_point(at, &transform, radius)
                }
            })
            .collect()
    }
}

/// A collection resolved for one draw: its geometry and per-item styles.
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
    fn new(
        c: &CollectionArtist,
        ctx: &DrawContext<'_>,
        shapes: &[Path],
        transform: &Transform,
    ) -> Result<Self, String> {
        let Geometry {
            paths,
            master,
            transforms,
            offsets,
        } = Geometry::new(c, *ctx.layout(), shapes, transform)?;
        let (faces, edges) = resolve_colors(c)?;
        let rc = c.rc();
        let line_defaults = c.kind == CollectionKind::Line;
        let widths = c.linewidths_or_default();
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
            master,
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
        let path = self.paths[0].transformed(&combined);
        // matplotlib's `get_extents` takes NumPy minima and maxima, which a
        // non-finite vertex turns NaN or infinite: no shortcut then.
        let finite = path
            .vertices()
            .iter()
            .zip(path.resolved_codes())
            .all(|(vertex, code)| code == PathCode::ClosePoly || vertex.is_finite());
        let extents = path.extents();
        (finite && extents.width() < canvas.0 && extents.height() < canvas.1).then_some(combined)
    }
}

/// Draws the collection, using `draw_markers` when every item shares one
/// shape and style (matplotlib's single-path optimisation).
pub(super) fn draw_collection(
    c: &CollectionArtist,
    ctx: &mut DrawContext<'_>,
) -> Result<(), String> {
    let (shapes, transform) = c.current_shapes(*ctx.layout())?;
    if shapes.is_empty() {
        return Ok(());
    }
    let batch = Batch::new(c, ctx, &shapes, &transform)?;
    let hatches = resolve_hatch_colors(c, &batch.edges).map_err(|e| e.to_string())?;
    let mut gc = graphics_context(c, ctx, &hatches);
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
            hatchcolors: &hatches,
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

/// The shared graphics state: clipping, hatch (in the first of
/// `hatches`; items take theirs from the collection), caps and joins.
fn graphics_context(
    c: &CollectionArtist,
    ctx: &DrawContext<'_>,
    hatches: &[Rgba],
) -> GraphicsContext {
    let mut gc = ctx.base_gc(c.base());
    gc.alpha = None;
    // matplotlib only hands the renderer non-empty hatches.
    if let Some(pattern) = c.hatch.as_ref().filter(|p| !p.is_empty()) {
        gc.hatch = Some(Hatch {
            pattern: pattern.clone(),
            color: hatches.first().copied().unwrap_or(Rgba::BLACK),
            linewidth: c.rc().hatch_linewidth,
        });
    }
    if let Some(cap) = c.capstyle {
        gc.cap = cap;
    }
    if let Some(join) = c.joinstyle {
        gc.join = join;
    }
    gc
}

/// How the ellipses of a collection are scaled at draw time.
struct EllipseScales {
    /// Pixels per unit along both axes.
    scale: f64,
    /// For data units: the data transform without its translation, which
    /// replaces the collection's transform.
    data: Option<Affine2D>,
}

/// `EllipseCollection._set_transforms`: the scale of `units` in the axes
/// `axes`.
fn ellipse_scales(
    units: EllipseUnits,
    layout: LayoutContext<'_>,
    axes: Option<ArtistId>,
) -> Option<EllipseScales> {
    let plain = |scale: f64| Some(EllipseScales { scale, data: None });
    match units {
        EllipseUnits::Inches => return plain(layout.dpi()),
        EllipseUnits::Points => return plain(layout.dpi() / 72.0),
        EllipseUnits::Dots => return plain(1.0),
        _ => {}
    }
    let axes = axes?;
    let bbox = layout.axes_extent(axes)?;
    let view = layout.store().get_as::<AxesState>(axes)?.view_lim;
    match units {
        EllipseUnits::Xy => {
            let m = layout
                .resolve(&crate::transforms::Transform::Data(axes))?
                .affine_part()
                .matrix();
            Some(EllipseScales {
                scale: 1.0,
                data: Some(Affine2D::from_values(
                    m[0][0], m[1][0], m[0][1], m[1][1], 0.0, 0.0,
                )),
            })
        }
        EllipseUnits::X => plain(bbox.width() / view.width()),
        EllipseUnits::Y => plain(bbox.height() / view.height()),
        EllipseUnits::Width => plain(bbox.width()),
        _ => plain(bbox.height()),
    }
}

/// One affine per ellipse: the unit circle stretched to its half axes
/// (scaled by `scale`) and rotated.
fn ellipse_transforms(ellipses: &Ellipses, scale: f64) -> Vec<Affine2D> {
    ellipses
        .half_widths
        .iter()
        .zip(&ellipses.half_heights)
        .zip(&ellipses.angles)
        .map(|((w, h), angle)| {
            let (w, h) = (w * scale, h * scale);
            let (sin, cos) = (angle.sin(), angle.cos());
            Affine2D::from_values(w * cos, w * sin, h * -sin, h * cos, 0.0, 0.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::HatchColors;

    fn square(alpha: Option<f64>) -> CollectionArtist {
        let path = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
        let mut collection =
            CollectionArtist::new(CollectionKind::Poly, vec![path], crate::rc::params());
        collection.base_mut().alpha = alpha;
        collection
    }

    #[test]
    fn hatches_take_the_edges_or_the_faded_default_edge_colour() {
        let edges = [Rgba::new(1.0, 0.0, 0.0, 0.5), Rgba::BLACK];
        assert_eq!(
            resolve_hatch_colors(&square(Some(0.5)), &edges),
            Ok(edges.to_vec())
        );
        let black = resolve_hatch_colors(&square(Some(0.5)), &[]).unwrap();
        assert_eq!(
            black,
            [Rgba::new(0.0, 0.0, 0.0, 0.5)],
            "matplotlib fades the default"
        );
        assert_eq!(resolve_hatch_colors(&square(None), &[]).unwrap()[0].a, 1.0);
    }

    #[test]
    fn explicit_hatch_colours_take_the_alpha() {
        crate::rc::context(|| {
            crate::rc::set("hatch.color", "blue").unwrap();
            let colors = resolve_hatch_colors(&square(Some(0.25)), &[Rgba::BLACK]).unwrap();
            assert_eq!(colors, [Rgba::new(0.0, 0.0, 1.0, 0.25)]);
        });
        let mut both = square(None);
        both.hatchcolors = HatchColors::Colors(vec!["red".into(), "lime".into()]);
        let colors = resolve_hatch_colors(&both, &[Rgba::BLACK]).unwrap();
        assert_eq!(colors[1], Rgba::new(0.0, 1.0, 0.0, 1.0));
    }
}
