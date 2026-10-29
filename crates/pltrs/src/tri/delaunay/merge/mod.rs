//! Merging non-convex facets after each cone (qhull's `merge_r.c`, as
//! configured for 3-d Delaunay with a zero centrum radius, `C-0`).
//!
//! Coplanar and concave new facets are merged into their neighbours so
//! that the hull stays convex despite roundoff; cocircular input sites
//! produce such merges, and the merged (non-simplicial) facets are later
//! triangulated. This module holds the merge queues and the driver
//! (`qh_premerge`, `qh_all_merges`, `qh_getmergeset`); the submodules
//! implement the individual merges.

mod cycle;
mod degen;
mod facet;
mod forced;
mod pinched;
mod rename;

use std::cmp::Ordering;

use super::hull::{FacetId, Hull, HullResult, RidgeId, VertexId};

/// qhull's `qh_MAXnummerge`: the merge count of a facet saturates here.
const MAX_NUMMERGE: u16 = 511;

/// qhull's `qh_MAXnewcentrum`: a merged facet with more than `dim + 5`
/// vertices keeps its centrum, smaller ones recompute it.
const MAX_NEW_CENTRUM_VERTICES: usize = 3 + 5;

/// qhull's `mergeType`, in qhull's order: lower kinds are merged first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MergeKind {
    /// Coplanar facets (centrum within the centrum radius).
    Coplanar,
    /// Concave facets (centrum above the neighbour).
    Concave,
    /// Both concave and coplanar.
    ConcaveCoplanar,
    /// A flipped facet.
    Flip,
    /// A duplicate ridge between new facets.
    Dupridge,
    /// A facet with too few neighbours.
    Degen,
    /// A facet whose vertices are contained in a neighbour.
    Redundant,
    /// Two facets with the same vertices.
    Mirror,
    /// A new facet coplanar with its horizon facet.
    CoplanarHorizon,
}

/// A queued facet merge (qhull's `mergeT` for facets).
#[derive(Clone, Copy, Debug)]
pub(super) struct FacetMerge {
    /// The facet to merge.
    pub(super) facet1: FacetId,
    /// The facet to merge into (or `facet1` for degenerate merges).
    pub(super) facet2: FacetId,
    /// Why.
    pub(super) kind: MergeKind,
    /// Width of the merge, for ordering.
    pub(super) distance: f64,
}

/// A queued merge of the two vertices of duplicate ridges (qhull's
/// `mergeT` of type `MRGvertices`).
#[derive(Clone, Copy, Debug)]
pub(super) struct VertexMerge {
    /// The vertex to rename.
    pub(super) vertex1: VertexId,
    /// Its new name.
    pub(super) vertex2: VertexId,
    /// Distance between the vertices.
    pub(super) distance: f64,
    /// The duplicate ridges.
    pub(super) ridges: (RidgeId, RidgeId),
}

/// The distance range `(min, max)` of a facet's vertices to the facet it
/// merges into.
pub(super) type Span = (f64, f64);

/// The merge queues (`qh.facet_mergeset`, `qh.degen_mergeset`,
/// `qh.vertex_mergeset`), processed from the end.
#[derive(Clone, Debug, Default)]
pub(super) struct MergeSets {
    /// Coplanar, concave and forced merges.
    pub(super) facet: Vec<FacetMerge>,
    /// Degenerate and redundant facets.
    pub(super) degen: Vec<FacetMerge>,
    /// Pinched vertices.
    pub(super) vertex: Vec<VertexMerge>,
}

/// qhull's `qh_compare_facetmerge` as a sort order: merges processed first
/// (from the end of the queue) sort last.
fn merge_order(a: &FacetMerge, b: &FacetMerge) -> Ordering {
    if a.kind == b.kind {
        b.distance
            .partial_cmp(&a.distance)
            .unwrap_or(Ordering::Equal)
    } else {
        b.kind.cmp(&a.kind)
    }
}

impl Hull {
    /// The merge queues; merging is active between `premerge` and the end
    /// of the point's insertion.
    pub(super) fn merge_sets(&mut self) -> &mut MergeSets {
        self.merges.get_or_insert_with(MergeSets::default)
    }

    /// Merges the non-convex facets of the new cone (qhull's
    /// `qh_premerge`).
    pub(super) fn premerge(&mut self) -> HullResult<()> {
        self.merges = Some(MergeSets::default());
        if self.check_zero() {
            return Ok(());
        }
        let mut othermerge = false;
        self.mark_dupridges()?;
        self.merge_cycle_all(&mut othermerge)?;
        self.forced_merges(&mut othermerge)?;
        self.flipped_merges(&mut othermerge)?;
        self.get_merge_set_initial();
        self.all_merges(othermerge)
    }

