//! Assigning points to the outside and coplanar sets of facets
//! (`qh_partitionall`, `qh_partitionpoint`, `qh_partitioncoplanar`,
//! `qh_partitionvisible`) and picking the next point to add
//! (`qh_nextfurthest`, `qh_furthestnext`).

use super::geom;
use super::hull::{FacetId, Hull, HullError, HullResult, VertexId};
use super::lists::ListHead;

/// qhull's `qh_RATIOcoplanaroutside`: a coplanar point this many times
/// further than `MAXoutside` is repartitioned as an outside point.
const RATIO_COPLANAR_OUTSIDE: f64 = 30.0;

impl Hull {
    /// Distributes all points except `vertices` to the facets' outside sets,
    /// then the remaining points to coplanar sets (qhull's
    /// `qh_partitionall`).
    pub(super) fn partition_all(&mut self, vertices: &[VertexId]) -> HullResult<()> {
        let mut pending: Vec<Option<usize>> = (0..self.points.len()).map(Some).collect();
        for &v in vertices {
            pending[self.vertex(v).point] = None;
        }
        let dist_outside = self.dist_outside();
        for f in self.facets_from(Some(self.facet_list)) {
            let mut kept = Vec::with_capacity(pending.len());
            let mut best: Option<(usize, f64)> = None;
            let mut outside = Vec::new();
            for point in pending.iter().flatten().copied() {
                let dist = self.point_dist(point, f);
                if dist < dist_outside {
                    kept.push(Some(point));
                } else {
                    match best {
                        None => best = Some((point, dist)),
                        Some((previous, best_dist)) if dist > best_dist => {
                            outside.push(previous);
                            best = Some((point, dist));
                        }
                        Some(_) => outside.push(point),
                    }
                }
            }
            if let Some((point, dist)) = best {
                outside.push(point);
                self.facet_mut(f).furthest_dist = dist;
            }
            self.facet_mut(f).outside = outside;
            pending = kept;
        }
        self.findbestnew = true;
        for point in pending.into_iter().flatten() {
            self.partition_point(point, self.facet_list)?;
        }
        self.findbestnew = false;
        Ok(())
    }

    /// Assigns `point` to the outside or coplanar set of the best facet near
    /// `facet` (qhull's `qh_partitionpoint`).
    pub(super) fn partition_point(&mut self, point: usize, facet: FacetId) -> HullResult<()> {
        let best = if self.findbestnew {
            self.find_best_new(point, facet, false)?
        } else {
            self.find_best(point, facet, false, true, false)?
        };
        if self.facet(best.facet).visible {
            return Err(HullError::Topology);
        }
        let outside = best.outside || (self.narrow && best.dist >= -self.prec.max_coplanar);
        if !outside {
            return self.partition_coplanar(point, best.facet, Some(best.dist), self.findbestnew);
        }
        let bf = best.facet;
        let min_outside = self.prec.min_outside;
        let narrow = self.narrow;
        let mut new_outside = false;
        let target = self.facet_mut(bf);
        if target.outside.is_empty() {
            target.outside.push(point);
            new_outside = !narrow || best.dist > min_outside;
            target.furthest_dist = best.dist;
        } else {
            let previous = target.furthest_dist;
            if previous < best.dist {
                target.outside.push(point);
                target.furthest_dist = best.dist;
                new_outside = narrow && previous < min_outside && best.dist >= min_outside;
            } else {
                let last = target.outside.len() - 1;
                target.outside.insert(last, point);
            }
        }
        if new_outside && self.facet_next != Some(bf) {
            if self.facet(bf).newfacet {
                let next = self.facet_next.unwrap_or(FacetId::TAIL);
                if self.facet(next).newfacet {
                    self.facet_next = self.newfacet_list;
                }
            } else {
                self.move_to_end(bf);
                if self.newfacet_list.is_some() {
                    self.facet_mut(bf).newfacet = true;
                }
            }
        }
        Ok(())
    }

