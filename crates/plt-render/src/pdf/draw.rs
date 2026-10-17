//! Paths, markers, path collections, Gouraud triangles and images
//! (`draw_path`, `draw_markers`, `draw_path_collection`,
//! `draw_gouraud_triangles`, `draw_image`).

#![allow(
    clippy::float_cmp,
    reason = "matplotlib compares collection alphas exactly"
)]

use plt_core::{Affine2D, Path, Point, Rgba};

use super::PdfRenderer;
use super::object::Obj;
use super::path_ops;
use super::resources::{MarkerPaint, ShadingColors, TemplatePaint, extents, paint_op};
use super::state::PdfState;
use crate::collection::PathCollection;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::vector::segment_ends;

/// numpy's `allclose(a, b)` for scalars.
fn all_close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-8 + 1e-5 * b.abs()
}

/// `1 0 0 1 dx dy cm /Name Do`: places an `XObject` `(dx, dy)` further.
fn place(name: &str, dx: f64, dy: f64) -> [Obj; 9] {
    [
        Obj::Int(1),
        Obj::Int(0),
        Obj::Int(0),
        Obj::Int(1),
        Obj::Real(dx),
        Obj::Real(dy),
        Obj::op("cm"),
        Obj::name(name),
        Obj::op("Do"),
    ]
}