    /// Whether the new facets are clearly convex with their neighbours, so
    /// no merging is needed (qhull's `qh_checkzero` for the new facets).
    fn check_zero(&mut self) -> bool {
        let newfacets = self.facets_from(self.newfacet_list);
        for &f in &newfacets {
            let facet = self.facet(f);
            if !self.facet(facet.neighbors[0]).simplicial
                || facet.flipped
                || facet.dupridge
                || facet.normal.is_none()
            {
                return false;
            }
        }
        let limit = -2.0 * self.prec.dist_round;
        for &f in &newfacets {
            let visit = self.next_vertex_visit();
            let neighbors = self.facet(f).neighbors.clone();
            for (i, &neighbor) in neighbors.iter().enumerate().skip(1) {
                let v = self.facet(f).vertices[i];
                self.vertex_mut(v).visit_id = visit;
                if self.dist(&self.vertex_point(v), neighbor) >= limit {
                    return false;
                }
            }
            let horizon = neighbors[0];
            let opposite = self
                .facet(horizon)
                .vertices
                .iter()
                .copied()
                .find(|&v| self.vertex(v).visit_id != visit);
            if let Some(v) = opposite
                && self.dist(&self.vertex_point(v), f) >= limit
            {
                return false;
            }
        }
        true
    }

    /// Queues a facet merge (qhull's `qh_appendmergeset`).
    pub(super) fn append_merge(
        &mut self,
        facet: FacetId,
        neighbor: FacetId,
        kind: MergeKind,
        distance: f64,
    ) {
        if (self.facet(facet).redundant && kind != MergeKind::Mirror)
            || self.facet(neighbor).redundant
        {
            return;
        }
        if self.facet(facet).degenerate && kind == MergeKind::Degen {
            return;
        }
        let merge = FacetMerge {
            facet1: facet,
            facet2: neighbor,
            kind,
            distance,
        };
        match kind {
            MergeKind::Degen => {
                self.facet_mut(facet).degenerate = true;
                let degen = &mut self.merge_sets().degen;
                if degen
                    .last()
                    .is_none_or(|last| last.kind == MergeKind::Degen)
                {
                    degen.push(merge);
                } else {
                    degen.insert(0, merge);
                }
            }
            MergeKind::Redundant => {
                self.facet_mut(facet).redundant = true;
                self.merge_sets().degen.push(merge);
            }
            MergeKind::Mirror => {
                self.facet_mut(facet).redundant = true;
                self.facet_mut(neighbor).redundant = true;
                self.merge_sets().degen.push(merge);
            }
            _ => self.merge_sets().facet.push(merge),
        }
    }

    /// Queues a merge if `facet` and `neighbor` are coplanar or concave by
    /// the centrum test (qhull's `qh_test_appendmerge` and
    /// `qh_test_centrum_merge`).
    fn test_append_merge(&mut self, facet: FacetId, neighbor: FacetId) -> bool {
        let radius = self.prec.centrum_radius;
        let center = self.cached_centrum(facet);
        let dist = self.dist(&center, neighbor);
        let mut concave = dist > radius;
        let mut coplanar = !concave && dist >= -radius;
        let center = self.cached_centrum(neighbor);
        let dist2 = self.dist(&center, facet);
        if dist2 > radius {
            concave = true;
        } else if !coplanar && dist2 >= -radius {
            coplanar = true;
        }
        if !concave && !coplanar {
            return false;
        }
        if concave && coplanar {
            if dist > dist2 {
                self.append_merge(facet, neighbor, MergeKind::ConcaveCoplanar, dist);
            } else {
                self.append_merge(neighbor, facet, MergeKind::ConcaveCoplanar, dist2);
            }
        } else if concave {
            let width = if dist < dist2 { dist2 } else { dist };
            self.append_merge(facet, neighbor, MergeKind::Concave, width);
        } else {
            let (a, b) = (dist.abs(), dist2.abs());
            self.append_merge(
                facet,
                neighbor,
                MergeKind::Coplanar,
                if a > b { b } else { a },
            );
        }
        true
    }

    /// Sorts the facet merges so the most urgent is last.
    fn sort_merges(&mut self) {
        self.merge_sets().facet.sort_by(merge_order);
    }

