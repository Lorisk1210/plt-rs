//! Geometry conditioning and graphics-context translation, mirroring how
//! `_backend_agg.h` prepares paths, clip masks and strokes.

use pltrs_core::path::convert::{SketchParams, SnapMode as PathSnap, VertexSource};
use pltrs_core::{Affine2D, Bbox, Path};
use pltrs_raster::{
    Antialias, ClipBox, DashPattern, FillRule, LineCap, LineJoin, Mask, StrokeStyle,
};

use super::{CachedClip, RasterRenderer};
use crate::gc::{CapStyle, GraphicsContext, JoinStyle, Sketch, SnapMode};

/// matplotlib's `mpl_round`: round half away from zero.
pub(super) fn mpl_round(v: f64) -> f64 {
    (v + if v >= 0.0 { 0.5 } else { -0.5 }).trunc()
}

/// The path converters' snapping mode for a graphics context's.
pub(super) const fn snap_mode(mode: SnapMode) -> PathSnap {
    match mode {
        SnapMode::Auto => PathSnap::Auto,
        SnapMode::Always => PathSnap::Always,
        SnapMode::Never => PathSnap::Never,
    }
}

/// The sketch filter's parameters, if sketching is on.
pub(super) fn sketch_params(sketch: Option<Sketch>) -> Option<SketchParams> {
    sketch.map(Sketch::params)
}

/// The cached clip mask, if `active` (a free function so that callers can
/// borrow the renderer's other fields mutably at the same time).
pub(super) fn cached_mask(cache: Option<&CachedClip>, active: bool) -> Option<&Mask> {
    cache.filter(|_| active).map(|c| &c.mask)
}

/// How a draw call conditions its path before rasterization.
#[derive(Clone, Copy, Debug)]
pub(super) struct Conditioning {
    /// Whether the source path has codes (`PathNanRemover`'s `has_codes`).
    pub(super) has_codes: bool,
    /// Whether to clip line segments to the canvas (`PathClipper`).
    pub(super) clip: bool,
    /// Pixel snapping mode.
    pub(super) snap: PathSnap,
    /// Stroke width in pixels used to choose the snapping offset.
    pub(super) snap_width: f64,
    /// Simplification threshold, if simplifying.
    pub(super) simplify: Option<f64>,
    /// Sketch parameters, if sketching.
    pub(super) sketch: Option<SketchParams>,
}

impl RasterRenderer {
    /// Points to pixels at this canvas's resolution.
    pub(super) fn px(&self, points: f64) -> f64 {
        points * self.dpi / 72.0
    }

    /// Display (y up) to device pixels (y down), appended exactly like
    /// Agg's `trans *= scaling(1, -1); trans *= translation(0, height)`.
    pub(super) fn to_device(&self, transform: &Affine2D) -> Affine2D {
        transform
            .then(Affine2D::scaling(1.0, -1.0))
            .then(Affine2D::translation(0.0, f64::from(self.height())))
    }

    /// The canvas rectangle `PathClipper` clips to.
    fn canvas_bbox(&self) -> Bbox {
        Bbox::from_extents(0.0, 0.0, f64::from(self.width()), f64::from(self.height()))
    }

    /// The whole canvas as a clip box.
    pub(super) fn full_box(&self) -> ClipBox {
        ClipBox::full(self.width(), self.height())
    }

    /// The clip rectangle as `set_clipbox` computes it; `None` when unset
    /// (matplotlib also treats an all-zero rectangle as unset).
    pub(super) fn clip_box(&self, gc: &GraphicsContext) -> Option<ClipBox> {
        let rect = gc.clip_rect?;
        let edges = [rect.x0(), rect.y0(), rect.x1(), rect.y1()];
        if edges.iter().all(|&v| v == 0.0) {
            return None;
        }
        Some(ClipBox::from_matplotlib_cliprect(
            edges[0],
            edges[1],
            edges[2],
            edges[3],
            self.width(),
            self.height(),
        ))
    }

