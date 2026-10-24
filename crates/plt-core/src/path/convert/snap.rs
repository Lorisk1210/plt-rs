//! Pixel snapping (matplotlib's `PathSnapper`).

use super::{Vertex, VertexSource};
use crate::Affine2D;
use crate::path::{Path, PathCode};

/// When to snap vertices to pixel centres (matplotlib's `e_snap_mode`;
/// `gc.get_snap()` of `None`/`True`/`False`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SnapMode {
    /// Snap only paths of at most 1024 vertices made solely of horizontal
    /// and vertical lines (and moves).
    #[default]
    Auto,
    /// Never snap.
    Never,
    /// Always snap.
    Always,
}

impl SnapMode {
    /// matplotlib's tri-state `snap` argument: `None` means [`Self::Auto`].
    pub const fn from_option(snap: Option<bool>) -> Self {
        match snap {
            None => Self::Auto,
            Some(true) => Self::Always,
            Some(false) => Self::Never,
        }
    }
}

/// Rounds vertices to pixel centres; see [`VertexSource::snap`].
///
/// Each coordinate becomes `floor(v + 0.5) + offset`, where the offset is
/// `0.5` when the stroke width rounds (half away from zero) to an odd
/// number of pixels and `0` otherwise, so odd-width lines cover whole
/// pixels. Closures are never moved.
#[derive(Clone, Debug)]
pub struct Snapper<I> {
    source: I,
    offset: Option<f64>,
}

impl<I: VertexSource> Snapper<I> {
    pub(super) fn new(
        mut source: I,
        mode: SnapMode,
        total_vertices: usize,
        stroke_width: f64,
    ) -> Self {
        let snap = should_snap(&mut source, mode, total_vertices);
        source.rewind();
        Self {
            source,
            offset: snap.then(|| snap_offset(stroke_width)),
        }
    }

    /// Whether this snapper rounds vertices.
    pub const fn is_snapping(&self) -> bool {
        self.offset.is_some()
    }
}

/// `0.5` for odd rounded stroke widths, else `0` (`mpl_round_to_int`
/// rounds half away from zero and the cast truncates).
fn snap_offset(stroke_width: f64) -> f64 {
    let half = if stroke_width >= 0.0 { 0.5 } else { -0.5 };
    let rounded = (stroke_width + half) as i64;
    if rounded % 2 == 0 { 0.0 } else { 0.5 }
}

/// `floor(v + 0.5) + offset`: `v` rounded to a pixel edge, plus the
/// offset. The floor goes through a truncating integer conversion, exact
/// below 2^52 (larger magnitudes are whole already, or not finite), and
/// matches `f64::floor` bit for bit: the only floor it could get wrong is
/// that of -0.0, which `v + 0.5` never is. It spares the libm call that
/// `f64::floor` costs without SSE4.1.
fn snap_coordinate(v: f64, offset: f64) -> f64 {
    const WHOLE: f64 = 4_503_599_627_370_496.0;
    let shifted = v + 0.5;
    let floor = if shifted.abs() < WHOLE {
        let truncated = shifted as i64 as f64;
        if truncated > shifted {
            truncated - 1.0
        } else {
            truncated
        }
    } else {
        shifted
    };
    floor + offset
}

/// matplotlib's `should_snap`, scanning the (already clipped) stream.
fn should_snap<I: Iterator<Item = Vertex>>(
    source: &mut I,
    mode: SnapMode,
    total_vertices: usize,
) -> bool {
    match mode {
        SnapMode::Never => false,
        SnapMode::Always => true,
        SnapMode::Auto => {
            if total_vertices > 1024 {
                return false;
            }
            let Some(first) = source.next() else {
                return false;
            };
            let mut previous = first.point;
            for vertex in source {
                let p = vertex.point;
                match vertex.code {
                    PathCode::Curve3 | PathCode::Curve4 => return false,
                    PathCode::LineTo
                        if (previous.x - p.x).abs() >= 1e-4 && (previous.y - p.y).abs() >= 1e-4 =>
                    {
                        return false;
                    }
                    _ => {}
                }
                previous = p;
            }
            true
        }
    }
}

