//! A port of contourpy's `mpl2014` algorithm, matplotlib's default quad
//! grid contour generator.
//!
//! A line contour connects points of equal z on the grid edges, linearly
//! interpolated, with higher values on its left. The generator sweeps the
//! grid twice: first for open lines starting on a boundary, then for closed
//! loops in the interior. Saddle quads hold two line segments; the mean of
//! their corners decides how they connect.
//!
//! A filled contour is the region between two levels: closed polygons
//! alternating between interior sections (on either level, the upper one
//! reversed) and boundary sections, with holes attached to their outer
//! polygon during the sweep.
//!
//! The arithmetic and traversal order are reproduced exactly, including
//! the asymmetric interpolation that leaves some closed loops a rounding
//! error short of their start point (they are then emitted open). The
//! sweeps skip the quads that cannot start a contour (see `sweep`).

mod cache;
mod edge;
mod filled;
mod sweep;
mod trace;

use pltrs_core::{Path, Point};

use self::cache::{Cache, Exists, LevelIndex, VISITED_S, VISITED_W};
use self::edge::{Edge, QuadEdge, edge_point, is_boundary};
use self::filled::{ChunkFill, ParentCache};
use self::sweep::Bits;
use self::trace::{Field, Tracer};
use super::assemble::LevelPath;
use super::chunks::{ChunkLimits, Chunks};
use super::quad::QuadGrid;

/// contourpy's `Mpl2014ContourGenerator` with matplotlib's line and fill
/// types.
#[derive(Clone, Debug)]
pub(super) struct Mpl2014 {
    field: Field,
    corner_mask: bool,
    chunks: Chunks,
    cache: Cache,
    /// The quads where boundary lines can start, at every level.
    boundary: Bits,
    /// The quads that may start filled contours whatever the levels.
    complex: Bits,
    /// The parents of the polygons of the last filled level, whose memory
    /// (four bytes per point of a chunk) the next level reuses.
    parents: Option<Box<ParentCache>>,
}

impl Mpl2014 {
    /// A generator for `grid`, in `chunks`.
    pub(super) fn new(grid: QuadGrid<'_>, corner_mask: bool, chunks: Chunks) -> Self {
        let (nx, ny) = (grid.nx, grid.ny);
        let cache = Cache::new(
            nx,
            ny,
            grid.mask,
            corner_mask,
            (chunks.x_size, chunks.y_size),
        );
        Self {
            field: Field {
                x: grid.x,
                y: grid.y,
                z: grid.z,
                nx,
            },
            corner_mask,
            chunks,
            boundary: cache.boundary_quads(),
            complex: cache.complex_quads(),
            cache,
            parents: None,
        }
    }

    /// The lines at `level`.
    pub(super) fn lines(&mut self, level: f64) -> Path {
        let (chunks, nx) = (self.chunks, self.field.nx);
        let mut path = LevelPath::default();
        self.cache.init_levels(&self.field.z, level, level);
        let crossed = self.cache.crossed_quads();
        let levels = (level, level);
        let mut tracer = tracer(&self.field, &mut self.cache, self.corner_mask, levels, None);
        // Lines that start and end on boundaries.
        for chunk in chunks.limits() {
            for quad in self.boundary.quads(&chunk, nx) {
                start_boundary_lines(&mut tracer, quad, &mut path);
            }
        }
        // Closed loops in the interior.
        for chunk in chunks.limits() {
            for quad in crossed.quads(&chunk, nx) {
                // A saddle holds two loops: visit it again until done.
                while start_interior_loop(&mut tracer, quad, &mut path) {}
            }
        }
        path.finish()
    }

    /// The region `lower < z <= upper` (`lower < upper`).
    pub(super) fn filled(&mut self, lower: f64, upper: f64) -> Path {
        let (chunks, nx) = (self.chunks, self.field.nx);
        // Every chunk starts with a cleared cache, a reused one too.
        let parents = self.parents.take().map_or_else(
            || ParentCache::new(nx, chunks.x_size + 1, chunks.y_size + 1),
            |parents| *parents,
        );
        self.cache.init_levels(&self.field.z, lower, upper);
        let starts = self.cache.fill_start_quads(&self.complex);
        let levels = (lower, upper);
        let parents = Some(parents);
        let mut tracer = tracer(
            &self.field,
            &mut self.cache,
            self.corner_mask,
            levels,
            parents,
        );
        let mut path = LevelPath::default();
        for chunk in chunks.limits() {
            if let Some(parents) = &mut tracer.parents {
                parents.start_chunk(chunk.istart, chunk.jstart);
            }
            let mut fill = ChunkFill::new(&mut tracer);
            for quad in starts.quads(&chunk, nx) {
                fill.single_quad(quad);
            }
            let polygons = fill.into_polygons();
            clear_chunk_edges(&mut tracer, &chunks, &chunk);
            for ring in polygons.iter().flatten() {
                path.add_ring(ring);
            }
        }
        self.parents = tracer.parents.take().map(Box::new);
        path.finish()
    }
}

