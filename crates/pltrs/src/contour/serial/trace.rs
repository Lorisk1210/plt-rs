//! Following one contour through the quads of a chunk: along boundaries
//! and through the interior, turning by the z-levels of the points ahead
//! (contourpy's `follow_boundary`, `follow_interior` and
//! `move_to_next_boundary_edge`).
//!
//! A position is a quad and two offsets: `forward`, the direction of
//! travel, and `left`, the direction to its left, both as index offsets
//! (±1 in x, ±nx in y, or their sums on the diagonal of a corner). On a
//! boundary, the edge followed is the one on the right of the quad.

use pltrs_core::Point;

use super::cache::{
    Cache, Corner, LOOK_N, LOOK_S, START_BOUNDARY_S, START_BOUNDARY_W, START_CORNER, START_E,
    START_HOLE_N, START_N,
};
use super::{Field, Levels};

/// Where a contour is: the quad, the direction of travel and its left,
/// which level it follows and whether it runs along a boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Location {
    pub(super) quad: isize,
    pub(super) forward: isize,
    pub(super) left: isize,
    /// Following the upper level (filled contours only).
    pub(super) is_upper: bool,
    pub(super) on_boundary: bool,
}

impl Location {
    /// A location.
    pub(super) const fn new(
        quad: isize,
        forward: isize,
        left: isize,
        is_upper: bool,
        on_boundary: bool,
    ) -> Self {
        Self {
            quad,
            forward,
            left,
            is_upper,
            on_boundary,
        }
    }

    /// Whether `other` is at the same quad, facing the same way.
    fn same_place(&self, quad: isize, forward: isize, left: isize) -> bool {
        self.quad == quad && self.forward == forward && self.left == left
    }
}

/// Which of the two sweeps of a chunk is running: the first only counts
/// (and settles which starts remain), the second records the points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pass {
    Count,
    Record,
}

/// Where a contour goes on leaving a quad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Left,
    Straight,
    Right,
}

/// The state of tracing one chunk (contourpy's `ChunkLocal`, without the
/// output arrays).
#[derive(Debug)]
pub(super) struct Tracer<'a> {
    pub(super) field: &'a Field,
    pub(super) cache: &'a mut Cache,
    pub(super) levels: Levels,
    pub(super) corner_mask: bool,
    pub(super) pass: Pass,
    /// The quads where the outer polygon being traced passed a `LOOK_N`.
    pub(super) look_up_quads: Vec<isize>,
    /// The points of the line or ring being traced (second pass only).
    pub(super) points: Vec<Point>,
}