impl PdfRenderer {
    /// `draw_path`: unfilled, unhatched paths are clipped to the page and
    /// simplified (`writePath`).
    pub(super) fn pdf_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: Affine2D,
        fill: Option<Rgba>,
    ) {
        self.check_gc(gc, fill);
        let clip = fill.is_none() && gc.active_hatch().is_none();
        let ops = path_ops(
            path,
            transform,
            clip.then(|| self.canvas()),
            if clip {
                self.options.simplify.for_path(path)
            } else {
                None
            },
            gc.sketch.map(crate::gc::Sketch::params),
        );
        self.output(&[Obj::Raw(ops)]);
        let paint = paint_op(self.states.current.fills(), self.states.current.strokes());
        self.output(&[Obj::op(paint)]);
    }

    /// `draw_markers`: one form `XObject` placed at every vertex inside the
    /// page, unless drawing the marker paths directly is shorter.
    pub(super) fn pdf_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: Affine2D,
        path: &Path,
        transform: Affine2D,
        fill: Option<Rgba>,
    ) {
        let (marker_len, uses) = (marker.len(), path.len());
        if marker_len * uses < marker_len + uses + 5 {
            for at in segment_ends(path, transform, None) {
                let placed = marker_transform.translate(at.x, at.y);
                self.pdf_path(gc, marker, placed, fill);
            }
            return;
        }
        self.check_gc(gc, fill);
        let wanted = PdfState::from_gc(gc, fill, self.generation);
        let paint = MarkerPaint {
            fill: wanted.fills(),
            stroke: wanted.strokes(),
            join: gc.join,
            cap: gc.cap,
            linewidth: self.states.current.linewidth,
        };
        let name = self
            .resources
            .marker(&mut self.writer, marker, marker_transform, paint);
        self.output(&[Obj::op("q")]);
        let canvas = self.canvas();
        let (mut last_x, mut last_y) = (0.0, 0.0);
        for at in segment_ends(path, transform, Some(canvas)) {
            let inside = (0.0..=canvas.x1()).contains(&at.x) && (0.0..=canvas.y1()).contains(&at.y);
            if !inside {
                continue;
            }
            self.output(&place(&name, at.x - last_x, at.y - last_y));
            (last_x, last_y) = (at.x, at.y);
        }
        self.output(&[Obj::op("Q")]);
    }

    /// Whether every item of `collection` paints alike, so that one form
    /// per path can be reused: `(filled, stroked)`, or `None`.
    fn uniform_paint(
        gc: &GraphicsContext,
        collection: &PathCollection<'_>,
    ) -> Option<(bool, bool)> {
        let filled = match collection.facecolors.first() {
            None if gc.active_hatch().is_some() => return None,
            None => false,
            Some(first) if collection.facecolors.iter().all(|c| c.a == first.a) => first.a != 0.0,
            Some(_) => return None,
        };
        let stroked = match collection.edgecolors.first() {
            None => false,
            Some(_) if collection.linewidths.iter().all(|&w| w == 0.0) => false,
            Some(first) if collection.edgecolors.iter().all(|c| c.a == first.a) => first.a != 0.0,
            Some(_) => return None,
        };
        Some((filled, stroked))
    }

    /// `draw_path_collection`: one form `XObject` per path, placed per item,
    /// when every item paints alike and that is shorter.
    pub(super) fn pdf_path_collection(
        &mut self,
        gc: &GraphicsContext,
        collection: &PathCollection<'_>,
    ) {
        let len_path = collection.paths.first().map_or(0, Path::len);
        let uses = collection.uses_per_path();
        let paint =
            Self::uniform_paint(gc, collection).filter(|_| len_path + uses + 5 < len_path * uses);
        let Some((filled, stroked)) = paint else {
            for item in collection.items(gc) {
                self.pdf_path(&item.gc, item.path, item.transform, item.fill);
            }
            return;
        };
        let padding = collection
            .linewidths
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let padding = if padding.is_finite() { padding } else { 0.0 };
        let template = TemplatePaint {
            join: gc.join,
            cap: gc.cap,
            padding,
            filled,
            stroked,
        };
        let mut names = Vec::new();
        let mut boxes = Vec::new();
        for (path, transform) in collection.raw_paths() {
            names.push(
                self.resources
                    .path_template(&mut self.writer, path, transform, template),
            );
            boxes.push(extents(path, transform));
        }
        let push = self.states.push();
        self.output(&push);
        let canvas = self.canvas();
        let (mut last_x, mut last_y) = (0.0, 0.0);
        for item in collection.items(gc) {
            let bbox = boxes[item.path_index];
            let Point { x, y } = item.offset;
            if bbox.x1() + x < 0.0
                || bbox.x0() + x > canvas.x1()
                || bbox.y1() + y < 0.0
                || bbox.y0() + y > canvas.y1()
            {
                continue;
            }
            self.check_gc(&item.gc, item.fill);
            self.output(&place(&names[item.path_index], x - last_x, y - last_y));
            (last_x, last_y) = (x, y);
        }
        let pop = self.states.pop();
        self.output(&pop);
    }

    /// `draw_gouraud_triangles`: a shading with one alpha set through the
    /// graphics state, or a luminosity soft mask for varying alpha.
    pub(super) fn pdf_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: Affine2D,
    ) {
        let count = triangles.len().min(colors.len());
        if count == 0 {
            return;
        }
        let points: Vec<[Point; 3]> = triangles[..count]
            .iter()
            .map(|t| t.map(|p| transform.transform_point(p)))
            .collect();
        let colors = &colors[..count];
        let (name, _) = self.resources.gouraud(
            &mut self.writer,
            points.clone(),
            ShadingColors::Rgb(colors.to_vec()),
        );
        let alpha = colors[0][0].a;
        if colors.iter().flatten().all(|c| all_close(alpha, c.a)) {
            let opaque = GraphicsContext {
                alpha: Some(alpha),
                ..gc.clone()
            };
            self.check_gc(&opaque, None);
            self.output(&[Obj::name(name), Obj::op("sh")]);
        } else {
            let alphas = colors.iter().map(|t| t.map(|c| c.a)).collect();
            let (_, smask) =
                self.resources
                    .gouraud(&mut self.writer, points, ShadingColors::Gray(alphas));
            let state = self.resources.soft_mask_state(&mut self.writer, smask);
            self.output(&[
                Obj::op("q"),
                Obj::name(state),
                Obj::op("gs"),
                Obj::name(name),
                Obj::op("sh"),
                Obj::op("Q"),
            ]);
        }
    }

    /// `draw_image`: an image `XObject` scaled to its size at the image
    /// resolution, or mapped through `transform`.
    pub(super) fn pdf_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<Affine2D>,
    ) {
        let (w, h) = (image.width(), image.height());
        if w == 0 || h == 0 {
            return;
        }
        let gc = if transform.is_none() {
            // The opacity is already in the pixels.
            GraphicsContext {
                alpha: Some(1.0),
                ..gc.clone()
            }
        } else {
            gc.clone()
        };
        self.check_gc(&gc, None);
        let width = 72.0 * w as f64 / self.image_dpi;
        let height = 72.0 * h as f64 / self.image_dpi;
        let name = self.resources.image(&mut self.writer, image);
        let mut ops = vec![Obj::op("q")];
        match transform {
            None => ops.extend([
                Obj::Real(width),
                Obj::Int(0),
                Obj::Int(0),
                Obj::Real(height),
                Obj::Real(origin.x),
                Obj::Real(origin.y),
                Obj::op("cm"),
            ]),
            Some(transform) => {
                ops.extend([
                    Obj::Int(1),
                    Obj::Int(0),
                    Obj::Int(0),
                    Obj::Int(1),
                    Obj::Real(origin.x),
                    Obj::Real(origin.y),
                    Obj::op("cm"),
                ]);
                ops.extend(transform.values().map(Obj::Real));
                ops.push(Obj::op("cm"));
            }
        }
        ops.extend([Obj::name(name), Obj::op("Do"), Obj::op("Q")]);
        self.output(&ops);
    }
}