/// A pass between `lower` and `upper`, whose levels `cache` holds.
fn tracer<'a>(
    field: &'a Field,
    cache: &'a mut Cache,
    corner_mask: bool,
    (lower, upper): (f64, f64),
    parents: Option<ParentCache>,
) -> Tracer<'a> {
    Tracer {
        field,
        cache,
        corner_mask,
        lower,
        upper,
        parents,
        owner: 0,
    }
}

/// Clears the boundary-visited flags on the far edges of a chunk, which
/// the next chunks reuse.
fn clear_chunk_edges(tracer: &mut Tracer<'_>, chunks: &Chunks, chunk: &ChunkLimits) {
    let nx = tracer.field.nx;
    if chunk.jchunk < chunks.ny - 1 {
        for quad in chunk.istart + chunk.jend * nx..chunk.iend + chunk.jend * nx {
            tracer.cache.clear(quad, VISITED_S);
        }
    }
    if chunk.ichunk < chunks.nx - 1 {
        let end = chunk.iend + chunk.jend * nx;
        for quad in (chunk.iend + chunk.jstart * nx..end).step_by(nx) {
            tracer.cache.clear(quad, VISITED_W);
        }
    }
}

/// Starts the boundary lines of `quad`: every boundary edge (diagonals
/// of corners included) whose start point is above the level and end point
/// is not starts a line, checked S, W, N, E, then the diagonals. Stops
/// early once the quad is fully visited.
fn start_boundary_lines(tracer: &mut Tracer<'_>, quad: usize, path: &mut LevelPath) {
    const EDGES: [Edge; 8] = [
        Edge::S,
        Edge::W,
        Edge::N,
        Edge::E,
        Edge::Ne,
        Edge::Nw,
        Edge::Se,
        Edge::Sw,
    ];
    // Most quads have no boundary edge to start from.
    if !tracer.cache.touches_boundary(quad) {
        return;
    }
    let visited = LevelIndex::Lower.visited();
    for edge in EDGES {
        let cache = &*tracer.cache;
        if cache.exists(quad) == Exists::None || cache.has(quad, visited) {
            return;
        }
        let at = QuadEdge::new(quad, edge);
        let nx = tracer.field.nx;
        let above = |start: bool| cache.z_level(edge_point(at, start, nx)) >= 1;
        if !(is_boundary(cache, at) && above(true) && !above(false)) {
            continue;
        }
        let mut at = at;
        let mut line = Vec::new();
        tracer.follow_interior(&mut line, &mut at, LevelIndex::Lower, true, None);
        path.add_line(&line);
    }
}

/// Traces one interior loop starting in `quad`, if any; returns whether
/// the quad is a saddle that holds another, not yet traced, loop.
fn start_interior_loop(tracer: &mut Tracer<'_>, quad: usize, path: &mut LevelPath) -> bool {
    let level = LevelIndex::Lower;
    let cache = &*tracer.cache;
    if cache.exists(quad) == Exists::None || cache.has(quad, level.visited()) {
        return false;
    }
    let Some(edge) = tracer.start_edge(quad, level) else {
        return false;
    };
    let start = QuadEdge::new(quad, edge);
    let mut at = start;
    let mut line: Vec<Point> = Vec::new();
    // Matching the legacy output: a loop starting on a N edge gets its
    // first point at the end instead.
    let ignore_first = edge == Edge::N;
    tracer.follow_interior(
        &mut line,
        &mut at,
        level,
        !ignore_first,
        Some((start, level)),
    );
    if ignore_first && let Some(&first) = line.first() {
        line.push(first);
    }
    path.add_line(&line);
    tracer.cache.has(quad, level.saddle()) && !tracer.cache.has(quad, level.visited())
}

#[cfg(test)]
mod tests {
    use pltrs_core::PathCode::{self, ClosePoly as C, LineTo as L, MoveTo as M};

    use super::*;
    use crate::contour::QuadContourGenerator;
    use crate::numeric::Grid;
    use crate::testing::vertex_pairs;

    /// A generator for `z` at index coordinates.
    fn generator(
        rows: &[&[f64]],
        mask: Option<&[&[bool]]>,
        corner_mask: bool,
        chunk: usize,
    ) -> QuadContourGenerator {
        let z = Grid::from_rows(rows.iter().map(|r| r.to_vec())).unwrap();
        let (ny, nx) = z.shape();
        let x = Grid::from_fn(ny, nx, |_, j| j as f64);
        let y = Grid::from_fn(ny, nx, |i, _| i as f64);
        let mask = mask.map(|m| Grid::from_rows(m.iter().map(|r| r.to_vec())).unwrap());
        QuadContourGenerator::new(&x, &y, &z, mask.as_ref(), corner_mask, (chunk, chunk)).unwrap()
    }

