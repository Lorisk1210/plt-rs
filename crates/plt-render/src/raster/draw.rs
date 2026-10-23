//! The [`Renderer`] implementation: one method per `RendererAgg` entry
//! point, each conditioning and clipping exactly like `_backend_agg.h`.

use std::ops::Range;

use plt_core::hatch;
use plt_core::path::convert::{SnapMode as PathSnap, VertexSource};
use plt_core::{Affine2D, Path, PathCode, Point, Rgba};
use plt_raster::{Antialias, Clip, ClipBox, FillRule, Pixmap, Rgba8};
use plt_text::TextRun;

use super::RasterRenderer;
use super::condition::{Conditioning, cached_mask, mpl_round, sketch_params, snap_mode};
use crate::collection::PathCollection;
use crate::filter::FilterFn;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::quad_mesh::QuadMesh;
use crate::renderer::Renderer;

/// Whether an 8-bit colour would draw anything.
fn visible(color: Rgba) -> bool {
    Rgba8::from(color).a != 0
}

impl RasterRenderer {
    /// `RendererAgg::_draw_path`: face, hatch and stroke of an already
    /// conditioned path, under the clip box and mask prepared by the caller.
    fn draw_conditioned(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        face: Option<Rgba>,
        rect: Option<ClipBox>,
        masked: bool,
    ) {
        let aa = Antialias::from(gc.antialiased);
        if let Some(face) = face {
            let clip = Clip {
                rect,
                mask: cached_mask(self.clip_cache.as_ref(), masked),
            };
            let result =
                self.painter
                    .fill_path(&mut self.pixmap, path, face, FillRule::NonZero, aa, &clip);
            self.record(result);
        }
        if let Some(tile) = self.hatch_tile(gc) {
            let clip = Clip {
                rect,
                mask: cached_mask(self.clip_cache.as_ref(), masked),
            };
            let result = self.painter.fill_path_with_pattern(
                &mut self.pixmap,
                path,
                &tile,
                FillRule::NonZero,
                &clip,
            );
            self.record(result);
        }
        if gc.linewidth != 0.0 {
            let style = self.stroke_style(gc);
            let clip = Clip {
                rect,
                mask: cached_mask(self.clip_cache.as_ref(), masked),
            };
            let result = self.painter.stroke_path(
                &mut self.pixmap,
                path,
                gc.stroke_color(),
                &style,
                aa,
                &clip,
            );
            self.record(result);
        }
    }

    /// The hatch tile of `gc`: matplotlib's hatch path (density 6) scaled
    /// to an `int(dpi)`-pixel tile, filled and stroked with square caps.
    fn hatch_tile(&mut self, gc: &GraphicsContext) -> Option<Pixmap> {
        let spec = gc.hatch.as_ref()?;
        let unit = hatch::pattern_path(&spec.pattern, hatch::DEFAULT_DENSITY).ok()?;
        if unit.is_empty() {
            return None;
        }
        let size = self.dpi as u32;
        let s = f64::from(size);
        let tile_path = unit.transformed(&Affine2D::from_values(s, 0.0, 0.0, -s, 0.0, s));
        let linewidth = self.px(spec.linewidth);
        match self.painter.hatch_tile(
            size,
            &tile_path,
            spec.color,
            linewidth,
            Rgba8::TRANSPARENT_WHITE,
        ) {
            Ok(tile) => Some(tile),
            Err(err) => {
                self.errors.push(err);
                None
            }
        }
    }

    /// The marker path in stamp coordinates (`draw_markers`): flipped,
    /// snapped, and moved to pixel centres when not snapping.
    fn marker_geometry(
        &self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
    ) -> Path {
        let trans = marker_transform.then(Affine2D::scaling(1.0, -1.0));
        let has_codes = marker.codes().is_some();
        let width = self.px(gc.linewidth);
        let snapped = marker
            .iter_vertices()
            .transform(trans)
            .remove_nans(true, has_codes)
            .snap(snap_mode(gc.snap), marker.len(), width);
        if snapped.is_snapping() {
            return snapped.flatten_curves(true).collect_path();
        }
        // matplotlib shifts the (shared) marker transform by half a pixel
        // after the snapping decision was taken.
        marker
            .iter_vertices()
            .transform(trans.then(Affine2D::translation(0.5, 0.5)))
            .remove_nans(true, has_codes)
            .snap(PathSnap::Never, marker.len(), width)
            .flatten_curves(true)
            .collect_path()
    }
}

