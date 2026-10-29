//! The hand-drawn "sketch" filter (matplotlib's `Sketch`, on top of Agg's
//! `conv_segmentator`).

use std::f64::consts::PI;

use super::{Vertex, VertexSource};
use crate::Point;
use crate::path::PathCode;

/// The total length (in units, hence pieces) a [`Segmenter`] cuts per pass:
/// a thousand canvas-high strokes, yet a bound for hostile coordinates
/// (Agg would cut a `1e300`-long line into `1e300` pieces) and for paths
/// drawn again by every path effect into vector documents. Lines beyond
/// the budget are passed through whole.
const PIECE_BUDGET: f64 = 1_048_576.0;

/// Parameters of matplotlib's sketch filter (`set_sketch_params`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchParams {
    /// Amplitude of the wiggle perpendicular to the line, in pixels.
    pub scale: f64,
    /// Base wavelength of the wiggle along the line, in pixels.
    pub length: f64,
    /// Factor by which the wavelength randomly shrinks and expands.
    pub randomness: f64,
}

impl SketchParams {
    /// matplotlib's default wavelength.
    pub const DEFAULT_LENGTH: f64 = 128.0;
    /// matplotlib's default randomness.
    pub const DEFAULT_RANDOMNESS: f64 = 16.0;

    /// Parameters with matplotlib's defaults for unset (or zero) length and
    /// randomness, as `GraphicsContextBase.set_sketch_params` applies them.
    pub fn new(scale: f64, length: Option<f64>, randomness: Option<f64>) -> Self {
        let or_default = |v: Option<f64>, default| v.filter(|v| *v != 0.0).unwrap_or(default);
        Self {
            scale,
            length: or_default(length, Self::DEFAULT_LENGTH),
            randomness: or_default(randomness, Self::DEFAULT_RANDOMNESS),
        }
    }
}

/// matplotlib's own linear congruential generator (MSVC's constants), so
/// sketched output is reproducible bit for bit.
#[derive(Clone, Debug)]
struct Lcg(u32);

impl Lcg {
    fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(214_013).wrapping_add(2_531_011);
        f64::from(self.0) / 4_294_967_296.0
    }
}

/// Agg's `conv_adaptor_vpgen<_, vpgen_segmentator>` at approximation scale
/// 1: splits every line into pieces of at most one unit, within
/// [`PIECE_BUDGET`].
#[derive(Clone, Debug)]
struct Segmenter<I> {
    source: I,
    /// The command of the next generated vertex, or `None` when idle.
    cmd: Option<PathCode>,
    origin: Point,
    delta: Point,
    dl: f64,
    ddl: f64,
    start: Point,
    pending_close: bool,
    vertices: usize,
    /// The length that may still be cut into pieces in this pass.
    budget: f64,
}

impl<I: VertexSource> Segmenter<I> {
    const fn new(source: I) -> Self {
        Self {
            source,
            cmd: None,
            origin: Point::ORIGIN,
            delta: Point::ORIGIN,
            dl: 0.0,
            ddl: 0.0,
            start: Point::ORIGIN,
            pending_close: false,
            vertices: 0,
            budget: PIECE_BUDGET,
        }
    }

    fn move_to(&mut self, p: Point) {
        self.origin = p;
        self.delta = Point::ORIGIN;
        self.dl = 2.0;
        self.ddl = 2.0;
        self.cmd = Some(PathCode::MoveTo);
    }

    fn line_to(&mut self, p: Point) {
        self.origin += self.delta;
        self.delta = p - self.origin;
        let len = (self.delta.x * self.delta.x + self.delta.y * self.delta.y).sqrt();
        let len = if len < 1e-30 { 1e-30 } else { len };
        // Non-finite lengths would never terminate in Agg, and lengths beyond
        // the budget would take too long: emit the end point.
        let ddl = 1.0 / len;
        self.ddl = if ddl.is_finite() && ddl > 0.0 && len <= self.budget {
            self.budget -= len;
            ddl
        } else {
            1.0
        };
        self.dl = if self.cmd == Some(PathCode::MoveTo) {
            0.0
        } else {
            self.ddl
        };
        if self.cmd.is_none() {
            self.cmd = Some(PathCode::LineTo);
        }
    }

