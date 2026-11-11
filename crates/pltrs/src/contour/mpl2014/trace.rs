//! Following contours through the grid: across quad interiors and along
//! boundaries, with mpl2014's exact interpolation arithmetic.

use pltrs_core::Point;
use pltrs_core::numeric::sum_of_products;

use super::cache::{Cache, Corner, Exists, LevelIndex, VISITED_CORNER, VISITED_S, VISITED_W};
use super::edge::{
    Dir, Edge, QuadEdge, QuadPoints, edge_point, exit_edge, is_boundary, next_boundary_edge,
    next_quad,
};
use super::filled::ParentCache;
use crate::contour::quad::GridCoordinates;

/// Grid coordinates and values, row-major with `nx` points per row.
#[derive(Clone, Debug)]
pub(super) struct Field {
    pub(super) coords: GridCoordinates,
    pub(super) z: Vec<f64>,
    pub(super) nx: usize,
}

impl Field {
    /// The position of grid point `point`.
    fn point(&self, point: usize) -> Point {
        self.coords.at(point, self.nx)
    }
}

/// The state of one contouring pass: the cache, the levels and, for filled
/// contours, the parent cache that assigns holes to their outer polygons.
pub(super) struct Tracer<'a> {
    pub(super) field: &'a Field,
    pub(super) cache: &'a mut Cache,
    pub(super) corner_mask: bool,
    pub(super) lower: f64,
    pub(super) upper: f64,
    /// Present for filled contours only.
    pub(super) parents: Option<ParentCache>,
    /// The parent-cache entry recorded while tracing the current polygon
    /// (the polygon itself, or the parent of a hole).
    pub(super) owner: usize,
}

