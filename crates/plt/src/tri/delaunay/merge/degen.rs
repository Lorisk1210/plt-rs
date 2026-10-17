//! Degenerate and redundant facets left behind by merges
//! (`qh_merge_degenredundant`, `qh_degen_redundant_facet`,
//! `qh_test_redundant_neighbors`, `qh_test_degen_neighbors`,
//! `qh_maydropneighbor`, `qh_remove_extravertices`).

use super::MergeKind;
use crate::tri::delaunay::hull::{FacetId, Hull, HullError, HullResult};
use crate::tri::delaunay::sets;

impl Hull {
    /// Merges the queued degenerate and redundant facets (qhull's
    /// `qh_merge_degenredundant`); returns the number of merges.
    pub(super) fn merge_degen_redundant(&mut self) -> HullResult<usize> {
        let mut merges = 0;
        while let Some(merge) = self.merge_sets().degen.pop() {
            let facet1 = merge.facet1;
            if self.facet(facet1).visible {
                continue;
            }
            let f1 = self.facet_mut(facet1);
            f1.degenerate = false;
            f1.redundant = false;
            if merge.kind == MergeKind::Redundant {
                let facet3 = self.replacement(merge.facet2).ok_or(HullError::Topology)?;
                if facet1 == facet3 {
                    continue;
                }
                self.merge_facet(facet1, facet3, MergeKind::Redundant, None, false)?;
                merges += 1;
                continue;
            }
            let size = self.facet(facet1).neighbors.len();
            if size == 0 {
                self.will_delete(facet1, None);
                for v in self.facet(facet1).vertices.clone() {
                    sets::delete(&mut self.vertex_mut(v).neighbors, facet1);
                    if self.vertex(v).neighbors.is_empty() {
                        self.vertex_mut(v).deleted = true;
                        self.del_vertices.push(v);
                    }
                }
                merges += 1;
            } else if size < 3 {
                let (neighbor, _, span) = self.find_best_neighbor(facet1)?;
                self.merge_facet(facet1, neighbor, merge.kind, Some(span), false)?;
                merges += 1;
            }
        }
        Ok(merges)
    }

    /// Queues `facet` if its vertices are all in a neighbour or it has too
    /// few neighbours (qhull's `qh_degen_redundant_facet`).
    pub(super) fn degen_redundant_facet(&mut self, facet: FacetId) {
        if self.facet(facet).flipped {
            return;
        }
        for neighbor in self.facet(facet).neighbors.clone() {
            if self.facet(neighbor).flipped {
                continue;
            }
            if self.vertices_within(facet, neighbor) {
                self.append_merge(facet, neighbor, MergeKind::Redundant, 0.0);
                return;
            }
        }
        if self.facet(facet).neighbors.len() < 3 {
            self.append_merge(facet, facet, MergeKind::Degen, 0.0);
        }
    }

    /// Whether every vertex of `facet` is a vertex of `container`.
    fn vertices_within(&mut self, facet: FacetId, container: FacetId) -> bool {
        let visit = self.next_vertex_visit();
        for v in self.facet(container).vertices.clone() {
            self.vertex_mut(v).visit_id = visit;
        }
        self.facet(facet)
            .vertices
            .iter()
            .all(|&v| self.vertex(v).visit_id == visit)
    }

    /// Queues `facet` if degenerate, else the neighbours whose vertices are
    /// all in `facet` (qhull's `qh_test_redundant_neighbors`).
    pub(super) fn test_redundant_neighbors(&mut self, facet: FacetId) {
        if self.facet(facet).neighbors.len() < 3 {
            self.append_merge(facet, facet, MergeKind::Degen, 0.0);
            return;
        }
        let visit = self.next_vertex_visit();
        for v in self.facet(facet).vertices.clone() {
            self.vertex_mut(v).visit_id = visit;
        }
        for neighbor in self.facet(facet).neighbors.clone() {
            let n = self.facet(neighbor);
            if n.degenerate
                || n.redundant
                || n.dupridge
                || (self.facet(facet).flipped && !n.flipped)
            {
                continue;
            }
            if n.vertices.iter().all(|&v| self.vertex(v).visit_id == visit) {
                self.append_merge(neighbor, facet, MergeKind::Redundant, 0.0);
            }
        }
    }

    /// Queues the neighbours of `facet` left with too few neighbours
    /// (qhull's `qh_test_degen_neighbors`).
    pub(super) fn test_degen_neighbors(&mut self, facet: FacetId) {
        for neighbor in self.facet(facet).neighbors.clone() {
            let n = self.facet(neighbor);
            if n.degenerate || n.redundant || n.dupridge {
                continue;
            }
            if n.neighbors.len() < 3 {
                self.append_merge(neighbor, neighbor, MergeKind::Degen, 0.0);
            }
        }
    }

    /// Drops the neighbours of non-simplicial `facet` it no longer shares a
    /// ridge with (qhull's `qh_maydropneighbor`).
    pub(super) fn may_drop_neighbor(&mut self, facet: FacetId) -> HullResult<()> {
        if self.facet(facet).simplicial {
            return Err(HullError::Topology);
        }
        let visit = self.next_visit();
        for r in self.facet(facet).ridges.clone() {
            let (top, bottom) = (self.ridge(r).top, self.ridge(r).bottom);
            self.facet_mut(top).visit_id = visit;
            self.facet_mut(bottom).visit_id = visit;
        }
        let mut i = 0;
        while let Some(&neighbor) = self.facet(facet).neighbors.get(i) {
            if self.facet(neighbor).visit_id == visit {
                i += 1;
                continue;
            }
            if self.facet(neighbor).simplicial {
                return Err(HullError::Topology);
            }
            sets::delete(&mut self.facet_mut(neighbor).neighbors, facet);
            if self.facet(neighbor).neighbors.len() < 3 {
                self.append_merge(neighbor, neighbor, MergeKind::Degen, 0.0);
            }
            sets::delete(&mut self.facet_mut(facet).neighbors, neighbor);
        }
        if self.facet(facet).neighbors.len() < 3 {
            self.append_merge(facet, facet, MergeKind::Degen, 0.0);
        }
        Ok(())
    }

    /// Removes the vertices of non-simplicial `facet` that are on none of
    /// its ridges (qhull's `qh_remove_extravertices`); returns whether any
    /// was removed.
    pub(super) fn remove_extra_vertices(&mut self, facet: FacetId) -> bool {
        if self.facet(facet).simplicial {
            return false;
        }
        for v in self.facet(facet).vertices.clone() {
            self.vertex_mut(v).seen = false;
        }
        for r in self.facet(facet).ridges.clone() {
            for v in self.ridge(r).vertices.clone() {
                self.vertex_mut(v).seen = true;
            }
        }
        let mut removed = false;
        let mut i = 0;
        while let Some(&v) = self.facet(facet).vertices.get(i) {
            if self.vertex(v).seen {
                i += 1;
                continue;
            }
            removed = true;
            self.facet_mut(facet).vertices.remove(i);
            sets::delete(&mut self.vertex_mut(v).neighbors, facet);
            if self.vertex(v).neighbors.is_empty() {
                self.vertex_mut(v).deleted = true;
                self.del_vertices.push(v);
                // qhull re-examines the index only when the vertex survives,
                // so the vertex shifted into this slot is skipped.
                i += 1;
            }
        }
        removed
    }
}
