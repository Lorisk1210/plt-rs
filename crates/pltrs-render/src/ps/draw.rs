//! Paths, markers, path collections, images and Gouraud triangles
//! (`draw_path`, `draw_markers`, `draw_path_collection`, `draw_image`,
//! `draw_gouraud_triangles`, `_draw_ps`).

#![expect(
    clippy::float_cmp,
    reason = "matplotlib compares colours and line properties exactly"
)]

use std::fmt::Write as _;

use pltrs_core::path::convert::{StringCodes, StringOptions};
use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};

use super::PsRenderer;
use crate::collection::PathCollection;
use crate::gc::{CapStyle, GraphicsContext, JoinStyle};
use crate::image::RgbaImage;
use crate::vector::{general, hex, segment_ends, trimmed};

/// `_nums_to_str`: three decimals without trailing zeros, space-separated.
pub(super) fn nums(values: &[f64]) -> String {
    values
        .iter()
        .map(|&v| trimmed(v, 3))
        .collect::<Vec<_>>()
        .join(" ")
}

/// matplotlib's PostScript codes for join styles.
pub(super) const fn join_code(join: JoinStyle) -> u8 {
    match join {
        JoinStyle::Miter => 0,
        JoinStyle::Round => 1,
        JoinStyle::Bevel => 2,
    }
}

/// matplotlib's PostScript codes for cap styles.
pub(super) const fn cap_code(cap: CapStyle) -> u8 {
    match cap {
        CapStyle::Butt => 0,
        CapStyle::Round => 1,
        CapStyle::Projecting => 2,
    }
}

/// `_is_transparent`: no colour, or zero alpha.
fn is_transparent(color: Option<Rgba>) -> bool {
    color.is_none_or(|c| c.a == 0.0)
}

impl PsRenderer {
    /// [`is_transparent`], noting a partially transparent colour, which
    /// PostScript draws opaque (matplotlib logs it once per document).
    pub(super) fn transparent(&mut self, color: Option<Rgba>) -> bool {
        if color.is_some_and(|c| c.a != 0.0 && c.a != 1.0) {
            self.partially_transparent = true;
        }
        is_transparent(color)
    }
}

/// `setgray` for grey colours, `setrgbcolor` otherwise.
pub(super) fn color_cmd(rgb: [f64; 3]) -> String {
    if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
        format!("{} setgray", nums(&rgb[..1]))
    } else {
        format!("{} setrgbcolor", nums(&rgb))
    }
}

/// Lowercase hex of `data` with a line break every 64 bytes
/// (`bytes.hex("\n", -64)`).
fn hex_lines(data: &[u8]) -> String {
    data.chunks(64).map(hex).collect::<Vec<_>>().join("\n")
}

impl PsRenderer {
    /// `_convert_path`: PostScript path operators, optionally clipped to
    /// the page and simplified.
    pub(super) fn convert(
        &self,
        path: &Path,
        transform: Affine2D,
        clip: bool,
        simplify: Option<f64>,
    ) -> String {
        path.convert_to_string(&StringOptions {
            transform,
            clip: clip.then(|| Bbox::from_extents(0.0, 0.0, self.width * 72.0, self.height * 72.0)),
            simplify,
            sketch: None,
            precision: 6,
            codes: StringCodes::PS,
            postfix: true,
        })
    }

    /// `_get_clip_cmd`: the clip rectangle, then the clip path's procedure
    /// (defined on first use).
    pub(super) fn clip_cmd(&mut self, gc: &GraphicsContext) -> String {
        let mut cmd = String::new();
        if let Some(rect) = gc.clip_rect {
            let (x, y, w, h) = rect.bounds();
            let _ = writeln!(cmd, "{} rectclip", nums(&[x, y, w, h]));
        }
        let clip = gc
            .clip_path
            .as_ref()
            .filter(|clip| clip.path.vertices().iter().all(|p| p.is_finite()));
        if let Some(clip) = clip {
            let known = self.clip_paths.iter().position(|c| c == clip);
            let index = known.unwrap_or_else(|| {
                let path = self.convert(&clip.path, clip.transform, false, None);
                let index = self.clip_paths.len();
                let _ = writeln!(
                    self.body,
                    "/c{index} {{\n{path}\nclip\nnewpath\n}} bind def"
                );
                self.clip_paths.push(clip.clone());
                index
            });
            let _ = writeln!(cmd, "c{index}");
        }
        cmd
    }

