//! Merging new facets into their coplanar horizon facet (qhull's
//! `qh_mergecycle_all` and `qh_mergecycle*`): when the added point is
//! coplanar with a horizon facet, the new facets attached to it form a
//! cycle that is merged into the horizon facet at once.

use super::{MAX_NEW_CENTRUM_VERTICES, MAX_NUMMERGE, MergeKind};
use crate::tri::delaunay::hull::{FacetId, Hull, HullError, HullResult, VertexId};
use crate::tri::delaunay::inline::InlineVec;
use crate::tri::delaunay::sets;

impl Hull {
    /// Merges every cycle of new facets into its coplanar horizon facet
    /// (qhull's `qh_mergecycle_all`).
    pub(super) fn merge_cycle_all(&mut self, wasmerge: &mut bool) -> HullResult<()> {
        let mut cycles = 0;
        let mut cursor = self.newfacet_list;
        while let Some(facet) = cursor {
            let Some(mut next) = self.facet(facet).next else {
                break;
            };
            if self.facet(facet).normal.is_some() {
                cursor = Some(next);
                continue;
            }
            if !self.facet(facet).mergehorizon {
                return Err(HullError::Topology);
            }
            let horizon = self.facet(facet).neighbors[0];
            if self.facet(facet).link == Some(facet) {
                let vertices = self.facet(facet).vertices.clone();
                for &v in &vertices[1..] {
                    self.vertex_mut(v).delridge = true;
                }
                self.facet_mut(horizon).link = None;
                self.merge_facet(facet, horizon, MergeKind::CoplanarHorizon, None, true)?;
            } else {
                let count = self.close_cycle(facet)?;
                while self.facet(next).cycledone {
                    match self.facet(next).next {
                        Some(n) => next = n,
                        None => break,
                    }
                }
                self.facet_mut(horizon).link = None;
                self.merge_cycle(facet, horizon)?;
                let nummerge = usize::from(self.facet(horizon).nummerge) + count;
                self.facet_mut(horizon).nummerge =
                    u16::try_from(nummerge).map_or(MAX_NUMMERGE, |n| n.min(MAX_NUMMERGE));
            }
            cycles += 1;
            cursor = Some(next);
        }
        if cycles > 0 {
            for f in self.facets_from(self.newfacet_list) {
                if self.facet(f).coplanarhorizon {
                    self.test_redundant_neighbors(f);
                    self.facet_mut(f).coplanarhorizon = false;
                }
            }
            self.merge_degen_redundant()?;
            *wasmerge = true;
        }
        Ok(())
    }

    /// Marks the cycle starting at `facet` done and drops the facets that
    /// already have a hyperplane from it; returns the number of facets left.
    fn close_cycle(&mut self, facet: FacetId) -> HullResult<usize> {
        let mut count = 0;
        let mut previous = facet;
        let mut same = self.facet(facet).link;
        while let Some(s) = same {
            let next = self.facet(s).link;
            if self.facet(s).cycledone || self.facet(s).visible {
                return Err(HullError::Topology);
            }
            self.facet_mut(s).cycledone = true;
            if self.facet(s).normal.is_some() {
                let link = self.facet(s).link;
                self.facet_mut(previous).link = link;
                self.facet_mut(s).link = None;
            } else {
                previous = s;
                count += 1;
            }
            same = if s == facet { None } else { next };
        }
        Ok(count)
    }

    /// The facets of the cycle through `samecycle`, starting after it and
    /// ending with it (qhull's `FORALLsame_cycle_`).
    fn same_cycle(&self, samecycle: FacetId) -> Vec<FacetId> {
        let mut out = Vec::new();
        let mut same = self.facet(samecycle).link;
        while let Some(s) = same {
            out.push(s);
            same = if s == samecycle {
                None
            } else {
                self.facet(s).link
            };
        }
        out
    }

