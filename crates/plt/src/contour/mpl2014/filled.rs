//! Filled contours: polygons between two levels, with every hole assigned
//! to its outer polygon during the sweep.
//!
//! As the sweep moves up through a chunk, the [`ParentCache`] records for
//! each point the first polygon passing through it. A new hole takes as
//! parent the first polygon found at its start point or below it. The order
//! of polygon starts within a quad matters: the S-most polygon is started
//! first.

use plt_core::Point;

use super::cache::{Corner, Exists, LevelIndex, VISITED_CORNER, VISITED_S, VISITED_W};
use super::edge::{Edge, QuadEdge, QuadPoints};
use super::trace::Tracer;

/// The first polygon seen at each point of a chunk (outer polygons only:
/// holes record their parent).
#[derive(Clone, Debug)]
pub(super) struct ParentCache {
    nx: usize,
    x_points: usize,
    istart: usize,
    jstart: usize,
    lines: Vec<Option<usize>>,
}

impl ParentCache {
    /// A cache for chunks of `x_points × y_points` points of a grid with
    /// `nx` points per row.
    pub(super) fn new(nx: usize, x_points: usize, y_points: usize) -> Self {
        Self {
            nx,
            x_points,
            istart: 0,
            jstart: 0,
            lines: vec![None; x_points * y_points],
        }
    }

    /// Starts a new chunk whose SW point is `(istart, jstart)`.
    pub(super) fn start_chunk(&mut self, istart: usize, jstart: usize) {
        self.istart = istart;
        self.jstart = jstart;
        self.lines.fill(None);
    }

    fn index(&self, point: usize) -> usize {
        (point % self.nx - self.istart) + (point / self.nx - self.jstart) * self.x_points
    }

    /// The first polygon recorded at `point` or below it in the chunk.
    fn get(&self, point: usize) -> Option<usize> {
        let mut index = self.index(point);
        loop {
            if let Some(line) = self.lines[index] {
                return Some(line);
            }
            index = index.checked_sub(self.x_points)?;
        }
    }

    /// Records `owner` at `point` unless a polygon is already recorded.
    pub(super) fn set(&mut self, point: usize, owner: usize) {
        let index = self.index(point);
        self.lines[index].get_or_insert(owner);
    }
}

/// Whether a polygon is a hole (clockwise, inside another polygon).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Outer,
    Hole,
}

/// Whether a polygon starts by following a boundary or the interior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Along {
    Boundary,
    Interior,
}

/// A traced polygon ring.
#[derive(Clone, Debug)]
struct Ring {
    points: Vec<Point>,
    role: Role,
    children: Vec<usize>,
}

/// The polygons of one chunk, in creation order.
pub(super) struct ChunkFill<'t, 'a> {
    tracer: &'t mut Tracer<'a>,
    rings: Vec<Ring>,
}

impl<'t, 'a> ChunkFill<'t, 'a> {
    /// Starts filling a chunk.
    pub(super) fn new(tracer: &'t mut Tracer<'a>) -> Self {
        Self {
            tracer,
            rings: Vec::new(),
        }
    }

    /// The outer polygons, each followed by its holes, as closed rings
    /// (matplotlib's `append_contour_to_vertices_and_codes`).
    pub(super) fn into_polygons(self) -> Vec<Vec<Vec<Point>>> {
        self.rings
            .iter()
            .filter(|ring| ring.role == Role::Outer)
            .map(|outer| {
                std::iter::once(outer.points.clone())
                    .chain(outer.children.iter().map(|&c| self.rings[c].points.clone()))
                    .collect()
            })
            .collect()
    }

    /// Starts every polygon that begins in `quad` (matplotlib's
    /// `single_quad_filled`). Checks go from the S edge up to the N edge.
    /// z-levels and the grid topology are fixed during a pass, so they are
    /// read once; visited and saddle flags change as rings are traced.
    pub(super) fn single_quad(&mut self, quad: usize) {
        let cache = &*self.tracer.cache;
        let exists = cache.exists(quad);
        if exists == Exists::None {
            return;
        }
        let p = QuadPoints::new(quad, self.tracer.field.nx);
        let [sw, se, nw, ne] = [p.sw, p.se, p.nw, p.ne].map(|point| cache.z_level(point));
        let s_boundary = cache.boundary_s(quad) && cache.exists_s_edge(quad);
        let w_boundary = cache.boundary_w(quad) && cache.exists_w_edge(quad);
        let n_boundary = cache.boundary_n(quad) && cache.exists_n_edge(quad);

        if s_boundary {
            self.boundary_starts(QuadEdge::new(quad, Edge::S), VISITED_S, (sw, se));
        }
        if w_boundary {
            self.boundary_starts(QuadEdge::new(quad, Edge::W), VISITED_W, (nw, sw));
        }
        match exists {
            Exists::Corner(Corner::Sw) => {
                // The NE diagonal is a boundary: a hole following it.
                if !self.tracer.cache.has(quad, VISITED_CORNER) && nw == 1 && se == 1 {
                    let from = QuadEdge::new(quad, Edge::Ne);
                    self.start(from, LevelIndex::Lower, Role::Hole, Along::Boundary);
                }
            }
            Exists::Corner(Corner::Nw) => {
                self.interior_starts(quad, (nw, sw, ne), (Edge::N, Edge::Se));
            }
            Exists::Corner(Corner::Se) => {
                self.interior_starts(quad, (sw, se, ne), (Edge::Nw, Edge::E));
            }
            Exists::Corner(Corner::Ne) => {
                self.boundary_starts(QuadEdge::new(quad, Edge::Sw), VISITED_CORNER, (nw, se));
            }
            Exists::Quad | Exists::None => {}
        }
        // A full quad or NE corner only starts rings through its NE corner,
        // from N to E (lower level) or E to N (upper level): every other
        // start was made from an earlier quad.
        if matches!(exists, Exists::Quad | Exists::Corner(Corner::Ne)) {
            self.interior_starts(quad, (nw, se, ne), (Edge::N, Edge::E));
        }
        // A hole following the N boundary from E to W: the edge of an
        // interior masked region inside a polygon.
        let n_visited = self
            .tracer
            .cache
            .has(quad + self.tracer.field.nx, VISITED_S);
        if n_boundary && !n_visited && nw == 1 && ne == 1 {
            let from = QuadEdge::new(quad, Edge::N);
            self.start(from, LevelIndex::Lower, Role::Hole, Along::Boundary);
        }
    }

