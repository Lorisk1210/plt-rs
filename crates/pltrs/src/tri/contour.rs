//! Contours of fields on triangular grids (matplotlib's
//! `TriContourGenerator`).
//!
//! Lines that cross the boundary are found first by walking the boundary
//! edges, then closed loops by visiting every remaining triangle. Filled
//! contours additionally follow the boundary between crossings and add
//! untouched boundaries lying between the two levels. A triangle has one
//! line segment per level at most: the configuration of its three points
//! (above the level or not) gives the edge it leaves by.

use std::collections::HashMap;

use pltrs_core::numeric::{CONTRACTION, Contraction};
use pltrs_core::{Path, Point};

use super::Triangulation;
use super::triangulation::TriEdge;
use crate::contour::{ContourError, ContourSource, Generator, LevelPath};

/// Contour lines and filled contours of a field on a [`Triangulation`].
///
/// ```
/// use pltrs::tri::{TriContourGenerator, Triangulation};
///
/// let tri = Triangulation::new(&[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0], vec![[0, 1, 2]])?;
/// let mut generator = TriContourGenerator::new(&tri, &[0.0, 1.0, 1.0])?;
/// assert_eq!(generator.lines(0.5).vertices().len(), 2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct TriContourGenerator {
    tri: Triangulation,
    z: Vec<f64>,
    boundaries: Vec<Vec<TriEdge>>,
    /// Where each boundary edge sits: `(boundary, index)`.
    boundary_index: HashMap<TriEdge, (usize, usize)>,
    /// Lower-level flags for every triangle, then upper-level ones.
    interior_visited: Vec<bool>,
    boundaries_visited: Vec<Vec<bool>>,
    boundaries_used: Vec<bool>,
}

/// matplotlib's `ContourLine`: identical consecutive points are skipped.
#[derive(Clone, Debug, Default)]
struct Line(Vec<Point>);

impl Line {
    fn push(&mut self, point: Point) {
        if self.0.last() != Some(&point) {
            self.0.push(point);
        }
    }

    /// Repeats the first point, closing the line (skipped if it already
    /// ends there).
    fn close(&mut self) {
        if let Some(&first) = self.0.first() {
            self.push(first);
        }
    }
}

impl TriContourGenerator {
    /// A generator for one z value per point of `tri`.
    ///
    /// # Errors
    ///
    /// When `z` does not have one value per point.
    pub fn new(tri: &Triangulation, z: &[f64]) -> Result<Self, ContourError> {
        if z.len() != tri.npoints() {
            return Err(ContourError::ZLength {
                len: z.len(),
                points: tri.npoints(),
            });
        }
        let boundaries = tri.boundaries();
        let boundary_index = boundaries
            .iter()
            .enumerate()
            .flat_map(|(b, edges)| edges.iter().enumerate().map(move |(i, &e)| (e, (b, i))))
            .collect();
        Ok(Self {
            tri: tri.clone(),
            z: z.to_vec(),
            boundaries_visited: boundaries.iter().map(|b| vec![false; b.len()]).collect(),
            boundaries_used: vec![false; boundaries.len()],
            interior_visited: vec![false; 2 * tri.ntri()],
            boundaries,
            boundary_index,
        })
    }

    /// The contour lines at `level` as one compound path; loops end with a
    /// CLOSEPOLY (matplotlib's `create_contour` concatenated). No lines
    /// give an empty path without codes.
    pub fn lines(&mut self, level: f64) -> Path {
        self.clear_visited();
        let mut lines = Vec::new();
        self.boundary_lines(&mut lines, level);
        self.interior_lines(&mut lines, level, false);
        let mut path = LevelPath::default();
        for line in &lines {
            path.add_line(&line.0);
        }
        path.finish()
    }