    /// Merges the cycle of new facets through `samecycle` into `newfacet`,
    /// their coplanar horizon facet (qhull's `qh_mergecycle`).
    fn merge_cycle(&mut self, samecycle: FacetId, newfacet: FacetId) -> HullResult<()> {
        self.total_merges += 1;
        self.vertex_neighbors_all();
        let apex = self.facet(samecycle).vertices[0];
        self.make_ridges(newfacet)?;
        let cycle = self.same_cycle(samecycle);
        self.merge_cycle_neighbors(&cycle, newfacet)?;
        self.merge_cycle_ridges(&cycle, newfacet)?;
        self.merge_cycle_vertex_neighbors(&cycle, apex, newfacet);
        if self.facet(newfacet).vertices.first() != Some(&apex) {
            self.facet_mut(newfacet).vertices.insert(0, apex);
        }
        if !self.facet(newfacet).newfacet {
            let vertices = self.facet(newfacet).vertices.clone();
            self.renew_vertices(&vertices);
        }
        self.merge_cycle_facets(&cycle, newfacet);
        Ok(())
    }

    /// Replaces the cycle's facets by `newfacet` in their neighbours' sets
    /// (qhull's `qh_mergecycle_neighbors`).
    fn merge_cycle_neighbors(&mut self, cycle: &[FacetId], newfacet: FacetId) -> HullResult<()> {
        let same_visit = self.next_visit();
        for &s in cycle {
            if self.facet(s).visit_id == same_visit || self.facet(s).visible {
                return Err(HullError::Topology);
            }
            self.facet_mut(s).visit_id = same_visit;
        }
        let visit = self.next_visit();
        self.facet_mut(newfacet).visit_id = visit;
        let mut kept = InlineVec::new();
        for n in self.facet(newfacet).neighbors.clone() {
            if self.facet(n).visit_id != same_visit {
                self.facet_mut(n).visit_id = visit;
                kept.push(n);
            }
        }
        self.facet_mut(newfacet).neighbors = kept;
        for &same in cycle {
            for neighbor in self.facet(same).neighbors.clone() {
                if self.facet(neighbor).visit_id == same_visit {
                    continue;
                }
                if self.facet(neighbor).simplicial {
                    if self.facet(neighbor).visit_id == visit {
                        self.make_ridges(neighbor)?;
                        sets::delete(&mut self.facet_mut(neighbor).neighbors, same);
                    } else {
                        self.facet_mut(newfacet).neighbors.push(neighbor);
                        sets::replace(&mut self.facet_mut(neighbor).neighbors, same, newfacet);
                        self.facet_mut(neighbor).visit_id = visit;
                        let ridge = self.facet(neighbor).ridges.iter().copied().find(|&r| {
                            let ridge = self.ridge(r);
                            ridge.top == same || ridge.bottom == same
                        });
                        if let Some(r) = ridge {
                            let ridge = self.ridge_mut(r);
                            if ridge.top == same {
                                ridge.top = newfacet;
                            } else {
                                ridge.bottom = newfacet;
                            }
                        }
                    }
                } else {
                    sets::delete(&mut self.facet_mut(neighbor).neighbors, same);
                    if self.facet(neighbor).visit_id != visit {
                        self.facet_mut(neighbor).neighbors.push(newfacet);
                        self.facet_mut(newfacet).neighbors.push(neighbor);
                        self.facet_mut(neighbor).visit_id = visit;
                    }
                }
            }
        }
        Ok(())
    }

