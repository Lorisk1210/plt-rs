//! The serial cache: one bit field per quad holding the grid topology
//! (which quads and corners exist, which edges are boundaries) and the
//! state of the current contouring call.
//!
//! A quad is indexed by its NE point, so quad `i + j·nx` spans points
//! `i - 1..=i` × `j - 1..=j`; the entry of a quad also holds the z-level
//! of its NE point. Quads in column 0 and row 0 never exist. Indices are
//! signed because walking a contour adds signed offsets to them.

use crate::contour::quad::{signed, slot};

/// z above the lower level.
const Z_LEVEL_1: u32 = 1;
/// z above the upper level.
const Z_LEVEL_2: u32 = 1 << 1;
/// Both z-level bits.
const Z_LEVEL: u32 = Z_LEVEL_1 | Z_LEVEL_2;
/// The z-level of the quad's middle (two bits).
const MIDDLE: u32 = 0b11 << 2;
/// The E edge of the quad is a boundary.
const BOUNDARY_E: u32 = 1 << 4;
/// The N edge of the quad is a boundary.
const BOUNDARY_N: u32 = 1 << 5;
/// The whole quad exists.
const EXISTS_QUAD: u32 = 1 << 6;
/// Only the NE corner exists (the SW point is masked).
const EXISTS_NE_CORNER: u32 = 1 << 7;
/// Only the NW corner exists.
const EXISTS_NW_CORNER: u32 = 1 << 8;
/// Only the SE corner exists.
const EXISTS_SE_CORNER: u32 = 1 << 9;
/// Only the SW corner exists.
const EXISTS_SW_CORNER: u32 = 1 << 10;
/// Any of the corners.
const EXISTS_ANY_CORNER: u32 =
    EXISTS_NE_CORNER | EXISTS_NW_CORNER | EXISTS_SE_CORNER | EXISTS_SW_CORNER;
/// The quad or one of its corners.
const EXISTS_ANY: u32 = EXISTS_QUAD | EXISTS_ANY_CORNER;
/// An interior start from the E edge to the N (filled and lines).
pub(super) const START_E: u32 = 1 << 11;
/// An interior start from the N edge to the E (filled and lines).
pub(super) const START_N: u32 = 1 << 12;
/// A start on the E boundary (lines only).
pub(super) const START_BOUNDARY_E: u32 = 1 << 13;
/// A start on the N boundary (lines only).
pub(super) const START_BOUNDARY_N: u32 = 1 << 14;
/// A start on the S boundary.
pub(super) const START_BOUNDARY_S: u32 = 1 << 15;
/// A start on the W boundary.
pub(super) const START_BOUNDARY_W: u32 = 1 << 16;
/// A hole starting on the N boundary, E to W (filled only).
pub(super) const START_HOLE_N: u32 = 1 << 17;
/// A start on the diagonal of a corner.
pub(super) const START_CORNER: u32 = 1 << 18;
/// Every start.
const ANY_START: u32 = START_E
    | START_N
    | START_BOUNDARY_E
    | START_BOUNDARY_N
    | START_BOUNDARY_S
    | START_BOUNDARY_W
    | START_HOLE_N
    | START_CORNER;
/// Look N from here for the start of a hole (filled only).
pub(super) const LOOK_N: u32 = 1 << 19;
/// The hole start a `LOOK_N` below leads to.
pub(super) const LOOK_S: u32 = 1 << 20;
/// The row of this chunk has no starts (set on its first quad).
pub(super) const NO_STARTS_IN_ROW: u32 = 1 << 21;
/// Neither this row nor any above it in the chunk has starts.
pub(super) const NO_MORE_STARTS: u32 = 1 << 22;
/// What a new contouring call keeps: the grid topology.
const KEEP: u32 = EXISTS_ANY | BOUNDARY_N | BOUNDARY_E;

/// The corner of a quad left when the opposite point is masked (with
/// corner masking).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Corner {
    /// The triangle at the NE point.
    Ne,
    /// The triangle at the NW point.
    Nw,
    /// The triangle at the SE point.
    Se,
    /// The triangle at the SW point.
    Sw,
}

/// The cache entries of every quad, row-major with `nx` per row.
#[derive(Clone, Debug)]
pub(super) struct Cache {
    items: Vec<u32>,
    nx: isize,
}