    /// `vpgen_segmentator::vertex`.
    fn generate(&mut self) -> Option<Vertex> {
        let cmd = self.cmd?;
        self.cmd = Some(PathCode::LineTo);
        if self.dl >= 1.0 - self.ddl {
            self.dl = 1.0;
            self.cmd = None;
            return Some(Vertex::new(cmd, self.origin + self.delta));
        }
        let p = Point::new(
            self.origin.x + self.delta.x * self.dl,
            self.origin.y + self.delta.y * self.dl,
        );
        self.dl += self.ddl;
        Some(Vertex::new(cmd, p))
    }
}

impl<I: VertexSource> Iterator for Segmenter<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        loop {
            if let Some(vertex) = self.generate() {
                return Some(vertex);
            }
            if self.pending_close {
                // Agg reports the closure with zeroed coordinates.
                self.pending_close = false;
                return Some(Vertex::new(PathCode::ClosePoly, Point::ORIGIN));
            }
            let vertex = self.source.next()?;
            match vertex.code {
                PathCode::MoveTo => {
                    self.move_to(vertex.point);
                    self.start = vertex.point;
                    self.vertices = 1;
                }
                PathCode::ClosePoly => {
                    self.pending_close = true;
                    if self.vertices > 2 {
                        self.line_to(self.start);
                    }
                    self.vertices = 0;
                }
                PathCode::LineTo | PathCode::Curve3 | PathCode::Curve4 => {
                    self.line_to(vertex.point);
                    self.vertices += 1;
                }
            }
        }
    }
}

impl<I: VertexSource> VertexSource for Segmenter<I> {
    fn rewind(&mut self) {
        self.source.rewind();
        self.cmd = None;
        self.start = Point::ORIGIN;
        self.pending_close = false;
        self.vertices = 0;
        self.budget = PIECE_BUDGET;
    }
}

/// The parameters derived once per sketcher.
#[derive(Clone, Copy, Debug)]
struct Wiggle {
    scale: f64,
    p_scale: f64,
    log_randomness: f64,
}

/// Displaces unit-length pieces of every line along a randomly paced sine
/// wave; see [`VertexSource::sketch`].
#[derive(Clone, Debug)]
pub struct Sketcher<I> {
    segments: Segmenter<I>,
    wiggle: Option<Wiggle>,
    last: Point,
    has_last: bool,
    phase: f64,
    rng: Lcg,
}

impl<I: VertexSource> Sketcher<I> {
    pub(super) fn new(source: I, params: Option<SketchParams>) -> Self {
        let wiggle = params.filter(|p| p.scale != 0.0).map(|p| Wiggle {
            scale: p.scale,
            p_scale: (2.0 * PI) / (p.length * p.randomness),
            log_randomness: 2.0 * p.randomness.ln(),
        });
        let mut sketcher = Self {
            segments: Segmenter::new(source),
            wiggle,
            last: Point::ORIGIN,
            has_last: false,
            phase: 0.0,
            rng: Lcg(0),
        };
        sketcher.rewind();
        sketcher
    }
}

impl<I: VertexSource> Iterator for Sketcher<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        let Some(wiggle) = self.wiggle else {
            return self.segments.source.next();
        };
        let mut vertex = self.segments.next()?;
        if vertex.code == PathCode::MoveTo {
            self.has_last = false;
            self.phase = 0.0;
        }
        if self.has_last {
            // The cursor along the sine wave advances at a random rate.
            self.phase += (self.rng.next_f64() * wiggle.log_randomness).exp();
            let den = self.last.x - vertex.point.x;
            let num = self.last.y - vertex.point.y;
            let len_sq = num * num + den * den;
            self.last = vertex.point;
            if len_sq != 0.0 {
                let r = (self.phase * wiggle.p_scale).sin() * wiggle.scale;
                let r_over_len = r / len_sq.sqrt();
                vertex.point.x += r_over_len * num;
                vertex.point.y -= r_over_len * den;
            }
        } else {
            self.last = vertex.point;
        }
        self.has_last = true;
        Some(vertex)
    }
}

