//! The mpl2014 cache: one bit field per grid point holding the grid
//! topology (which quads and corners exist, which edges are boundaries)
//! and the state of the current contouring pass.
//!
//! Each quad's entry also describes the point at its SW corner and its W
//! and S edges, so every point and edge is stored exactly once.

/// z above the lower level (filled) or above the level (lines).
const Z_LEVEL_1: u32 = 0x0001;
/// z above the upper level.
const Z_LEVEL_2: u32 = 0x0002;
/// Both z-level bits.
const Z_LEVEL: u32 = Z_LEVEL_1 | Z_LEVEL_2;
/// The S edge of the quad is a boundary.
pub(super) const BOUNDARY_S: u32 = 0x0400;
/// The W edge of the quad is a boundary.
pub(super) const BOUNDARY_W: u32 = 0x0800;
/// The whole quad exists.
const EXISTS_QUAD: u32 = 0x1000;
/// Only the SW corner exists (the NE point is masked).
const EXISTS_SW_CORNER: u32 = 0x2000;
/// Only the SE corner exists.
const EXISTS_SE_CORNER: u32 = 0x3000;
/// Only the NW corner exists.
const EXISTS_NW_CORNER: u32 = 0x4000;
/// Only the NE corner exists.
const EXISTS_NE_CORNER: u32 = 0x5000;
/// All existence bits (the values above overlap: test with equality).
const EXISTS: u32 = 0x7000;
/// A filled contour has followed the S boundary edge.
pub(super) const VISITED_S: u32 = 0x10000;
/// A filled contour has followed the W boundary edge.
pub(super) const VISITED_W: u32 = 0x20000;
/// A filled contour has followed the diagonal edge of a corner.
pub(super) const VISITED_CORNER: u32 = 0x40000;

/// The lower or upper of the two levels of a pass (a line contour uses
/// only the lower one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LevelIndex {
    /// The lower level; higher values lie left of the contour.
    Lower,
    /// The upper level of a filled contour; higher values lie right.
    Upper,
}

impl LevelIndex {
    /// The z-level value this level index is compared against.
    pub(super) const fn z(self) -> u32 {
        match self {
            Self::Lower => Z_LEVEL_1,
            Self::Upper => Z_LEVEL_2,
        }
    }

    /// The quad has been traversed on this level.
    pub(super) const fn visited(self) -> u32 {
        match self {
            Self::Lower => 0x0004,
            Self::Upper => 0x0008,
        }
    }

    /// The quad is a saddle on this level.
    pub(super) const fn saddle(self) -> u32 {
        match self {
            Self::Lower => 0x0010,
            Self::Upper => 0x0020,
        }
    }

    /// Contours turn left through the saddle.
    pub(super) const fn saddle_left(self) -> u32 {
        match self {
            Self::Lower => 0x0040,
            Self::Upper => 0x0080,
        }
    }

    /// The second visit of the saddle starts on its S or W edge.
    pub(super) const fn saddle_start_sw(self) -> u32 {
        match self {
            Self::Lower => 0x0100,
            Self::Upper => 0x0200,
        }
    }
}

/// One of the four triangular halves of a quad left when the opposite
/// point is masked (with `corner_mask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Corner {
    /// The triangle at the SW point.
    Sw,
    /// The triangle at the SE point.
    Se,
    /// The triangle at the NW point.
    Nw,
    /// The triangle at the NE point.
    Ne,
}

/// How much of a quad takes part in contouring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Exists {
    /// Nothing: the quad is masked (or lies past the last row/column).
    None,
    /// The whole quad.
    Quad,
    /// One triangular corner.
    Corner(Corner),
}

/// The cache entries of every grid point, row-major with `nx` per row.
#[derive(Clone, Debug)]
pub(super) struct Cache {
    items: Vec<u32>,
    nx: usize,
}

impl Cache {
    /// The topology of an `nx × ny` grid with optional point mask, split
    /// into chunks of `chunk` quads (matplotlib's `init_cache_grid`).
    pub(super) fn new(
        nx: usize,
        ny: usize,
        mask: Option<&[bool]>,
        corner_mask: bool,
        chunk: (usize, usize),
    ) -> Self {
        let mut cache = Self {
            items: vec![0; nx * ny],
            nx,
        };
        match mask {
            None => cache.init_unmasked(ny, chunk),
            Some(mask) => {
                cache.init_existence(ny, mask, corner_mask);
                cache.init_boundaries(ny, corner_mask, chunk);
            }
        }
        cache
    }