impl Cache {
    /// The topology of an `nx × ny` grid with an optional point mask, in
    /// chunks of `(x, y)` quads (contourpy's `init_cache_grid`).
    pub(super) fn new(
        nx: usize,
        ny: usize,
        mask: Option<&[bool]>,
        corner_mask: bool,
        chunk: (usize, usize),
    ) -> Self {
        let mut cache = Self {
            items: vec![0; nx * ny],
            nx: signed(nx),
        };
        match mask {
            None => cache.init_unmasked(nx, ny, chunk),
            Some(mask) => {
                cache.init_existence(nx, ny, mask, corner_mask);
                cache.init_boundaries(nx, ny, corner_mask, chunk);
            }
        }
        cache
    }

    fn init_unmasked(&mut self, nx: usize, ny: usize, (x_chunk, y_chunk): (usize, usize)) {
        for j in 0..ny {
            for i in 0..nx {
                let mut item = 0;
                if i > 0 && j > 0 {
                    item |= EXISTS_QUAD;
                }
                if (i % x_chunk == 0 || i == nx - 1) && j > 0 {
                    item |= BOUNDARY_E;
                }
                if (j % y_chunk == 0 || j == ny - 1) && i > 0 {
                    item |= BOUNDARY_N;
                }
                self.items[i + j * nx] = item;
            }
        }
    }

    /// Stage 1 with a mask: which quads or corners exist.
    fn init_existence(&mut self, nx: usize, ny: usize, mask: &[bool], corner_mask: bool) {
        for j in 1..ny {
            for i in 1..nx {
                let quad = i + j * nx;
                let config = (u8::from(mask[quad - 1]) << 3)
                    | (u8::from(mask[quad]) << 2)
                    | (u8::from(mask[quad - nx - 1]) << 1)
                    | u8::from(mask[quad - nx]);
                self.items[quad] = match (corner_mask, config) {
                    (_, 0) => EXISTS_QUAD,
                    (true, 1) => EXISTS_NW_CORNER,
                    (true, 2) => EXISTS_NE_CORNER,
                    (true, 4) => EXISTS_SW_CORNER,
                    (true, 8) => EXISTS_SE_CORNER,
                    _ => 0,
                };
            }
        }
    }

    /// Stage 2 with a mask: an edge is a boundary where it exists on one
    /// side only, or on a chunk edge where it exists on both.
    fn init_boundaries(
        &mut self,
        nx: usize,
        ny: usize,
        corner_mask: bool,
        (x_chunk, y_chunk): (usize, usize),
    ) {
        let edge = |item: u32, corners: u32| {
            let mask = if corner_mask {
                EXISTS_QUAD | corners
            } else {
                EXISTS_QUAD
            };
            item & mask != 0
        };
        for j in 0..ny {
            for i in 0..nx {
                let quad = i + j * nx;
                let item = self.items[quad];
                let exists_e = edge(item, EXISTS_NE_CORNER | EXISTS_SE_CORNER);
                let east_exists_w =
                    i < nx - 1 && edge(self.items[quad + 1], EXISTS_NW_CORNER | EXISTS_SW_CORNER);
                let exists_n = edge(item, EXISTS_NW_CORNER | EXISTS_NE_CORNER);
                let north_exists_s =
                    j < ny - 1 && edge(self.items[quad + nx], EXISTS_SW_CORNER | EXISTS_SE_CORNER);
                if exists_e != east_exists_w || (i % x_chunk == 0 && exists_e && east_exists_w) {
                    self.items[quad] |= BOUNDARY_E;
                }
                if exists_n != north_exists_s || (j % y_chunk == 0 && exists_n && north_exists_s) {
                    self.items[quad] |= BOUNDARY_N;
                }
            }
        }
    }

    fn item(&self, quad: isize) -> u32 {
        self.items[slot(quad)]
    }

    /// Whether any of the `flags` are set on `quad`.
    pub(super) fn has(&self, quad: isize, flags: u32) -> bool {
        self.item(quad) & flags != 0
    }

    /// Sets `flags` on `quad`.
    pub(super) fn set(&mut self, quad: isize, flags: u32) {
        self.items[slot(quad)] |= flags;
    }

    /// Clears `flags` on `quad`.
    pub(super) fn clear(&mut self, quad: isize, flags: u32) {
        self.items[slot(quad)] &= !flags;
    }

    /// Starts a contouring call at `quad`: forgets everything but the
    /// topology, and records the z-level of its NE point.
    pub(super) fn reset(&mut self, quad: isize, z_level: u32) {
        let item = &mut self.items[slot(quad)];
        *item = (*item & KEEP) | z_level;
    }

    /// The z-level of a point: 0 at or below the lower level, 1 between
    /// the levels, 2 above the upper level.
    pub(super) fn z_level(&self, point: isize) -> u32 {
        self.item(point) & Z_LEVEL
    }

    /// The z-level of the NE point of `quad`.
    pub(super) fn z_ne(&self, quad: isize) -> u32 {
        self.z_level(quad)
    }

