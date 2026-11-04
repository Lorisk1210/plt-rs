//! Locating the facet a point is furthest above (`qh_findbest`,
//! `qh_findbestnew`, `qh_findbesthorizon`, `qh_findbestlower`, ...).

use super::geom;
use super::hull::{FacetId, Hull, HullError, HullResult};
use super::precision::QhullMinMax;

/// qhull's `qh_USEfindbestnew` threshold on the number of merges, above
/// which searches widen.
const MANY_MERGES: u32 = 50;

/// The search rules of `qh_findbest`.
#[derive(Clone, Copy, Debug)]
struct Descent {
    /// Keep searching past clearly outside facets.
    best_outside: bool,
    /// Only follow new facets.
    new_facets: bool,
    /// Only return an upper Delaunay facet if the point is clearly above it.
    no_upper: bool,
}

/// Where a descent stopped without a clearly outside facet: the best lower
/// facet (if any), its distance and the last facet visited.
type Stop = (Option<FacetId>, f64, FacetId);

/// The facet found for a point.
#[derive(Clone, Copy, Debug)]
pub(super) struct Best {
    /// The facet.
    pub(super) facet: FacetId,
    /// The point's distance above it.
    pub(super) dist: f64,
    /// Whether the point is clearly outside.
    pub(super) outside: bool,
}

impl Hull {
    /// Whether so many merges happened that points are partitioned by
    /// exhaustive search (qhull's `qh_USEfindbestnew`).
    pub(super) fn many_merges(&self) -> bool {
        self.total_merges > MANY_MERGES
    }

    /// 2 after many merges, 1 before (the `qh_USEfindbestnew` factor).
    fn search_factor(&self) -> f64 {
        if self.many_merges() { 2.0 } else { 1.0 }
    }

    /// Distance above which a point is clearly outside (`qh_DISToutside`).
    pub(super) fn dist_outside(&self) -> f64 {
        self.search_factor() * (2.0 * self.prec.min_outside).max_qhull(self.max_outside)
    }

    /// Distance below the best facet within which coplanar facets are
    /// searched (`qh_SEARCHdist`).
    fn search_dist(&self) -> f64 {
        self.search_factor()
            * (self.max_outside
                + 2.0 * self.prec.dist_round
                + self.prec.min_visible.max_qhull(self.prec.max_coplanar))
    }

    /// Directed search from `start` for the facet furthest below `point`
    /// (qhull's `qh_findbest`). Stops at a clearly outside facet unless
    /// `best_outside`; with `new_facets` only new facets are followed; with
    /// `no_upper` upper Delaunay facets are only returned if clearly below.
    pub(super) fn find_best(
        &mut self,
        point: usize,
        start: FacetId,
        best_outside: bool,
        new_facets: bool,
        no_upper: bool,
    ) -> HullResult<Best> {
        let rules = Descent {
            best_outside,
            new_facets,
            no_upper,
        };
        let (mut best, mut best_dist, last) = match self.descend(point, start, rules) {
            Ok(found) => return Ok(found),
            Err(stop) => stop,
        };
        let mut test_horizon = true;
        if new_facets {
            match best {
                None => {
                    let newfacets = self.newfacet_list.ok_or(HullError::Topology)?;
                    return self.find_best_new(point, newfacets, best_outside);
                }
                Some(facet) if !self.findbest_notsharp && best_dist < -self.prec.dist_round => {
                    if self.sharp_new_facets() {
                        let found = self.find_best_new(point, facet, best_outside)?;
                        self.findbestnew = true;
                        best = Some(found.facet);
                        best_dist = found.dist;
                        test_horizon = false;
                    } else {
                        self.findbest_notsharp = true;
                    }
                }
                Some(_) => {}
            }
        }
        let mut facet = if let Some(facet) = best {
            facet
        } else {
            let (facet, dist) = self.find_best_lower(last, point)?;
            best_dist = dist;
            facet
        };
        if test_horizon {
            (facet, best_dist) = self.find_best_horizon(point, facet, no_upper, best_dist);
        }
        Ok(Best {
            facet,
            dist: best_dist,
            outside: best_dist >= self.prec.min_outside,
        })
    }