impl Renderer for RasterRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        self.size
    }

    fn dpi(&self) -> f64 {
        self.dpi
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let chunked = fill.is_none()
            && gc.hatch.is_none()
            && self.chunksize > 100
            && path.len() > self.chunksize
            && self.simplify.applies_to(path);
        if chunked {
            for chunk in chunks(path, self.chunksize) {
                self.draw_whole_path(gc, &chunk, transform, None);
            }
        } else {
            self.draw_whole_path(gc, path, transform, fill);
        }
    }

    fn draw_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let marker_path = self.marker_geometry(gc, marker, marker_transform);
        let to_pixels = transform
            .then(Affine2D::scaling(1.0, -1.0))
            .then(Affine2D::translation(0.5, f64::from(self.height()) + 0.5));
        let positions: Vec<Point> = path
            .iter_vertices()
            .transform(to_pixels)
            .remove_nans(false, false)
            .flatten_curves(true)
            .map(|v| v.point)
            .collect();
        let face = fill.map(|c| gc.fill_color(c)).filter(|c| c.a != 0.0);
        let edge = Some(gc.stroke_color()).filter(|&c| visible(c));
        let style = self.solid_stroke_style(gc);
        let stamp =
            match self
                .painter
                .marker_stamp(&marker_path, face.is_some(), edge.map(|_| &style))
            {
                Ok(stamp) => stamp,
                Err(err) => {
                    self.errors.push(err);
                    return;
                }
            };
        // Markers clip to the clip box inclusively; under a clip path
        // (rasterized unclipped here) the clip box is not applied at all.
        let masked = self.prepare_clip_mask(gc, None);
        let rect = if masked {
            None
        } else {
            self.clip_box(gc).map(ClipBox::inclusive)
        };
        let clip = Clip {
            rect,
            mask: cached_mask(self.clip_cache.as_ref(), masked),
        };
        let result = stamp.draw(&mut self.pixmap, &positions, face, edge, &clip);
        self.record(result);
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        if collection.is_empty() {
            return;
        }
        let rect = self.clip_box(gc);
        let masked = self.prepare_clip_mask(gc, Some(rect.unwrap_or_else(|| self.full_box())));
        let clip = collection.facecolors.is_empty() && gc.hatch.is_none();
        let mut conditioned = std::mem::take(&mut self.conditioned);
        for item in collection.items(gc) {
            let conditioning = Conditioning {
                has_codes: true,
                clip,
                snap: snap_mode(item.gc.snap),
                snap_width: self.px(item.gc.linewidth),
                simplify: None,
                sketch: sketch_params(item.gc.sketch),
            };
            let transform = self.to_device(&item.transform);
            self.condition_into(item.path, transform, &conditioning, &mut conditioned);
            let face = item.fill.map(|c| item.gc.fill_color(c));
            self.draw_conditioned(&item.gc, &conditioned, face, rect, masked);
        }
        self.conditioned = conditioned;
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        let rect = self.clip_box(gc);
        let masked = self.prepare_clip_mask(gc, Some(rect.unwrap_or_else(|| self.full_box())));
        let to_pixels = self.to_device(transform);
        for (triangle, colors) in triangles.iter().zip(colors) {
            let points = triangle.map(|p| to_pixels.transform_point(p));
            if points.iter().any(|p| p.x.is_nan() || p.y.is_nan()) {
                continue;
            }
            let clip = Clip {
                rect,
                mask: cached_mask(self.clip_cache.as_ref(), masked),
            };
            let result = self
                .painter
                .gouraud_triangle(&mut self.pixmap, points, *colors, &clip);
            self.record(result);
        }
    }

    /// matplotlib's wrapper rounds the position and forces the image alpha
    /// to 1 (images arrive with their opacity applied).
    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        _transform: Option<&Affine2D>,
    ) {
        let (Ok(width), Ok(height)) = (u32::try_from(image.width()), u32::try_from(image.height()))
        else {
            return;
        };
        let Ok(source) = Pixmap::from_vec(width, height, image.data().to_vec()) else {
            return;
        };
        let x = mpl_round(origin.x);
        let y = mpl_round(origin.y);
        let top = f64::from(self.height()) - (y + f64::from(height));
        let rect = self.clip_box(gc);
        let raster_box = rect.unwrap_or_else(|| self.full_box());
        let masked = self.prepare_clip_mask(gc, Some(raster_box));
        // Masked images go through the rasterizer clip box; unmasked ones
        // through `renderer_base`, which includes the far edges.
        let clip = if masked {
            Clip {
                rect: Some(raster_box),
                mask: cached_mask(self.clip_cache.as_ref(), true),
            }
        } else {
            Clip::rect(rect.map_or(raster_box, ClipBox::inclusive))
        };
        let result =
            plt_raster::draw_image(&mut self.pixmap, &source, x as i32, top as i32, 1.0, &clip);
        self.record(result);
    }

    /// Glyph outlines filled at `FreeType`'s pen positions, unsnapped; see
    /// the `text` module.
    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.draw_text_run(gc, run, origin, angle);
    }

    /// Agg's quad mesh generator walks every cell down its first column
    /// first. Colours given per cell (or one for all) are split with the
    /// cells, which are then drawn a block at a time, so that their
    /// outlines need not all exist at once.
    fn draw_quad_mesh(&mut self, gc: &GraphicsContext, mesh: &QuadMesh<'_>) {
        let widths = [gc.linewidth];
        let flags = [mesh.antialiased];
        let cells = mesh.cell_count();
        let splits = |colors: &[Rgba]| colors.len() <= 1 || colors.len() == cells;
        if !(splits(mesh.facecolors) && splits(mesh.edgecolors)) {
            let paths = mesh.agg_cell_paths();
            self.draw_path_collection(gc, &mesh.collection(&paths, &widths, &flags));
            return;
        }
        let mut paths = Vec::new();
        for start in (0..cells).step_by(MESH_BLOCK) {
            let block = start..cells.min(start + MESH_BLOCK);
            mesh.agg_cell_paths_into(block.clone(), &mut paths);
            let collection = PathCollection {
                facecolors: block_colors(mesh.facecolors, &block),
                edgecolors: block_colors(mesh.edgecolors, &block),
                ..mesh.collection(&paths, &widths, &flags)
            };
            self.draw_path_collection(gc, &collection);
        }
    }

    fn start_filter(&mut self) {
        self.start_filtering();
    }

    fn stop_filter(&mut self, filter: &FilterFn<'_>) {
        self.stop_filtering(filter);
    }

    /// Agg composites each image straight onto the canvas.
    fn option_image_nocomposite(&self) -> bool {
        true
    }
}

