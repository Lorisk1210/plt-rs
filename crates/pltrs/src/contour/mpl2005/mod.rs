//! A port of matplotlib's original contour tracer, contourpy's `mpl2005`
//! algorithm (gcntr, from the GIST package).
//!
//! The mesh is classified by z-value against the levels and the edges
//! where curves may start are marked. A first pass traces every curve,
//! erasing the start marks it passes and deciding where holes are joined
//! to the curves around them by slits; a second pass traces the remaining
//! starts, recording the points. Each curve is then cut at its slit
//! points and its pieces rejoined into subpaths (`reorder`). The tracer
//! cannot mask corners: a masked point removes the four zones around it.
//!
//! The arithmetic and the traversal order are gcntr's, so the vertices are
//! equal to the last bit and come in the same order.

mod init;
mod site;

use pltrs_core::{Path, Point};

use self::init::mask_zones;
use self::site::{KIND_SLIT_UP, Site};
use super::assemble::LevelPath;
use super::chunks::Chunks;
use super::quad::{GridCoordinates, QuadGrid, signed, slot};

/// contourpy's `Mpl2005ContourGenerator` with matplotlib's line and fill
/// types.
#[derive(Clone, Debug)]
pub(super) struct Mpl2005 {
    mesh: Mesh,
    /// Points per column.
    jmax: isize,
    /// Zones per chunk in i and j (filled contours only).
    chunk: (isize, isize),
    /// Which zones exist, when points are masked.
    zones: Option<Vec<bool>>,
    /// The classification of every point, zone and edge (gcntr's `data`).
    data: Vec<u16>,
    /// The saddle decisions of every zone.
    saddle: Vec<u8>,
}

/// The coordinates and values of the mesh points, row-major.
#[derive(Clone, Debug)]
struct Mesh {
    coords: GridCoordinates,
    z: Vec<f64>,
    /// Points per row.
    imax: isize,
}

impl Mesh {
    /// The position of mesh point `point`.
    fn point(&self, point: usize) -> Point {
        self.coords.at(point, slot(self.imax))
    }
}

impl Mpl2005 {
    /// A generator for `grid`, in `chunks`.
    pub(super) fn new(grid: QuadGrid<'_>, chunks: Chunks) -> Self {
        let (imax, jmax) = (grid.nx, grid.ny);
        Self {
            zones: grid.mask.map(|mask| mask_zones(imax, jmax, mask)),
            mesh: Mesh {
                coords: grid.coords,
                z: grid.z,
                imax: signed(imax),
            },
            jmax: signed(jmax),
            chunk: (signed(chunks.x_size), signed(chunks.y_size)),
            data: vec![0; imax * jmax + imax + 1],
            saddle: vec![0; imax * jmax],
        }
    }

    /// The lines at `level`.
    pub(super) fn lines(&mut self, level: f64) -> Path {
        self.trace([level, level]).finish()
    }

    /// The region `lower < z <= upper` (`lower < upper`).
    pub(super) fn filled(&mut self, lower: f64, upper: f64) -> Path {
        self.trace([lower, upper]).finish()
    }

    /// Traces every curve at `zlevel` (gcntr's `cntr_trace`).
    fn trace(&mut self, zlevel: [f64; 2]) -> LevelPath {
        let filled = zlevel[1] > zlevel[0];
        let count = self.data_init(zlevel);
        let mut site = Site::new(&self.mesh, &mut self.data, &mut self.saddle, zlevel, count);
        while site.curve_tracer() != 0 {}
        site.start_second_pass();
        let mut path = LevelPath::default();
        while site.curve_tracer() > 0 {
            reorder(&site.points, &site.kinds, filled, &mut path);
            site.points.clear();
            site.kinds.clear();
        }
        path
    }
}