    /// Moves the cycle's ridges to `newfacet`, dropping the ridges inside
    /// the cycle and adding ridges to simplicial neighbours (qhull's
    /// `qh_mergecycle_ridges`).
    fn merge_cycle_ridges(&mut self, cycle: &[FacetId], newfacet: FacetId) -> HullResult<()> {
        let same_visit = self.visit_id - 1;
        let ridges: Vec<_> = self
            .facet(newfacet)
            .ridges
            .iter()
            .copied()
            .filter(|&r| self.facet(self.ridge(r).other(newfacet)).visit_id != same_visit)
            .collect();
        self.facet_mut(newfacet).ridges = ridges;
        for &same in cycle {
            for r in self.facet(same).ridges.clone() {
                let ridge = self.ridge(r);
                let neighbor = if ridge.top == same {
                    self.ridge_mut(r).top = newfacet;
                    self.ridge(r).bottom
                } else if ridge.bottom == same {
                    self.ridge_mut(r).bottom = newfacet;
                    self.ridge(r).top
                } else if ridge.top == newfacet || ridge.bottom == newfacet {
                    self.push_ridge(newfacet, r);
                    continue;
                } else {
                    return Err(HullError::Topology);
                };
                if neighbor == newfacet {
                    continue;
                }
                if self.facet(neighbor).visit_id == same_visit {
                    sets::delete(&mut self.facet_mut(neighbor).ridges, r);
                } else {
                    self.push_ridge(newfacet, r);
                }
            }
            self.facet_mut(same).ridges.clear();
            if !self.facet(same).simplicial {
                continue;
            }
            let neighbors = self.facet(same).neighbors.clone();
            for (i, &neighbor) in neighbors.iter().enumerate() {
                if self.facet(neighbor).visit_id == same_visit || !self.facet(neighbor).simplicial {
                    continue;
                }
                let vertices = sets::without_nth(&self.facet(same).vertices, i);
                let (top, bottom) = if self.facet(same).toporient ^ (i & 1 == 1) {
                    (newfacet, neighbor)
                } else {
                    (neighbor, newfacet)
                };
                let r = self.new_ridge(vertices, top, bottom);
                self.push_ridge(newfacet, r);
                self.push_ridge(neighbor, r);
            }
        }
        Ok(())
    }

    /// Replaces the cycle's facets by `newfacet` in the neighbour sets of
    /// their vertices, deleting vertices left in no other facet (qhull's
    /// `qh_mergecycle_vneighbors`).
    fn merge_cycle_vertex_neighbors(
        &mut self,
        cycle: &[FacetId],
        apex: VertexId,
        newfacet: FacetId,
    ) {
        let merge_id = self.visit_id - 1;
        self.facet_mut(newfacet).visit_id = merge_id;
        let mut vertices = self.base_vertices(cycle, apex);
        vertices.push(apex);
        for v in vertices {
            self.vertex_mut(v).delridge = true;
            let facets = &self.facets;
            let neighbors = &mut self.vertices[v.index()].neighbors;
            neighbors.retain(|n| facets[n.index()].visit_id != merge_id);
            neighbors.push(newfacet);
            if neighbors.len() < 2 {
                sets::delete_sorted(&mut self.facet_mut(newfacet).vertices, v);
                self.vertex_mut(v).deleted = true;
                self.del_vertices.push(v);
            }
        }
    }

    /// The vertices of the cycle's facets other than the apex, in order of
    /// first appearance (qhull's `qh_basevertices`).
    fn base_vertices(&mut self, cycle: &[FacetId], apex: VertexId) -> Vec<VertexId> {
        let visit = self.next_vertex_visit();
        self.vertex_mut(apex).visit_id = visit;
        let mut vertices = Vec::new();
        for &same in cycle {
            if self.facet(same).mergeridge {
                continue;
            }
            for v in self.facet(same).vertices.clone() {
                let vertex = self.vertex_mut(v);
                if vertex.visit_id != visit {
                    vertex.visit_id = visit;
                    vertex.seen = false;
                    vertices.push(v);
                }
            }
        }
        vertices
    }

    /// Moves `newfacet` to the end of the new facets and deletes the cycle
    /// (qhull's `qh_mergecycle_facets`).
    fn merge_cycle_facets(&mut self, cycle: &[FacetId], newfacet: FacetId) {
        self.move_to_end(newfacet);
        let facet = self.facet_mut(newfacet);
        facet.newfacet = true;
        facet.simplicial = false;
        facet.newmerge = true;
        for &same in cycle {
            self.will_delete(same, Some(newfacet));
        }
        if self.facet(newfacet).center.is_some()
            && self.facet(newfacet).vertices.len() <= MAX_NEW_CENTRUM_VERTICES
        {
            self.facet_mut(newfacet).center = None;
        }
    }
}