impl<I: VertexSource> Iterator for Snapper<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        let mut vertex = self.source.next()?;
        if let Some(offset) = self.offset
            && vertex.is_vertex()
        {
            vertex.point.x = snap_coordinate(vertex.point.x, offset);
            vertex.point.y = snap_coordinate(vertex.point.y, offset);
        }
        Some(vertex)
    }
}

impl<I: VertexSource> VertexSource for Snapper<I> {
    fn rewind(&mut self) {
        self.source.rewind();
    }
}

impl Path {
    /// Transforms and snaps a path of straight segments in one pass,
    /// storing in `out` what
    /// `self.iter_vertices().transform(transform).remove_nans(true, has_codes)`
    /// `.snap(mode, self.len(), stroke_width).collect_path_into(out)` stores
    /// (for either `has_codes`).
    ///
    /// Returns `false`, leaving `out` unspecified, when the path has curves
    /// or a vertex is not finite once transformed: those need the stages
    /// themselves.
    ///
    /// ```
    /// use plt_core::path::convert::SnapMode;
    /// use plt_core::{Affine2D, Path, Point};
    ///
    /// let line = Path::polyline(vec![Point::new(0.2, 0.2), Point::new(3.6, 0.2)]);
    /// let mut out = Path::default();
    /// assert!(line.transform_and_snap_into(Affine2D::identity(), SnapMode::Auto, 1.0, &mut out));
    /// assert_eq!(out.vertices(), &[Point::new(0.5, 0.5), Point::new(4.5, 0.5)]);
    /// ```
    pub fn transform_and_snap_into(
        &self,
        transform: Affine2D,
        mode: SnapMode,
        stroke_width: f64,
        out: &mut Path,
    ) -> bool {
        let codes = out.codes.get_or_insert_with(Vec::new);
        out.vertices.clear();
        codes.clear();
        for (i, &point) in self.vertices.iter().enumerate() {
            let code = self.code_at(i);
            let point = match code {
                PathCode::Curve3 | PathCode::Curve4 => return false,
                // Closures keep their stored point, as `Transformed` leaves it.
                PathCode::ClosePoly => point,
                PathCode::MoveTo | PathCode::LineTo => transform.transform_point(point),
            };
            if !point.is_finite() {
                return false;
            }
            out.vertices.push(point);
            codes.push(code);
        }
        out.interpolation_steps = 1;
        // With every point finite, the NaN filter passes the stream through
        // unchanged on both of the snapper's passes.
        if should_snap(&mut out.iter_vertices(), mode, self.len()) {
            let offset = snap_offset(stroke_width);
            let codes = out.codes.as_deref().unwrap_or_default();
            for (point, &code) in out.vertices.iter_mut().zip(codes) {
                if code != PathCode::ClosePoly {
                    point.x = snap_coordinate(point.x, offset);
                    point.y = snap_coordinate(point.y, offset);
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;
    use crate::path::Path;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn snapped(path: &Path, mode: SnapMode, width: f64) -> Vec<Point> {
        path.iter_vertices()
            .snap(mode, path.len(), width)
            .map(|v| v.point)
            .collect()
    }

    #[test]
    fn rectilinear_paths_snap_to_pixel_centres() {
        let path = Path::polyline(vec![p(0.3, 0.3), p(10.6, 0.3), p(10.6, 5.2)]);
        assert_eq!(
            snapped(&path, SnapMode::Auto, 1.0),
            vec![p(0.5, 0.5), p(11.5, 0.5), p(11.5, 5.5)]
        );
        assert_eq!(
            snapped(&path, SnapMode::Auto, 2.0),
            vec![p(0.0, 0.0), p(11.0, 0.0), p(11.0, 5.0)]
        );
        // 2.5 rounds half away from zero to 3: odd.
        assert_eq!(snapped(&path, SnapMode::Auto, 2.5)[0], p(0.5, 0.5));
        assert_eq!(snapped(&path, SnapMode::Auto, 0.4)[0], p(0.0, 0.0));
    }

    #[test]
    fn diagonals_and_curves_disable_auto_snapping() {
        let diagonal = Path::polyline(vec![p(0.3, 0.3), p(10.6, 7.7)]);
        assert_eq!(snapped(&diagonal, SnapMode::Auto, 1.0)[0], p(0.3, 0.3));
        assert_eq!(snapped(&diagonal, SnapMode::Always, 1.0)[0], p(0.5, 0.5));
        assert_eq!(snapped(&diagonal, SnapMode::Never, 1.0)[0], p(0.3, 0.3));
        let curve = Path::builder()
            .move_to(p(0.3, 0.3))
            .quad_to(p(1.0, 0.3), p(2.0, 0.3))
            .build();
        let stream = curve.iter_vertices().snap(SnapMode::Auto, 3, 1.0);
        assert!(!stream.is_snapping());
    }

    /// The stages `transform_and_snap_into` stands for.
    fn staged(path: &Path, transform: Affine2D, mode: SnapMode, has_codes: bool) -> Path {
        path.iter_vertices()
            .transform(transform)
            .remove_nans(true, has_codes)
            .snap(mode, path.len(), 1.0)
            .collect_path()
    }

    #[test]
    fn one_pass_snapping_matches_the_stages() {
        let transform = Affine2D::scaling(3.3, -2.1).then(Affine2D::translation(0.4, 50.2));
        let paths = [
            Path::polyline(vec![p(0.3, 0.3), p(10.6, 0.3), p(10.6, 5.2), p(0.3, 5.2)]),
            Path::polyline(vec![p(0.3, 0.3), p(10.6, 7.7)]),
            Path::closed(vec![p(1.0, 1.0), p(4.0, 1.0), p(4.0, 3.0), p(9.0, 9.0)]),
            Path::builder()
                .move_to(p(0.0, 0.0))
                .line_to(p(2.0, 0.0))
                .close()
                .move_to(p(5.0, 5.0))
                .line_to(p(5.0, 8.0))
                .build(),
            Path::default(),
        ];
        for path in &paths {
            for mode in [SnapMode::Auto, SnapMode::Always, SnapMode::Never] {
                let mut out = Path::polyline(vec![p(9.0, 9.0)]);
                assert!(path.transform_and_snap_into(transform, mode, 1.0, &mut out));
                for has_codes in [false, true] {
                    assert_eq!(out, staged(path, transform, mode, has_codes), "{path:?}");
                }
            }
        }
    }

    #[test]
    fn one_pass_snapping_declines_curves_and_non_finite_points() {
        let mut out = Path::default();
        let curve = Path::builder()
            .move_to(p(0.3, 0.3))
            .quad_to(p(1.0, 0.3), p(2.0, 0.3))
            .build();
        let gap = Path::polyline(vec![p(0.0, 0.0), p(f64::NAN, 1.0), p(2.0, 2.0)]);
        let huge = Path::polyline(vec![p(0.0, 0.0), p(f64::MAX, 1.0)]);
        let scale = Affine2D::scaling(10.0, 10.0);
        for path in [curve, gap, huge] {
            assert!(!path.transform_and_snap_into(scale, SnapMode::Auto, 1.0, &mut out));
        }
    }

    #[test]
    fn snapped_coordinates_floor_like_libm() {
        let samples = [
            0.0,
            -0.0,
            0.3,
            -0.3,
            0.5,
            -0.5,
            -0.7,
            1.5,
            -1.5,
            2.499_999,
            -2.5,
            1e15 + 0.25,
            -1e15 - 0.75,
            4.5e15,
            -4.6e15,
            1e300,
            -1e300,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
        ];
        let mut rng = 0x9E37_79B9_7F4A_7C15_u64;
        let random = (0..10_000).map(|_| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            f64::from_bits(rng)
        });
        for v in samples.into_iter().chain(random) {
            for offset in [0.0, 0.5] {
                let expected = (v + 0.5).floor() + offset;
                let got = snap_coordinate(v, offset);
                assert!(
                    got.to_bits() == expected.to_bits() || (got.is_nan() && expected.is_nan()),
                    "{v:e}"
                );
            }
        }
    }

    #[test]
    fn long_paths_are_not_auto_snapped() {
        let path = Path::polyline((0..1025).map(|i| p(f64::from(i) + 0.2, 0.2)).collect());
        assert!(
            !path
                .iter_vertices()
                .snap(SnapMode::Auto, path.len(), 1.0)
                .is_snapping()
        );
    }
}
