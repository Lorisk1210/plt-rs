//! A port of contourpy's `serial` algorithm (also what its `threaded`
//! algorithm computes, chunk by chunk).
//!
//! One pass over the grid classifies every point by z-level and marks the
//! quads where contours start: on a boundary, or in the interior for
//! closed loops (and, for filled contours, holes). Each chunk is then
//! swept for starts and every contour traced from its start, turning
//! through saddles by the z-level of their middle. Filled contours come
//! out as closed rings, every outer polygon followed by its holes.
//!
//! The arithmetic and the traversal order are those of contourpy, so the
//! vertices are equal to the last bit and come in the same order.

mod cache;
mod march;
mod starts;
mod trace;

use pltrs_core::{Path, Point};

use self::cache::Cache;
use self::march::ChunkMarch;
use self::trace::{Pass, Tracer};
use super::assemble::LevelPath;
use super::chunks::Chunks;
use super::quad::{QuadGrid, signed, slot};

/// contourpy's `SerialContourGenerator` with matplotlib's line and fill
/// types.
#[derive(Clone, Debug)]
pub(super) struct Serial {
    field: Field,
    ny: isize,
    corner_mask: bool,
    chunks: Chunks,
    cache: Cache,
    levels: Levels,
}

/// The coordinates and values of the grid points, row-major.
#[derive(Clone, Debug)]
pub(super) struct Field {
    x: Vec<f64>,
    y: Vec<f64>,
    z: Vec<f64>,
    /// Points per row.
    nx: isize,
}

impl Field {
    /// The position of a point.
    fn point(&self, point: isize) -> Point {
        let i = slot(point);
        Point::new(self.x[i], self.y[i])
    }

    /// The value at a point.
    fn z_at(&self, point: isize) -> f64 {
        self.z[slot(point)]
    }

    /// The mean of the values at the corners of a quad.
    fn middle_z(&self, quad: isize) -> f64 {
        let nx = self.nx;
        0.25 * (self.z_at(quad - nx - 1)
            + self.z_at(quad - nx)
            + self.z_at(quad - 1)
            + self.z_at(quad))
    }
}

/// The levels of the current contouring call.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Levels {
    /// Filled contours between `lower` and `upper`, or lines at `lower`.
    filled: bool,
    lower: f64,
    upper: f64,
}

impl Levels {
    /// The z-level of a value: 0 at or below the lower level (or NaN), 1
    /// above it, 2 above the upper level of filled contours.
    fn z_level(self, z: f64) -> u32 {
        if self.filled && z > self.upper {
            2
        } else {
            u32::from(z > self.lower)
        }
    }
}

/// The quads of a chunk, by NE point: columns `istart..=iend`, rows
/// `jstart..=jend` (contourpy's chunk limits).
#[derive(Clone, Copy, Debug)]
pub(super) struct QuadRange {
    istart: isize,
    iend: isize,
    jstart: isize,
    jend: isize,
}

impl Serial {
    /// A generator for `grid`, in `chunks`.
    pub(super) fn new(grid: QuadGrid<'_>, corner_mask: bool, chunks: Chunks) -> Self {
        Self {
            cache: Cache::new(
                grid.nx,
                grid.ny,
                grid.mask,
                corner_mask,
                (chunks.x_size, chunks.y_size),
            ),
            field: Field {
                x: grid.x,
                y: grid.y,
                z: grid.z,
                nx: signed(grid.nx),
            },
            ny: signed(grid.ny),
            corner_mask,
            chunks,
            levels: Levels::default(),
        }
    }

    /// The lines at `level`: each starts with a MOVETO and ends with a
    /// CLOSEPOLY when its last point is exactly its first.
    pub(super) fn lines(&mut self, level: f64) -> Path {
        self.march(Levels {
            filled: false,
            lower: level,
            upper: level,
        })
        .finish()
    }

    /// The region `lower < z <= upper` as closed rings (`lower < upper`).
    pub(super) fn filled(&mut self, lower: f64, upper: f64) -> Path {
        self.march(Levels {
            filled: true,
            lower,
            upper,
        })
        .finish()
    }

    fn nx(&self) -> isize {
        self.field.nx
    }

    fn ny(&self) -> isize {
        self.ny
    }

    /// Quads per chunk in x and y.
    fn chunk_sizes(&self) -> (isize, isize) {
        (signed(self.chunks.x_size), signed(self.chunks.y_size))
    }