    /// The filled contour `lower <= z < upper` as one compound path of
    /// rings, each ending with a CLOSEPOLY except lone points (matplotlib's
    /// `create_filled_contour`). The path always has codes, possibly none.
    ///
    /// # Errors
    ///
    /// When `lower >= upper`.
    pub fn filled(&mut self, lower: f64, upper: f64) -> Result<Path, ContourError> {
        if lower >= upper {
            return Err(ContourError::LevelOrder { lower, upper });
        }
        self.clear_visited();
        self.boundaries_visited
            .iter_mut()
            .for_each(|b| b.fill(false));
        self.boundaries_used.fill(false);
        let mut lines = Vec::new();
        self.boundary_lines_filled(&mut lines, lower, upper);
        self.untouched_boundaries(&mut lines, lower, upper);
        self.interior_lines(&mut lines, lower, false);
        self.interior_lines(&mut lines, upper, true);
        let mut path = LevelPath::default();
        for line in &lines {
            path.add_polyline(&line.0, line.0.len() > 1);
        }
        Ok(path.finish_coded())
    }

    fn clear_visited(&mut self) {
        self.interior_visited.fill(false);
    }

    fn point_z(&self, point: usize) -> f64 {
        self.z[point]
    }

    /// z at the start (`end = false`) or end of a triangle edge.
    fn edge_z(&self, (tri, edge): TriEdge, end: bool) -> f64 {
        let corner = if end { (edge + 1) % 3 } else { edge };
        self.point_z(self.tri.triangles()[tri][corner])
    }

    /// Where `level` crosses edge `edge` of `tri`: `p1 * f + p2 * (1 - f)`
    /// with `f = (z2 - level) / (z2 - z1)`.
    fn interp(&self, (tri, edge): TriEdge, level: f64) -> Point {
        let corners = self.tri.triangles()[tri];
        let (p1, p2) = (corners[edge], corners[(edge + 1) % 3]);
        let (z1, z2) = (self.point_z(p1), self.point_z(p2));
        let fraction = (z2 - level) / (z2 - z1);
        interp_point(
            CONTRACTION,
            self.tri.point(p1),
            self.tri.point(p2),
            fraction,
        )
    }

    /// The edge by which a contour at `level` leaves `tri`, if it crosses
    /// it; upper levels run the other way.
    fn exit_edge(&self, tri: usize, level: f64, on_upper: bool) -> Option<usize> {
        let corners = self.tri.triangles()[tri];
        let above = |i: usize| u8::from(self.point_z(corners[i]) >= level);
        let mut config = above(0) | (above(1) << 1) | (above(2) << 2);
        if on_upper {
            config = 7 - config;
        }
        match config {
            1 | 3 => Some(2),
            2 | 6 => Some(0),
            4 | 5 => Some(1),
            _ => None,
        }
    }

    fn visited_index(&self, tri: usize, on_upper: bool) -> usize {
        if on_upper { tri + self.tri.ntri() } else { tri }
    }

    /// Follows a contour through triangle interiors from `at`, adding the
    /// crossing points, until it leaves through a boundary edge
    /// (`end_on_boundary`) or returns to a visited triangle. `at` ends as
    /// the last edge crossed.
    ///
    /// A walk through more triangle edges than there are repeats itself,
    /// which only triangles that are their own neighbours make happen; it
    /// ends there, where matplotlib's goes on forever.
    fn follow_interior(
        &mut self,
        line: &mut Line,
        at: &mut TriEdge,
        end_on_boundary: bool,
        level: f64,
        on_upper: bool,
    ) {
        line.push(self.interp(*at, level));
        for _ in 0..=3 * self.tri.ntri() {
            let visited = self.visited_index(at.0, on_upper);
            if !end_on_boundary && self.interior_visited[visited] {
                break;
            }
            let Some(edge) = self.exit_edge(at.0, level, on_upper) else {
                break;
            };
            at.1 = edge;
            self.interior_visited[visited] = true;
            line.push(self.interp(*at, level));
            match self.tri.neighbor_edge(at.0, edge) {
                Some(next) => *at = next,
                None => break,
            }
        }
    }

    /// Open lines crossing the boundary: each boundary edge going from at
    /// or above the level to below it starts one.
    fn boundary_lines(&mut self, lines: &mut Vec<Line>, level: f64) {
        for b in 0..self.boundaries.len() {
            let mut end_above = false;
            for i in 0..self.boundaries[b].len() {
                let edge = self.boundaries[b][i];
                let start_above = if i == 0 {
                    self.edge_z(edge, false) >= level
                } else {
                    end_above
                };
                end_above = self.edge_z(edge, true) >= level;
                if start_above && !end_above {
                    let mut line = Line::default();
                    let mut at = edge;
                    self.follow_interior(&mut line, &mut at, true, level, false);
                    lines.push(line);
                }
            }
        }
    }

