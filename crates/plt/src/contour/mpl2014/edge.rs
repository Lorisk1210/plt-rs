//! Quad edges and moving between them.
//!
//! Edges are ordered anticlockwise around the quad (or corner triangle)
//! they belong to:
//!
//! ```text
//!          Full quad                    NW corner (others similar)
//!
//!  POINT_NW   Edge N   POINT_NE        POINT_NW   Edge N   POINT_NE
//!          +----<-----+                        +----<-----+
//!          |          |                        |         /
//!          |          |                        | quad  /
//!  Edge W  V   quad   ^  Edge E        Edge W  V     ^
//!          |          |                        |   /  Edge SE
//!          |          |                        | /
//!          +---->-----+                        +
//!  POINT_SW   Edge S   POINT_SE        POINT_SW
//! ```

use super::cache::{Cache, Corner, Exists};

/// An edge of a quad, including the diagonal edges of corner triangles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Edge {
    /// East.
    E,
    /// North.
    N,
    /// West.
    W,
    /// South.
    S,
    /// The diagonal of a SW corner.
    Ne,
    /// The diagonal of a SE corner.
    Nw,
    /// The diagonal of a NE corner.
    Sw,
    /// The diagonal of a NW corner.
    Se,
}

/// The direction of travel from one quad to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Dir {
    /// Turn right.
    Right,
    /// Go straight on.
    Straight,
    /// Turn left.
    Left,
}

/// An edge of a particular quad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct QuadEdge {
    /// The quad (index of its SW point).
    pub(super) quad: usize,
    /// The edge.
    pub(super) edge: Edge,
}

impl QuadEdge {
    /// The edge `edge` of `quad`.
    pub(super) const fn new(quad: usize, edge: Edge) -> Self {
        Self { quad, edge }
    }
}

/// The four point indices of a quad on a grid with `nx` points per row.
#[derive(Clone, Copy, Debug)]
pub(super) struct QuadPoints {
    pub(super) sw: usize,
    pub(super) se: usize,
    pub(super) nw: usize,
    pub(super) ne: usize,
}

impl QuadPoints {
    /// The corners of `quad`.
    pub(super) const fn new(quad: usize, nx: usize) -> Self {
        Self {
            sw: quad,
            se: quad + 1,
            nw: quad + nx,
            ne: quad + nx + 1,
        }
    }
}

/// The point where `edge` starts (`start`) or ends, following the
/// anticlockwise ordering (matplotlib's `get_edge_point_index`).
pub(super) fn edge_point(edge: QuadEdge, start: bool, nx: usize) -> usize {
    let p = QuadPoints::new(edge.quad, nx);
    let (from, to) = match edge.edge {
        Edge::E => (p.se, p.ne),
        Edge::N => (p.ne, p.nw),
        Edge::W => (p.nw, p.sw),
        Edge::S => (p.sw, p.se),
        Edge::Ne => (p.se, p.nw),
        Edge::Nw => (p.ne, p.sw),
        Edge::Sw => (p.nw, p.se),
        Edge::Se => (p.sw, p.ne),
    };
    if start { from } else { to }
}

/// The edge to leave `quad` by after entering through `edge` and turning
/// `dir` (matplotlib's `get_exit_edge`).
pub(super) fn exit_edge(cache: &Cache, at: QuadEdge, dir: Dir) -> Edge {
    let left = dir == Dir::Left;
    let right = dir == Dir::Right;
    if let Exists::Corner(corner) = cache.exists(at.quad) {
        // A corner is a triangle: the two other edges are left and right.
        return match at.edge {
            Edge::E if corner == Corner::Se => pick(left, Edge::S, Edge::Nw),
            Edge::E => pick(right, Edge::N, Edge::Sw),
            Edge::N if corner == Corner::Nw => pick(right, Edge::W, Edge::Se),
            Edge::N => pick(left, Edge::E, Edge::Sw),
            Edge::W if corner == Corner::Sw => pick(right, Edge::S, Edge::Ne),
            Edge::W => pick(left, Edge::N, Edge::Se),
            Edge::S if corner == Corner::Sw => pick(left, Edge::W, Edge::Ne),
            Edge::S => pick(right, Edge::E, Edge::Nw),
            Edge::Ne => pick(left, Edge::S, Edge::W),
            Edge::Nw => pick(left, Edge::E, Edge::S),
            Edge::Sw => pick(left, Edge::N, Edge::E),
            Edge::Se => pick(left, Edge::W, Edge::N),
        };
    }
    // A full quad: the other three edges are left, straight and right.
    let (to_left, straight, to_right) = match at.edge {
        Edge::E => (Edge::S, Edge::W, Edge::N),
        Edge::N => (Edge::E, Edge::S, Edge::W),
        Edge::W => (Edge::N, Edge::E, Edge::S),
        // Diagonal edges belong to corners only and never enter here.
        Edge::S | Edge::Ne | Edge::Nw | Edge::Sw | Edge::Se => (Edge::W, Edge::N, Edge::E),
    };
    match dir {
        Dir::Left => to_left,
        Dir::Straight => straight,
        Dir::Right => to_right,
    }
}

const fn pick(condition: bool, yes: Edge, no: Edge) -> Edge {
    if condition { yes } else { no }
}

