//! matplotlib's `Path.cleaned` / `Path.iter_segments` on top of the
//! conditioning pipeline (C++ `cleanup_path`).

use super::{
    Clipper, CurveFlattener, NanRemover, PathVertices, Simplifier, SketchParams, Sketcher,
    SnapMode, Snapper, Transformed, Vertex, VertexSource,
};
use crate::path::{Path, PathCode, Segment};
use crate::{Affine2D, Bbox};

/// matplotlib's `path.simplify` and `path.simplify_threshold` rcParams,
/// which decide `Path.should_simplify`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimplifySettings {
    /// `path.simplify`.
    pub enabled: bool,
    /// `path.simplify_threshold`, in pixels.
    pub threshold: f64,
}

impl Default for SimplifySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: Self::DEFAULT_THRESHOLD,
        }
    }
}

impl SimplifySettings {
    /// matplotlib's default `path.simplify_threshold` (literally
    /// `0.111111111111`, not 1/9).
    pub const DEFAULT_THRESHOLD: f64 = 0.111_111_111_111;

    /// matplotlib's `Path.should_simplify`: a positive threshold, enabled
    /// simplification, at least 128 vertices and only moves and lines.
    pub fn applies_to(&self, path: &Path) -> bool {
        self.threshold > 0.0
            && self.enabled
            && path.len() >= 128
            && path.codes().is_none_or(|codes| {
                codes
                    .iter()
                    .all(|c| matches!(c, PathCode::MoveTo | PathCode::LineTo))
            })
    }

    /// The simplification threshold to use for `path` when its
    /// simplification follows `should_simplify` (`simplify=None`).
    pub fn for_path(&self, path: &Path) -> Option<f64> {
        self.applies_to(path).then_some(self.threshold)
    }
}

/// The parameters of matplotlib's `Path.cleaned` (and of `cleanup_path`).
/// The defaults are those of `Path.cleaned`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CleanupOptions {
    /// Applied to every vertex first (matplotlib applies the identity when
    /// no transform is given, which also turns `-0.0` into `0.0`).
    pub transform: Affine2D,
    /// Break the path at non-finite vertices.
    pub remove_nans: bool,
    /// Clip lines to this rectangle (grown by one unit); ignored unless
    /// `x0 < x1` and `y0 < y1`.
    pub clip: Option<Bbox>,
    /// Pixel snapping.
    pub snap: SnapMode,
    /// The stroke width deciding the snapping offset.
    pub stroke_width: f64,
    /// Simplify with this threshold (`None`: no simplification).
    pub simplify: Option<f64>,
    /// Keep Bézier curves (otherwise they are flattened).
    pub curves: bool,
    /// Apply the sketch filter (flattens curves).
    pub sketch: Option<SketchParams>,
}

impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            transform: Affine2D::IDENTITY,
            remove_nans: false,
            clip: None,
            snap: SnapMode::Never,
            stroke_width: 1.0,
            simplify: None,
            curves: false,
            sketch: None,
        }
    }
}

impl CleanupOptions {
    /// The defaults of matplotlib's `Path.iter_segments` for `path`: NaNs
    /// removed, curves kept, no snapping, and simplification as decided by
    /// `should_simplify` under `settings`.
    pub fn segment_defaults(path: &Path, settings: &SimplifySettings) -> Self {
        Self {
            remove_nans: true,
            curves: true,
            simplify: settings.for_path(path),
            ..Self::default()
        }
    }
}

/// The full `cleanup_path` pipeline; see [`Path::conditioned`].
pub type Conditioned<'a> = Sketcher<
    CurveFlattener<Simplifier<Snapper<Clipper<NanRemover<Transformed<PathVertices<'a>>>>>>>,
>;

