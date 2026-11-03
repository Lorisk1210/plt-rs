//! Merges that are required for a valid hull: duplicate ridges and flipped
//! facets (`qh_mark_dupridges`, `qh_forcedmerges`, `qh_flippedmerges`), and
//! the choice of the neighbour to merge into (`qh_findbestneighbor`).

use super::{MergeKind, Span};
use crate::tri::delaunay::hull::{FacetId, Hull, HullError, HullResult};
use crate::tri::delaunay::precision::QhullMinMax;

/// qhull's `qh_WIDEduplicate`: a duplicate-ridge merge this many times wider
/// than the hull's precision is an error.
const WIDE_DUPLICATE: f64 = 100.0;

/// qhull's `qh_BESTcentrum` and `qh_BESTcentrum2`: facets with more
/// vertices than `2 * dim + 20` compare neighbours by centrum.
const BEST_CENTRUM_VERTICES: usize = 2 * 3 + 20;

/// qhull's `qh_BESTnonconvex`: facets with more vertices than `dim + 15`
/// first try their non-convex neighbours.
const BEST_NONCONVEX_VERTICES: usize = 3 + 15;

impl Hull {
    /// Queues merges for duplicate ridges among the new facets and makes the
    /// ridges of the facets involved explicit (qhull's `qh_mark_dupridges`
    /// with `allmerges`).
    pub(super) fn mark_dupridges(&mut self) -> HullResult<()> {
        let newfacets = self.facets_from(self.newfacet_list);
        for &f in &newfacets {
            let facet = self.facet_mut(f);
            facet.mergeridge = false;
            facet.mergeridge2 = false;
        }
        let mut merges = 0;
        for &f in &newfacets {
            if !self.facet(f).dupridge {
                continue;
            }
            for neighbor in self.facet(f).neighbors.clone() {
                if neighbor == FacetId::MERGE_RIDGE {
                    self.facet_mut(f).mergeridge = true;
                    continue;
                }
                if !self.facet(neighbor).dupridge {
                    continue;
                }
                let one_sided = !self.facet(neighbor).neighbors.contains(&f);
                if one_sided || self.facet(f).vertices == self.facet(neighbor).vertices {
                    self.append_merge(f, neighbor, MergeKind::Dupridge, 0.0);
                    let facet = self.facet_mut(f);
                    facet.mergeridge = true;
                    facet.mergeridge2 = true;
                    merges += 1;
                    if !one_sided {
                        break;
                    }
                }
            }
        }
        if merges == 0 {
            return Ok(());
        }
        for &f in &newfacets {
            if self.facet(f).mergeridge && !self.facet(f).mergeridge2 {
                self.make_ridges(f)?;
            }
        }
        let dupridges: Vec<_> = self
            .merge_sets()
            .facet
            .iter()
            .filter(|m| m.kind == MergeKind::Dupridge)
            .map(|m| (m.facet1, m.facet2))
            .collect();
        for (facet1, facet2) in dupridges {
            if self.facet(facet2).mergeridge2 && self.facet(facet2).neighbors.contains(&facet1) {
                return Err(HullError::Topology);
            }
            self.facet_mut(facet2).neighbors.push(facet1);
            self.make_ridges(facet1)?;
        }
        Ok(())
    }

    /// Merges the facets of each duplicate ridge, the narrower way (qhull's
    /// `qh_forcedmerges`).
    pub(super) fn forced_merges(&mut self, wasmerge: &mut bool) -> HullResult<()> {
        let others = std::mem::take(&mut self.merge_sets().facet);
        let mut wasdupridge = false;
        let mut merged = false;
        for merge in others.iter().filter(|m| m.kind == MergeKind::Dupridge) {
            wasdupridge = true;
            let facet1 = self.replacement(merge.facet1).ok_or(HullError::Topology)?;
            let facet2 = self.replacement(merge.facet2).ok_or(HullError::Topology)?;
            if facet1 == facet2 {
                continue;
            }
            if !self.facet(facet2).neighbors.contains(&facet1) {
                return Err(HullError::Topology);
            }
            let (min1, max1, dist1) = self.get_distance(facet1, facet2);
            let (min2, max2, dist2) = self.get_distance(facet2, facet1);
            self.check_dupridge(dist1, dist2)?;
            let narrow = self.prec.narrow_dupridge();
            let (flipped1, flipped2) = (self.facet(facet1).flipped, self.facet(facet2).flipped);
            let first_into_second = if dist1 < dist2 {
                !(flipped2 && !flipped1 && dist2 < narrow)
            } else {
                flipped1 && !flipped2 && dist1 < narrow
            };
            if first_into_second {
                self.merge_facet(
                    facet1,
                    facet2,
                    MergeKind::Dupridge,
                    Some((min1, max1)),
                    false,
                )?;
            } else {
                self.merge_facet(
                    facet2,
                    facet1,
                    MergeKind::Dupridge,
                    Some((min2, max2)),
                    false,
                )?;
            }
            self.merge_degen_redundant()?;
            merged = true;
        }
        let rest = others.into_iter().filter(|m| m.kind != MergeKind::Dupridge);
        self.merge_sets().facet.extend(rest);
        if wasdupridge {
            for f in self.facets_from(self.newfacet_list) {
                if !self.facet(f).dupridge {
                    continue;
                }
                let facet = self.facet_mut(f);
                facet.dupridge = false;
                facet.mergeridge = false;
                facet.mergeridge2 = false;
                if facet.neighbors.len() < 3 {
                    self.append_merge(f, f, MergeKind::Degen, 0.0);
                }
            }
            self.merge_degen_redundant()?;
        }
        *wasmerge |= merged;
        Ok(())
    }

