//! Where labels go along contour lines, in display pixels (matplotlib's
//! `print_label`, `locate_label`, `too_close` and
//! `_find_closest_point_on_path`).

use std::cmp::Ordering;

use plt_core::Point;

use crate::stats::pairwise_sum;

/// Whether a line of `line.len()` vertices is long enough to hold a label
/// `width` pixels wide: many vertices, or a wide enough extent.
#[expect(
    clippy::cast_precision_loss,
    reason = "vertex counts are far below 2^52"
)]
pub(super) fn long_enough(line: &[Point], width: f64) -> bool {
    if line.len() as f64 > 10.0 * width {
        return true;
    }
    let ptp = |coord: fn(&Point) -> f64| {
        let values = line.iter().map(coord);
        let max = values.clone().fold(f64::NEG_INFINITY, np_max);
        let min = values.fold(f64::INFINITY, np_min);
        max - min
    };
    !line.is_empty() && (ptp(|p| p.x) > 1.2 * width || ptp(|p| p.y) > 1.2 * width)
}

/// Whether `p` lies within `1.2 * width` of an existing label.
pub(super) fn too_close(p: Point, width: f64, labels: &[Point]) -> bool {
    let thresh = (1.2 * width).powi(2);
    labels
        .iter()
        .any(|l| (p.x - l.x).powi(2) + (p.y - l.y).powi(2) < thresh)
}

/// The straightest stretch of `line` for a label `width` pixels wide that
/// is not too close to an existing label: the line is cut into blocks of
/// about the label's width (the last one wrapping around to the start),
/// each scored by the summed distance of its points from its chord. If
/// every block is too close, the straightest one is used anyway. Returns
/// the block's middle vertex and its index.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "numpy's ceil/int of non-negative pixel widths and vertex counts"
)]
pub(super) fn locate(line: &[Point], width: f64, labels: &[Point]) -> (Point, usize) {
    let size = line.len();
    let n_blocks = if width > 1.0 {
        (size as f64 / width).ceil() as usize
    } else {
        1
    };
    let block_size = if n_blocks == 1 { size } else { width as usize };
    let at = |block: usize, k: usize| line[(block * block_size + k) % size];
    let distances: Vec<f64> = (0..n_blocks)
        .map(|block| {
            let (first, last) = (at(block, 0), at(block, block_size - 1));
            let chord = (last.x - first.x).hypot(last.y - first.y);
            let terms: Vec<f64> = (0..block_size)
                .map(|k| {
                    let p = at(block, k);
                    let s =
                        (first.y - p.y) * (last.x - first.x) - (first.x - p.x) * (last.y - first.y);
                    s.abs() / chord
                })
                .collect();
            pairwise_sum(&terms)
        })
        .collect();
    let half = block_size / 2;
    let mut order: Vec<usize> = (0..n_blocks).collect();
    // numpy's argsort: NaN last (small inputs sort stably).
    order.sort_by(|&a, &b| nan_last(distances[a], distances[b]));
    let candidates = order.iter().chain(order.first()).copied();
    let mut chosen = (at(order[0], half), order[0]);
    for block in candidates {
        chosen = (at(block, half), block);
        if !too_close(chosen.0, width, labels) {
            break;
        }
    }
    (chosen.0, (chosen.1 * block_size + half) % size)
}

fn nan_last(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (false, false) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
    }
}

/// The point of the polyline `line` closest to `p`: its squared distance,
/// the point, and the vertex index ending the segment it lies on (0 for a
/// single vertex).
pub(super) fn closest_point(line: &[Point], p: Point) -> Option<(f64, Point, usize)> {
    match line {
        [] => None,
        [only] => Some(((p.x - only.x).powi(2) + (p.y - only.y).powi(2), *only, 0)),
        _ => {
            let candidates = line.windows(2).map(|w| {
                let (a, b) = (w[0], w[1]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let mut norm = dx.powi(2) + dy.powi(2);
                if norm == 0.0 {
                    norm = 1.0;
                }
                let rel = np_clip(((p.x - a.x) * dx + (p.y - a.y) * dy) / norm);
                let proj = Point::new(a.x + rel * dx, a.y + rel * dy);
                ((proj.x - p.x).powi(2) + (proj.y - p.y).powi(2), proj)
            });
            // numpy's argmin: the first minimum, or the first NaN.
            let mut best: Option<(usize, f64, Point)> = None;
            for (i, (d2, proj)) in candidates.enumerate() {
                let better = match best {
                    None => true,
                    Some((_, current, _)) => !current.is_nan() && (d2.is_nan() || d2 < current),
                };
                if better {
                    best = Some((i, d2, proj));
                }
            }
            best.map(|(i, d2, proj)| (d2, proj, i + 1))
        }
    }
}

/// `np.clip(v, 0, 1)`.
fn np_clip(v: f64) -> f64 {
    if v.is_nan() { v } else { v.clamp(0.0, 1.0) }
}

/// numpy's `max` reduction step: NaN wins.
fn np_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// numpy's `min` reduction step: NaN wins.
fn np_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(coords: &[(f64, f64)]) -> Vec<Point> {
        coords.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    #[test]
    fn long_enough_by_extent_or_vertex_count() {
        let short = line(&[(0.0, 0.0), (5.0, 0.0)]);
        assert!(!long_enough(&short, 10.0));
        assert!(long_enough(&short, 4.0));
        assert!(!long_enough(&[], 1.0));
        assert!(long_enough(&vec![Point::ORIGIN; 30], 2.0));
    }

    #[test]
    fn locate_prefers_straight_blocks() {
        // A straight run followed by a zig-zag.
        let mut points = line(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
        points.extend(line(&[(4.0, 3.0), (5.0, -3.0), (6.0, 3.0), (7.0, -3.0)]));
        let (p, index) = locate(&points, 4.0, &[]);
        assert_eq!((p, index), (Point::new(2.0, 0.0), 2));
        let (_, index) = locate(&points, 4.0, &[Point::new(2.0, 0.0)]);
        assert_eq!(index, 6, "the straight block is too close to a label");
    }

    #[test]
    fn closest_point_projects_onto_segments() {
        let path = line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        let (d2, proj, index) = closest_point(&path, Point::new(4.0, 2.0)).unwrap();
        assert_eq!((d2, proj, index), (4.0, Point::new(4.0, 0.0), 1));
        let (_, proj, index) = closest_point(&path, Point::new(12.0, 7.0)).unwrap();
        assert_eq!((proj, index), (Point::new(10.0, 7.0), 2));
        assert_eq!(closest_point(&[], Point::ORIGIN), None);
        assert_eq!(
            closest_point(&path[..1], Point::new(3.0, 4.0)),
            Some((25.0, Point::ORIGIN, 0))
        );
        assert!(too_close(Point::new(1.0, 0.0), 1.0, &[Point::ORIGIN]));
    }
}
