//! Sweeping one chunk for contour starts and tracing each contour from
//! its start (contourpy's `march_chunk`, `line`, `closed_line` and
//! `closed_line_wrapper`).
//!
//! A chunk is swept twice. The first sweep only counts, and while tracing
//! clears the starts of contours already traced from elsewhere, so that
//! the second sweep, which records the points, traces each contour once.
//! Filled contours trace holes during the first sweep to mark where their
//! outer polygon will find them (`LOOK_N` below each hole, `LOOK_S` at its
//! start); the second sweep traces each hole right after its outer
//! polygon.

use super::QuadRange;
use super::cache::{
    Corner, LOOK_N, LOOK_S, NO_MORE_STARTS, NO_STARTS_IN_ROW, START_BOUNDARY_E, START_BOUNDARY_N,
    START_BOUNDARY_S, START_BOUNDARY_W, START_CORNER, START_E, START_HOLE_N, START_N,
};
use super::trace::{Location, Pass, Tracer};
use crate::contour::assemble::LevelPath;

/// Whether a ring of a filled contour is an outer polygon or a hole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ring {
    Outer,
    Hole,
}

/// What a sweep has traced so far.
#[derive(Clone, Copy, Debug, Default)]
struct Counts {
    points: usize,
    lines: usize,
    holes: usize,
}

impl Counts {
    /// The contours started from the sweep (holes start from their outer
    /// polygon in the second sweep, so they never count).
    fn starts(self, filled: bool) -> usize {
        if filled {
            self.lines - self.holes
        } else {
            self.lines
        }
    }
}

/// Tracing the contours of one chunk into a path.
#[derive(Debug)]
pub(super) struct ChunkMarch<'a> {
    pub(super) tracer: Tracer<'a>,
    pub(super) range: QuadRange,
    pub(super) path: &'a mut LevelPath,
    counts: Counts,
}

impl<'a> ChunkMarch<'a> {
    /// A march over `range`.
    pub(super) fn new(tracer: Tracer<'a>, range: QuadRange, path: &'a mut LevelPath) -> Self {
        Self {
            tracer,
            range,
            path,
            counts: Counts::default(),
        }
    }

    /// Counts, then records, the contours of the chunk.
    pub(super) fn run(mut self) {
        self.sweep(Pass::Count);
        if self.counts.points > 0 {
            self.counts = Counts::default();
            self.sweep(Pass::Record);
        }
    }

    fn nx(&self) -> isize {
        self.tracer.field.nx
    }

    fn filled(&self) -> bool {
        self.tracer.levels.filled
    }

    fn has(&self, quad: isize, flag: u32) -> bool {
        self.tracer.cache.has(quad, flag)
    }

    /// Starts every contour of the chunk, row by row, skipping the rows
    /// known to have no starts.
    fn sweep(&mut self, pass: Pass) {
        self.tracer.pass = pass;
        let (nx, range, filled) = (self.nx(), self.range, self.filled());
        let mut j_final_start = range.jstart;
        for j in range.jstart..=range.jend {
            let row = range.istart + j * nx;
            if self.has(row, NO_MORE_STARTS) {
                break;
            }
            if self.has(row, NO_STARTS_IN_ROW) {
                continue;
            }
            let before = self.counts.starts(filled);
            for quad in row..=row + range.iend - range.istart {
                if !self.tracer.cache.any_start(quad) {
                    continue;
                }
                if filled {
                    self.start_filled(quad);
                } else {
                    self.start_lines(quad);
                }
            }
            if self.counts.starts(filled) > before {
                j_final_start = j;
            } else {
                self.tracer.cache.set(row, NO_STARTS_IN_ROW);
            }
        }
        if j_final_start < range.jend {
            self.tracer
                .cache
                .set(range.istart + (j_final_start + 1) * nx, NO_MORE_STARTS);
        }
    }

    /// Traces the filled contours starting at `quad`. Each start is looked
    /// up afresh, as tracing may clear the later ones.
    fn start_filled(&mut self, quad: isize) {
        let nx = self.nx();
        let holes_from_outers = self.tracer.pass == Pass::Record;
        if self.has(quad, START_BOUNDARY_S) {
            let upper = self.tracer.cache.z_sw(quad) == 2;
            self.closed_line_wrapper(Location::new(quad, 1, nx, upper, true), Ring::Outer);
        }
        if self.has(quad, START_BOUNDARY_W) {
            let upper = self.tracer.cache.z_nw(quad) == 2;
            self.closed_line_wrapper(Location::new(quad, -nx, 1, upper, true), Ring::Outer);
        }
        if self.has(quad, START_CORNER) {
            let cache = &*self.tracer.cache;
            let start = match cache.corner(quad) {
                Some(Corner::Ne) => Some((-nx + 1, nx + 1, cache.z_nw(quad) == 2, Ring::Outer)),
                Some(Corner::Nw) => Some((nx + 1, nx - 1, cache.z_sw(quad) == 2, Ring::Outer)),
                Some(Corner::Se) => Some((-nx - 1, -nx + 1, cache.z_ne(quad) == 2, Ring::Outer)),
                _ => (!holes_from_outers).then_some((nx - 1, -nx - 1, false, Ring::Hole)),
            };
            if let Some((forward, left, upper, ring)) = start {
                self.closed_line_wrapper(Location::new(quad, forward, left, upper, true), ring);
            }
        }
        if self.has(quad, START_N) {
            let upper = self.tracer.cache.z_nw(quad) > 0;
            self.closed_line_wrapper(Location::new(quad, -nx, 1, upper, false), Ring::Outer);
        }
        if holes_from_outers {
            return;
        }
        if self.has(quad, START_E) {
            let upper = self.tracer.cache.z_ne(quad) > 0;
            self.closed_line_wrapper(Location::new(quad, -1, -nx, upper, false), Ring::Hole);
        }
        if self.has(quad, START_HOLE_N) {
            self.closed_line_wrapper(Location::new(quad, -1, -nx, false, true), Ring::Hole);
        }
    }