    fn init_unmasked(&mut self, ny: usize, (x_chunk, y_chunk): (usize, usize)) {
        let nx = self.nx;
        for j in 0..ny {
            for i in 0..nx {
                let mut item = 0;
                if i < nx - 1 && j < ny - 1 {
                    item |= EXISTS_QUAD;
                }
                if (i % x_chunk == 0 || i == nx - 1) && j < ny - 1 {
                    item |= BOUNDARY_W;
                }
                if (j % y_chunk == 0 || j == ny - 1) && i < nx - 1 {
                    item |= BOUNDARY_S;
                }
                self.items[i + j * nx] = item;
            }
        }
    }

    /// Stage 1 with a mask: which quads or corners exist.
    fn init_existence(&mut self, ny: usize, mask: &[bool], corner_mask: bool) {
        let nx = self.nx;
        for j in 0..ny - 1 {
            for i in 0..nx - 1 {
                let quad = i + j * nx;
                let config = (u8::from(mask[quad + nx]) << 3)
                    | (u8::from(mask[quad + nx + 1]) << 2)
                    | (u8::from(mask[quad]) << 1)
                    | u8::from(mask[quad + 1]);
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

    /// Stage 2 with a mask: W and S boundaries, using the already known
    /// boundaries of the quads to the W and S.
    fn init_boundaries(
        &mut self,
        ny: usize,
        corner_mask: bool,
        (x_chunk, y_chunk): (usize, usize),
    ) {
        let nx = self.nx;
        for j in 0..ny {
            for i in 0..nx {
                let quad = i + j * nx;
                let (boundary_w, boundary_s) = if corner_mask {
                    let w_none = i == 0 || self.exists(quad - 1) == Exists::None;
                    let s_none = j == 0 || self.exists(quad - nx) == Exists::None;
                    let w_has_e_edge = i > 0 && self.exists_e_edge(quad - 1);
                    let s_has_n_edge = j > 0 && self.exists_n_edge(quad - nx);
                    let none = self.exists(quad) == Exists::None;
                    let w_edge = self.exists_w_edge(quad);
                    let s_edge = self.exists_s_edge(quad);
                    (
                        (w_edge && w_none)
                            || (none && w_has_e_edge)
                            || (i % x_chunk == 0 && w_edge && w_has_e_edge),
                        (s_edge && s_none)
                            || (none && s_has_n_edge)
                            || (j % y_chunk == 0 && s_edge && s_has_n_edge),
                    )
                } else {
                    let here = self.exists_quad(quad);
                    let w_quad = i > 0 && self.exists_quad(quad - 1);
                    let s_quad = j > 0 && self.exists_quad(quad - nx);
                    (
                        here != w_quad || (i % x_chunk == 0 && here && w_quad),
                        here != s_quad || (j % y_chunk == 0 && here && s_quad),
                    )
                };
                if boundary_w {
                    self.items[quad] |= BOUNDARY_W;
                }
                if boundary_s {
                    self.items[quad] |= BOUNDARY_S;
                }
            }
        }
    }

    /// Resets the per-pass state and classifies every point against the
    /// levels (matplotlib's `init_cache_levels`); `lower == upper` for
    /// line contours.
    #[expect(
        clippy::float_cmp,
        reason = "a single level is passed as two identical bounds"
    )]
    pub(super) fn init_levels(&mut self, z: &[f64], lower: f64, upper: f64) {
        let keep = EXISTS | BOUNDARY_S | BOUNDARY_W;
        let two_levels = lower != upper;
        for (item, &z) in self.items.iter_mut().zip(z) {
            *item &= keep;
            if two_levels && z > upper {
                *item |= Z_LEVEL_2;
            } else if z > lower {
                *item |= Z_LEVEL_1;
            }
        }
    }

    /// Whether any of `flags` is set for `quad`.
    pub(super) fn has(&self, quad: usize, flags: u32) -> bool {
        self.items[quad] & flags != 0
    }

    /// Sets `flags` for `quad`.
    pub(super) fn set(&mut self, quad: usize, flags: u32) {
        self.items[quad] |= flags;
    }

    /// Clears `flags` for `quad`.
    pub(super) fn clear(&mut self, quad: usize, flags: u32) {
        self.items[quad] &= !flags;
    }

    /// 0 (at or below the lower level), 1 (between) or 2 (above the upper
    /// level) for a point.
    pub(super) fn z_level(&self, point: usize) -> u32 {
        self.items[point] & Z_LEVEL
    }

    /// Which part of `quad` exists.
    pub(super) fn exists(&self, quad: usize) -> Exists {
        match self.items[quad] & EXISTS {
            EXISTS_QUAD => Exists::Quad,
            EXISTS_SW_CORNER => Exists::Corner(Corner::Sw),
            EXISTS_SE_CORNER => Exists::Corner(Corner::Se),
            EXISTS_NW_CORNER => Exists::Corner(Corner::Nw),
            EXISTS_NE_CORNER => Exists::Corner(Corner::Ne),
            _ => Exists::None,
        }
    }

    /// Whether the whole quad exists.
    pub(super) fn exists_quad(&self, quad: usize) -> bool {
        self.exists(quad) == Exists::Quad
    }

    /// Whether exactly this corner of the quad exists.
    pub(super) fn exists_corner(&self, quad: usize, corner: Corner) -> bool {
        self.exists(quad) == Exists::Corner(corner)
    }

    /// Whether the quad's W edge exists.
    pub(super) fn exists_w_edge(&self, quad: usize) -> bool {
        matches!(
            self.exists(quad),
            Exists::Quad | Exists::Corner(Corner::Sw | Corner::Nw)
        )
    }

    /// Whether the quad's E edge exists.
    pub(super) fn exists_e_edge(&self, quad: usize) -> bool {
        matches!(
            self.exists(quad),
            Exists::Quad | Exists::Corner(Corner::Se | Corner::Ne)
        )
    }

    /// Whether the quad's S edge exists.
    pub(super) fn exists_s_edge(&self, quad: usize) -> bool {
        matches!(
            self.exists(quad),
            Exists::Quad | Exists::Corner(Corner::Sw | Corner::Se)
        )
    }

    /// Whether the quad's N edge exists.
    pub(super) fn exists_n_edge(&self, quad: usize) -> bool {
        matches!(
            self.exists(quad),
            Exists::Quad | Exists::Corner(Corner::Nw | Corner::Ne)
        )
    }

    /// Whether the quad's S edge is a boundary.
    pub(super) fn boundary_s(&self, quad: usize) -> bool {
        self.has(quad, BOUNDARY_S)
    }

    /// Whether the quad's W edge is a boundary.
    pub(super) fn boundary_w(&self, quad: usize) -> bool {
        self.has(quad, BOUNDARY_W)
    }

    /// Whether the quad's N edge is a boundary.
    pub(super) fn boundary_n(&self, quad: usize) -> bool {
        self.boundary_s(quad + self.nx)
    }

    /// Whether the quad's E edge is a boundary.
    pub(super) fn boundary_e(&self, quad: usize) -> bool {
        self.boundary_w(quad + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmasked_grid_has_quads_and_outer_boundaries() {
        let cache = Cache::new(3, 3, None, true, (2, 2));
        assert!(cache.exists_quad(0) && cache.exists_quad(4));
        assert_eq!(cache.exists(2), Exists::None, "last column has no quad");
        assert!(cache.boundary_w(0) && cache.boundary_w(2) && !cache.boundary_w(1));
        assert!(cache.boundary_s(0) && cache.boundary_s(6) && !cache.boundary_s(3));
    }

    #[test]
    fn masked_point_leaves_corners() {
        let mask = [false, false, false, false, true, false, false, false, false];
        let corners = Cache::new(3, 3, Some(&mask), true, (2, 2));
        assert!(corners.exists_corner(0, Corner::Sw));
        assert!(corners.exists_corner(1, Corner::Se));
        assert!(corners.exists_corner(3, Corner::Nw));
        assert!(corners.exists_corner(4, Corner::Ne));
        let plain = Cache::new(3, 3, Some(&mask), false, (2, 2));
        assert!((0..9).all(|q| plain.exists(q) == Exists::None));
    }

    #[test]
    fn levels_classify_points() {
        let mut cache = Cache::new(2, 2, None, true, (1, 1));
        cache.init_levels(&[0.0, 1.0, 2.0, f64::NAN], 0.5, 1.5);
        let levels: Vec<u32> = (0..4).map(|p| cache.z_level(p)).collect();
        assert_eq!(levels, [0, 1, 2, 0]);
        cache.init_levels(&[0.0, 1.0, 2.0, 0.5], 0.5, 0.5);
        let levels: Vec<u32> = (0..4).map(|p| cache.z_level(p)).collect();
        assert_eq!(levels, [0, 1, 1, 0]);
    }
}