/// Appends the subpaths of one traced curve: its runs of points between
/// slit points, rejoined where one ends at the start of a later one;
/// filled rings and lines ending at their start are closed (gcntr's
/// `reorder`).
fn reorder(points: &[Point], kinds: &[u8], filled: bool, path: &mut LevelPath) {
    let last = points.len().saturating_sub(1);
    let mut segments: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (i, &kind) in kinds.iter().enumerate() {
        match start {
            Some(first) if kind >= KIND_SLIT_UP || i == last => {
                segments.push((first, i));
                start = None;
            }
            None if kind < KIND_SLIT_UP && i < last => start = Some(i),
            _ => {}
        }
    }
    let mut subpath: Vec<Option<usize>> = vec![None; segments.len()];
    let mut count = 0;
    for first in 0..segments.len() {
        if subpath[first].is_some() {
            continue;
        }
        subpath[first] = Some(count);
        let mut end = points[segments[first].1];
        for next in first + 1..segments.len() {
            if subpath[next].is_none() && points[segments[next].0] == end {
                subpath[next] = Some(count);
                end = points[segments[next].1];
            }
        }
        count += 1;
    }
    for id in 0..count {
        let mut vertices: Vec<Point> = Vec::new();
        for (&(first, end), _) in segments
            .iter()
            .zip(&subpath)
            .filter(|(_, owner)| **owner == Some(id))
        {
            // Later segments repeat the point they join at.
            let skip = usize::from(!vertices.is_empty());
            vertices.extend_from_slice(&points[first + skip..=end]);
        }
        let closed = filled || vertices.first() == vertices.last();
        path.add_polyline(&vertices, closed);
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::PathCode::{ClosePoly as C, LineTo as L, MoveTo as M};

    use super::site::KIND_ZONE;
    use super::*;
    use crate::testing::vertex_pairs;

    /// A generator for `rows` at index coordinates.
    fn generator(rows: &[&[f64]], mask: Option<&[bool]>, chunk: usize) -> Mpl2005 {
        let (ny, nx) = (rows.len(), rows[0].len());
        let z: Vec<f64> = rows.concat();
        let coords = GridCoordinates::Axes {
            x: (0..nx).map(|i| i as f64).collect(),
            y: (0..ny).map(|j| j as f64).collect(),
        };
        let grid = QuadGrid {
            coords,
            z,
            mask,
            nx,
            ny,
        };
        Mpl2005::new(grid, Chunks::new(nx, ny, (chunk, chunk)))
    }

    #[test]
    fn a_loop_around_a_pit_closes() {
        let pit: [&[f64]; 3] = [&[1.0, 1.0, 1.0], &[1.0, 0.0, 1.0], &[1.0, 1.0, 1.0]];
        let path = generator(&pit, None, 0).lines(0.5);
        assert_eq!(path.codes().unwrap(), [M, L, L, L, C]);
        assert_eq!(
            vertex_pairs(&path),
            [(1.0, 0.5), (0.5, 1.0), (1.0, 1.5), (1.5, 1.0), (1.0, 0.5)]
        );
    }

    #[test]
    fn a_hole_is_joined_to_its_outer_ring_by_a_slit() {
        let rows: [&[f64]; 5] = [
            &[2.0; 5],
            &[2.0, 1.0, 1.0, 1.0, 2.0],
            &[2.0, 1.0, 0.0, 1.0, 2.0],
            &[2.0, 1.0, 1.0, 1.0, 2.0],
            &[2.0; 5],
        ];
        let band = generator(&rows, None, 0).filled(0.5, 1.5);
        let codes = band.codes().unwrap();
        assert_eq!(
            codes.iter().filter(|&&c| c == M).count(),
            2,
            "ring and hole"
        );
        assert_eq!(codes.last(), Some(&C));
    }

    #[test]
    fn masked_points_remove_whole_zones() {
        let mut mask = [false; 9];
        mask[4] = true;
        let ramp: [&[f64]; 3] = [&[0.0, 1.0, 2.0], &[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]];
        assert_eq!(generator(&ramp, Some(&mask), 0).lines(1.5).codes(), None);
    }

    #[test]
    fn slit_points_split_curves_into_rejoined_segments() {
        let p = |x: f64| Point::new(x, 0.0);
        // A run ending at a slit point, a run starting there and ending at
        // another, and a last point that starts nothing.
        let points = [p(0.0), p(1.0), p(2.0), p(2.0), p(3.0), p(9.0), p(6.0)];
        let (zone, up, down) = (KIND_ZONE, KIND_SLIT_UP, KIND_SLIT_UP + 1);
        let kinds = [zone, zone, up, zone, zone, down, zone];
        let mut path = LevelPath::default();
        reorder(&points, &kinds, false, &mut path);
        let path = path.finish();
        assert_eq!(points_of(&path), [0.0, 1.0, 2.0, 3.0, 9.0]);
        assert_eq!(path.codes().unwrap(), [M, L, L, L, L]);
        let mut ring = LevelPath::default();
        reorder(&points, &kinds, true, &mut ring);
        assert_eq!(ring.finish().codes().unwrap().last(), Some(&C), "filled");
    }

    fn points_of(path: &Path) -> Vec<f64> {
        path.vertices().iter().map(|p| p.x).collect()
    }
}