    /// `_draw_ps`: wraps the path commands `ps` with the properties of
    /// `gc`, then fills, hatches and strokes as requested.
    fn draw_ps(&mut self, ps: &str, gc: &GraphicsContext, face: Option<Rgba>, paint: bool) {
        let rgb = gc.stroke_color();
        let might_stroke = gc.linewidth > 0.0 && !self.transparent(Some(rgb));
        let stroke = paint && might_stroke;
        let face_transparent = self.transparent(face);
        let fill = paint && !face_transparent;
        let hatch = gc.hatch.clone();
        if might_stroke {
            self.set_line(gc);
        }
        if might_stroke || hatch.is_some() {
            self.set_color([rgb.r, rgb.g, rgb.b], true);
        }
        self.body.push_str("gsave\n");
        let clip = self.clip_cmd(gc);
        self.body.push_str(&clip);
        self.body.push_str(ps.trim());
        self.body.push('\n');
        if let (true, Some(face)) = (fill, face) {
            let nested = stroke || hatch.is_some();
            if nested {
                self.body.push_str("gsave\n");
            }
            self.set_color([face.r, face.g, face.b], false);
            self.body.push_str("fill\n");
            if nested {
                self.body.push_str("grestore\n");
            }
        }
        if let Some(hatch) = hatch {
            let name = self.create_hatch(&hatch.pattern, hatch.linewidth);
            let c = hatch.color;
            let _ = writeln!(
                self.body,
                "gsave\n{} {name} setpattern fill grestore",
                nums(&[c.r, c.g, c.b])
            );
        }
        if stroke {
            self.body.push_str("stroke\n");
        }
        self.body.push_str("grestore\n");
    }