    /// The greedy descent of `qh_findbest`: moves to the first better
    /// neighbour until none is better. Returns a clearly outside facet as
    /// `Ok`, else `Err` with the best lower facet, its distance and the last
    /// facet visited.
    fn descend(&mut self, point: usize, start: FacetId, rules: Descent) -> Result<Best, Stop> {
        let p = self.points[point];
        let min_outside = self.prec.min_outside;
        let clearly_outside = |hull: &Self, f: FacetId, dist: f64| {
            !rules.best_outside
                && dist >= min_outside
                && (!hull.facet(f).upperdelaunay || !rules.no_upper)
        };
        let mut best_dist = -f64::MAX / 2.0;
        let mut best = None;
        let visit = self.next_visit();
        if !self.facet(start).flipped {
            let dist = self.dist(&p, start);
            if clearly_outside(self, start, dist) {
                return Ok(Best {
                    facet: start,
                    dist,
                    outside: true,
                });
            }
            best_dist = dist;
            if !self.facet(start).upperdelaunay {
                best = Some(start);
            }
        }
        self.facet_mut(start).visit_id = visit;
        let mut cursor = Some(start);
        let mut last = start;
        while let Some(facet) = cursor {
            last = facet;
            cursor = None;
            for i in 0..self.facet(facet).neighbors.len() {
                let neighbor = self.facet(facet).neighbors[i];
                let n = self.facet(neighbor);
                if (!n.newfacet && rules.new_facets) || n.visit_id == visit {
                    continue;
                }
                self.facet_mut(neighbor).visit_id = visit;
                if self.facet(neighbor).flipped {
                    continue;
                }
                let dist = self.dist(&p, neighbor);
                if dist > best_dist {
                    if clearly_outside(self, neighbor, dist) {
                        return Ok(Best {
                            facet: neighbor,
                            dist,
                            outside: true,
                        });
                    }
                    let lower = !self.facet(neighbor).upperdelaunay;
                    if lower || best.is_none() {
                        if lower {
                            best = Some(neighbor);
                        }
                        best_dist = dist;
                        cursor = Some(neighbor);
                        break;
                    }
                }
            }
        }
        Err((best, best_dist, last))
    }

    /// Exhaustive search of the new facets from `start`, then of the
    /// horizon of the best one (qhull's `qh_findbestnew`).
    pub(super) fn find_best_new(
        &mut self,
        point: usize,
        start: FacetId,
        best_outside: bool,
    ) -> HullResult<Best> {
        if self.facet(start).next.is_none() {
            return Err(HullError::Topology);
        }
        let p = self.points[point];
        let dist_outside = (!best_outside).then(|| self.dist_outside());
        let min_outside = self.prec.min_outside;
        let mut best_dist = -f64::MAX / 2.0;
        let mut best = None;
        let visit = self.next_visit();
        for pass in 0..2 {
            let mut cursor = if pass == 0 {
                Some(start)
            } else {
                self.listed(self.newfacet_list)
            };
            while let Some(facet) = self.listed(cursor) {
                if facet == start && pass == 1 {
                    break;
                }
                self.facet_mut(facet).visit_id = visit;
                if !self.facet(facet).flipped {
                    let dist = self.dist(&p, facet);
                    if dist > best_dist && (!self.facet(facet).upperdelaunay || dist >= min_outside)
                    {
                        best = Some(facet);
                        if dist_outside.is_some_and(|limit| dist >= limit) {
                            return Ok(Best {
                                facet,
                                dist,
                                outside: true,
                            });
                        }
                        best_dist = dist;
                    }
                }
                cursor = self.facet(facet).next;
            }
        }
        let (facet, dist) = self.find_best_horizon(point, best.unwrap_or(start), false, best_dist);
        Ok(Best {
            facet,
            dist,
            outside: dist >= min_outside,
        })
    }