/// Cells of a quad mesh drawn as one path collection.
const MESH_BLOCK: usize = 4096;

/// The colours of the cells in `block`: their own when given per cell,
/// else the one colour (or none) for all.
fn block_colors<'a>(colors: &'a [Rgba], block: &Range<usize>) -> &'a [Rgba] {
    if colors.len() > 1 {
        &colors[block.clone()]
    } else {
        colors
    }
}

impl RasterRenderer {
    /// Draws `path` in one piece (matplotlib's `_renderer.draw_path`).
    fn draw_whole_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let face = fill.map(|c| gc.fill_color(c)).filter(|c| c.a != 0.0);
        let rect = self.clip_box(gc);
        let masked = self.prepare_clip_mask(gc, Some(rect.unwrap_or_else(|| self.full_box())));
        let clip = face.is_none() && gc.hatch.is_none();
        let snap_width = if gc.stroke_color().a == 0.0 {
            0.0
        } else {
            self.px(gc.linewidth)
        };
        let conditioning = Conditioning {
            has_codes: path.codes().is_some(),
            clip,
            snap: snap_mode(gc.snap),
            snap_width,
            simplify: if clip {
                self.simplify.for_path(path)
            } else {
                None
            },
            sketch: sketch_params(gc.sketch),
        };
        let mut conditioned = std::mem::take(&mut self.conditioned);
        self.condition_into(
            path,
            self.to_device(transform),
            &conditioning,
            &mut conditioned,
        );
        self.draw_conditioned(gc, &conditioned, face, rect, masked);
        self.conditioned = conditioned;
    }
}

/// The pieces matplotlib's `RendererAgg.draw_path` splits a long path into
/// for `agg.path.chunksize`: equal runs of vertices, each starting with a
/// move, where the vertex before each run's start is left out.
fn chunks(path: &Path, chunksize: usize) -> Vec<Path> {
    let points = path.len();
    let count = points.div_ceil(chunksize);
    let size = points.div_ceil(count);
    let starts: Vec<usize> = (0..points).step_by(size).collect();
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = starts.get(i + 1).map_or(points, |next| next - 1);
            let vertices = path.vertices()[start..end].to_vec();
            match path.codes() {
                Some(codes) => {
                    let mut codes = codes[start..end].to_vec();
                    if let Some(first) = codes.first_mut() {
                        *first = PathCode::MoveTo;
                    }
                    Path::new(vertices, codes).expect("chunks start with a move")
                }
                None => Path::polyline(vertices),
            }
        })
        .collect()
}
