//! Where labels go along contour lines, in display pixels (matplotlib's
//! `print_label`, `locate_label` and `too_close`).

use pltrs_core::Point;
use pltrs_core::numeric::{np_maximum, np_minimum};

use crate::numeric::argsort;
use crate::stats::pairwise_sum;

/// Whether a line of `line.len()` vertices (display pixels) is long
/// enough to hold a label `width` pixels wide: many vertices, or a wide
/// enough extent (matplotlib's `ContourLabeler.print_label`).
///
/// ```
/// use pltrs::contour::print_label;
/// use pltrs::transforms::Point;
/// let line = [Point::new(0.0, 0.0), Point::new(50.0, 0.0)];
/// assert!(print_label(&line, 20.0));
/// assert!(!print_label(&line, 100.0));
/// ```
pub fn print_label(line: &[Point], width: f64) -> bool {
    if line.len() as f64 > 10.0 * width {
        return true;
    }
    let ptp = |coord: fn(&Point) -> f64| {
        let values = line.iter().map(coord);
        let max = values.clone().fold(f64::NEG_INFINITY, np_maximum);
        let min = values.fold(f64::INFINITY, np_minimum);
        max - min
    };
    !line.is_empty() && (ptp(|p| p.x) > 1.2 * width || ptp(|p| p.y) > 1.2 * width)
}

/// Whether `p` lies within `1.2 * width` of one of the `labels`
/// (display pixels; matplotlib's `ContourLabeler.too_close`).
///
/// ```
/// use pltrs::contour::too_close;
/// use pltrs::transforms::Point;
/// assert!(too_close(Point::new(1.0, 0.0), 1.0, &[Point::ORIGIN]));
/// assert!(!too_close(Point::new(5.0, 0.0), 1.0, &[Point::ORIGIN]));
/// ```
pub fn too_close(p: Point, width: f64, labels: &[Point]) -> bool {
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
/// the block's middle vertex and its index (matplotlib's
/// `ContourLabeler.locate_label`); `None` for a line without vertices or
/// a width that cuts it into no block (an infinite one, which matplotlib
/// refuses to count).
///
/// ```
/// use pltrs::contour::locate_label;
/// use pltrs::transforms::Point;
/// let line: Vec<Point> = (0..8).map(|i| Point::new(f64::from(i), 0.0)).collect();
/// assert_eq!(locate_label(&line, 4.0, &[]), Some((Point::new(2.0, 0.0), 2)));
/// assert_eq!(locate_label(&line, f64::INFINITY, &[]), None);
/// ```
pub fn locate_label(line: &[Point], width: f64, labels: &[Point]) -> Option<(Point, usize)> {
    let size = line.len();
    let n_blocks = if width > 1.0 {
        (size as f64 / width).ceil() as usize
    } else {
        1
    };
    if size == 0 || n_blocks == 0 {
        return None;
    }
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
    let order = argsort(&distances);
    let candidates = order.iter().chain(order.first()).copied();
    let mut chosen = (at(order[0], half), order[0]);
    for block in candidates {
        chosen = (at(block, half), block);
        if !too_close(chosen.0, width, labels) {
            break;
        }
    }
    Some((chosen.0, (chosen.1 * block_size + half) % size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::points;

    #[test]
    fn long_enough_by_extent_or_vertex_count() {
        let short = points(&[(0.0, 0.0), (5.0, 0.0)]);
        assert!(!print_label(&short, 10.0));
        assert!(print_label(&short, 4.0));
        assert!(!print_label(&[], 1.0));
        assert!(print_label(&vec![Point::ORIGIN; 30], 2.0));
    }

    #[test]
    fn locate_prefers_straight_blocks() {
        // A straight run followed by a zig-zag.
        let mut path = points(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]);
        path.extend(points(&[(4.0, 3.0), (5.0, -3.0), (6.0, 3.0), (7.0, -3.0)]));
        let (p, index) = locate_label(&path, 4.0, &[]).unwrap();
        assert_eq!((p, index), (Point::new(2.0, 0.0), 2));
        let (_, index) = locate_label(&path, 4.0, &[Point::new(2.0, 0.0)]).unwrap();
        assert_eq!(index, 6, "the straight block is too close to a label");
        assert_eq!(
            locate_label(&[], 4.0, &[]),
            None,
            "no vertex to place it at"
        );
        assert_eq!(locate_label(&path, f64::INFINITY, &[]), None, "no block");
    }

    #[test]
    fn too_close_within_a_label_width() {
        assert!(too_close(Point::new(1.0, 0.0), 1.0, &[Point::ORIGIN]));
        assert!(!too_close(Point::new(5.0, 0.0), 1.0, &[Point::ORIGIN]));
        assert!(!too_close(Point::ORIGIN, 1.0, &[]));
    }
}