    /// Searches the neighbours of `start` and of the coplanar facets found
    /// on the way for a better facet (qhull's `qh_findbesthorizon` without
    /// `ischeckmax`).
    pub(super) fn find_best_horizon(
        &mut self,
        point: usize,
        start: FacetId,
        no_upper: bool,
        mut best_dist: f64,
    ) -> (FacetId, f64) {
        let p = self.points[point];
        let min_outside = self.prec.min_outside;
        let search_dist = self.search_dist();
        let mut min_search = best_dist - search_dist;
        let mut best = start;
        let mut coplanar = std::mem::take(&mut self.coplanar_facets);
        coplanar.clear();
        let visit = self.next_visit();
        self.facet_mut(start).visit_id = visit;
        let mut facet = start;
        let mut next_facet = None;
        loop {
            for i in 0..self.facet(facet).neighbors.len() {
                let neighbor = self.facet(facet).neighbors[i];
                if self.facet(neighbor).visit_id == visit {
                    continue;
                }
                self.facet_mut(neighbor).visit_id = visit;
                if !self.facet(neighbor).flipped {
                    let dist = self.dist(&p, neighbor);
                    if dist > best_dist {
                        if !self.facet(neighbor).upperdelaunay || (!no_upper && dist >= min_outside)
                        {
                            min_search = dist - search_dist;
                            if dist > best_dist + search_dist {
                                coplanar.clear();
                            }
                            best = neighbor;
                            best_dist = dist;
                        }
                    } else if dist < min_search {
                        continue;
                    }
                }
                if let Some(next) = next_facet {
                    coplanar.push(next);
                }
                next_facet = Some(neighbor);
            }
            match next_facet.take().or_else(|| coplanar.pop()) {
                Some(next) => facet = next,
                None => break,
            }
        }
        self.coplanar_facets = coplanar;
        (best, best_dist)
    }

    /// The best non-upper neighbour of upper Delaunay facet `upper`, or of
    /// its vertex nearest to `point`, or of all facets (qhull's
    /// `qh_findbestlower`).
    fn find_best_lower(&mut self, upper: FacetId, point: usize) -> HullResult<(FacetId, f64)> {
        let p = self.points[point];
        let candidates = |hull: &Self, facets: &[FacetId]| {
            let mut best: Option<(FacetId, f64)> = None;
            for &n in facets {
                let facet = hull.facet(n);
                if facet.upperdelaunay || facet.flipped {
                    continue;
                }
                let dist = hull.dist(&p, n);
                if dist > best.map_or(-f64::MAX / 2.0, |(_, d)| d) {
                    best = Some((n, dist));
                }
            }
            best
        };
        if let Some(found) = candidates(self, &self.facet(upper).neighbors) {
            return Ok(found);
        }
        let vertex = self
            .facet(upper)
            .vertices
            .iter()
            .copied()
            .fold((None, f64::MAX), |(best, best_dist), v| {
                let dist = geom::point_dist2(&self.vertex_point(v), &p, 2);
                if dist < best_dist {
                    (Some(v), dist)
                } else {
                    (best, best_dist)
                }
            })
            .0
            .ok_or(HullError::Topology)?;
        self.vertex_neighbors_all();
        if let Some(found) = candidates(self, &self.vertex(vertex).neighbors) {
            return Ok(found);
        }
        self.find_facet_all(point).ok_or(HullError::Topology)
    }

    /// The first facet clearly below `point`, or else the nearest lower
    /// facet (qhull's `qh_findfacet_all` with `noupper`).
    fn find_facet_all(&self, point: usize) -> Option<(FacetId, f64)> {
        let p = self.points[point];
        let mut best: Option<(FacetId, f64)> = None;
        for f in self.facets_from(Some(self.facet_list)) {
            let facet = self.facet(f);
            if facet.flipped || facet.normal().is_none() || facet.visible || facet.upperdelaunay {
                continue;
            }
            let dist = self.dist(&p, f);
            if dist > best.map_or(-f64::MAX, |(_, d)| d) {
                best = Some((f, dist));
                if dist > self.prec.min_outside {
                    break;
                }
            }
        }
        best
    }

    /// Whether the new facets' normals point into different octants
    /// (qhull's `qh_sharpnewfacets`).
    fn sharp_new_facets(&self) -> bool {
        let facets = self.facets_from(self.newfacet_list);
        let Some((&first, rest)) = facets.split_first() else {
            return false;
        };
        let octant = |f: FacetId| self.normal(f).map(|c| c > 0.0);
        let reference = octant(first);
        rest.iter().any(|&f| octant(f) != reference)
    }
}
