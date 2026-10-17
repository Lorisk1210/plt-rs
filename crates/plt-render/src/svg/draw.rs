//! Paths, markers, path collections and images (`draw_path`,
//! `draw_markers`, `draw_path_collection`, `draw_image`).

#![allow(
    clippy::float_cmp,
    reason = "matplotlib's SVG writer leaves out exactly-default values"
)]

use plt_codec::png::{self, ImageRef};
use plt_core::path::convert::{StringCodes, StringOptions};
use plt_core::{Affine2D, Bbox, Path, Point, Rgba};

use super::SvgRenderer;
use super::defs::path_data;
use super::style::{TransformOp, css, style_dict, transform_attr};
use crate::collection::PathCollection;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::vector::{
    VectorError, base64_encode, flipped_rows, repr_bytes, repr_str, segment_ends, short,
};

impl SvgRenderer {
    /// The inline style of a filled and/or stroked element.
    fn style(&mut self, gc: &GraphicsContext, face: Option<Rgba>) -> String {
        let hatch = self.hatch_id(gc, face);
        css(&style_dict(gc, face, hatch.as_deref()))
    }

    /// `draw_path`: unfilled, unhatched paths are clipped to the canvas
    /// and simplified.
    pub(super) fn svg_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: Affine2D,
        face: Option<Rgba>,
    ) {
        let clip = face.is_none() && gc.active_hatch().is_none();
        let d = path.convert_to_string(&StringOptions {
            transform: transform.then(self.flip()),
            clip: clip.then(|| Bbox::from_extents(0.0, 0.0, self.width, self.height)),
            simplify: if clip {
                self.options.simplify.for_path(path)
            } else {
                None
            },
            sketch: gc.sketch.map(crate::gc::Sketch::params),
            precision: 6,
            codes: StringCodes::SVG,
            postfix: false,
        });
        let link = self.open_link(gc);
        let mut attributes = vec![("d", d)];
        attributes.extend(self.clip_attribute(gc));
        attributes.push(("style", self.style(gc, face)));
        self.writer.element("path", "", &attributes);
        self.close_link(link);
    }

    /// `draw_markers`: the marker is defined once per path and style, then
    /// placed with `<use>` at every vertex.
    pub(super) fn svg_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: Affine2D,
        path: &Path,
        transform: Affine2D,
        face: Option<Rgba>,
    ) {
        if path.vertices().is_empty() {
            return;
        }
        let data = path_data(marker, marker_transform.then(Affine2D::scaling(1.0, -1.0)));
        let hatch = self.hatch_id(gc, face);
        let style = style_dict(gc, face, hatch.as_deref());
        let key = (data, css(&style));
        let id = if let Some(id) = self.markers.get(&key) {
            id.clone()
        } else {
            let content = format!("({}, {})", repr_str(&key.0), repr_str(&key.1));
            let id = self.make_id("m", &content);
            let stroke: Vec<_> = style
                .into_iter()
                .filter(|(name, _)| name.starts_with("stroke"))
                .collect();
            self.writer.start("defs", &[]);
            self.writer.element(
                "path",
                "",
                &[
                    ("id", id.clone()),
                    ("d", key.0.clone()),
                    ("style", css(&stroke)),
                ],
            );
            self.writer.end(true);
            self.markers.insert(key, id.clone());
            id
        };
        let clip: Vec<_> = self.clip_attribute(gc).into_iter().collect();
        self.writer.start("g", &clip);
        let link = self.open_link(gc);
        // matplotlib multiplies the canvas size, already in points, by 72
        // again, so this clip box never removes anything that matters.
        let canvas = Bbox::from_extents(0.0, 0.0, self.width * 72.0, self.height * 72.0);
        for at in segment_ends(path, transform.then(self.flip()), Some(canvas)) {
            let style = self.style(gc, face);
            self.writer.element(
                "use",
                "",
                &[
                    ("xlink:href", format!("#{id}")),
                    ("x", short(at.x)),
                    ("y", short(at.y)),
                    ("style", style),
                ],
            );
        }
        self.close_link(link);
        self.writer.end(true);
    }

    /// `draw_path_collection`: shapes are defined once and placed with
    /// `<use>` when that is shorter than writing every path.
    pub(super) fn svg_path_collection(
        &mut self,
        gc: &GraphicsContext,
        collection: &PathCollection<'_>,
    ) {
        let len_path = collection.paths.first().map_or(0, |p| p.vertices().len());
        let uses = collection.uses_per_path();
        if len_path + 9 * uses + 3 >= (len_path + 5) * uses {
            for item in collection.items(gc) {
                self.svg_path(&item.gc, item.path, item.transform, item.fill);
            }
            return;
        }
        self.writer.start("defs", &[]);
        let mut ids = Vec::new();
        for (i, (path, transform)) in collection.raw_paths().enumerate() {
            let d = path_data(path, transform.then(Affine2D::scaling(1.0, -1.0)));
            let id = format!(
                "C{:x}_{:x}_{}",
                self.path_collection_id,
                i,
                self.make_id("", &d)
            );
            self.writer
                .element("path", "", &[("id", id.clone()), ("d", d)]);
            ids.push(id);
        }
        self.writer.end(true);
        for item in collection.items(gc) {
            let link = self.open_link(&item.gc);
            let clip = self.clip_attribute(&item.gc);
            if let Some(clip) = &clip {
                self.writer.start("g", std::slice::from_ref(clip));
            }
            let style = self.style(&item.gc, item.fill);
            self.writer.element(
                "use",
                "",
                &[
                    ("xlink:href", format!("#{}", ids[item.path_index])),
                    ("x", short(item.offset.x)),
                    ("y", short(self.height - item.offset.y)),
                    ("style", style),
                ],
            );
            if clip.is_some() {
                self.writer.end(true);
            }
            self.close_link(link);
        }
        self.path_collection_id += 1;
    }

    /// `draw_image`: a base64 PNG, flipped back by its transform.
    pub(super) fn svg_image(
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
        let clip = self.clip_attribute(gc);
        if let Some(clip) = &clip {
            self.writer.start("g", std::slice::from_ref(clip));
        }
        let link = self.open_link(gc);
        let rows = flipped_rows(image);
        let encoded = ImageRef::rgba8(w as u32, h as u32, &rows)
            .and_then(png::encode)
            .map_err(VectorError::from);
        if let Some(file) = self.problems.record(encoded) {
            let id = gc
                .gid
                .clone()
                .filter(|gid| !gid.is_empty())
                .unwrap_or_else(|| self.make_id("image", &repr_bytes(&file)));
            let mut attributes = vec![
                (
                    "xlink:href",
                    format!("data:image/png;base64,\n{}", base64_encode(&file)),
                ),
                ("id", id),
            ];
            let (w, h) = (w as f64, h as f64);
            match transform {
                None => {
                    let width = 72.0 * w / self.image_dpi;
                    let height = 72.0 * h / self.image_dpi;
                    attributes.extend([
                        (
                            "transform",
                            transform_attr(&[
                                TransformOp::ScaleXY(1.0, -1.0),
                                TransformOp::Translate(0.0, -height),
                            ]),
                        ),
                        ("x", short(origin.x)),
                        ("y", short(-(self.height - origin.y - height))),
                        ("width", short(width)),
                        ("height", short(height)),
                    ]);
                }
                Some(transform) => {
                    if let Some(alpha) = gc.alpha.filter(|&a| a != 1.0) {
                        attributes.push(("opacity", short(alpha)));
                    }
                    let flipped = Affine2D::scaling(1.0 / w, 1.0 / h).then(transform).then(
                        Affine2D::translation(origin.x, origin.y)
                            .scale(1.0, -1.0)
                            .translate(0.0, self.height),
                    );
                    attributes.extend([
                        ("width", short(w)),
                        ("height", short(h)),
                        ("transform", transform_attr(&[TransformOp::Matrix(flipped)])),
                        (
                            "style",
                            "image-rendering:crisp-edges;image-rendering:pixelated".to_owned(),
                        ),
                    ]);
                }
            }
            self.writer.element("image", "", &attributes);
        }
        self.close_link(link);
        if clip.is_some() {
            self.writer.end(true);
        }
    }
}
