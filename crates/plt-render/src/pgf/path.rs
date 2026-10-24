//! Paths, markers, hatches, clipping and images (matplotlib's
//! `draw_path`, `draw_markers`, `draw_image`, `_print_pgf_clip`,
//! `_print_pgf_path_styles` and `_print_pgf_path`).

#![expect(
    clippy::float_cmp,
    reason = "matplotlib compares opacities and line widths exactly"
)]

use std::fmt::Write as _;

use plt_codec::png::{self, ImageRef};
use plt_core::path::convert::CleanupOptions;
use plt_core::{Affine2D, Bbox, Path, PathCode, Point, Rgba, Segment};

use super::{PgfRenderer, latex_points, num};
use crate::gc::{CapStyle, GraphicsContext, Hatch, JoinStyle};
use crate::image::RgbaImage;
use crate::svg::ImageFile;
use crate::vector::{VectorError, flipped_rows, segment_starts};

/// TeX's largest dimension, in points.
const MAX_DIMENSION: f64 = 16383.0;

impl PgfRenderer {
    /// The largest coordinate TeX can handle, in display pixels.
    fn max_coordinate(&self) -> f64 {
        MAX_DIMENSION / super::LATEX_POINTS_PER_INCH * self.dpi
    }

    /// The rectangle of every coordinate TeX can handle.
    fn tex_extent(&self) -> Bbox {
        let max = self.max_coordinate();
        Bbox::from_extents(-max, -max, max, max)
    }

    /// `\pgfqpoint` of a point in display pixels.
    fn point(&self, p: Point) -> String {
        format!(
            r"\pgfqpoint{{{}in}}{{{}in}}",
            num(self.inches(p.x)),
            num(self.inches(p.y))
        )
    }

