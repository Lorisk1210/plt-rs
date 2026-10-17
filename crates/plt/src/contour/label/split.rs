//! Cutting a contour line around a label and finding the label's rotation
//! (matplotlib's `_split_path_and_get_label_rotation`).

use plt_core::{Path, PathCode, Point};

use crate::numeric::interp;
use crate::ticker::pymath::py_mod;
use crate::transforms::ResolvedTransform;

/// The display geometry shared by all labels of a `clabel` call.
#[derive(Clone, Debug)]
pub(super) struct LabelGeometry {
    /// Data to display coordinates.
    pub(super) to_display: ResolvedTransform,
    /// Display to data coordinates.
    pub(super) to_data: ResolvedTransform,
    /// Keep labels upright (rotations within ±90°).
    pub(super) rightside_up: bool,
}

/// Where a label of `width` pixels goes on `path` after vertex `index`,
/// at display position `screen`: its rotation in degrees and the path cut
/// around it, leaving `spacing` pixels on each side. A closed loop is
/// opened at the label. The angle comes from the points half a label width
/// before and after the label, measured along the line.
pub(super) fn split_path(
    geometry: &LabelGeometry,
    path: &Path,
    index: usize,
    screen: Point,
    (width, spacing): (f64, f64),
) -> (f64, Path) {
    let mut xys = path.vertices().to_vec();
    let mut codes = path.resolved_codes();
    let mut idx = index;
    // Insert the label point unless a vertex is already there.
    let pos = geometry.to_data.transform_point(screen);
    if !allclose(pos, xys[idx]) {
        xys.insert(idx, pos);
        codes.insert(idx, PathCode::LineTo);
    }
    // The connected component holding the label.
    let start = (0..=idx)
        .rev()
        .find(|&i| codes[i] == PathCode::MoveTo)
        .unwrap_or(0);
    let stop = (idx + 1..codes.len())
        .find(|&i| codes[i] == PathCode::MoveTo)
        .unwrap_or(codes.len());
    let mut component = xys[start..stop].to_vec();
    idx -= start;
    let closed = codes[stop - 1] == PathCode::ClosePoly;
    if closed {
        // Rotate the loop to start at the label.
        let n = component.len();
        let mut rotated = component[idx.min(n - 1)..n - 1].to_vec();
        rotated.extend_from_slice(&component[..=idx]);
        component = rotated;
        idx = 0;
    }

    // Cumulative path lengths in pixels as the curvilinear coordinate.
    let screen_xys: Vec<Point> = component
        .iter()
        .map(|&p| geometry.to_display.transform_point(p))
        .collect();
    let mut cpls = Vec::with_capacity(screen_xys.len());
    let mut total = 0.0;
    cpls.push(0.0);
    for w in screen_xys.windows(2) {
        total += (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
        cpls.push(total);
    }
    let origin = cpls[idx];
    for c in &mut cpls {
        *c -= origin;
    }

    let mut targets = [-width / 2.0, width / 2.0];
    if closed {
        // For closed paths, target from the other end.
        targets[0] += cpls[cpls.len() - 1] - cpls[0];
    }
    let xs = |points: &[Point]| points.iter().map(|p| p.x).collect::<Vec<_>>();
    let ys = |points: &[Point]| points.iter().map(|p| p.y).collect::<Vec<_>>();
    let (sx, sy) = (xs(&screen_xys), ys(&screen_xys));
    let ends = |values: &[f64]| targets.map(|t| interp(t, &cpls, values, None));
    let ([sx0, sx1], [sy0, sy1]) = (ends(&sx), ends(&sy));
    let mut angle = (sy1 - sy0).atan2(sx1 - sx0).to_degrees();
    if geometry.rightside_up {
        // Never upside down.
        angle = py_mod(angle + 90.0, 180.0) - 90.0;
    }

    // Expand by the spacing and cut there; -1 marks out of range.
    targets[0] -= spacing;
    targets[1] += spacing;
    #[expect(
        clippy::cast_precision_loss,
        reason = "vertex counts are far below 2^52"
    )]
    let positions: Vec<f64> = (0..cpls.len()).map(|i| i as f64).collect();
    let bounds = Some((-1.0, -1.0));
    let i0 = interp(targets[0], &cpls, &positions, bounds).floor();
    let i1 = interp(targets[1], &cpls, &positions, bounds).ceil();
    let (cx, cy) = (xs(&component), ys(&component));
    let ([x0, x1], [y0, y1]) = (
        targets.map(|t| interp(t, &cpls, &cx, None)),
        targets.map(|t| interp(t, &cpls, &cy, None)),
    );
    let cut = [Point::new(x0, y0), Point::new(x1, y1)];
    let pieces = cut_component(&component, closed, (index_of(i0), index_of(i1)), cut);

    let mut out_xys = xys[..start].to_vec();
    let mut out_codes = codes[..start].to_vec();
    for piece in &pieces {
        out_xys.extend_from_slice(piece);
        out_codes.push(PathCode::MoveTo);
        out_codes.extend(std::iter::repeat_n(PathCode::LineTo, piece.len() - 1));
    }
    out_xys.extend_from_slice(&xys[stop..]);
    out_codes.extend_from_slice(&codes[stop..]);
    let path = Path::new(out_xys, out_codes).expect("every piece starts with a MOVETO");
    (angle, path)
}

