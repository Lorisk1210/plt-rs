//! The canonical geometry of contour paths (`canon_path` in
//! `spec/reference/probes/test_contour.py`).

use std::cmp::Ordering;

use pltrs_core::{Path, PathCode};

use crate::json::Json;

type Vertex = (f64, f64);

/// One level's compound path as a sorted list of subpaths
/// `{"closed": bool, "vertices": [[x, y], ...]}`: split at every MOVETO,
/// closed iff the last code is CLOSEPOLY (whose repeated vertex is dropped
/// and the ring rotated to start at its smallest vertex, direction kept),
/// `-0.0` written as `0.0`.
pub(super) fn canonical(path: &Path) -> Json {
    let vertices: Vec<Vertex> = path
        .vertices()
        .iter()
        .map(|p| (p.x + 0.0, p.y + 0.0))
        .collect();
    let codes = path.resolved_codes();
    let mut starts: Vec<usize> = (0..codes.len())
        .filter(|&i| codes[i] == PathCode::MoveTo)
        .collect();
    starts.push(codes.len());
    let mut subpaths: Vec<(Vec<Vertex>, bool)> = starts
        .windows(2)
        .map(|w| {
            let (s, e) = (w[0], w[1]);
            let closed = codes[e - 1] == PathCode::ClosePoly;
            let ring = if closed {
                rotate_min(&vertices[s..e - 1])
            } else {
                vertices[s..e].to_vec()
            };
            (ring, closed)
        })
        .collect();
    subpaths.sort_by(|a, b| compare_vertices(&a.0, &b.0).then(a.1.cmp(&b.1)));
    Json::Array(
        subpaths
            .into_iter()
            .map(|(ring, closed)| {
                let points = ring
                    .into_iter()
                    .map(|(x, y)| Json::from(vec![x, y]))
                    .collect();
                Json::Object(vec![
                    ("closed".into(), Json::from(closed)),
                    ("vertices".into(), Json::Array(points)),
                ])
            })
            .collect(),
    )
}

/// The rotation of a cyclic vertex list that starts at its smallest
/// vertex, the lexicographically smallest one if that vertex repeats.
fn rotate_min(ring: &[Vertex]) -> Vec<Vertex> {
    let Some(min) = ring.iter().copied().min_by(compare_vertex) else {
        return Vec::new();
    };
    (0..ring.len())
        .filter(|&i| compare_vertex(&ring[i], &min) == Ordering::Equal)
        .map(|i| [&ring[i..], &ring[..i]].concat())
        .min_by(|a, b| compare_vertices(a, b))
        .unwrap_or_default()
}

fn compare_vertex(a: &Vertex, b: &Vertex) -> Ordering {
    a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
}

/// Python's sequence order: element by element, a proper prefix first.
fn compare_vertices(a: &[Vertex], b: &[Vertex]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(p, q)| compare_vertex(p, q))
        .find(|o| o.is_ne())
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_core::Point;

    #[test]
    fn rings_rotate_to_their_smallest_vertex() {
        let ring = [(1.0, 0.0), (0.0, 1.0), (0.0, 0.5), (2.0, 2.0)];
        assert_eq!(
            rotate_min(&ring),
            [(0.0, 0.5), (2.0, 2.0), (1.0, 0.0), (0.0, 1.0)]
        );
        let repeated = [(0.0, 0.0), (1.0, 1.0), (0.0, 0.0), (0.5, 0.5)];
        assert_eq!(rotate_min(&repeated)[1], (0.5, 0.5));
    }

    #[test]
    fn subpaths_are_sorted_and_classified() {
        let path = Path::new(
            vec![
                Point::new(2.0, 0.0),
                Point::new(3.0, 0.0),
                Point::new(1.0, -0.0),
                Point::new(1.0, 1.0),
                Point::new(0.0, 1.0),
                Point::new(1.0, -0.0),
            ],
            vec![
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::LineTo,
                PathCode::ClosePoly,
            ],
        )
        .unwrap();
        let json = canonical(&path);
        let expected = Json::parse(
            r#"[{"closed": true, "vertices": [[0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]},
                {"closed": false, "vertices": [[2.0, 0.0], [3.0, 0.0]]}]"#,
        )
        .unwrap();
        assert_eq!(json, expected);
        assert_eq!(
            canonical(&Path::polyline(Vec::new())),
            Json::Array(Vec::new())
        );
    }
}