    /// Runs `path` through the path converters into device pixels, stored
    /// in `out`. Stages that would pass every vertex through are left out
    /// of the chain: most paths, and the many small ones of collections,
    /// need no clipping, simplification, curve flattening or sketching,
    /// and those with finite points are transformed and snapped in one
    /// pass.
    pub(super) fn condition_into(
        &self,
        path: &Path,
        transform: Affine2D,
        c: &Conditioning,
        out: &mut Path,
    ) {
        if !c.clip
            && c.simplify.is_none()
            && c.sketch.is_none()
            && path.transform_and_snap_into(transform, c.snap, c.snap_width, out)
        {
            return;
        }
        let source = path
            .iter_vertices()
            .transform(transform)
            .remove_nans(true, c.has_codes);
        if c.clip {
            snap_and_finish(source.clip_to(Some(self.canvas_bbox())), path, c, out);
        } else {
            snap_and_finish(source, path, c, out);
        }
    }

    /// Makes the clip mask of `gc` current, rasterizing it with the
    /// rasterizer clip box `rect` (`None`: unclipped) unless the same clip
    /// path and transform were rasterized last (matplotlib's
    /// `render_clippath`). Returns whether a clip path applies.
    pub(super) fn prepare_clip_mask(
        &mut self,
        gc: &GraphicsContext,
        rect: Option<ClipBox>,
    ) -> bool {
        let Some(clip) = gc.clip_path.as_ref().filter(|c| !c.path.is_empty()) else {
            return false;
        };
        if self
            .clip_cache
            .as_ref()
            .is_some_and(|cached| cached.key == *clip)
        {
            return true;
        }
        let path = &clip.path;
        let has_codes = path.codes().is_some();
        let simplify = if has_codes {
            None
        } else {
            self.simplify.for_path(path)
        };
        let conditioned = path
            .iter_vertices()
            .transform(self.to_device(&clip.transform))
            .remove_nans(true, has_codes)
            .snap(snap_mode(gc.snap), path.len(), 0.0)
            .simplify(simplify)
            .flatten_curves(true)
            .collect_path();
        let (width, height) = (self.width(), self.height());
        match self.painter.coverage_mask(
            &conditioned,
            width,
            height,
            FillRule::NonZero,
            Antialias::On,
            rect,
        ) {
            Ok(mask) => {
                self.clip_cache = Some(CachedClip {
                    key: clip.clone(),
                    mask,
                });
                true
            }
            Err(err) => {
                self.errors.push(err);
                self.clip_cache = None;
                false
            }
        }
    }

    /// The stroke `_draw_path` configures: the solid style plus the dash
    /// pattern scaled to pixels. Non-anti-aliased dashed strokes get rounded
    /// widths and lengths, as in matplotlib.
    pub(super) fn stroke_style(&self, gc: &GraphicsContext) -> StrokeStyle {
        let style = self.solid_stroke_style(gc);
        let Some(pattern) = self.dash_pattern(gc) else {
            return style;
        };
        let width = style.width;
        let style = style.with_dashes(Some(pattern));
        if gc.antialiased {
            style
        } else {
            StrokeStyle {
                width: if width < 0.5 { 0.5 } else { mpl_round(width) },
                ..style
            }
        }
    }

    /// The undashed stroke of `gc` (as markers use it): width, caps, joins
    /// (matplotlib's `"miter"` is Agg's `miter_join_revert`) and a miter
    /// limit equal to the width.
    pub(super) fn solid_stroke_style(&self, gc: &GraphicsContext) -> StrokeStyle {
        let width = self.px(gc.linewidth);
        let cap = match gc.cap {
            CapStyle::Butt => LineCap::Butt,
            CapStyle::Projecting => LineCap::Square,
            CapStyle::Round => LineCap::Round,
        };
        let join = match gc.join {
            JoinStyle::Miter => LineJoin::MiterRevert,
            JoinStyle::Round => LineJoin::Round,
            JoinStyle::Bevel => LineJoin::Bevel,
        };
        StrokeStyle::new(width)
            .with_cap(cap)
            .with_join(join)
            .with_miter_limit(width)
    }

