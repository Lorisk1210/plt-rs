//! Pixel snapping (matplotlib's `PathSnapper`).

use super::{Vertex, VertexSource};
use crate::path::PathCode;

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
            vertex.point.x = (vertex.point.x + 0.5).floor() + offset;
            vertex.point.y = (vertex.point.y + 0.5).floor() + offset;
        }
        Some(vertex)
    }
}

impl<I: VertexSource> VertexSource for Snapper<I> {
    fn rewind(&mut self) {
        self.source.rewind();
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
