//! Renders a reference case with plt-raster, translating each op exactly
//! as matplotlib's `RendererAgg` drives Agg — i.e. what the future raster
//! backend has to do.

use plt_core::{Affine2D, Path, Point};
use plt_raster::{
    Antialias, Clip, ClipBox, DashPattern, FillRule, LineCap, LineJoin, Mask, Painter, Pixmap,
    Rgba8, StrokeStyle, draw_coverage, draw_image,
};

use super::{Case, Op, test_coverage, test_image};

/// matplotlib's hatch tile size at dpi 72.
const HATCH_SIZE: u32 = 72;

struct Backend<'a> {
    case: &'a Case,
    painter: Painter,
    pixmap: Pixmap,
}

/// Renders `case` onto a fresh matplotlib canvas.
pub(crate) fn render(case: &Case) -> Pixmap {
    let mut backend = Backend {
        case,
        painter: Painter::new(),
        pixmap: Pixmap::filled(case.width, case.height, Rgba8::TRANSPARENT_WHITE).expect("canvas"),
    };
    for op in &case.ops {
        backend.run(op);
    }
    backend.pixmap
}

impl Backend<'_> {
    fn path(&self, id: &str) -> &Path {
        &self.case.paths[id]
    }

    fn target(&self, op: &Op) -> &Path {
        self.path(op.target.as_deref().expect("op needs a path"))
    }

    /// The clip box as `set_clipbox` computes it (the full canvas if unset).
    fn cliprect(&self, op: &Op) -> Option<ClipBox> {
        let (w, h) = (self.case.width, self.case.height);
        op.floats("clip").map(|v| {
            let hf = f64::from(h);
            ClipBox::from_matplotlib_cliprect(v[0], hf - v[3], v[2], hf - v[1], w, h)
        })
    }

    fn rect_or_full(&self, op: &Op) -> ClipBox {
        self.cliprect(op)
            .unwrap_or_else(|| ClipBox::full(self.case.width, self.case.height))
    }

    /// The clip mask as `render_clippath` computes it; `rect` is the
    /// rasterizer clip box active at that time.
    fn mask(&mut self, op: &Op, rect: Option<ClipBox>) -> Option<Mask> {
        let id = op.arg("clippath")?;
        let path = self.case.paths[id].clone();
        Some(
            self.painter
                .coverage_mask(
                    &path,
                    self.case.width,
                    self.case.height,
                    FillRule::NonZero,
                    Antialias::On,
                    rect,
                )
                .expect("mask"),
        )
    }

    fn antialias(op: &Op) -> Antialias {
        Antialias::from(op.arg("aa") != Some("0"))
    }

    /// The `conv_stroke` (and `conv_dash`) settings of `_draw_path`.
    fn stroke_style(op: &Op) -> StrokeStyle {
        let lw = op.number("lw", 1.0);
        let aa = Self::antialias(op) == Antialias::On;
        let cap = match op.arg("cap").unwrap_or("butt") {
            "round" => LineCap::Round,
            "projecting" => LineCap::Square,
            _ => LineCap::Butt,
        };
        let join = match op.arg("join").unwrap_or("round") {
            "miter" => LineJoin::MiterRevert,
            "bevel" => LineJoin::Bevel,
            _ => LineJoin::Round,
        };
        let mut style = StrokeStyle::new(lw)
            .with_cap(cap)
            .with_join(join)
            .with_miter_limit(lw);
        if let Some(spec) = op.arg("dashes") {
            let (offset, lengths) = spec.split_once(':').expect("offset:lengths");
            let mut lengths = super::parse_floats(lengths);
            if !aa {
                // Non-AA dashes and widths are rounded (dash_to_stroke).
                for v in &mut lengths {
                    *v = v.trunc() + 0.5;
                }
                style.width = if lw < 0.5 { 0.5 } else { lw.round() };
            }
            let pattern = DashPattern::from_lengths(&lengths, offset.parse().expect("offset"));
            style = style.with_dashes(Some(pattern.expect("dash pattern")));
        }
        style
    }

    fn run(&mut self, op: &Op) {
        match op.kind.as_str() {
            "fill" => self.fill(op),
            "stroke" => self.stroke(op),
            "hatch" => self.hatch(op),
            "markers" => self.markers(op),
            "gouraud" => self.gouraud(op),
            "image" => self.image(op),
            "text" => self.text(op),
            other => panic!("unknown op {other}"),
        }
    }

    fn fill(&mut self, op: &Op) {
        let rect = self.rect_or_full(op);
        let mask = self.mask(op, Some(rect));
        let clip = Clip {
            rect: Some(rect),
            mask: mask.as_ref(),
        };
        let path = self.target(op).clone();
        let color = op.color("color").expect("color");
        self.painter
            .fill_path(
                &mut self.pixmap,
                &path,
                color,
                FillRule::NonZero,
                Self::antialias(op),
                &clip,
            )
            .expect("fill");
    }

    fn stroke(&mut self, op: &Op) {
        let rect = self.rect_or_full(op);
        let mask = self.mask(op, Some(rect));
        let clip = Clip {
            rect: Some(rect),
            mask: mask.as_ref(),
        };
        let path = self.target(op).clone();
        let color = op.color("color").expect("color");
        self.painter
            .stroke_path(
                &mut self.pixmap,
                &path,
                color,
                &Self::stroke_style(op),
                Self::antialias(op),
                &clip,
            )
            .expect("stroke");
    }

    fn hatch(&mut self, op: &Op) {
        let rect = self.rect_or_full(op);
        let mask = self.mask(op, Some(rect));
        let clip = Clip {
            rect: Some(rect),
            mask: mask.as_ref(),
        };
        let path = self.target(op).clone();
        if let Some(face) = op.color("face") {
            self.painter
                .fill_path(
                    &mut self.pixmap,
                    &path,
                    face,
                    FillRule::NonZero,
                    Self::antialias(op),
                    &clip,
                )
                .expect("face");
        }
        let s = f64::from(HATCH_SIZE);
        let to_tile = Affine2D::from_values(s, 0.0, 0.0, -s, 0.0, s);
        let hatch = self
            .path(op.arg("hatch").expect("hatch path"))
            .transformed(&to_tile);
        let tile = self
            .painter
            .hatch_tile(
                HATCH_SIZE,
                &hatch,
                op.color("hcolor").expect("hatch colour"),
                op.number("hlw", 1.0),
                Rgba8::TRANSPARENT_WHITE,
            )
            .expect("tile");
        self.painter
            .fill_path_with_pattern(&mut self.pixmap, &path, &tile, FillRule::NonZero, &clip)
            .expect("hatch");
    }

    fn markers(&mut self, op: &Op) {
        let half = Affine2D::translation(0.5, 0.5);
        let marker = self.target(op).transformed(&half);
        let style = Self::stroke_style(op);
        let face = op.color("face");
        let stamp = self
            .painter
            .marker_stamp(&marker, face.is_some(), Some(&style))
            .expect("stamp");
        let positions: Vec<Point> = op
            .floats("at")
            .expect("positions")
            .chunks(2)
            .map(|c| half.transform_point(Point::new(c[0], c[1])))
            .collect();
        // Marker clip paths are rasterized unclipped, and then the clip box
        // is ignored; otherwise the box applies inclusively.
        let mask = self.mask(op, None);
        let rect = if mask.is_some() {
            None
        } else {
            self.cliprect(op).map(ClipBox::inclusive)
        };
        let clip = Clip {
            rect,
            mask: mask.as_ref(),
        };
        stamp
            .draw(&mut self.pixmap, &positions, face, op.color("edge"), &clip)
            .expect("markers");
    }

    fn gouraud(&mut self, op: &Op) {
        let rect = self.rect_or_full(op);
        let mask = self.mask(op, Some(rect));
        let clip = Clip {
            rect: Some(rect),
            mask: mask.as_ref(),
        };
        let v = op.floats("tri").expect("triangle");
        let points = [0, 1, 2].map(|i| Point::new(v[2 * i], v[2 * i + 1]));
        let colors = ["c0", "c1", "c2"].map(|k| op.color(k).expect("corner colour"));
        self.painter
            .gouraud_triangle(&mut self.pixmap, points, colors, &clip)
            .expect("gouraud");
    }

    fn image(&mut self, op: &Op) {
        let at = op.floats("at").expect("at");
        let size = op.floats("size").expect("size");
        let image = test_image(size[0] as u32, size[1] as u32);
        let rect = self.rect_or_full(op);
        let mask = self.mask(op, Some(rect));
        let clip = match &mask {
            Some(mask) => Clip::rect(rect).with_mask(mask),
            None => Clip {
                rect: self.cliprect(op).map(ClipBox::inclusive),
                mask: None,
            },
        };
        // RendererAgg's wrapper forces gc.alpha = 1 for images.
        draw_image(
            &mut self.pixmap,
            &image,
            at[0] as i32,
            at[1] as i32,
            1.0,
            &clip,
        )
        .expect("image");
    }

    fn text(&mut self, op: &Op) {
        let at = op.floats("at").expect("at");
        let size = op.floats("size").expect("size");
        let coverage = test_coverage(size[0] as u32, size[1] as u32);
        let clip = Clip {
            rect: self.cliprect(op),
            mask: None,
        };
        let color = op.color("color").expect("color");
        draw_coverage(
            &mut self.pixmap,
            &coverage,
            at[0] as i32,
            at[1] as i32,
            color,
            &clip,
        )
        .expect("text");
    }
}