    /// Tests every new facet against its neighbours (qhull's
    /// `qh_getmergeset_initial`).
    fn get_merge_set_initial(&mut self) {
        let visit = self.next_visit();
        for f in self.facets_from(self.newfacet_list) {
            self.facet_mut(f).visit_id = visit;
            for i in 0..self.facet(f).neighbors.len() {
                let neighbor = self.facet(f).neighbors[i];
                if self.facet(neighbor).visit_id == visit || !self.test_append_merge(f, neighbor) {
                    continue;
                }
                if let Some(&ridge) = self
                    .facet(neighbor)
                    .ridges
                    .iter()
                    .find(|&&r| self.ridge(r).other(neighbor) == f)
                {
                    self.ridge_mut(ridge).nonconvex = true;
                }
            }
            self.facet_mut(f).tested = true;
            for r in self.facet(f).ridges.clone() {
                self.ridge_mut(r).tested = true;
            }
        }
        self.sort_merges();
    }

    /// Tests the untested ridges of the untested new facets (qhull's
    /// `qh_getmergeset`).
    pub(super) fn get_merge_set(&mut self) {
        let visit = self.next_visit();
        for f in self.facets_from(self.newfacet_list) {
            if self.facet(f).tested {
                continue;
            }
            self.facet_mut(f).visit_id = visit;
            for n in self.facet(f).neighbors.clone() {
                self.facet_mut(n).seen = false;
            }
            for r in self.facet(f).ridges.clone() {
                let ridge = self.ridge(r);
                if ridge.tested && !ridge.nonconvex {
                    continue;
                }
                let neighbor = ridge.other(f);
                if self.facet(neighbor).seen {
                    let ridge = self.ridge_mut(r);
                    ridge.tested = true;
                    ridge.nonconvex = false;
                } else if self.facet(neighbor).visit_id != visit {
                    self.facet_mut(neighbor).seen = true;
                    self.ridge_mut(r).nonconvex = false;
                    let nonconvex = self.test_append_merge(f, neighbor);
                    let ridge = self.ridge_mut(r);
                    ridge.nonconvex = nonconvex;
                    ridge.tested = true;
                }
            }
            self.facet_mut(f).tested = true;
        }
        self.sort_merges();
    }

    /// Performs the queued merges until none are left, reducing vertices in
    /// between (qhull's `qh_all_merges`).
    pub(super) fn all_merges(&mut self, mut othermerge: bool) -> HullResult<()> {
        let mut wasmerge;
        loop {
            wasmerge = false;
            while !self.merge_sets().facet.is_empty() || !self.merge_sets().degen.is_empty() {
                if !self.merge_sets().degen.is_empty() {
                    self.merge_degen_redundant()?;
                    wasmerge = true;
                }
                while let Some(merge) = self.merge_sets().facet.pop() {
                    let (f1, f2) = (self.facet(merge.facet1), self.facet(merge.facet2));
                    if f1.visible || f2.visible {
                        continue;
                    }
                    if merge.kind == MergeKind::Coplanar
                        && ((!f1.tested && f1.newfacet) || (!f2.tested && f2.newfacet))
                    {
                        continue;
                    }
                    self.merge_nonconvex(merge.facet1, merge.facet2, merge.kind)?;
                    self.merge_degen_redundant()?;
                    wasmerge = true;
                }
                self.get_merge_set();
            }
            if self.vertex_neighbors && (wasmerge || othermerge) {
                wasmerge = false;
                othermerge = false;
                if self.reduce_vertices()? {
                    self.get_merge_set();
                    continue;
                }
            }
            break;
        }
        if wasmerge || othermerge {
            for f in self.facets_from(self.newfacet_list) {
                self.facet_mut(f).newmerge = false;
            }
        }
        Ok(())
    }

    /// Merges a coplanar or concave pair by merging the facet with the
    /// closer best neighbour (qhull's `qh_merge_nonconvex`).
    fn merge_nonconvex(
        &mut self,
        facet1: FacetId,
        facet2: FacetId,
        kind: MergeKind,
    ) -> HullResult<()> {
        let (best, other) = if self.facet(facet1).newfacet {
            (facet1, facet2)
        } else {
            (facet2, facet1)
        };
        let (best_neighbor, dist, span) = self.find_best_neighbor(best)?;
        let (neighbor, dist2, span2) = self.find_best_neighbor(other)?;
        if dist < dist2 {
            self.merge_facet(best, best_neighbor, kind, Some(span), false)
        } else {
            self.merge_facet(other, neighbor, kind, Some(span2), false)
        }
    }
}