    /// Strokes and fills `path`, then tiles its hatch over it.
    pub(super) fn pgf_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.line(r"\begin{pgfscope}");
        self.clip(gc);
        self.path_styles(gc, fill);
        self.path_commands(Some(gc), path, transform, fill);
        self.path_draw(gc.linewidth != 0.0, fill.is_some());
        self.line(r"\end{pgfscope}");
        if let Some(hatch) = &gc.hatch {
            self.hatch(gc, hatch, path, transform, fill);
        }
    }

    /// Fills the path with its hatch: a one-inch tile repeated over its
    /// extents, clipped to it.
    fn hatch(
        &mut self,
        gc: &GraphicsContext,
        hatch: &Hatch,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let Ok(tile) =
            plt_core::hatch::pattern_path(&hatch.pattern, plt_core::hatch::DEFAULT_DENSITY)
        else {
            return;
        };
        self.line(r"\begin{pgfscope}");
        self.path_styles(gc, fill);
        self.clip(gc);
        self.path_commands(Some(gc), path, transform, fill);
        self.line(r"\pgfusepath{clip}");
        self.line(
            r"\pgfsys@defobject{currentpattern}{\pgfqpoint{0in}{0in}}{\pgfqpoint{1in}{1in}}{",
        );
        self.line(r"\begin{pgfscope}");
        self.line(&format!(
            r"\pgfsetlinewidth{{{}pt}}",
            num(latex_points(hatch.linewidth))
        ));
        self.define_color("currenthatch", hatch.color);
        self.line(r"\pgfsetstrokecolor{currenthatch}");
        self.line(&format!(r"\pgfsetstrokeopacity{{{}}}", num(hatch.color.a)));
        self.line(r"\pgfpathrectangle{\pgfqpoint{0in}{0in}}{\pgfqpoint{1in}{1in}}");
        self.line(r"\pgfusepath{clip}");
        self.path_commands(None, &tile, &Affine2D::scaling(self.dpi, self.dpi), None);
        self.path_draw(true, false);
        self.line(r"\end{pgfscope}");
        self.line("}");
        let extents = path.transformed(transform).extents();
        let (x0, y0) = (self.inches(extents.x0()), self.inches(extents.y0()));
        let (x1, y1) = (self.inches(extents.x1()), self.inches(extents.y1()));
        self.line(&format!(
            r"\pgfsys@transformshift{{{}in}}{{{}in}}",
            num(x0),
            num(y0)
        ));
        let repeats = |span: f64| {
            if span.is_finite() {
                span.ceil().max(0.0)
            } else {
                0.0
            }
        };
        let (columns, rows) = (repeats(x1 - x0), repeats(y1 - y0));
        for _ in 0..whole(rows) {
            for _ in 0..whole(columns) {
                self.line(r"\pgfsys@useobject{currentpattern}{}");
                self.line(r"\pgfsys@transformshift{1in}{0in}");
            }
            self.line(&format!(
                r"\pgfsys@transformshift{{-{}in}}{{0in}}",
                whole(columns)
            ));
            self.line(r"\pgfsys@transformshift{0in}{1in}");
        }
        self.line(r"\end{pgfscope}");
    }

    /// Draws `marker` at every vertex of `path`: defined once as a PGF
    /// object, then placed.
    pub(super) fn pgf_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.line(r"\begin{pgfscope}");
        self.clip(gc);
        self.path_styles(gc, fill);
        let (low, high) = numpy_extents(&marker.transformed(marker_transform));
        self.line(&format!(
            r"\pgfsys@defobject{{currentmarker}}{{{}}}{{{}}}{{",
            self.point(low),
            self.point(high)
        ));
        self.path_commands(None, marker, marker_transform, None);
        self.path_draw(gc.linewidth != 0.0, fill.is_some());
        self.line("}");
        for at in segment_starts(path, *transform, Some(self.tex_extent())) {
            self.line(r"\begin{pgfscope}");
            self.line(&format!(
                r"\pgfsys@transformshift{{{}in}}{{{}in}}",
                num(self.inches(at.x)),
                num(self.inches(at.y))
            ));
            self.line(r"\pgfsys@useobject{currentmarker}{}");
            self.line(r"\end{pgfscope}");
        }
        self.line(r"\end{pgfscope}");
    }

    /// Writes the image as a PNG file beside the document and includes it.
    pub(super) fn pgf_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        let (width, height) = (image.width(), image.height());
        if width == 0 || height == 0 {
            return;
        }
        let Some(stem) = self.options.image_stem.clone() else {
            self.problems.record::<()>(Err(VectorError::Unsupported(
                "streamed pgf-code does not support raster graphics: save to a file",
            )));
            return;
        };
        let rows = flipped_rows(image);
        let encoded = u32::try_from(width)
            .ok()
            .zip(u32::try_from(height).ok())
            .ok_or(VectorError::Unsupported("image too large for PNG"))
            .and_then(|(w, h)| {
                ImageRef::rgba8(w, h, &rows)
                    .and_then(png::encode)
                    .map_err(VectorError::Image)
            });
        let Some(png) = self.problems.record(encoded) else {
            return;
        };
        let name = format!("{stem}-img{}.png", self.images.len());
        self.images.push(ImageFile {
            name: name.clone(),
            png,
        });
        self.line(r"\begin{pgfscope}");
        self.clip(gc);
        let (size, interpolate) = match transform {
            None => {
                self.line(&format!(
                    r"\pgfsys@transformshift{{{}in}}{{{}in}}",
                    num(self.inches(origin.x)),
                    num(self.inches(origin.y))
                ));
                (
                    (self.pixels_inches(width), self.pixels_inches(height)),
                    "true",
                )
            }
            Some(transform) => {
                let [a, b, c, d, e, f] = transform.values();
                let s = 1.0 / self.dpi;
                self.line(&format!(
                    r"\pgfsys@transformcm{{{}}}{{{}}}{{{}}}{{{}}}{{{}in}}{{{}in}}",
                    num(a * s),
                    num(b * s),
                    num(c * s),
                    num(d * s),
                    num((e + origin.x) * s),
                    num((f + origin.y) * s)
                ));
                // The transform includes the scale.
                ((1.0, 1.0), "false")
            }
        };
        self.line(&format!(
            r"\pgftext[left,bottom]{{\includegraphics[interpolate={interpolate},width={}in,height={}in]{{{name}}}}}",
            num(size.0),
            num(size.1)
        ));
        self.line(r"\end{pgfscope}");
    }

    /// A pixel count in inches.
    fn pixels_inches(&self, pixels: usize) -> f64 {
        self.inches(pixels as f64)
    }

    /// Clips to the clip rectangle and clip path of `gc`.
    pub(super) fn clip(&mut self, gc: &GraphicsContext) {
        if let Some(bbox) = gc.clip_rect {
            self.line(&format!(
                r"\pgfpathrectangle{{{}}}{{{}}}",
                self.point(Point::new(bbox.x0(), bbox.y0())),
                self.point(Point::new(bbox.width(), bbox.height()))
            ));
            self.line(r"\pgfusepath{clip}");
        }
        if let Some(clip) = &gc.clip_path {
            self.path_commands(Some(gc), &clip.path, &clip.transform, None);
            self.line(r"\pgfusepath{clip}");
        }
    }

    /// Defines the colour `name` as the RGB of `color`.
    pub(super) fn define_color(&mut self, name: &str, color: Rgba) {
        self.line(&format!(
            r"\definecolor{{{name}}}{{rgb}}{{{},{},{}}}",
            num(color.r),
            num(color.g),
            num(color.b)
        ));
    }

    /// Cap, join, fill and stroke colours, opacities, line width and dash
    /// pattern.
    fn path_styles(&mut self, gc: &GraphicsContext, fill: Option<Rgba>) {
        self.line(match gc.cap {
            CapStyle::Butt => r"\pgfsetbuttcap",
            CapStyle::Round => r"\pgfsetroundcap",
            CapStyle::Projecting => r"\pgfsetrectcap",
        });
        self.line(match gc.join {
            JoinStyle::Miter => r"\pgfsetmiterjoin",
            JoinStyle::Round => r"\pgfsetroundjoin",
            JoinStyle::Bevel => r"\pgfsetbeveljoin",
        });
        let (fill_opacity, stroke_opacity) = match gc.alpha {
            Some(alpha) => (alpha, alpha),
            None => (fill.map_or(1.0, |face| face.a), gc.foreground.a),
        };
        if let Some(face) = fill {
            self.define_color("currentfill", face);
            self.line(r"\pgfsetfillcolor{currentfill}");
            if fill_opacity != 1.0 {
                self.line(&format!(r"\pgfsetfillopacity{{{}}}", num(fill_opacity)));
            }
        }
        self.line(&format!(
            r"\pgfsetlinewidth{{{}pt}}",
            num(latex_points(gc.linewidth))
        ));
        self.define_color("currentstroke", gc.foreground);
        self.line(r"\pgfsetstrokecolor{currentstroke}");
        if stroke_opacity != 1.0 {
            self.line(&format!(r"\pgfsetstrokeopacity{{{}}}", num(stroke_opacity)));
        }
        if let Some(pattern) = &gc.dashes.pattern {
            let dashes = pattern.iter().fold(String::new(), |mut out, &dash| {
                let _ = write!(out, "{{{}pt}}", num(dash));
                out
            });
            self.line(&format!(
                r"\pgfsetdash{{{dashes}}}{{{}pt}}",
                num(gc.dashes.offset)
            ));
        } else {
            self.line(r"\pgfsetdash{}{0pt}");
        }
    }

    /// The path's segments as PGF path commands, clipped to the clip
    /// rectangle of `gc` (unless filled) and to what TeX can handle, then
    /// decorated to look sketched.
    fn path_commands(
        &mut self,
        gc: Option<&GraphicsContext>,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let max = self.max_coordinate();
        let clip = match gc.and_then(|gc| gc.clip_rect) {
            Some(rect) if fill.is_none() => Bbox::from_extents(
                rect.x0().max(-max),
                rect.y0().max(-max),
                rect.x1().min(max),
                rect.y1().min(max),
            ),
            _ => self.tex_extent(),
        };
        let options = CleanupOptions {
            transform: *transform,
            clip: Some(clip),
            ..CleanupOptions::segment_defaults(path, &self.options.simplify)
        };
        for segment in path.iter_segments(&options) {
            let command = match segment {
                Segment::MoveTo(p) => format!(r"\pgfpathmoveto{{{}}}", self.point(p)),
                Segment::LineTo(p) => format!(r"\pgfpathlineto{{{}}}", self.point(p)),
                Segment::QuadTo(c, p) => format!(
                    r"\pgfpathquadraticcurveto{{{}}}{{{}}}",
                    self.point(c),
                    self.point(p)
                ),
                Segment::CubicTo(c1, c2, p) => format!(
                    r"\pgfpathcurveto{{{}}}{{{}}}{{{}}}",
                    self.point(c1),
                    self.point(c2),
                    self.point(p)
                ),
                Segment::Close => r"\pgfpathclose".to_owned(),
            };
            self.line(&command);
        }
        if let Some(sketch) = gc.and_then(|gc| gc.sketch) {
            // PGF's random steps resemble matplotlib's sketch with half the
            // length and twice the amplitude; the randomness seeds them.
            self.line(r"\usepgfmodule{decorations}");
            self.line(r"\usepgflibrary{decorations.pathmorphing}");
            self.line(&format!(
                r"\pgfkeys{{/pgf/decoration/.cd, segment length = {}in, amplitude = {}in}}",
                num(self.inches(sketch.length * 0.5)),
                num(self.inches(sketch.scale * 2.0))
            ));
            self.line(&format!(
                r"\pgfmathsetseed{{{}}}",
                sketch.randomness.trunc()
            ));
            self.line(r"\pgfdecoratecurrentpath{random steps}");
        }
    }

    /// Uses the current path to stroke and/or fill.
    fn path_draw(&mut self, stroke: bool, fill: bool) {
        let actions: Vec<&str> = [(stroke, "stroke"), (fill, "fill")]
            .into_iter()
            .filter_map(|(used, action)| used.then_some(action))
            .collect();
        self.line(&format!(r"\pgfusepath{{{}}}", actions.join(",")));
    }
}