    /// Fails if a duplicate-ridge merge would be far wider than the hull's
    /// precision (qhull's `qh_check_dupridge`).
    fn check_dupridge(&self, dist1: f64, dist2: f64) -> HullResult<()> {
        let merge_dist = if dist1 > dist2 { dist2 } else { dist1 };
        let outer = self.max_outside.max_qhull(self.prec.dist_round) + self.prec.dist_round;
        let inner = self.min_vertex - self.prec.dist_round;
        let previous = outer
            .max_qhull(inner)
            .max_qhull(self.prec.one_merge + self.prec.dist_round)
            .max_qhull(self.prec.min_outside + self.prec.dist_round);
        if merge_dist / previous > WIDE_DUPLICATE {
            return Err(HullError::Precision);
        }
        Ok(())
    }

    /// Merges each flipped new facet into its best neighbour (qhull's
    /// `qh_flippedmerges`).
    pub(super) fn flipped_merges(&mut self, wasmerge: &mut bool) -> HullResult<()> {
        for f in self.facets_from(self.newfacet_list) {
            if self.facet(f).flipped && !self.facet(f).visible {
                self.append_merge(f, f, MergeKind::Flip, 0.0);
            }
        }
        let others = std::mem::take(&mut self.merge_sets().facet);
        let mut merged = false;
        for merge in &others {
            if merge.kind != MergeKind::Flip || self.facet(merge.facet1).visible {
                continue;
            }
            let (neighbor, _, span) = self.find_best_neighbor(merge.facet1)?;
            self.merge_facet(merge.facet1, neighbor, MergeKind::Flip, Some(span), false)?;
            merged = true;
        }
        for merge in others {
            if !self.facet(merge.facet1).visible && !self.facet(merge.facet2).visible {
                self.merge_sets().facet.push(merge);
            }
        }
        self.merge_degen_redundant()?;
        *wasmerge |= merged;
        Ok(())
    }

    /// The neighbour of `facet` with the narrowest merge, the merge width
    /// and its vertex distance span (qhull's `qh_findbestneighbor`).
    pub(super) fn find_best_neighbor(
        &mut self,
        facet: FacetId,
    ) -> HullResult<(FacetId, f64, Span)> {
        let size = self.facet(facet).vertices.len();
        let by_centrum = size > BEST_CENTRUM_VERTICES;
        if by_centrum {
            self.cached_centrum(facet);
        }
        let mut best: Option<(FacetId, f64, Span)> = None;
        if size > BEST_NONCONVEX_VERTICES {
            for r in self.facet(facet).ridges.clone() {
                if self.ridge(r).nonconvex {
                    let neighbor = self.ridge(r).other(facet);
                    self.test_best_neighbor(by_centrum, facet, neighbor, &mut best);
                }
            }
        }
        if best.is_none() {
            for neighbor in self.facet(facet).neighbors.clone() {
                self.test_best_neighbor(by_centrum, facet, neighbor, &mut best);
            }
        }
        let (neighbor, dist, mut span) = best.ok_or(HullError::Topology)?;
        if by_centrum {
            let (min, max, _) = self.get_distance(facet, neighbor);
            span = (min, max);
        }
        Ok((neighbor, dist, span))
    }

    /// Replaces `best` if merging `facet` into `neighbor` is narrower
    /// (qhull's `qh_findbest_test`).
    fn test_best_neighbor(
        &mut self,
        by_centrum: bool,
        facet: FacetId,
        neighbor: FacetId,
        best: &mut Option<(FacetId, f64, Span)>,
    ) {
        if self.facet(facet).flipped
            && self.facet(neighbor).flipped
            && best.is_some_and(|(b, _, _)| !self.facet(b).flipped)
        {
            return;
        }
        let (dist, span) = if by_centrum {
            let center = *self
                .facet(facet)
                .center
                .as_deref()
                .expect("centrum computed for the test");
            let dist = self.dist(&center, neighbor) * 3.0;
            if dist < 0.0 {
                (-dist, (dist, 0.0))
            } else {
                (dist, (0.0, dist))
            }
        } else {
            let (min, max, dist) = self.get_distance(facet, neighbor);
            (dist, (min, max))
        };
        if dist < best.map_or(f64::MAX, |(_, d, _)| d) {
            *best = Some((neighbor, dist, span));
        }
    }
}