    /// Closed loops at `level` through unvisited, unmasked triangles.
    fn interior_lines(&mut self, lines: &mut Vec<Line>, level: f64, on_upper: bool) {
        for tri in 0..self.tri.ntri() {
            let visited = self.visited_index(tri, on_upper);
            if self.interior_visited[visited] || self.tri.is_masked(tri) {
                continue;
            }
            self.interior_visited[visited] = true;
            let Some(edge) = self.exit_edge(tri, level, on_upper) else {
                continue;
            };
            let Some(mut at) = self.tri.neighbor_edge(tri, edge) else {
                continue;
            };
            let mut line = Line::default();
            self.follow_interior(&mut line, &mut at, false, level, on_upper);
            line.close();
            lines.push(line);
        }
    }

    /// Filled rings that cross the boundary: alternately through the
    /// interior and along the boundary until back at their start.
    fn boundary_lines_filled(&mut self, lines: &mut Vec<Line>, lower: f64, upper: f64) {
        for b in 0..self.boundaries.len() {
            for i in 0..self.boundaries[b].len() {
                if self.boundaries_visited[b][i] {
                    continue;
                }
                let start = self.boundaries[b][i];
                let (z_start, z_end) = (self.edge_z(start, false), self.edge_z(start, true));
                let incr_upper = z_start < upper && z_end >= upper;
                let decr_lower = z_start >= lower && z_end < lower;
                if !(decr_lower || incr_upper) {
                    continue;
                }
                let mut line = Line::default();
                let mut at = start;
                let mut on_upper = incr_upper;
                // Each turn starts on a boundary edge, on either level:
                // more turns than that repeat one, which only triangles
                // that are their own neighbours make happen.
                for _ in 0..=2 * self.boundary_index.len() {
                    let level = if on_upper { upper } else { lower };
                    self.follow_interior(&mut line, &mut at, true, level, on_upper);
                    match self.follow_boundary(&mut line, &mut at, lower, upper, on_upper) {
                        Some(next) => on_upper = next,
                        None => break,
                    }
                    if at == start {
                        break;
                    }
                }
                line.close();
                lines.push(line);
            }
        }
    }

    /// Whole boundaries no line touched whose points lie between the
    /// levels.
    fn untouched_boundaries(&self, lines: &mut Vec<Line>, lower: f64, upper: f64) {
        for (boundary, used) in self.boundaries.iter().zip(&self.boundaries_used) {
            let Some(&first) = boundary.first() else {
                continue;
            };
            let z = self.edge_z(first, false);
            if *used || !(z >= lower && z < upper) {
                continue;
            }
            let mut line = Line::default();
            for &(tri, edge) in boundary {
                line.push(self.tri.point(self.tri.triangles()[tri][edge]));
            }
            line.close();
            lines.push(line);
        }
    }

    /// Follows the boundary from the edge `at` until z crosses a level,
    /// adding the boundary points passed. Returns whether it stopped on the
    /// upper level (`None` if `at` is not a boundary edge, or if a lap of
    /// the boundary and its first edge again cross no level, which only
    /// triangles that are their own neighbours make happen).
    fn follow_boundary(
        &mut self,
        line: &mut Line,
        at: &mut TriEdge,
        lower: f64,
        upper: f64,
        mut on_upper: bool,
    ) -> Option<bool> {
        let (boundary, mut index) = *self.boundary_index.get(at)?;
        self.boundaries_used[boundary] = true;
        let mut first_edge = true;
        let mut z_end = 0.0;
        for _ in 0..=self.boundaries[boundary].len() {
            self.boundaries_visited[boundary][index] = true;
            let z_start = if first_edge {
                self.edge_z(*at, false)
            } else {
                z_end
            };
            z_end = self.edge_z(*at, true);
            let crossing = if z_end > z_start {
                if (on_upper || !first_edge) && z_end >= lower && z_start < lower {
                    Some(false)
                } else if z_end >= upper && z_start < upper {
                    Some(true)
                } else {
                    None
                }
            } else if (!on_upper || !first_edge) && z_start >= upper && z_end < upper {
                Some(true)
            } else if z_start >= lower && z_end < lower {
                Some(false)
            } else {
                None
            };
            first_edge = false;
            if let Some(upper_level) = crossing {
                on_upper = upper_level;
                return Some(on_upper);
            }
            index = (index + 1) % self.boundaries[boundary].len();
            *at = self.boundaries[boundary][index];
            line.push(self.tri.point(self.tri.triangles()[at.0][at.1]));
        }
        None
    }
}