    /// Outer rings starting on the boundary edge `from`, whose start and
    /// end points have z-levels `(a, b)`, into the interior or along the
    /// boundary, unless the edge was already followed.
    fn boundary_starts(&mut self, from: QuadEdge, visited: u32, (a, b): (u32, u32)) {
        let rules = [
            (a >= 1 && b == 0, LevelIndex::Lower, Along::Interior),
            (a < 2 && b == 2, LevelIndex::Upper, Along::Interior),
            (a <= 1 && b == 1, LevelIndex::Lower, Along::Boundary),
            (a == 2 && b == 1, LevelIndex::Upper, Along::Boundary),
        ];
        for (applies, level, along) in rules {
            if applies && !self.tracer.cache.has(from.quad, visited) {
                self.start(from, level, Role::Outer, along);
            }
        }
    }

    /// Rings crossing the corner of `quad` between the points with
    /// z-levels `(a, b)` on one side and `c` on the other: outer rings
    /// start on `outer_edge`, holes on `hole_edge`. In a saddle only the
    /// ring leaving through the E edge is started here.
    fn interior_starts(
        &mut self,
        quad: usize,
        (a, b, c): (u32, u32, u32),
        (outer_edge, hole_edge): (Edge, Edge),
    ) {
        let rules = [
            (
                a == 0 && b == 0 && c >= 1,
                LevelIndex::Lower,
                outer_edge,
                Role::Outer,
            ),
            (
                a < 2 && b < 2 && c == 2,
                LevelIndex::Upper,
                hole_edge,
                Role::Hole,
            ),
            (
                a == 2 && b == 2 && c < 2,
                LevelIndex::Upper,
                outer_edge,
                Role::Outer,
            ),
            (
                a >= 1 && b >= 1 && c == 0,
                LevelIndex::Lower,
                hole_edge,
                Role::Hole,
            ),
        ];
        for (applies, level, edge, role) in rules {
            let cache = &*self.tracer.cache;
            // Corners are never saddles, so this only restricts full quads.
            let saddle_ok = !cache.has(quad, level.saddle())
                || cache.has(quad, level.saddle_left()) == (role == Role::Outer);
            if applies && saddle_ok && !cache.has(quad, level.visited()) {
                self.start(QuadEdge::new(quad, edge), level, role, Along::Interior);
            }
        }
    }

    /// Traces one polygon ring from `from` until it closes (matplotlib's
    /// `start_filled`): a ring started on a boundary ends on one, a ring
    /// started in the interior ends there on its start level.
    fn start(&mut self, from: QuadEdge, start_level: LevelIndex, role: Role, mut along: Along) {
        let id = self.rings.len();
        let parent = match role {
            Role::Hole => self
                .tracer
                .parents
                .as_ref()
                .and_then(|parents| parents.get(from.quad + 1)),
            Role::Outer => None,
        };
        // A hole whose parent cannot be found is kept as an outer ring.
        let role = if parent.is_some() {
            Role::Hole
        } else {
            Role::Outer
        };
        if let Some(parent) = parent {
            self.rings[parent].children.push(id);
        }
        self.rings.push(Ring {
            points: Vec::new(),
            role,
            children: Vec::new(),
        });
        self.tracer.owner = parent.unwrap_or(id);

        let mut points = Vec::new();
        let mut at = from;
        let mut level = start_level;
        loop {
            match along {
                Along::Interior => {
                    self.tracer.follow_interior(
                        &mut points,
                        &mut at,
                        level,
                        false,
                        Some((from, start_level)),
                    );
                }
                Along::Boundary => {
                    level = self
                        .tracer
                        .follow_boundary(&mut points, &mut at, level, from);
                }
            }
            if at == from && (along == Along::Boundary || level == start_level) {
                break;
            }
            along = match along {
                Along::Boundary => Along::Interior,
                Along::Interior => Along::Boundary,
            };
        }
        self.rings[id].points = points;
    }
}