/// Crosses `edge` into the neighbouring quad, arriving through the
/// opposite edge (matplotlib's `move_to_next_quad`). Only straight edges
/// lead to neighbours: diagonals are always boundaries.
pub(super) fn next_quad(at: QuadEdge, nx: usize) -> QuadEdge {
    match at.edge {
        Edge::E => QuadEdge::new(at.quad + 1, Edge::W),
        Edge::N => QuadEdge::new(at.quad + nx, Edge::S),
        Edge::W => QuadEdge::new(at.quad - 1, Edge::E),
        Edge::S | Edge::Ne | Edge::Nw | Edge::Sw | Edge::Se => QuadEdge::new(at.quad - nx, Edge::N),
    }
}

/// The boundary edge following `at` anticlockwise around the unmasked
/// region (matplotlib's `move_to_next_boundary_edge`): from the end point
/// of `at`, try turning left by 135°, then 90°, 45°, straight on, ...
pub(super) fn next_boundary_edge(
    cache: &Cache,
    at: QuadEdge,
    nx: usize,
    corner_mask: bool,
) -> QuadEdge {
    let point = edge_point(at, false, nx);
    let mut index = match at.edge {
        Edge::E => 0,
        Edge::Se => 1,
        Edge::S => 2,
        Edge::Sw => 3,
        Edge::W => 4,
        Edge::Nw => 5,
        Edge::N => 6,
        Edge::Ne => 7,
    };
    // Without corners only the odd (straight-edge) candidates exist.
    let step = if corner_mask { 1 } else { 2 };
    if !corner_mask {
        index += 1;
    }
    let start = index;
    loop {
        if let Some(next) = boundary_candidate(cache, point, index, nx) {
            return next;
        }
        index = (index + step) % 8;
        if index == start {
            break;
        }
    }
    // A consistent cache always closes its boundaries; this is unreachable.
    QuadEdge::new(point, at.edge)
}

/// Candidate `index` of [`next_boundary_edge`] around `point`, if it is a
/// boundary (the original reads the same cache entries; candidates before
/// the grid start do not exist).
fn boundary_candidate(cache: &Cache, point: usize, index: usize, nx: usize) -> Option<QuadEdge> {
    let (quad, edge) = match index {
        0 => (point.checked_sub(nx + 1), Edge::Nw),
        1 => (point.checked_sub(nx + 1), Edge::N),
        2 => (point.checked_sub(1), Edge::Ne),
        3 => (point.checked_sub(1), Edge::E),
        4 => (Some(point), Edge::Se),
        5 => (Some(point), Edge::S),
        6 => (point.checked_sub(nx), Edge::Sw),
        _ => (point.checked_sub(nx), Edge::W),
    };
    quad.map(|q| QuadEdge::new(q, edge))
        .filter(|&candidate| is_boundary(cache, candidate))
}

/// Whether `at` is a boundary: an edge between existing and masked parts,
/// a chunk boundary, or the diagonal of a corner.
pub(super) fn is_boundary(cache: &Cache, at: QuadEdge) -> bool {
    match at.edge {
        Edge::E => cache.boundary_e(at.quad),
        Edge::N => cache.boundary_n(at.quad),
        Edge::W => cache.boundary_w(at.quad),
        Edge::S => cache.boundary_s(at.quad),
        Edge::Ne => cache.exists_corner(at.quad, Corner::Sw),
        Edge::Nw => cache.exists_corner(at.quad, Corner::Se),
        Edge::Sw => cache.exists_corner(at.quad, Corner::Ne),
        Edge::Se => cache.exists_corner(at.quad, Corner::Nw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_points_run_anticlockwise() {
        let nx = 4;
        let q = QuadEdge::new(5, Edge::E);
        assert_eq!((edge_point(q, true, nx), edge_point(q, false, nx)), (6, 10));
        let q = QuadEdge::new(5, Edge::Nw);
        assert_eq!((edge_point(q, true, nx), edge_point(q, false, nx)), (10, 5));
    }

    #[test]
    fn full_quad_exits_and_neighbours() {
        let cache = Cache::new(3, 3, None, true, (2, 2));
        let at = QuadEdge::new(0, Edge::S);
        assert_eq!(exit_edge(&cache, at, Dir::Left), Edge::W);
        assert_eq!(exit_edge(&cache, at, Dir::Straight), Edge::N);
        assert_eq!(exit_edge(&cache, at, Dir::Right), Edge::E);
        assert_eq!(
            next_quad(QuadEdge::new(0, Edge::N), 3),
            QuadEdge::new(3, Edge::S)
        );
        assert_eq!(
            next_quad(QuadEdge::new(4, Edge::W), 3),
            QuadEdge::new(3, Edge::E)
        );
    }

    #[test]
    fn boundary_walks_around_the_grid() {
        let cache = Cache::new(3, 3, None, false, (2, 2));
        let mut at = QuadEdge::new(0, Edge::S);
        let mut visited = vec![at];
        for _ in 0..7 {
            at = next_boundary_edge(&cache, at, 3, false);
            assert!(is_boundary(&cache, at));
            visited.push(at);
        }
        assert_eq!(next_boundary_edge(&cache, at, 3, false), visited[0]);
        assert_eq!(visited[1], QuadEdge::new(1, Edge::S));
        assert_eq!(visited[2], QuadEdge::new(1, Edge::E));
    }
}