/// A vertex index from numpy's interpolation, `None` for the -1 marker.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "floored indices of short paths"
)]
fn index_of(value: f64) -> Option<usize> {
    (value >= 0.0).then_some(value as usize)
}

/// The parts of the component left outside the label, from `cut[1]`
/// round to `cut[0]` for loops, before `cut[0]` and after `cut[1]` for
/// open lines (parts beyond the line's ends are dropped).
fn cut_component(
    component: &[Point],
    closed: bool,
    (i0, i1): (Option<usize>, Option<usize>),
    cut: [Point; 2],
) -> Vec<Vec<Point>> {
    let mut pieces = Vec::new();
    if closed {
        if let (Some(i0), Some(i1)) = (i0, i1) {
            let mut piece = vec![cut[1]];
            if i1 <= i0 {
                piece.extend_from_slice(
                    &component[i1.min(component.len())..(i0 + 1).min(component.len())],
                );
            }
            piece.push(cut[0]);
            pieces.push(piece);
        }
        return pieces;
    }
    if let Some(i0) = i0 {
        let mut piece = component[..=i0].to_vec();
        piece.push(cut[0]);
        pieces.push(piece);
    }
    if let Some(i1) = i1 {
        let mut piece = vec![cut[1]];
        piece.extend_from_slice(&component[i1..]);
        pieces.push(piece);
    }
    pieces
}

/// numpy's `allclose(a, b)` for two points.
fn allclose(a: Point, b: Point) -> bool {
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-8 + 1e-5 * b.abs();
    close(a.x, b.x) && close(a.y, b.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> LabelGeometry {
        LabelGeometry {
            to_display: ResolvedTransform::identity(),
            to_data: ResolvedTransform::identity(),
            rightside_up: true,
        }
    }

    #[test]
    fn open_line_is_cut_around_the_label() {
        let points: Vec<Point> = (0..11).map(|i| Point::new(f64::from(i), 0.0)).collect();
        let path = Path::polyline(points);
        let (angle, cut) = split_path(&identity(), &path, 5, Point::new(5.0, 0.0), (4.0, 1.0));
        assert_eq!(angle, 0.0);
        let parts = cut.subpaths();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].vertices().last(), Some(&Point::new(2.0, 0.0)));
        assert_eq!(parts[1].vertices()[0], Point::new(8.0, 0.0));
    }

    #[test]
    fn loops_open_at_the_label_and_angles_stay_upright() {
        let square = Path::closed(vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(-10.0, 10.0),
            Point::new(-10.0, 0.0),
            Point::new(0.0, 0.0),
        ]);
        let (angle, cut) = split_path(&identity(), &square, 1, Point::new(0.0, 10.0), (4.0, 0.0));
        assert!((angle - 45.0).abs() < 1e-12 || (angle + 45.0).abs() < 1e-12);
        let parts = cut.subpaths();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].vertices()[0], Point::new(-2.0, 10.0));
        assert_eq!(parts[0].vertices().last(), Some(&Point::new(0.0, 8.0)));
    }
}