impl<I: VertexSource> VertexSource for Sketcher<I> {
    fn rewind(&mut self) {
        self.has_last = false;
        self.phase = 0.0;
        if self.wiggle.is_some() {
            self.rng = Lcg(0);
            self.segments.rewind();
        } else {
            self.segments.source.rewind();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Path;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn lcg_matches_matplotlib() {
        let mut rng = Lcg(0);
        assert_eq!(rng.next_f64(), 2_531_011.0 / 4_294_967_296.0);
        assert!((rng.next_f64() - 0.117_791_085_038_334_13).abs() < 1e-15);
    }

    #[test]
    fn lines_are_split_into_unit_pieces() {
        let path = Path::polyline(vec![p(0.0, 0.0), p(10.0, 0.0)]);
        let flat = SketchParams::new(1e-300, None, None);
        let out: Vec<Vertex> = path.iter_vertices().sketch(Some(flat)).collect();
        assert!((10..=12).contains(&out.len()));
        assert_eq!(out[0].code, PathCode::MoveTo);
        assert_eq!(out.last().map(|v| v.point.x), Some(10.0));
    }

    #[test]
    fn wiggle_is_perpendicular_and_bounded() {
        let path = Path::polyline(vec![p(0.0, 0.0), p(100.0, 0.0)]);
        let params = SketchParams::new(2.0, None, None);
        let out: Vec<Vertex> = path.iter_vertices().sketch(Some(params)).collect();
        assert!((100..=102).contains(&out.len()));
        assert_eq!(out.last().map(|v| v.point.x), Some(100.0));
        // Agg may emit the end point one step early (accumulated steps).
        for (i, v) in out[..out.len() - 1].iter().enumerate() {
            assert!((v.point.x - i as f64).abs() < 1e-9);
            assert!(v.point.y.abs() <= 2.0);
        }
        assert!(out.iter().any(|v| v.point.y.abs() > 0.1));
    }

    #[test]
    fn zero_scale_passes_through() {
        let path = Path::polyline(vec![p(0.0, 0.0), p(10.0, 0.0)]);
        let params = SketchParams::new(0.0, None, None);
        assert_eq!(path.iter_vertices().sketch(Some(params)).count(), 2);
    }

    #[test]
    fn non_finite_lines_terminate() {
        let path = Path::polyline(vec![p(0.0, 0.0), p(f64::INFINITY, 0.0), p(f64::NAN, 1.0)]);
        let params = SketchParams::new(1.0, None, None);
        assert_eq!(path.iter_vertices().sketch(Some(params)).count(), 3);
    }

    #[test]
    fn huge_lines_are_not_cut_beyond_the_budget() {
        // Agg would cut the first line into 1e300 pieces.
        let path = Path::polyline(vec![p(0.0, 0.0), p(1e300, 0.0), p(1e300, 10.0)]);
        let params = SketchParams::new(1.0, None, None);
        assert_eq!(path.iter_vertices().sketch(Some(params)).count(), 12);
    }

    #[test]
    fn long_paths_are_cut_within_the_budget() {
        // Found by fuzzing: a path crossing the canvas back and forth,
        // sketched for a path effect or two into SVG, took minutes.
        let zigzag = (0..100).map(|i| p(f64::from(i % 2) * 50_000.0, f64::from(i)));
        let path = Path::polyline(zigzag.collect());
        let params = SketchParams::new(1.0, None, None);
        let pieces = path.iter_vertices().sketch(Some(params)).count();
        assert!(pieces <= PIECE_BUDGET as usize + 100, "{pieces} pieces");
        assert!(pieces > PIECE_BUDGET as usize / 2, "{pieces} pieces");
    }
}