impl Tracer<'_> {
    /// The contour value of a level index.
    pub(super) fn level_value(&self, level: LevelIndex) -> f64 {
        match level {
            LevelIndex::Lower => self.lower,
            LevelIndex::Upper => self.upper,
        }
    }

    fn record_parent(&mut self, quad: usize) {
        if let Some(parents) = &mut self.parents {
            parents.set(quad, self.owner);
        }
    }

    /// Appends the point where `level` crosses `at` (matplotlib's
    /// `edge_interp`/`interp`): `xy1 * f + xy2 * (1 - f)` with
    /// `f = (z2 - level) / (z2 - z1)`, which is not symmetric in rounding.
    pub(super) fn interp(&self, at: QuadEdge, level: f64, line: &mut Vec<Point>) {
        let nx = self.field.nx;
        let (p1, p2) = (edge_point(at, true, nx), edge_point(at, false, nx));
        let z = &self.field.z;
        let fraction = (z[p2] - level) / (z[p2] - z[p1]);
        let (xy1, xy2) = (self.field.point(p1), self.field.point(p2));
        line.push(Point::new(
            sum_of_products(xy1.x, fraction, xy2.x, 1.0 - fraction),
            sum_of_products(xy1.y, fraction, xy2.y, 1.0 - fraction),
        ));
    }

    /// Appends the coordinates of grid point `point`.
    fn point_xy(&self, point: usize, line: &mut Vec<Point>) {
        line.push(self.field.point(point));
    }

    /// Follows a contour across quad interiors from `at` until it reaches a
    /// boundary or, with `stop_at`, returns to that edge on that level
    /// (matplotlib's `follow_interior`). Adds the exit point of every quad
    /// traversed, plus the entry point if `want_initial_point`.
    pub(super) fn follow_interior(
        &mut self,
        line: &mut Vec<Point>,
        at: &mut QuadEdge,
        level: LevelIndex,
        want_initial_point: bool,
        stop_at: Option<(QuadEdge, LevelIndex)>,
    ) {
        let value = self.level_value(level);
        if want_initial_point {
            self.interp(*at, value, line);
        }
        loop {
            let dir = self.turn(*at, level, value);
            at.edge = exit_edge(self.cache, *at, dir);
            match at.edge {
                Edge::E => self.record_parent(at.quad + 1),
                Edge::W => self.record_parent(at.quad),
                _ => {}
            }
            self.interp(*at, value, line);
            if is_boundary(self.cache, *at) {
                break;
            }
            *at = next_quad(*at, self.field.nx);
            if stop_at == Some((*at, level)) {
                break;
            }
        }
    }

    /// The direction to leave the quad entered through `at`, updating its
    /// visited and saddle flags.
    fn turn(&mut self, at: QuadEdge, level: LevelIndex, value: f64) -> Dir {
        let quad = at.quad;
        let p = QuadPoints::new(quad, self.field.nx);
        let upper = level == LevelIndex::Upper;
        if self.cache.has(quad, level.saddle()) {
            self.cache.set(quad, level.visited());
            return if self.cache.has(quad, level.saddle_left()) {
                Dir::Left
            } else {
                Dir::Right
            };
        }
        if let Exists::Corner(corner) = self.cache.exists(quad) {
            // The point opposite the entry edge decides the turn; upper
            // levels have higher values on the right.
            let opposite = match at.edge {
                Edge::E if corner == Corner::Se => p.sw,
                Edge::N if corner == Corner::Nw => p.sw,
                Edge::W if corner == Corner::Sw => p.se,
                Edge::S if corner == Corner::Sw => p.nw,
                Edge::E | Edge::Se => p.nw,
                Edge::N | Edge::Nw => p.se,
                Edge::W | Edge::S | Edge::Sw => p.ne,
                Edge::Ne => p.sw,
            };
            self.cache.set(quad, level.visited());
            return if (self.cache.z_level(opposite) >= level.z()) ^ upper {
                Dir::Right
            } else {
                Dir::Left
            };
        }
        let (left, right) = match at.edge {
            Edge::E => (p.sw, p.nw),
            Edge::N => (p.se, p.sw),
            Edge::W => (p.ne, p.se),
            // Diagonal edges belong to corners only and never enter here.
            Edge::S | Edge::Ne | Edge::Nw | Edge::Sw | Edge::Se => (p.nw, p.ne),
        };
        let mut config = (u8::from(self.cache.z_level(left) >= level.z()) << 1)
            | u8::from(self.cache.z_level(right) >= level.z());
        if upper {
            config = 3 - config;
        }
        if config != 1 {
            self.cache.set(quad, level.visited());
            return match config {
                0 => Dir::Left,
                3 => Dir::Right,
                _ => Dir::Straight,
            };
        }
        // A new saddle: the mean of the corners decides, and the second
        // visit is remembered in the cache.
        let z = &self.field.z;
        let zmid = 0.25 * (z[p.sw] + z[p.se] + z[p.nw] + z[p.ne]);
        self.cache.set(quad, level.saddle());
        if matches!(at.edge, Edge::N | Edge::E) {
            self.cache.set(quad, level.saddle_start_sw());
        }
        if (zmid > value) ^ upper {
            Dir::Right
        } else {
            self.cache.set(quad, level.saddle_left());
            Dir::Left
        }
    }

    /// Follows a filled contour along the boundary from `at` until it
    /// leaves into the interior or returns to `start` (matplotlib's
    /// `follow_boundary`). Adds the end point of every boundary edge and,
    /// on leaving, the crossing point. Returns the level it ends on.
    pub(super) fn follow_boundary(
        &mut self,
        line: &mut Vec<Point>,
        at: &mut QuadEdge,
        mut level: LevelIndex,
        start: QuadEdge,
    ) -> LevelIndex {
        let nx = self.field.nx;
        let mut end_level = 0;
        let mut first_edge = true;
        loop {
            let start_level = if first_edge {
                self.cache.z_level(edge_point(*at, true, nx))
            } else {
                end_level
            };
            let end_point = edge_point(*at, false, nx);
            end_level = self.cache.z_level(end_point);
            let mut stop = false;
            if start_level <= level.z() && end_level == 2 {
                // Increasing z: continue on (or switch to) the upper level.
                level = LevelIndex::Upper;
                stop = true;
            } else if start_level >= 1 && end_level == 0 {
                // Decreasing z: continue on (or switch to) the lower level.
                level = LevelIndex::Lower;
                stop = true;
            }
            if !first_edge && !stop && *at == start {
                break;
            }
            self.mark_boundary_visited(*at);
            if stop {
                self.interp(*at, self.level_value(level), line);
                break;
            }
            *at = next_boundary_edge(self.cache, *at, nx, self.corner_mask);
            // Label the parent of the start of the new edge.
            match at.edge {
                Edge::W | Edge::Sw | Edge::S | Edge::Se => {
                    if !self.cache.exists_corner(at.quad, Corner::Se) {
                        self.record_parent(at.quad);
                    }
                }
                Edge::E | Edge::Ne | Edge::N | Edge::Nw => {
                    if !self.cache.exists_corner(at.quad, Corner::Sw) {
                        self.record_parent(at.quad + 1);
                    }
                }
            }
            self.point_xy(end_point, line);
            first_edge = false;
        }
        level
    }

    fn mark_boundary_visited(&mut self, at: QuadEdge) {
        let nx = self.field.nx;
        match at.edge {
            Edge::E => self.cache.set(at.quad + 1, VISITED_W),
            Edge::N => self.cache.set(at.quad + nx, VISITED_S),
            Edge::W => self.cache.set(at.quad, VISITED_W),
            Edge::S => self.cache.set(at.quad, VISITED_S),
            Edge::Ne | Edge::Nw | Edge::Sw | Edge::Se => self.cache.set(at.quad, VISITED_CORNER),
        }
    }

    /// The edge a contour on `level` starts from within `quad`, if any
    /// (matplotlib's `get_start_edge`).
    pub(super) fn start_edge(&self, quad: usize, level: LevelIndex) -> Option<Edge> {
        match self.cache.exists(quad) {
            Exists::Corner(corner) => self.corner_start_edge(quad, corner, level),
            Exists::Quad | Exists::None => self.quad_start_edge(quad, level),
        }
    }

    fn above(&self, point: usize, level: LevelIndex) -> u8 {
        u8::from(self.cache.z_level(point) >= level.z())
    }

    fn corner_start_edge(&self, quad: usize, corner: Corner, level: LevelIndex) -> Option<Edge> {
        // Points 1, 2, 3 and edges 12, 23, 31 anticlockwise, drawn for the
        // NE corner and rotated for the others.
        let p = QuadPoints::new(quad, self.field.nx);
        let (points, edges) = match corner {
            Corner::Sw => ([p.se, p.sw, p.nw], [Edge::S, Edge::W, Edge::Ne]),
            Corner::Se => ([p.ne, p.se, p.sw], [Edge::E, Edge::S, Edge::Nw]),
            Corner::Nw => ([p.sw, p.nw, p.ne], [Edge::W, Edge::N, Edge::Se]),
            Corner::Ne => ([p.nw, p.ne, p.se], [Edge::N, Edge::E, Edge::Sw]),
        };
        let [e12, e23, e31] = edges;
        let mut config = (self.above(points[0], level) << 2)
            | (self.above(points[1], level) << 1)
            | self.above(points[2], level);
        if level == LevelIndex::Upper {
            config = 7 - config;
        }
        match config {
            1 | 5 => Some(e23),
            2 | 3 => Some(e12),
            4 | 6 => Some(e31),
            _ => None,
        }
    }

    fn quad_start_edge(&self, quad: usize, level: LevelIndex) -> Option<Edge> {
        let p = QuadPoints::new(quad, self.field.nx);
        let mut config = (self.above(p.nw, level) << 3)
            | (self.above(p.ne, level) << 2)
            | (self.above(p.sw, level) << 1)
            | self.above(p.se, level);
        if level == LevelIndex::Upper {
            config = 15 - config;
        }
        // A known saddle is resumed from the edge its first visit left for
        // the second one.
        let resume_sw =
            !self.cache.has(quad, level.saddle()) || self.cache.has(quad, level.saddle_start_sw());
        match config {
            1 | 3 | 11 => Some(Edge::E),
            2 | 10 | 14 => Some(Edge::S),
            4 | 5 | 7 => Some(Edge::N),
            8 | 12 | 13 => Some(Edge::W),
            6 => Some(if resume_sw { Edge::S } else { Edge::N }),
            9 => Some(if resume_sw { Edge::W } else { Edge::E }),
            _ => None,
        }
    }
}