impl Path {
    /// The vertex stream of matplotlib's `cleanup_path`: transform, NaN
    /// removal, clipping, snapping, simplification, then curve flattening
    /// and sketching when requested.
    pub fn conditioned(&self, options: &CleanupOptions) -> Conditioned<'_> {
        let clip = options.clip.filter(|r| r.x0() < r.x1() && r.y0() < r.y1());
        let sketch = options.sketch.filter(|s| s.scale != 0.0);
        self.iter_vertices()
            .transform(options.transform)
            .remove_nans(options.remove_nans, self.codes.is_some())
            .clip_to(clip)
            .snap(options.snap, self.len(), options.stroke_width)
            .simplify(options.simplify)
            .flatten_curves(!options.curves || sketch.is_some())
            .sketch(sketch)
    }

    /// matplotlib's `Path.cleaned`. Unlike matplotlib, the result has no
    /// trailing `STOP` vertex (plt-rs paths have no such code).
    #[must_use]
    pub fn cleaned(&self, options: &CleanupOptions) -> Self {
        Self {
            interpolation_steps: self.interpolation_steps,
            ..self.conditioned(options).collect_path()
        }
    }

    /// matplotlib's `Path.iter_segments`: the conditioned path as typed
    /// segments (see [`CleanupOptions::segment_defaults`] for matplotlib's
    /// defaults).
    pub fn iter_segments(&self, options: &CleanupOptions) -> VertexSegments<Conditioned<'_>> {
        self.conditioned(options).segments()
    }
}

/// Groups a vertex stream into [`Segment`]s; see [`VertexSource::segments`].
///
/// Like matplotlib's `iter_segments`, a command takes as many vertices as
/// its first code needs, whatever their own codes; a stream ending inside a
/// curve ends the iteration.
#[derive(Clone, Debug)]
pub struct VertexSegments<I> {
    source: I,
}

impl<I> VertexSegments<I> {
    pub(super) const fn new(source: I) -> Self {
        Self { source }
    }
}

impl<I: Iterator<Item = Vertex>> Iterator for VertexSegments<I> {
    type Item = Segment;

    fn next(&mut self) -> Option<Segment> {
        let Vertex { code, point } = self.source.next()?;
        Some(match code {
            PathCode::MoveTo => Segment::MoveTo(point),
            PathCode::LineTo => Segment::LineTo(point),
            PathCode::Curve3 => Segment::QuadTo(point, self.source.next()?.point),
            PathCode::Curve4 => {
                let control = self.source.next()?.point;
                Segment::CubicTo(point, control, self.source.next()?.point)
            }
            PathCode::ClosePoly => Segment::Close,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn should_simplify_rules() {
        let settings = SimplifySettings::default();
        let line = |n: i32| Path::polyline((0..n).map(|i| p(f64::from(i), 0.0)).collect());
        assert!(!settings.applies_to(&line(127)));
        assert!(settings.applies_to(&line(128)));
        let off = SimplifySettings {
            enabled: false,
            ..settings
        };
        assert!(!off.applies_to(&line(200)));
        let zero = SimplifySettings {
            threshold: 0.0,
            ..settings
        };
        assert_eq!(zero.for_path(&line(200)), None);
        let closed = Path::polygon((0..200).map(|i| p(f64::from(i), f64::from(i % 2))));
        assert!(!settings.applies_to(&closed));
    }

    #[test]
    fn cleaned_square_keeps_its_closure() {
        let square = Path::polygon([p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]);
        let cleaned = square.cleaned(&CleanupOptions::default());
        assert_eq!(cleaned.vertices(), square.vertices());
        assert_eq!(cleaned.codes(), square.codes());
    }

    #[test]
    fn iter_segments_flattens_on_request() {
        let circle = Path::unit_circle();
        let options = CleanupOptions {
            transform: Affine2D::scaling(100.0, 100.0),
            ..CleanupOptions::segment_defaults(&circle, &SimplifySettings::default())
        };
        let curved: Vec<Segment> = circle.iter_segments(&options).collect();
        assert_eq!(curved.len(), 10);
        assert!(matches!(curved[1], Segment::CubicTo(..)));
        let flat = CleanupOptions {
            curves: false,
            ..options
        };
        assert!(
            circle
                .iter_segments(&flat)
                .all(|s| !matches!(s, Segment::QuadTo(..) | Segment::CubicTo(..)))
        );
    }

    #[test]
    fn inverted_clip_rectangles_disable_clipping() {
        let line = Path::polyline(vec![p(-10.0, 5.0), p(20.0, 5.0)]);
        let options = CleanupOptions {
            clip: Some(Bbox::from_extents(10.0, 10.0, 0.0, 0.0)),
            ..CleanupOptions::default()
        };
        assert_eq!(line.cleaned(&options).vertices(), line.vertices());
    }
}