/// Checks z and builds the generator of a triangular contour set
/// (matplotlib's `TriContourSet._contour_args`): z must be finite at every
/// point of an unmasked triangle, and positive when log-scaled.
pub(crate) fn source(
    tri: &Triangulation,
    z: &[f64],
    logscale: bool,
) -> Result<ContourSource<TriContourGenerator>, ContourError> {
    let generator = TriContourGenerator::new(tri, z)?;
    let mut used: Vec<usize> = tri.masked_triangles().into_iter().flatten().collect();
    used.sort_unstable();
    used.dedup();
    let used_z: Vec<f64> = used.iter().map(|&p| z[p]).collect();
    if used_z.iter().any(|v| !v.is_finite()) {
        return Err(ContourError::NonFiniteZ);
    }
    let zmin = used_z
        .iter()
        .copied()
        .reduce(f64::min)
        .ok_or(ContourError::NoTriangles)?;
    let zmax = used_z
        .iter()
        .copied()
        .reduce(f64::max)
        .ok_or(ContourError::NoTriangles)?;
    if logscale && zmin <= 0.0 {
        return Err(ContourError::LogOfNonPositive);
    }
    let extreme = |values: &[f64], pick: fn(f64, f64) -> f64| {
        values.iter().copied().reduce(pick).unwrap_or(f64::NAN)
    };
    Ok(ContourSource {
        generator,
        zmin: Some(zmin),
        zmax: Some(zmax),
        data_lim: (
            Point::new(extreme(tri.x(), f64::min), extreme(tri.y(), f64::min)),
            Point::new(extreme(tri.x(), f64::max), extreme(tri.y(), f64::max)),
        ),
        boolean: false,
        warnings: Vec::new(),
    })
}

impl Generator for TriContourGenerator {
    fn lines(&mut self, level: f64) -> Path {
        Self::lines(self, level)
    }

    fn filled(&mut self, lower: f64, upper: f64) -> Result<Path, ContourError> {
        Self::filled(self, lower, upper)
    }
}

