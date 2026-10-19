//! Classifying the quads for one contouring call: the z-level of every
//! point and where contours start (contourpy's
//! `init_cache_levels_and_starts`).
//!
//! A quad's configuration packs the z-levels of its corners (two bits
//! each for filled contours, one for lines) in the order NW, NE, SW, SE,
//! leaving out the masked point of a corner. Each configuration has its
//! own starts; saddles also record the z-level of their middle, which
//! decides how contours pass through them.

use super::cache::{
    Corner, NO_MORE_STARTS, NO_STARTS_IN_ROW, START_BOUNDARY_E, START_BOUNDARY_N, START_BOUNDARY_S,
    START_BOUNDARY_W, START_CORNER, START_E, START_HOLE_N, START_N,
};
use super::{QuadRange, Serial};

/// `flag` when `condition` holds.
fn flag_if(condition: bool, flag: u32) -> u32 {
    if condition { flag } else { 0 }
}

/// What a quad's start flags depend on besides its configuration: the
/// start each boundary edge would have, 0 where the edge is not a
/// boundary.
#[derive(Clone, Copy, Debug)]
struct Edges {
    n: u32,
    e: u32,
    s: u32,
    w: u32,
    /// `START_HOLE_N` where a hole may start on the N boundary.
    hole: u32,
}

impl Edges {
    /// A line leaving N: on the N boundary, else from the interior unless
    /// the E edge is a boundary (and `interior` holds).
    fn line_n(self, interior: bool) -> u32 {
        if self.n != 0 {
            self.n
        } else {
            flag_if(self.e == 0 && interior, START_N)
        }
    }

    /// A line leaving E: on the E boundary, else from the interior unless
    /// the N edge is a boundary (and `interior` holds).
    fn line_e(self, interior: bool) -> u32 {
        if self.e != 0 {
            self.e
        } else {
            flag_if(self.n == 0 && interior, START_E)
        }
    }
}

/// Packs z-levels of `bits` bits each, first one highest.
fn config(levels: &[u32], bits: u32) -> u32 {
    levels.iter().fold(0, |packed, &z| (packed << bits) | z)
}

impl Serial {
    /// Classifies the quads of `range`, or of the whole grid when it is
    /// `None` (then the points W and S of a chunk are already classified).
    pub(super) fn init_levels_and_starts(&mut self, range: Option<QuadRange>) {
        let ordered = range.is_none();
        let nx = self.nx();
        let (chunk_istart, istart, iend, jstart, jend) = match range {
            Some(r) => (
                r.istart,
                if r.istart > 1 { r.istart } else { 0 },
                r.iend,
                if r.jstart > 1 { r.jstart } else { 0 },
                r.jend,
            ),
            None => (1, 0, nx - 1, 0, self.ny() - 1),
        };
        let mut j_final_start = jstart - 1;
        // Out of order, the points W and S of the chunk are classified here.
        let calc_w = !ordered && istart == chunk_istart;
        for j in jstart..=jend {
            let row = istart + j * nx;
            let mut start_in_row = false;
            let calc_s = !ordered && j == jstart;
            let mut z_nw = if istart == 0 {
                0
            } else if calc_w {
                self.z_to_level(row - 1)
            } else {
                self.cache.z_nw(row)
            };
            let mut z_sw = if istart == 0 || j == 0 {
                0
            } else if calc_w || calc_s {
                self.z_to_level(row - nx - 1)
            } else {
                self.cache.z_sw(row)
            };
            for (quad, i) in (row..).zip(istart..=iend) {
                let z_se = if j == 0 {
                    0
                } else if calc_s {
                    self.z_to_level(quad - nx)
                } else {
                    self.cache.z_se(quad)
                };
                let z_ne = self.z_to_level(quad);
                self.cache.reset(quad, z_ne);
                if self.cache.exists_any(quad) {
                    let starts = self.quad_starts(quad, i, j, [z_nw, z_ne, z_sw, z_se]);
                    self.cache.set(quad, starts);
                    start_in_row |= starts != 0;
                }
                z_nw = z_ne;
                z_sw = z_se;
            }
            if start_in_row {
                j_final_start = j;
            } else if j > 0 {
                self.cache.set(chunk_istart + j * nx, NO_STARTS_IN_ROW);
            }
        }
        if j_final_start < jend {
            self.cache
                .set(chunk_istart + (j_final_start + 1) * nx, NO_MORE_STARTS);
        }
    }

    /// The z-level of the value at `point`.
    fn z_to_level(&self, point: isize) -> u32 {
        self.levels.z_level(self.field.z_at(point))
    }