    /// Traces the lines starting at `quad`, boundary starts first.
    fn start_lines(&mut self, quad: isize) {
        let nx = self.nx();
        let starts = [
            (START_BOUNDARY_S, nx, -1, true),
            (START_BOUNDARY_W, 1, nx, true),
            (START_BOUNDARY_E, -1, -nx, true),
            (START_BOUNDARY_N, -nx, 1, true),
            (START_E, -1, -nx, false),
            (START_N, -nx, 1, false),
        ];
        for (flag, forward, left, on_boundary) in starts {
            if self.has(quad, flag) {
                self.line(Location::new(quad, forward, left, false, on_boundary));
            }
        }
        if self.has(quad, START_CORNER) {
            let (forward, left) = match self.tracer.cache.corner(quad) {
                Some(Corner::Ne) => (nx + 1, nx - 1),
                Some(Corner::Nw) => (nx - 1, -nx - 1),
                Some(Corner::Se) => (-nx + 1, nx + 1),
                _ => (-nx - 1, -nx + 1),
            };
            self.line(Location::new(quad, forward, left, false, true));
        }
    }

    /// Traces one line from `start`.
    fn line(&mut self, start: Location) {
        let mut location = start;
        let mut point_count = 0;
        let looped = self
            .tracer
            .follow_interior(&mut location, &start, &mut point_count);
        if self.tracer.pass == Pass::Record {
            self.path.add_line(&self.tracer.points);
            self.tracer.points.clear();
        }
        if self.tracer.pass == Pass::Count && !start.on_boundary && !looped {
            // An interior start that is not a loop belongs to a line from a
            // boundary, traced later from there: don't count its first
            // point twice.
            point_count -= 1;
        } else {
            self.counts.lines += 1;
        }
        self.counts.points += point_count;
    }

    /// Traces one ring of a filled contour from `start`, alternating
    /// between boundary and interior until it closes.
    fn closed_line(&mut self, start: Location, ring: Ring) {
        if ring == Ring::Hole && self.tracer.pass == Pass::Count {
            self.set_look_flags(start.quad);
        }
        let mut location = start;
        let mut point_count = 0;
        let mut closed = false;
        while !closed {
            closed = if location.on_boundary {
                self.tracer
                    .follow_boundary(&mut location, &start, &mut point_count)
            } else {
                self.tracer
                    .follow_interior(&mut location, &start, &mut point_count)
            };
            location.on_boundary = !location.on_boundary;
        }
        if self.tracer.pass == Pass::Record {
            self.path.add_polyline(&self.tracer.points, true);
            self.tracer.points.clear();
        }
        self.counts.points += point_count;
        self.counts.lines += 1;
        if ring == Ring::Hole {
            self.counts.holes += 1;
        }
    }

    /// Traces a ring and, in the second sweep, the holes of an outer one
    /// right after it.
    fn closed_line_wrapper(&mut self, start: Location, ring: Ring) {
        if self.tracer.pass == Pass::Count {
            self.closed_line(start, ring);
            return;
        }
        let nx = self.nx();
        self.tracer.look_up_quads.clear();
        self.closed_line(start, ring);
        // Tracing holes may find more of them.
        let mut next = 0;
        while let Some(&look_n) = self.tracer.look_up_quads.get(next) {
            let quad = self.find_look_s(look_n);
            let cache = &*self.tracer.cache;
            let hole = if cache.has(quad, START_E) {
                Location::new(quad, -1, -nx, cache.z_ne(quad) > 0, false)
            } else if cache.has(quad, START_HOLE_N) {
                Location::new(quad, -1, -nx, false, true)
            } else {
                Location::new(quad, nx - 1, -nx - 1, false, true)
            };
            self.closed_line(hole, Ring::Hole);
            next += 1;
        }
    }

    /// The hole start N of a `LOOK_N` quad.
    fn find_look_s(&self, look_n: isize) -> isize {
        let mut quad = look_n;
        while !self.has(quad, LOOK_S) {
            quad += self.nx();
        }
        quad
    }

    /// Marks the start of a hole and, straight S of it, where the outer
    /// polygon around it passes.
    fn set_look_flags(&mut self, hole_start: isize) {
        let nx = self.nx();
        let cache = &mut *self.tracer.cache;
        cache.set(hole_start, LOOK_S);
        let mut quad = hole_start;
        loop {
            if cache.boundary_s(quad)
                || matches!(cache.corner(quad), Some(Corner::Ne | Corner::Nw))
                || cache.z_se(quad) != 1
            {
                cache.set(quad, LOOK_N);
                return;
            }
            quad -= nx;
        }
    }
}