/// `a * fraction + b * (1 - fraction)`, matplotlib's `XY` arithmetic in
/// `TriContourGenerator::interp` as `contraction`'s compiler builds it:
/// clang rounds both products (they pass through `XY::operator*`), GCC
/// inlines the operators and fuses the first into the sum (its
/// `widening_mul` GIMPLE).
fn interp_point(contraction: Contraction, a: Point, b: Point, fraction: f64) -> Point {
    let rest = 1.0 - fraction;
    match contraction {
        Contraction::Gcc => Point::new(
            contraction.fused(a.x, fraction, b.x * rest),
            contraction.fused(a.y, fraction, b.y * rest),
        ),
        Contraction::Unfused | Contraction::Clang => {
            Point::new(a.x * fraction + b.x * rest, a.y * fraction + b.y * rest)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_interpolate_as_each_compiler_builds_matplotlib() {
        let (a, b) = (Point::new(1.0 / 3.0, 0.0), Point::new(0.2, 0.0));
        // a.x * 0.6 is fused under GCC only.
        assert_eq!(
            interp_point(Contraction::Gcc, a, b, 0.6).x,
            0.279_999_999_999_999_97
        );
        assert_eq!(interp_point(Contraction::Clang, a, b, 0.6).x, 0.28);
        assert_eq!(interp_point(Contraction::Unfused, a, b, 0.6).x, 0.28);
    }

    /// A unit square split around a centre peak.
    fn fan() -> (Triangulation, Vec<f64>) {
        let tri = Triangulation::new(
            &[0.0, 1.0, 0.0, 1.0, 0.5],
            &[0.0, 0.0, 1.0, 1.0, 0.5],
            vec![[0, 1, 4], [1, 3, 4], [3, 2, 4], [2, 0, 4]],
        )
        .unwrap();
        (tri, vec![0.0, 0.0, 0.0, 0.0, 1.0])
    }

    #[test]
    fn loop_around_a_peak() {
        let (tri, z) = fan();
        let mut generator = TriContourGenerator::new(&tri, &z).unwrap();
        let path = generator.lines(0.5);
        assert_eq!(path.vertices().len(), 5);
        assert_eq!(path.codes().unwrap()[4], pltrs_core::PathCode::ClosePoly);
        assert_eq!(path.vertices()[0], path.vertices()[4]);
        assert_eq!(generator.lines(2.0).codes(), None);
    }

    #[test]
    fn filled_band_covers_the_square() {
        let (tri, z) = fan();
        let mut generator = TriContourGenerator::new(&tri, &z).unwrap();
        let below = generator.filled(-1.0, 0.5).unwrap();
        assert_eq!(
            below
                .codes()
                .unwrap()
                .iter()
                .filter(|&&c| c == pltrs_core::PathCode::MoveTo)
                .count(),
            2
        );
        assert_eq!(generator.filled(2.0, 3.0).unwrap().codes(), Some(&[][..]));
        assert!(generator.filled(1.0, 1.0).is_err());
    }

    #[test]
    fn triangles_repeating_a_point_end_their_contours() {
        // Each of these used to send the generator round in circles.
        let (x, y) = ([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]);
        let z = [0.0, 1.0, 2.0, 3.0];
        for triangles in [vec![[0, 1, 0]], vec![[0, 0, 0]], vec![[0, 1, 2], [0, 2, 0]]] {
            let tri = Triangulation::new(&x, &y, triangles).unwrap();
            let mut generator = TriContourGenerator::new(&tri, &z).unwrap();
            generator.lines(0.5);
            generator.filled(0.5, 1.5).unwrap();
        }
    }

    #[test]
    fn z_must_match_points() {
        let (tri, _) = fan();
        assert!(matches!(
            TriContourGenerator::new(&tri, &[0.0]),
            Err(ContourError::ZLength { len: 1, points: 5 })
        ));
    }

    #[test]
    fn masked_triangles_leave_a_hole() {
        // A 4 × 4 point grid whose centre square is masked.
        let x: Vec<f64> = (0..16).map(|p| f64::from(p % 4)).collect();
        let y: Vec<f64> = (0..16).map(|p| f64::from(p / 4)).collect();
        let triangles = (0..9)
            .flat_map(|s| {
                let a = s / 3 * 4 + s % 3;
                [[a, a + 1, a + 5], [a, a + 5, a + 4]]
            })
            .collect();
        let mask = (0..18).map(|t| t / 2 == 4).collect();
        let tri = Triangulation::new(&x, &y, triangles)
            .unwrap()
            .with_mask(Some(mask))
            .unwrap();
        let mut generator = TriContourGenerator::new(&tri, &[1.0; 16]).unwrap();
        let rings = generator.filled(0.0, 2.0).unwrap().subpaths();
        assert_eq!(rings.len(), 2, "outer boundary and the hole");
        assert_eq!(generator.lines(0.5).codes(), None);
        let (_, z) = fan();
        assert!(matches!(
            source(&tri, &z, false),
            Err(ContourError::ZLength { .. })
        ));
    }

    #[test]
    fn source_checks_used_points() {
        let (tri, mut z) = fan();
        assert_eq!(source(&tri, &z, false).unwrap().zmax, Some(1.0));
        assert!(matches!(
            source(&tri, &z, true),
            Err(ContourError::LogOfNonPositive)
        ));
        z[1] = f64::NAN;
        assert!(matches!(
            source(&tri, &z, false),
            Err(ContourError::NonFiniteZ)
        ));
    }
}