    /// Assigns `point`, coplanar with or inside the hull, to the coplanar
    /// set of the best facet (qhull's `qh_partitioncoplanar`). Without a
    /// known `dist` the best facet is searched for (among all new facets if
    /// `all_new`); a point that turns out clearly outside is repartitioned.
    pub(super) fn partition_coplanar(
        &mut self,
        point: usize,
        facet: FacetId,
        dist: Option<f64>,
        all_new: bool,
    ) -> HullResult<()> {
        let (best, best_dist) = if let Some(dist) = dist {
            (facet, dist)
        } else {
            let found = if all_new {
                self.find_best_new(point, facet, true)?
            } else {
                self.find_best(point, facet, true, false, true)?
            };
            (found.facet, found.dist)
        };
        if self.facet(best).visible {
            return Err(HullError::Topology);
        }
        if best_dist > self.max_outside {
            let mut repartition = dist.is_none()
                && facet != best
                && geom::angle(&self.normal(facet), &self.normal(best)) < 0.0;
            if !repartition && best_dist > self.max_outside_target * RATIO_COPLANAR_OUTSIDE {
                if facet == best {
                    if facet.id() == self.repart_facetid {
                        return Err(HullError::Topology);
                    }
                    self.repart_facetid = facet.id();
                }
                repartition = true;
            }
            if repartition {
                let previous = self.findbestnew;
                self.findbestnew = false;
                let result = self.partition_point(point, best);
                self.findbestnew = previous;
                self.repart_facetid = 0;
                return result;
            }
            self.repart_facetid = 0;
            self.max_outside = best_dist;
        }
        let furthest = self.facet(best).coplanar.last().copied();
        let append_last = furthest.is_none_or(|p| self.point_dist(p, best) < best_dist);
        let set = &mut self.facet_mut(best).coplanar;
        if append_last {
            set.push(point);
        } else {
            let last = set.len() - 1;
            set.insert(last, point);
        }
        Ok(())
    }

    /// Repartitions the outside and coplanar points of the visible facets
    /// and the points of deleted vertices into the new facets (qhull's
    /// `qh_partitionvisible`).
    pub(super) fn partition_visible(&mut self) -> HullResult<()> {
        for visible in self.visible_facets() {
            let facet = self.facet(visible);
            if facet.outside.is_empty() && facet.coplanar.is_empty() {
                continue;
            }
            let target = self
                .replacement(visible)
                .or(self.newfacet_list)
                .ok_or(HullError::Topology)?;
            if self.facet(target).next.is_none() {
                return Err(HullError::Topology);
            }
            // Points never go to visible facets: the sets are lent out
            // while their points are repartitioned.
            let outside = std::mem::take(&mut self.facet_mut(visible).outside);
            for &point in &outside {
                self.partition_point(point, target)?;
            }
            self.facet_mut(visible).outside = outside;
            let coplanar = std::mem::take(&mut self.facet_mut(visible).coplanar);
            for &point in &coplanar {
                self.partition_coplanar(point, target, None, self.findbestnew)?;
            }
            self.facet_mut(visible).coplanar = coplanar;
        }
        for i in 0..self.del_vertices.len() {
            let v = self.del_vertices[i];
            if !self.vertex(v).partitioned {
                let target = self.listed(self.newfacet_list).ok_or(HullError::Topology)?;
                self.partition_coplanar(self.vertex(v).point, target, None, true)?;
                self.vertex_mut(v).partitioned = true;
            }
        }
        Ok(())
    }

    /// Removes and returns the furthest point of the next facet with
    /// outside points, advancing `facet_next` past empty facets (qhull's
    /// `qh_nextfurthest`).
    pub(super) fn next_furthest(&mut self) -> Option<(usize, FacetId)> {
        while let Some(facet) = self.listed(self.facet_next) {
            if self.facet(facet).outside.is_empty() {
                self.facet_next = self.facet(facet).next;
                continue;
            }
            if self.narrow && self.facet(facet).furthest_dist < self.prec.min_outside {
                self.facet_next = self.facet(facet).next;
                continue;
            }
            let point = self
                .facet_mut(facet)
                .outside
                .pop()
                .expect("checked non-empty");
            return Some((point, facet));
        }
        None
    }

    /// Moves the facet with the furthest outside point to `facet_next`
    /// (qhull's `qh_furthestnext`).
    pub(super) fn furthest_next(&mut self) {
        let mut best: Option<(FacetId, f64)> = None;
        for f in self.facets_from(Some(self.facet_list)) {
            let facet = self.facet(f);
            if !facet.outside.is_empty() && facet.furthest_dist > best.map_or(-f64::MAX, |(_, d)| d)
            {
                best = Some((f, facet.furthest_dist));
            }
        }
        if let Some((f, _)) = best {
            self.remove_facet(f);
            self.prepend_facet(f, ListHead::FacetNext);
        }
    }

    /// Moves every remaining outside point to a coplanar set after the
    /// build of a narrow hull (qhull's `qh_outcoplanar`).
    pub(super) fn out_coplanar(&mut self) -> HullResult<()> {
        for f in self.facets_from(Some(self.facet_list)) {
            let mut i = 0;
            while let Some(&point) = self.facet(f).outside.get(i) {
                let dist = self.point_dist(point, f);
                self.partition_coplanar(point, f, Some(dist), self.findbestnew)?;
                i += 1;
            }
            self.facet_mut(f).outside.clear();
        }
        Ok(())
    }
}