    /// Classifies the grid for `levels` and traces the chunks in order.
    fn march(&mut self, levels: Levels) -> LevelPath {
        self.levels = levels;
        let (nx, ny) = (self.nx(), self.ny());
        let ranges: Vec<QuadRange> = self
            .chunks
            .limits()
            .map(|chunk| QuadRange {
                istart: signed(chunk.istart) + 1,
                iend: signed(chunk.iend).min(nx - 1),
                jstart: signed(chunk.jstart) + 1,
                jend: signed(chunk.jend).min(ny - 1),
            })
            .collect();
        let single = ranges.len() == 1;
        if single {
            self.init_levels_and_starts(None);
        }
        let mut path = LevelPath::default();
        for range in ranges {
            if !single {
                self.init_levels_and_starts(Some(range));
            }
            let tracer = Tracer {
                field: &self.field,
                cache: &mut self.cache,
                levels: self.levels,
                corner_mask: self.corner_mask,
                pass: Pass::Count,
                look_up_quads: Vec::new(),
                points: Vec::new(),
            };
            ChunkMarch::new(tracer, range, &mut path).run();
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::PathCode::{ClosePoly as C, LineTo as L, MoveTo as M};

    use super::*;
    use crate::testing::vertex_pairs;

    /// A generator for `rows` at index coordinates.
    fn generator(
        rows: &[&[f64]],
        mask: Option<&[bool]>,
        corner_mask: bool,
        chunk: usize,
    ) -> Serial {
        let (ny, nx) = (rows.len(), rows[0].len());
        let z: Vec<f64> = rows.concat();
        let x: Vec<f64> = (0..ny).flat_map(|_| (0..nx).map(|i| i as f64)).collect();
        let y: Vec<f64> = (0..ny)
            .flat_map(|j| (0..nx).map(move |_| j as f64))
            .collect();
        let grid = QuadGrid {
            x,
            y,
            z,
            mask,
            nx,
            ny,
        };
        Serial::new(grid, corner_mask, Chunks::new(nx, ny, (chunk, chunk)))
    }

    const PIT: [&[f64]; 3] = [&[1.0, 1.0, 1.0], &[1.0, 0.0, 1.0], &[1.0, 1.0, 1.0]];

    #[test]
    fn a_loop_around_a_pit_closes() {
        let path = generator(&PIT, None, true, 0).lines(0.5);
        assert_eq!(path.codes().unwrap(), [M, L, L, L, C]);
        assert_eq!(
            vertex_pairs(&path),
            [(1.0, 0.5), (0.5, 1.0), (1.0, 1.5), (1.5, 1.0), (1.0, 0.5)]
        );
        assert_eq!(generator(&PIT, None, true, 0).lines(1.5).codes(), None);
    }

    #[test]
    fn lines_cross_from_boundary_to_boundary() {
        let ramp: [&[f64]; 2] = [&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0]];
        let path = generator(&ramp, None, true, 0).lines(1.5);
        assert_eq!(path.codes().unwrap(), [M, L]);
        assert_eq!(vertex_pairs(&path), [(1.5, 1.0), (1.5, 0.0)]);
    }

    #[test]
    fn a_filled_band_around_a_pit_has_a_hole() {
        let rows: [&[f64]; 5] = [
            &[2.0; 5],
            &[2.0, 1.0, 1.0, 1.0, 2.0],
            &[2.0, 1.0, 0.0, 1.0, 2.0],
            &[2.0, 1.0, 1.0, 1.0, 2.0],
            &[2.0; 5],
        ];
        let band = generator(&rows, None, true, 0).filled(0.5, 1.5);
        let rings = band.subpaths();
        assert_eq!(rings.len(), 2, "an outer ring and its hole");
        assert!(band.codes().unwrap().ends_with(&[C]));
        // Each ring repeats its first point.
        for ring in &rings {
            assert_eq!(ring.vertices().first(), ring.vertices().last());
        }
    }

    #[test]
    fn everything_in_the_band_is_one_boundary_ring() {
        let flat: [&[f64]; 3] = [&[1.0; 3], &[1.0; 3], &[1.0; 3]];
        let whole = generator(&flat, None, true, 0).filled(0.0, 2.0);
        assert_eq!(whole.subpaths().len(), 1);
        assert_eq!(whole.vertices().len(), 9, "8 boundary points, closed");
        let chunked = generator(&flat, None, true, 1).filled(0.0, 2.0);
        assert_eq!(chunked.subpaths().len(), 4);
    }

    #[test]
    fn a_masked_centre_leaves_corner_triangles() {
        let mut mask = [false; 9];
        mask[4] = true;
        let ramp: [&[f64]; 3] = [&[0.0, 1.0, 2.0], &[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]];
        let corners = generator(&ramp, Some(&mask), true, 0).lines(1.5);
        assert_eq!(corners.subpaths().len(), 2, "split by the masked centre");
        let whole = generator(&ramp, Some(&mask), false, 0).lines(1.5);
        assert_eq!(whole.codes(), None, "every quad touches the mask");
    }

    #[test]
    fn nan_values_count_as_below() {
        assert_eq!(Levels::default().z_level(f64::NAN), 0);
        let band = Levels {
            filled: true,
            lower: 0.0,
            upper: 1.0,
        };
        assert_eq!([0.5, 2.0, 0.0].map(|z| band.z_level(z)), [1, 2, 0]);
    }
}