    /// `draw_path`.
    pub(super) fn ps_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: Affine2D,
        face: Option<Rgba>,
    ) {
        let clip = face.is_none() && gc.hatch.is_none();
        let simplify = if clip {
            self.options.simplify.for_path(path)
        } else {
            None
        };
        let ps = self.convert(path, transform, clip, simplify);
        self.draw_ps(&ps, gc, face, true);
    }

    /// `draw_markers`: a marker procedure `o` called at every vertex.
    pub(super) fn ps_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: Affine2D,
        path: &Path,
        transform: Affine2D,
        face: Option<Rgba>,
    ) {
        let color = (!self.transparent(face))
            .then_some(face)
            .flatten()
            .map(|f| color_cmd([f.r, f.g, f.b]));
        let mut cmd: Vec<String> = ["/o {", "gsave", "newpath", "translate"]
            .map(str::to_owned)
            .to_vec();
        let alpha = gc.alpha.unwrap_or_else(|| gc.stroke_color().a);
        let stroke = gc.linewidth > 0.0 && alpha > 0.0;
        if stroke {
            cmd.push(format!("{:.1} setlinewidth", gc.linewidth));
            cmd.push(format!("{} setlinejoin\n", join_code(gc.join)));
            cmd.push(format!("{} setlinecap\n", cap_code(gc.cap)));
        }
        cmd.push(self.convert(marker, marker_transform, false, None));
        if face.is_some() {
            if stroke {
                cmd.push("gsave".to_owned());
            }
            if let Some(color) = color {
                cmd.push(color);
                cmd.push("fill".to_owned());
            }
            if stroke {
                cmd.push("grestore".to_owned());
            }
        }
        if stroke {
            cmd.push("stroke".to_owned());
        }
        cmd.push("grestore".to_owned());
        cmd.push("} bind def".to_owned());
        let page = Bbox::from_extents(0.0, 0.0, self.width * 72.0, self.height * 72.0);
        for at in segment_ends(path, transform, Some(page)) {
            cmd.push(format!("{} {} o", general(at.x), general(at.y)));
        }
        self.draw_ps(&cmd.join("\n"), gc, face, false);
    }

    /// `draw_path_collection`: one procedure per path when that is
    /// shorter than writing every path.
    pub(super) fn ps_path_collection(
        &mut self,
        gc: &GraphicsContext,
        collection: &PathCollection<'_>,
    ) {
        let len_path = collection.paths.first().map_or(0, |p| p.vertices().len());
        let uses = collection.uses_per_path();
        if len_path + 3 * uses + 3 >= (len_path + 2) * uses {
            for item in collection.items(gc) {
                self.ps_path(&item.gc, item.path, item.transform, item.fill);
            }
            return;
        }
        let mut names = Vec::new();
        for (i, (path, transform)) in collection.raw_paths().enumerate() {
            let name = format!("p{}_{i}", self.path_collection_id);
            let data = self.convert(path, transform, false, None);
            let _ = writeln!(
                self.body,
                "/{name} {{\nnewpath\ntranslate\n{data}\n}} bind def"
            );
            names.push(name);
        }
        for item in collection.items(gc) {
            let ps = format!(
                "{} {} {}",
                general(item.offset.x),
                general(item.offset.y),
                names[item.path_index]
            );
            self.draw_ps(&ps, &item.gc, item.fill, true);
        }
        self.path_collection_id += 1;
    }

    /// `draw_image`: hex RGB data read by `colorimage` (alpha is lost).
    pub(super) fn ps_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<Affine2D>,
    ) {
        let (w, h) = (image.width(), image.height());
        let rgb: Vec<u8> = image
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let magnification = self.image_dpi / 72.0;
        let (matrix, xscale, yscale) = match transform {
            None => (
                "1 0 0 1 0 0".to_owned(),
                w as f64 / magnification,
                h as f64 / magnification,
            ),
            Some(t) => (t.values().map(crate::vector::repr).join(" "), 1.0, 1.0),
        };
        let clip = self.clip_cmd(gc);
        let _ = writeln!(
            self.body,
            "gsave\n{clip}\n{} {} translate\n[{matrix}] concat\n{} {} scale\n\
             /DataString {w} string def\n{w} {h} 8 [ {w} 0 0 -{h} 0 {h} ]\n{{\n\
             currentfile DataString readhexstring pop\n}} bind false 3 colorimage\n\
             {}\ngrestore",
            general(origin.x),
            general(origin.y),
            general(xscale),
            general(yscale),
            hex_lines(&rgb)
        );
    }

    /// `draw_gouraud_triangles`: a type 4 shading of 32-bit coordinates
    /// (clipping and alpha are not applied, as in matplotlib).
    pub(super) fn ps_gouraud_triangles(
        &mut self,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: Affine2D,
    ) {
        let count = triangles.len().min(colors.len());
        if count == 0 {
            return;
        }
        let points: Vec<Point> = triangles[..count]
            .iter()
            .flatten()
            .map(|&p| transform.transform_point(p))
            .collect();
        let min = points
            .iter()
            .fold(Point::new(f64::INFINITY, f64::INFINITY), |a, &p| a.min(p));
        let max = points
            .iter()
            .fold(Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY), |a, &p| {
                a.max(p)
            });
        let pad = 4096.0;
        let (min, max) = (
            Point::new(min.x - pad, min.y - pad),
            Point::new(max.x + pad, max.y + pad),
        );
        let factor = Point::new(
            (f64::from(u32::MAX) / (max.x - min.x)).ceil(),
            (f64::from(u32::MAX) / (max.y - min.y)).ceil(),
        );
        let mut data = Vec::with_capacity(points.len() * 12);
        for (point, color) in points.iter().zip(colors[..count].iter().flatten()) {
            data.push(0);
            data.extend_from_slice(&(((point.x - min.x) * factor.x) as u32).to_be_bytes());
            data.extend_from_slice(&(((point.y - min.y) * factor.y) as u32).to_be_bytes());
            data.extend([color.r, color.g, color.b].map(|c| (c * 255.0) as u8));
        }
        let _ = writeln!(
            self.body,
            "gsave\n<< /ShadingType 4\n   /ColorSpace [/DeviceRGB]\n   /BitsPerCoordinate 32\n   \
             /BitsPerComponent 8\n   /BitsPerFlag 8\n   /AntiAlias true\n   \
             /Decode [ {} {} {} {} 0 1 0 1 0 1 ]\n   /DataSource <\n{}\n>\n>>\nshfill\ngrestore",
            general(min.x),
            general(max.x),
            general(min.y),
            general(max.y),
            hex_lines(&data)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_helpers() {
        assert_eq!(nums(&[75.6, 223.2, 1.0, 0.0]), "75.6 223.2 1 0");
        assert_eq!(color_cmd([0.5, 0.5, 0.5]), "0.5 setgray");
        assert_eq!(color_cmd([1.0, 0.0, 0.25]), "1 0 0.25 setrgbcolor");
        assert_eq!(
            hex_lines(&[0xab; 65])
                .lines()
                .map(str::len)
                .collect::<Vec<_>>(),
            [128, 2]
        );
        assert!(is_transparent(None));
        assert!(is_transparent(Some(Rgba::TRANSPARENT)));
        assert!(!is_transparent(Some(Rgba::new(0.0, 0.0, 0.0, 0.5))));
    }
}