    /// `Dashes::dash_to_stroke`: pairs (odd lists repeated) in pixels.
    fn dash_pattern(&self, gc: &GraphicsContext) -> Option<DashPattern> {
        if gc.dashes.is_solid() {
            return None;
        }
        let scale = self.dpi / 72.0;
        let lengths: Vec<f64> = gc
            .dashes
            .lengths()
            .iter()
            .map(|&v| {
                let v = v * scale;
                if gc.antialiased { v } else { v.trunc() + 0.5 }
            })
            .collect();
        DashPattern::from_lengths(&lengths, gc.dashes.offset * scale).ok()
    }
}

/// The rest of [`RasterRenderer::condition_into`] after clipping: snaps
/// `source` (the stream of `path`), then simplifies, flattens and
/// sketches it into `out`.
fn snap_and_finish<S: VertexSource>(source: S, path: &Path, c: &Conditioning, out: &mut Path) {
    let snapped = source.snap(c.snap, path.len(), c.snap_width);
    if c.simplify.is_some() || c.sketch.is_some() {
        snapped
            .simplify(c.simplify)
            .flatten_curves(true)
            .sketch(c.sketch)
            .collect_path_into(out);
    } else if path.has_curves() {
        snapped.flatten_curves(true).collect_path_into(out);
    } else {
        snapped.collect_path_into(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gc::Dashes;

    #[test]
    fn rounding_and_mode_mapping() {
        assert_eq!(mpl_round(2.5), 3.0);
        assert_eq!(mpl_round(-2.5), -3.0);
        assert_eq!(mpl_round(1.49), 1.0);
        assert_eq!(snap_mode(SnapMode::Never), PathSnap::Never);
        let s = sketch_params(Some(Sketch {
            scale: 2.0,
            length: 0.0,
            randomness: 3.0,
        }))
        .unwrap();
        assert_eq!(
            (s.length, s.randomness),
            (SketchParams::DEFAULT_LENGTH, 3.0)
        );
    }

    #[test]
    fn stroke_styles_follow_matplotlib() {
        let r = RasterRenderer::new(10.0, 10.0, 144.0).unwrap();
        let gc = GraphicsContext {
            linewidth: 1.5,
            join: JoinStyle::Miter,
            cap: CapStyle::Projecting,
            ..GraphicsContext::default()
        };
        let style = r.stroke_style(&gc);
        assert_eq!((style.width, style.miter_limit), (3.0, 3.0));
        assert_eq!(
            (style.cap, style.join),
            (LineCap::Square, LineJoin::MiterRevert)
        );
        assert!(style.dashes.is_none());

        let mut dashed = gc.clone();
        dashed.dashes = Dashes::new(0.5, vec![1.3, 0.6, 2.0]);
        let style = r.stroke_style(&dashed);
        let pattern = style.dashes.unwrap();
        assert_eq!(pattern.pairs(), &[(2.6, 1.2), (4.0, 2.6), (1.2, 4.0)]);
        assert_eq!(pattern.offset(), 1.0);

        dashed.antialiased = false;
        let style = r.stroke_style(&dashed);
        assert_eq!(style.width, 3.0);
        assert_eq!(style.dashes.unwrap().pairs()[0], (2.5, 1.5));
    }

    #[test]
    fn clip_boxes_follow_set_clipbox() {
        let r = RasterRenderer::new(40.0, 30.0, 72.0).unwrap();
        let mut gc = GraphicsContext::default();
        assert_eq!(r.clip_box(&gc), None);
        gc.clip_rect = Some(Bbox::from_extents(0.0, 0.0, 0.0, 0.0));
        assert_eq!(r.clip_box(&gc), None);
        gc.clip_rect = Some(Bbox::from_extents(10.4, 5.5, 30.5, 20.2));
        assert_eq!(r.clip_box(&gc), Some(ClipBox::new(10, 10, 31, 25)));
        let flipped = r
            .to_device(&Affine2D::identity())
            .transform_point(pltrs_core::Point::new(1.0, 2.0));
        assert_eq!(flipped, pltrs_core::Point::new(1.0, 28.0));
    }
}