    /// Computes and records the z-level of the middle of `quad`.
    fn middle_level(&mut self, quad: isize) -> u32 {
        let level = self.levels.z_level(self.field.middle_z(quad));
        self.cache.set_middle_z_level(quad, level);
        level
    }

    /// The starts of the existing quad (or corner) `quad` in column `i`,
    /// row `j`, given the z-levels of its NW, NE, SW and SE points.
    fn quad_starts(&mut self, quad: isize, i: isize, j: isize, z: [u32; 4]) -> u32 {
        let (x_chunk, y_chunk) = self.chunk_sizes();
        let n = self.cache.boundary_n(quad);
        let edges = Edges {
            n: flag_if(n, START_BOUNDARY_N),
            e: flag_if(self.cache.boundary_e(quad), START_BOUNDARY_E),
            s: flag_if(self.cache.boundary_s(quad), START_BOUNDARY_S),
            w: flag_if(self.cache.boundary_w(quad), START_BOUNDARY_W),
            hole: flag_if(
                n && !self.cache.has(quad - 1, START_HOLE_N)
                    && j % y_chunk > 0
                    && j != self.ny() - 1
                    && i % x_chunk > 1,
                START_HOLE_N,
            ),
        };
        let [nw, ne, sw, se] = z;
        let filled = self.levels.filled;
        let bits = if filled { 2 } else { 1 };
        match (self.cache.corner(quad), filled) {
            (None, true) => self.filled_quad(quad, config(&z, bits), edges),
            (None, false) => self.line_quad(quad, config(&z, bits), edges),
            (Some(Corner::Nw), true) => filled_nw(config(&[nw, ne, sw], bits), edges),
            (Some(Corner::Nw), false) => line_nw(config(&[nw, ne, sw], bits), edges),
            (Some(Corner::Ne), true) => filled_ne(config(&[nw, ne, se], bits), edges),
            (Some(Corner::Ne), false) => line_ne(config(&[nw, ne, se], bits), edges),
            (Some(Corner::Sw), true) => filled_sw(config(&[nw, sw, se], bits), edges),
            (Some(Corner::Sw), false) => line_sw(config(&[nw, sw, se], bits), edges),
            (Some(Corner::Se), true) => filled_se(config(&[ne, sw, se], bits), edges),
            (Some(Corner::Se), false) => line_se(config(&[ne, sw, se], bits), edges),
        }
    }

    /// The starts of a whole quad for filled contours.
    fn filled_quad(&mut self, quad: isize, config: u32, edges: Edges) -> u32 {
        let Edges { s, w, hole: h, .. } = edges;
        match config {
            1 | 2 | 17 | 18 | 34 | 68 | 102 | 136 | 152 | 153 | 168 | 169 => s,
            4 | 5 | 6 | 8 | 9 | 21 | 22 | 25 | 38 | 72 | 98 | 132 | 145 | 148 | 149 | 161 | 162
            | 164 | 165 | 166 => s | w,
            10 | 26 | 42 | 64 | 106 | 128 | 144 | 160 => w,
            16 | 154 => START_N,
            32 | 138 => START_E | START_N,
            33 | 69 | 70 | 100 | 101 | 137 => s | START_E,
            37 | 73 | 97 | 133 => s | w | START_E,
            74 | 96 => w | START_E,
            80 | 90 => w | h,
            81 | 82 | 88 | 89 => s | w | h,
            84..=86 => s | h,
            // Saddles.
            20 | 24 | 36 | 40 | 41 | 65 | 66 | 104 | 105 | 129 | 130 | 134 | 146 | 150 => {
                let m = self.middle_level(quad);
                s | w
                    | match config {
                        20 | 24 => flag_if(m == 0, START_N),
                        36 => flag_if(m == 0, START_N) | START_E,
                        40 => flag_if(m < 2, START_E) | flag_if(m == 0, START_N),
                        41 | 104 | 105 => flag_if(m < 2, START_E),
                        65 | 66 | 129 => flag_if(m > 0, START_E),
                        130 => flag_if(m > 0, START_E) | flag_if(m == 2, START_N),
                        134 => flag_if(m == 2, START_N) | START_E,
                        _ => flag_if(m == 2, START_N),
                    }
            }
            _ => 0,
        }
    }