    /// The z-level of the NW point of `quad`.
    pub(super) fn z_nw(&self, quad: isize) -> u32 {
        self.z_level(quad - 1)
    }

    /// The z-level of the SE point of `quad`.
    pub(super) fn z_se(&self, quad: isize) -> u32 {
        self.z_level(quad - self.nx)
    }

    /// The z-level of the SW point of `quad`.
    pub(super) fn z_sw(&self, quad: isize) -> u32 {
        self.z_level(quad - self.nx - 1)
    }

    /// The z-level of the middle of a saddle quad.
    pub(super) fn middle_z_level(&self, quad: isize) -> u32 {
        (self.item(quad) & MIDDLE) >> 2
    }

    /// Records the z-level of the middle of a saddle quad.
    pub(super) fn set_middle_z_level(&mut self, quad: isize, z_level: u32) {
        self.set(quad, z_level << 2);
    }

    /// Whether the E edge of `quad` is a boundary.
    pub(super) fn boundary_e(&self, quad: isize) -> bool {
        self.has(quad, BOUNDARY_E)
    }

    /// Whether the N edge of `quad` is a boundary.
    pub(super) fn boundary_n(&self, quad: isize) -> bool {
        self.has(quad, BOUNDARY_N)
    }

    /// Whether the S edge of `quad` is a boundary.
    pub(super) fn boundary_s(&self, quad: isize) -> bool {
        self.has(quad - self.nx, BOUNDARY_N)
    }

    /// Whether the W edge of `quad` is a boundary.
    pub(super) fn boundary_w(&self, quad: isize) -> bool {
        self.has(quad - 1, BOUNDARY_E)
    }

    /// Whether the whole quad exists.
    pub(super) fn exists_quad(&self, quad: isize) -> bool {
        self.has(quad, EXISTS_QUAD)
    }

    /// Whether the quad or one of its corners exists.
    pub(super) fn exists_any(&self, quad: isize) -> bool {
        self.has(quad, EXISTS_ANY)
    }

    /// The corner of `quad` that exists, if only a corner does.
    pub(super) fn corner(&self, quad: isize) -> Option<Corner> {
        match self.item(quad) & EXISTS_ANY_CORNER {
            EXISTS_NE_CORNER => Some(Corner::Ne),
            EXISTS_NW_CORNER => Some(Corner::Nw),
            EXISTS_SE_CORNER => Some(Corner::Se),
            EXISTS_SW_CORNER => Some(Corner::Sw),
            _ => None,
        }
    }

    /// Whether a contour starts at `quad`.
    pub(super) fn any_start(&self, quad: isize) -> bool {
        self.has(quad, ANY_START)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmasked_grids_have_boundaries_on_their_edges_and_chunk_edges() {
        let cache = Cache::new(4, 3, None, true, (2, 2));
        let quad = |i: isize, j: isize| i + 4 * j;
        assert!(!cache.exists_any(quad(0, 1)) && !cache.exists_any(quad(1, 0)));
        assert!(cache.exists_quad(quad(1, 1)));
        assert!(cache.boundary_w(quad(1, 1)) && cache.boundary_s(quad(1, 1)));
        assert!(cache.boundary_e(quad(2, 1)), "chunk edge");
        assert!(!cache.boundary_e(quad(1, 1)));
        assert!(cache.boundary_e(quad(3, 1)) && cache.boundary_n(quad(3, 2)));
    }

    #[test]
    fn a_masked_point_leaves_the_opposite_corners() {
        // The centre of a 3 × 3 grid is masked.
        let mut mask = [false; 9];
        mask[4] = true;
        let corners = Cache::new(3, 3, Some(&mask), true, (2, 2));
        assert_eq!(corners.corner(4), Some(Corner::Sw));
        assert_eq!(corners.corner(5), Some(Corner::Se));
        assert_eq!(corners.corner(7), Some(Corner::Nw));
        assert_eq!(corners.corner(8), Some(Corner::Ne));
        let whole = Cache::new(3, 3, Some(&mask), false, (2, 2));
        assert!((4..9).all(|quad| !whole.exists_any(quad)));
    }

    #[test]
    fn a_new_call_keeps_only_the_topology() {
        let mut cache = Cache::new(3, 3, None, true, (2, 2));
        cache.set(4, START_E | LOOK_N);
        cache.set_middle_z_level(4, 2);
        cache.reset(4, 1);
        assert!(!cache.any_start(4) && !cache.has(4, LOOK_N));
        assert_eq!((cache.middle_z_level(4), cache.z_ne(4)), (0, 1));
        assert!(cache.exists_quad(4));
    }
}