    /// Signed areas of the rings of a filled path.
    fn ring_areas(path: &Path) -> Vec<f64> {
        path.subpaths()
            .iter()
            .map(|ring| {
                let v = ring.vertices();
                let n = v.len() - 1;
                (0..n)
                    .map(|i| v[i].x * v[(i + 1) % n].y - v[(i + 1) % n].x * v[i].y)
                    .sum::<f64>()
                    / 2.0
            })
            .collect()
    }

    const PIT: [&[f64]; 3] = [&[1.0, 1.0, 1.0], &[1.0, 0.0, 1.0], &[1.0, 1.0, 1.0]];

    #[test]
    fn loop_around_a_pit_closes() {
        let path = generator(&PIT, None, true, 0).lines(0.5);
        assert_eq!(path.codes().unwrap(), [M, L, L, L, C]);
        assert_eq!(
            vertex_pairs(&path),
            [(1.0, 0.5), (0.5, 1.0), (1.0, 1.5), (1.5, 1.0), (1.0, 0.5)]
        );
        assert_eq!(generator(&PIT, None, true, 0).lines(1.5).codes(), None);
    }

    #[test]
    fn asymmetric_interpolation_leaves_a_loop_open() {
        let rows: [&[f64]; 3] = [
            &[1.0, 1.0, 1.0, 1.0],
            &[1.0, 0.3, 1.0, 1.0],
            &[1.0, 1.0, 1.0, 1.0],
        ];
        let path = generator(&rows, None, true, 0).lines(0.5);
        assert_eq!(path.codes().unwrap(), [M, L, L, L, L]);
        let v = vertex_pairs(&path);
        assert_eq!(
            (v[0], v[4]),
            (
                (1.0, 0.714_285_714_285_714_2),
                (1.0, 0.714_285_714_285_714_3)
            )
        );
    }

    #[test]
    fn exact_saddle_level_counts_as_below() {
        let path = generator(&[&[1.0, 0.0], &[0.0, 1.0]], None, true, 0).lines(0.5);
        assert_eq!(path.codes().unwrap(), [M, L, M, L]);
        assert_eq!(
            vertex_pairs(&path),
            [(0.5, 0.0), (0.0, 0.5), (0.5, 1.0), (1.0, 0.5)]
        );
    }

    #[test]
    fn filled_annulus_is_an_anticlockwise_ring_with_a_clockwise_hole() {
        let bowl: Vec<Vec<f64>> = (0..5)
            .map(|i: i32| {
                (0..5)
                    .map(|j: i32| (f64::from(i) - 2.0).powi(2) + (f64::from(j) - 2.0).powi(2))
                    .collect()
            })
            .collect();
        let rows: Vec<&[f64]> = bowl.iter().map(Vec::as_slice).collect();
        let band = generator(&rows, None, true, 0).filled(0.5, 3.0).unwrap();
        let areas = ring_areas(&band);
        assert_eq!(areas.len(), 2);
        assert!(areas[0] > 0.0 && areas[1] < 0.0);
        let codes = band.codes().unwrap();
        assert_eq!(
            codes.iter().filter(|&&c| c == PathCode::ClosePoly).count(),
            2
        );
    }

    #[test]
    fn corner_masking_keeps_the_triangles_opposite_a_masked_point() {
        let mask: [&[bool]; 3] = [&[false; 3], &[false, true, false], &[false; 3]];
        let ramp: [&[f64]; 3] = [&[0.0, 1.0, 2.0], &[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]];
        let mut corners = generator(&ramp, Some(&mask), true, 0);
        assert_eq!(
            corners.lines(1.5).subpaths().len(),
            2,
            "split by the masked centre"
        );
        let mut whole = generator(&ramp, Some(&mask), false, 0);
        assert_eq!(
            whole.lines(1.5).codes(),
            None,
            "every quad touches the mask"
        );
        assert_eq!(whole.filled(-1.0, 5.0).unwrap().codes(), None);
    }

    #[test]
    fn chunks_split_polygons() {
        let flat: [&[f64]; 3] = [&[1.0; 3], &[1.0; 3], &[1.0; 3]];
        let whole = generator(&flat, None, true, 0).filled(0.0, 2.0).unwrap();
        assert_eq!(whole.subpaths().len(), 1);
        let chunked = generator(&flat, None, true, 1).filled(0.0, 2.0).unwrap();
        assert_eq!(chunked.subpaths().len(), 4);
        let area: f64 = ring_areas(&chunked).iter().sum();
        assert_eq!(area, 4.0);
    }
}