    /// The starts of a whole quad for lines.
    fn line_quad(&mut self, quad: isize, config: u32, edges: Edges) -> u32 {
        match config {
            1 | 3 => edges.e,
            2 | 10 | 14 => edges.s,
            4 => edges.line_n(true),
            5 | 7 => edges.n,
            // Saddles.
            6 => {
                let m = self.middle_level(quad);
                edges.line_n(m == 0) | edges.s
            }
            9 => {
                let m = self.middle_level(quad);
                edges.line_e(m == 1) | edges.w
            }
            8 | 12 | 13 => edges.w,
            11 => edges.line_e(true),
            _ => 0,
        }
    }
}

/// The starts of a NW corner for filled contours (`config`: NW, NE, SW).
fn filled_nw(config: u32, edges: Edges) -> u32 {
    let Edges { w, hole: h, .. } = edges;
    match config {
        1 | 5 | 9 | 10 | 16 | 17 | 25 | 26 | 32 | 33 | 37 | 41 => w,
        2 | 6 | 18 | 24 | 36 | 40 => w | START_CORNER,
        4 | 8 | 34 | 38 => START_CORNER,
        20 | 22 => w | h | START_CORNER,
        21 => w | h,
        _ => 0,
    }
}

/// The starts of a NW corner for lines (`config`: NW, NE, SW).
fn line_nw(config: u32, edges: Edges) -> u32 {
    match config {
        1 | 5 => START_CORNER,
        2 | 3 => edges.n,
        4 | 6 => edges.w,
        _ => 0,
    }
}

/// The starts of a NE corner for filled contours (`config`: NW, NE, SE).
fn filled_ne(config: u32, edges: Edges) -> u32 {
    match config {
        1 | 2 | 5 | 6 | 10 | 16 | 26 | 32 | 36 | 37 | 40 | 41 => START_CORNER,
        4 | 38 => START_N,
        8 | 34 => START_E | START_N,
        9 | 17 | 18 | 24 | 25 | 33 => START_CORNER | START_E,
        20..=22 => edges.hole | START_CORNER,
        _ => 0,
    }
}

/// The starts of a NE corner for lines (`config`: NW, NE, SE).
fn line_ne(config: u32, edges: Edges) -> u32 {
    match config {
        1 => edges.e,
        2 => edges.line_n(true),
        3 => edges.n,
        4 | 6 => START_CORNER,
        5 => edges.line_e(true),
        _ => 0,
    }
}

/// The starts of a SW corner for filled contours (`config`: NW, SW, SE).
fn filled_sw(config: u32, edges: Edges) -> u32 {
    let Edges { s, w, .. } = edges;
    match config {
        1 | 2 | 40 | 41 => s,
        4 | 5 | 6 | 8 | 9 | 18 | 24 | 33 | 34 | 36 | 37 | 38 => s | w,
        10 | 16 | 26 | 32 => w,
        17 | 25 => s | w | START_CORNER,
        20..=22 => {
            if s != 0 {
                s
            } else {
                START_CORNER
            }
        }
        _ => 0,
    }
}

/// The starts of a SW corner for lines (`config`: NW, SW, SE).
fn line_sw(config: u32, edges: Edges) -> u32 {
    match config {
        1 | 3 => START_CORNER,
        2 | 6 => edges.s,
        4 | 5 => edges.w,
        _ => 0,
    }
}

/// The starts of a SE corner for filled contours (`config`: NE, SW, SE).
fn filled_se(config: u32, edges: Edges) -> u32 {
    match config {
        10 | 16 | 26 | 32 => START_CORNER,
        1 | 2 | 4 | 5 | 6 | 8 | 9 | 17 | 18 | 20 | 21 | 22 | 24 | 25 | 33 | 34 | 36 | 37 | 38
        | 40 | 41 => edges.s,
        _ => 0,
    }
}

/// The starts of a SE corner for lines (`config`: NE, SW, SE).
fn line_se(config: u32, edges: Edges) -> u32 {
    match config {
        1 | 3 => edges.e,
        2 | 6 => edges.s,
        4 | 5 => START_CORNER,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configurations_pack_levels_first_highest() {
        assert_eq!(config(&[2, 1, 0, 2], 2), 0b10_01_00_10);
        assert_eq!(config(&[1, 0, 1], 1), 0b101);
    }

    #[test]
    fn line_starts_prefer_boundaries() {
        let edges = Edges {
            n: 0,
            e: START_BOUNDARY_E,
            s: 0,
            w: 0,
            hole: 0,
        };
        assert_eq!(
            edges.line_n(true),
            0,
            "the line leaves through the E boundary"
        );
        assert_eq!(edges.line_e(false), START_BOUNDARY_E);
        let interior = Edges { e: 0, ..edges };
        assert_eq!(interior.line_n(true), START_N);
        assert_eq!(interior.line_n(false), 0);
    }
}