/// The corners of `path`'s extents as matplotlib's `Path.get_extents`
/// finds them: for paths of lines, `NumPy`'s minimum and maximum of the
/// move and line vertices, NaN as soon as one coordinate is; infinities
/// pointing inwards for no vertices.
fn numpy_extents(path: &Path) -> (Point, Point) {
    if path.has_curves() {
        let bbox = path.extents();
        return (
            Point::new(bbox.x0(), bbox.y0()),
            Point::new(bbox.x1(), bbox.y1()),
        );
    }
    let nan_aware = |pick: fn(f64, f64) -> f64| {
        move |a: f64, b: f64| {
            if a.is_nan() || b.is_nan() {
                f64::NAN
            } else {
                pick(a, b)
            }
        }
    };
    let (min, max) = (nan_aware(f64::min), nan_aware(f64::max));
    path.vertices()
        .iter()
        .zip(path.resolved_codes())
        .filter(|&(_, code)| code != PathCode::ClosePoly)
        .fold(
            (
                Point::new(f64::INFINITY, f64::INFINITY),
                Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |(low, high), (p, _)| {
                (
                    Point::new(min(low.x, p.x), min(low.y, p.y)),
                    Point::new(max(high.x, p.x), max(high.y, p.y)),
                )
            },
        )
}

/// A whole, non-negative count as an integer.
fn whole(count: f64) -> u64 {
    count as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extents_propagate_nan_like_numpy() {
        let line = Path::polyline(vec![Point::new(0.0, 1.0), Point::new(2.0, -1.0)]);
        assert_eq!(
            numpy_extents(&line),
            (Point::new(0.0, -1.0), Point::new(2.0, 1.0))
        );
        let broken = Path::polyline(vec![Point::new(0.0, 1.0), Point::new(f64::NAN, -1.0)]);
        let (low, high) = numpy_extents(&broken);
        assert!(low.x.is_nan() && high.x.is_nan());
        assert_eq!((low.y, high.y), (-1.0, 1.0));
        let (low, high) = numpy_extents(&Path::polyline(Vec::new()));
        assert_eq!((low.x, high.x), (f64::INFINITY, f64::NEG_INFINITY));
    }
}