impl Tracer<'_> {
    fn nx(&self) -> isize {
        self.field.nx
    }

    fn recording(&self) -> bool {
        self.pass == Pass::Record
    }

    /// Follows a boundary from `location` until the contour leaves it into
    /// the interior or returns to `start`; returns whether it returned.
    pub(super) fn follow_boundary(
        &mut self,
        location: &mut Location,
        start: &Location,
        point_count: &mut usize,
    ) -> bool {
        let nx = self.nx();
        let (mut quad, mut forward, mut left) = (location.quad, location.forward, location.left);
        let start_point = self.boundary_start_point(location);
        let mut end_point = start_point + forward;
        let mut end_z = self.cache.z_level(end_point);
        // The first point is somewhere along the first edge: at its start
        // for a boundary start.
        *point_count += 1;
        if self.recording() {
            if self.cache.z_level(start_point) == 1 {
                self.point(start_point);
            } else {
                self.interp(start_point, end_point, location.is_upper);
            }
        }
        let mut finished = false;
        loop {
            if start.same_place(quad, forward, left) {
                if start.on_boundary && *point_count > 1 {
                    finished = true;
                    break;
                }
            } else if self.pass == Pass::Count {
                self.clear_boundary_start(quad, forward, left);
            }
            if end_z != 1 {
                // Leave the boundary along this level.
                location.is_upper = end_z == 2;
                (forward, left) = (left, -forward);
                break;
            }
            *point_count += 1;
            if self.recording() {
                self.point(end_point);
                if self.levels.filled
                    && self.cache.has(quad, LOOK_N)
                    && (left == nx || left == nx + 1 || forward == nx + 1)
                {
                    self.look_up_quads.push(quad);
                }
            }
            self.move_to_next_boundary_edge(&mut quad, &mut forward, &mut left);
            end_point += forward;
            end_z = self.cache.z_level(end_point);
        }
        location.quad = quad;
        location.forward = forward;
        location.left = left;
        finished
    }

    /// Clears the boundary start at a place a contour passes, which it
    /// would otherwise trace again.
    fn clear_boundary_start(&mut self, quad: isize, forward: isize, left: isize) {
        let nx = self.nx();
        let start = if left == nx {
            START_BOUNDARY_S
        } else if forward == -nx {
            START_BOUNDARY_W
        } else if left == -nx {
            START_HOLE_N
        } else {
            let start_corner = match self.cache.corner(quad) {
                Some(Corner::Ne) => left == nx + 1,
                Some(Corner::Nw) => forward == nx + 1,
                Some(Corner::Se) => forward == -nx - 1,
                Some(Corner::Sw) => left == -nx - 1,
                None => false,
            };
            if start_corner { START_CORNER } else { 0 }
        };
        self.cache.clear(quad, start);
    }

    /// Follows the interior from `location` until the contour reaches a
    /// boundary or returns to `start`; returns whether it returned. Adds
    /// the entry point of every quad visited, and the exit point of the
    /// last one for lines.
    pub(super) fn follow_interior(
        &mut self,
        location: &mut Location,
        start: &Location,
        point_count: &mut usize,
    ) -> bool {
        let (mut quad, mut forward, mut left) = (location.quad, location.forward, location.left);
        let is_upper = location.is_upper;
        let (mut left_point, mut start_corner_diagonal) = self.interior_start_left_point(location);
        let mut right_point = left_point - left;
        let want_look_n = self.levels.filled && self.recording();
        let mut finished = false;
        loop {
            if self.recording() {
                self.interp(left_point, right_point, is_upper);
            }
            *point_count += 1;
            if start.same_place(quad, forward, left)
                && is_upper == start.is_upper
                && !start.on_boundary
                && *point_count > 1
            {
                finished = true;
                break;
            }
            let mut opposite_left = left_point + forward;
            let mut opposite_right = right_point + forward;
            if start_corner_diagonal {
                let opposite;
                (forward, left, opposite) = self.diagonal_entry(quad);
                (opposite_left, opposite_right) = (opposite, opposite);
            }
            let (direction, corner_opposite_is_right) = self.direction(
                quad,
                forward,
                [opposite_left, opposite_right].map(|p| self.cache.z_level(p)),
                is_upper,
                start_corner_diagonal,
            );
            if self.pass == Pass::Count
                && !start.same_place(quad, forward, left)
                && self.clear_interior_start(quad, forward, left, direction, is_upper)
                && !self.levels.filled
                && quad < start.quad
            {
                // The rest of this line was counted from that start.
                break;
            }
            let mut reached_boundary = false;
            match direction {
                Direction::Left => {
                    (forward, left) = (left, -forward);
                    right_point = opposite_left;
                }
                Direction::Right => {
                    (forward, left) = (-left, forward);
                    left_point = opposite_right;
                }
                Direction::Straight if self.cache.exists_quad(quad) => {
                    left_point = opposite_left;
                    right_point = opposite_right;
                }
                Direction::Straight if start_corner_diagonal => right_point = opposite_right,
                Direction::Straight => {
                    // Straight on in a corner reaches its diagonal boundary.
                    reached_boundary = true;
                    if corner_opposite_is_right {
                        right_point = opposite_right;
                    } else {
                        left_point = opposite_left;
                    }
                    (forward, left) = self.diagonal_exit(quad);
                }
            }
            if want_look_n && forward == 1 && self.answers_look_n(quad, is_upper) {
                self.look_up_quads.push(quad);
            }
            if !reached_boundary && self.boundary_ahead(quad, forward) {
                reached_boundary = true;
                (forward, left) = (left, -forward);
            }
            if reached_boundary {
                if !self.levels.filled {
                    *point_count += 1;
                    if self.recording() {
                        self.interp(left_point, right_point, false);
                    }
                }
                break;
            }
            quad += forward;
            start_corner_diagonal = false;
        }
        location.quad = quad;
        location.forward = forward;
        location.left = left;
        location.is_upper = is_upper;
        finished
    }

    /// Facing 45° left of the diagonal of the corner `quad` instead: the
    /// direction, its left and the point opposite the diagonal.
    fn diagonal_entry(&self, quad: isize) -> (isize, isize, isize) {
        let nx = self.nx();
        match self.cache.corner(quad) {
            Some(Corner::Nw) => (-1, -nx, quad - 1),
            Some(Corner::Ne) => (nx, -1, quad),
            Some(Corner::Sw) => (-nx, 1, quad - nx - 1),
            _ => (1, nx, quad - nx),
        }
    }

    /// Leaving the corner `quad` along its diagonal boundary: the
    /// direction and its left.
    fn diagonal_exit(&self, quad: isize) -> (isize, isize) {
        let nx = self.nx();
        match self.cache.corner(quad) {
            Some(Corner::Nw) => (nx + 1, nx - 1),
            Some(Corner::Ne) => (-nx + 1, nx + 1),
            Some(Corner::Sw) => (nx - 1, -nx - 1),
            _ => (-nx - 1, -nx + 1),
        }
    }

    /// Where a contour facing `forward` in `quad` goes, given the z-levels
    /// `[left, right]` of the points ahead; and whether, in a corner, the
    /// point opposite the entry edge is the right one.
    fn direction(
        &self,
        quad: isize,
        forward: isize,
        [z_left, z_right]: [u32; 2],
        is_upper: bool,
        diagonal: bool,
    ) -> (Direction, bool) {
        let nx = self.nx();
        let z_test = if is_upper { 2 } else { 0 };
        let (to_left, to_right) = (z_left == z_test, z_right == z_test);
        if self.cache.exists_quad(quad) {
            let direction = match (to_left, to_right) {
                (true, _) if to_right || self.cache.middle_z_level(quad) == z_test => {
                    Direction::Left
                }
                (false, true) => Direction::Straight,
                _ => Direction::Right,
            };
            return (direction, false);
        }
        if diagonal {
            let direction = if to_left {
                Direction::Straight
            } else {
                Direction::Right
            };
            return (direction, true);
        }
        let opposite_is_right = match self.cache.corner(quad) {
            Some(Corner::Nw) => forward == -nx,
            Some(Corner::Ne) => forward == -1,
            Some(Corner::Sw) => forward == 1,
            _ => forward == nx,
        };
        let direction = match (opposite_is_right, to_right, to_left) {
            (true, true, _) | (false, _, false) => Direction::Straight,
            (true, false, _) => Direction::Right,
            (false, _, true) => Direction::Left,
        };
        (direction, opposite_is_right)
    }

    /// Whether the edge of `quad` ahead (facing `forward`) is a boundary.
    fn boundary_ahead(&self, quad: isize, forward: isize) -> bool {
        match forward {
            1 => self.cache.boundary_e(quad),
            -1 => self.cache.boundary_w(quad),
            f if f > 0 => self.cache.boundary_n(quad),
            _ => self.cache.boundary_s(quad),
        }
    }

    /// Whether a contour on the given level, crossing the E edge of `quad`,
    /// answers its `LOOK_N`: of a quad with both look flags, only the one
    /// on the other side of its NE point does.
    fn answers_look_n(&self, quad: isize, is_upper: bool) -> bool {
        self.cache.has(quad, LOOK_N)
            && (!self.cache.has(quad, LOOK_S)
                || if is_upper {
                    self.cache.z_ne(quad) < 2
                } else {
                    self.cache.z_ne(quad) > 0
                })
    }

    /// Clears the interior start of `quad` if the contour passing it
    /// leaves the same way; returns whether it did.
    fn clear_interior_start(
        &mut self,
        quad: isize,
        forward: isize,
        left: isize,
        direction: Direction,
        is_upper: bool,
    ) -> bool {
        let nx = self.nx();
        let on_side = |z: u32| if is_upper { z > 0 } else { z < 2 };
        let start = if self.cache.has(quad, START_E)
            && forward == -1
            && left == -nx
            && direction == Direction::Right
            && on_side(self.cache.z_ne(quad))
        {
            START_E
        } else if self.cache.has(quad, START_N)
            && forward == -nx
            && left == 1
            && direction == Direction::Left
            && on_side(self.cache.z_nw(quad))
        {
            START_N
        } else {
            return false;
        };
        self.cache.clear(quad, start);
        true
    }

    /// The point a boundary edge starts at.
    fn boundary_start_point(&self, location: &Location) -> isize {
        let (quad, forward, left) = (location.quad, location.forward, location.left);
        let nx = self.nx();
        if forward > 0 {
            if forward == nx {
                quad - nx
            } else if left == nx {
                quad - nx - 1
            } else if self.cache.corner(quad) == Some(Corner::Sw) {
                quad - nx
            } else {
                quad - nx - 1
            }
        } else if forward == -nx {
            quad - 1
        } else if left == -nx {
            quad
        } else if self.cache.corner(quad) == Some(Corner::Ne) {
            quad - 1
        } else {
            quad
        }
    }

    /// The left point of the edge a contour enters a quad by, and whether
    /// it enters by the diagonal of a corner.
    fn interior_start_left_point(&self, location: &Location) -> (isize, bool) {
        let (quad, forward, left) = (location.quad, location.forward, location.left);
        let nx = self.nx();
        if forward > 0 {
            if forward == nx {
                (quad - nx - 1, false)
            } else if left == nx {
                (quad - 1, false)
            } else if self.cache.corner(quad) == Some(Corner::Nw) {
                (quad - nx - 1, true)
            } else {
                (quad - 1, true)
            }
        } else if forward == -nx {
            (quad, false)
        } else if left == -nx {
            (quad - nx, false)
        } else if self.cache.corner(quad) == Some(Corner::Sw) {
            (quad - nx, true)
        } else {
            (quad, true)
        }
    }

    /// Moves to the next boundary edge clockwise around the end point of
    /// the current one.
    fn move_to_next_boundary_edge(&self, quad: &mut isize, forward: &mut isize, left: &mut isize) {
        let nx = self.nx();
        // The edges leaving the NE point of a quad, clockwise from the
        // diagonal towards SW (0) to the edge towards S (7): first the
        // quad whose NE point is the end point, and the edge arrived by.
        let mut edge = if *forward > 0 {
            if *forward == nx {
                0
            } else if *left == nx {
                *quad -= nx;
                2
            } else if self.cache.corner(*quad) == Some(Corner::Sw) {
                *quad -= 1;
                7
            } else {
                1
            }
        } else if *forward == -nx {
            *quad -= nx + 1;
            4
        } else if *left == -nx {
            *quad -= 1;
            6
        } else if self.cache.corner(*quad) == Some(Corner::Ne) {
            *quad -= nx;
            3
        } else {
            *quad -= nx + 1;
            5
        };
        // Without corners, only the edges along the grid lines (odd).
        if !self.corner_mask {
            edge += 1;
        }
        loop {
            let q = *quad;
            let next = match edge {
                0 => (self.cache.corner(q) == Some(Corner::Se)).then_some((q, -nx - 1, -nx + 1)),
                1 => self.cache.boundary_n(q).then_some((q, -1, -nx)),
                2 => (self.cache.corner(q + nx) == Some(Corner::Sw)).then_some((
                    q + nx,
                    nx - 1,
                    -nx - 1,
                )),
                3 => self.cache.boundary_e(q + nx).then_some((q + nx, nx, -1)),
                4 => (self.cache.corner(q + nx + 1) == Some(Corner::Nw)).then_some((
                    q + nx + 1,
                    nx + 1,
                    nx - 1,
                )),
                5 => self.cache.boundary_n(q + 1).then_some((q + nx + 1, 1, nx)),
                6 => (self.cache.corner(q + 1) == Some(Corner::Ne)).then_some((
                    q + 1,
                    -nx + 1,
                    nx + 1,
                )),
                _ => self.cache.boundary_e(q).then_some((q + 1, -nx, 1)),
            };
            if let Some(next) = next {
                (*quad, *forward, *left) = next;
                return;
            }
            edge = if self.corner_mask {
                (edge + 1) % 8
            } else {
                (edge + 2) % 8
            };
        }
    }

    /// Adds a grid point.
    fn point(&mut self, point: isize) {
        let point = self.field.point(point);
        self.points.push(point);
    }

    /// Adds the point between `point0` and `point1` where z crosses the
    /// lower or upper level.
    fn interp(&mut self, point0: isize, point1: isize, is_upper: bool) {
        let level = if is_upper {
            self.levels.upper
        } else {
            self.levels.lower
        };
        let (z0, z1) = (self.field.z_at(point0), self.field.z_at(point1));
        let frac = (z1 - level) / (z1 - z0);
        let (p0, p1) = (self.field.point(point0), self.field.point(point1));
        self.points.push(Point::new(
            p0.x * frac + p1.x * (1.0 - frac),
            p0.y * frac + p1.y * (1.0 - frac),
        ));
    }
}
