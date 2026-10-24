//! Merging one facet into another (`qh_mergefacet`, `qh_mergesimplex`,
//! `qh_mergeneighbors`, `qh_mergevertices`, `qh_mergeridges`) and making
//! the ridges of a simplicial facet explicit (`qh_makeridges`).

use super::{MAX_NEW_CENTRUM_VERTICES, MAX_NUMMERGE, MergeKind, Span};
use crate::tri::delaunay::hull::{FacetId, Hull, HullError, HullResult, RidgeId, VertexId};
use crate::tri::delaunay::inline::InlineVec;
use crate::tri::delaunay::precision::QhullMinMax;
use crate::tri::delaunay::sets;

/// qhull's `qh_WIDEmaxoutside`: a merge wider than this many `ONEmerge` is
/// a precision error.
const WIDE_MAX_OUTSIDE: f64 = 100.0;

impl Hull {
    /// Merges `facet1` into `facet2` (qhull's `qh_mergefacet`). `span` is
    /// the distance range of `facet1`'s vertices to `facet2`, if known;
    /// `merge_apex` merges a new facet into its horizon facet.
    pub(super) fn merge_facet(
        &mut self,
        facet1: FacetId,
        facet2: FacetId,
        kind: MergeKind,
        span: Option<Span>,
        merge_apex: bool,
    ) -> HullResult<()> {
        self.total_merges += 1;
        if let Some((min, max)) = span {
            let limit = (WIDE_MAX_OUTSIDE * self.prec.one_merge)
                .max_qhull(self.facet(facet1).max_outside)
                .max_qhull(self.facet(facet2).max_outside);
            if max > limit || -min > limit {
                return Err(HullError::Precision);
            }
        }
        if facet1 == facet2 || self.facet(facet1).visible || self.facet(facet2).visible {
            return Err(HullError::Topology);
        }
        if self.num_facets - self.num_visible <= 4 {
            return Err(HullError::Topology);
        }
        self.vertex_neighbors_all();
        self.make_ridges(facet1)?;
        self.make_ridges(facet2)?;
        if let Some((min, max)) = span {
            self.max_outside = self.max_outside.max_qhull(max);
            let wide = self.prec.wide_facet;
            let f2 = self.facet_mut(facet2);
            f2.max_outside = f2.max_outside.max_qhull(max);
            if self.min_vertex > min {
                self.min_vertex = min;
            }
            let f2 = self.facet_mut(facet2);
            if !f2.keepcentrum && (max > wide || min < -wide) {
                f2.keepcentrum = true;
            }
        }
        let nummerge = self.facet(facet1).nummerge + self.facet(facet2).nummerge + 1;
        let f2 = self.facet_mut(facet2);
        f2.nummerge = nummerge.min(MAX_NUMMERGE);
        f2.newmerge = true;
        f2.dupridge = false;
        self.update_tested(facet1, facet2);
        if self.facet(facet1).vertices.len() == 3 {
            self.merge_simplex(facet1, facet2, merge_apex)?;
        } else {
            let visit = self.next_vertex_visit();
            for v in self.facet(facet2).vertices.clone() {
                self.vertex_mut(v).visit_id = visit;
            }
            self.merge_neighbors(facet1, facet2)?;
            self.merge_vertices(facet1, facet2);
            self.merge_ridges(facet1, facet2);
            self.merge_vertex_neighbors(facet1, facet2);
            if !self.facet(facet2).newfacet {
                let vertices = self.facet(facet2).vertices.clone();
                self.renew_vertices(&vertices);
            }
        }
        self.move_to_end(facet2);
        let f2 = self.facet_mut(facet2);
        f2.newfacet = true;
        f2.tested = false;
        if kind != MergeKind::CoplanarHorizon {
            self.test_redundant_neighbors(facet2);
            self.test_degen_neighbors(facet1);
            self.degen_redundant_facet(facet2);
        }
        self.will_delete(facet1, Some(facet2));
        Ok(())
    }

    /// Clears the tested flags touched by merging `facet1` into `facet2`
    /// and decides whether `facet2` keeps its centrum (qhull's
    /// `qh_updatetested`).
    fn update_tested(&mut self, facet1: FacetId, facet2: FacetId) {
        self.facet_mut(facet2).tested = false;
        for r in self.facet(facet1).ridges.clone() {
            self.ridge_mut(r).tested = false;
        }
        if self.facet(facet2).center.is_none() {
            return;
        }
        let size = self.facet(facet2).vertices.len();
        let f2 = self.facet_mut(facet2);
        if !f2.keepcentrum {
            if size > MAX_NEW_CENTRUM_VERTICES {
                f2.keepcentrum = true;
            }
        } else if size == 3 {
            f2.keepcentrum = false;
        }
        if !f2.keepcentrum {
            f2.center = None;
            for r in self.facet(facet2).ridges.clone() {
                self.ridge_mut(r).tested = false;
            }
        }
    }

    /// Merges simplicial `facet1` into `facet2` by adding its opposite
    /// vertex and moving its ridges (qhull's `qh_mergesimplex`).
    fn merge_simplex(
        &mut self,
        facet1: FacetId,
        facet2: FacetId,
        merge_apex: bool,
    ) -> HullResult<()> {
        let (opposite, is_new) = if merge_apex {
            let opposite = self.facet(facet1).vertices[0];
            if !self.facet(facet2).newfacet {
                let vertices = self.facet(facet2).vertices.clone();
                self.renew_vertices(&vertices);
            }
            let is_new = self.facet(facet2).vertices.first() != Some(&opposite);
            if is_new {
                self.facet_mut(facet2).vertices.insert(0, opposite);
            }
            (opposite, is_new)
        } else {
            let opposite = self.opposite_vertex(facet1, facet2)?;
            let is_new = self.add_facet_vertex(facet2, opposite);
            if !self.facet(facet2).newfacet {
                let vertices = self.facet(facet2).vertices.clone();
                self.renew_vertices(&vertices);
            } else if !self.vertex(opposite).newfacet {
                self.remove_vertex(opposite);
                self.append_vertex(opposite);
            }
            (opposite, is_new)
        };
        for v in self.facet(facet1).vertices.clone() {
            if v == opposite && is_new {
                sets::replace(&mut self.vertex_mut(v).neighbors, facet1, facet2);
            } else {
                sets::delete(&mut self.vertex_mut(v).neighbors, facet1);
                if self.vertex(v).neighbors.len() < 2 {
                    self.merge_vertex_delete(v, facet2);
                }
            }
        }
        let visit = self.next_visit();
        for n in self.facet(facet2).neighbors.clone() {
            self.facet_mut(n).visit_id = visit;
        }
        let mut i = 0;
        while let Some(&r) = self.facet(facet1).ridges.get(i) {
            let other = self.ridge(r).other(facet1);
            if other == facet2 {
                self.delete_ridge_merge(r);
                sets::delete(&mut self.facet_mut(facet2).neighbors, facet1);
                continue;
            }
            if self.facet(other).dupridge && !self.facet(other).neighbors.contains(&facet1) {
                return Err(HullError::Topology);
            }
            self.push_ridge(facet2, r);
            if self.facet(other).visit_id == visit {
                if self.facet(other).simplicial {
                    self.make_ridges(other)?;
                }
                if self.facet(other).neighbors.first() == Some(&facet1) {
                    sets::delete(&mut self.facet_mut(other).neighbors, facet2);
                    sets::replace(&mut self.facet_mut(other).neighbors, facet1, facet2);
                } else {
                    sets::delete(&mut self.facet_mut(other).neighbors, facet1);
                }
            } else {
                self.facet_mut(facet2).neighbors.push(other);
                sets::replace(&mut self.facet_mut(other).neighbors, facet1, facet2);
                self.facet_mut(other).visit_id = visit;
            }
            let ridge = self.ridge_mut(r);
            if ridge.top == facet1 {
                ridge.top = facet2;
            } else {
                ridge.bottom = facet2;
            }
            i += 1;
        }
        Ok(())
    }

    /// The vertex of simplicial `facet1` not on its ridge with `facet2`,
    /// flagging the ridge's vertices (part of `qh_mergesimplex`).
    fn opposite_vertex(&mut self, facet1: FacetId, facet2: FacetId) -> HullResult<VertexId> {
        for v in self.facet(facet1).vertices.clone() {
            self.vertex_mut(v).seen = false;
        }
        if let Some(&r) = self
            .facet(facet1)
            .ridges
            .iter()
            .find(|&&r| self.ridge(r).other(facet1) == facet2)
        {
            for v in self.ridge(r).vertices.clone() {
                let vertex = self.vertex_mut(v);
                vertex.seen = true;
                vertex.delridge = true;
            }
        }
        let vertices = &self.facet(facet1).vertices;
        vertices
            .iter()
            .copied()
            .find(|&v| !self.vertex(v).seen)
            .ok_or(HullError::Topology)
    }

    /// Inserts `vertex` into `facet`'s sorted vertex set unless present
    /// (qhull's `qh_addfacetvertex`); returns whether it was added.
    pub(super) fn add_facet_vertex(&mut self, facet: FacetId, vertex: VertexId) -> bool {
        let vertices = &self.facets[facet.index()].vertices;
        let position = vertices.iter().position(|&v| v <= vertex);
        match position {
            Some(i) if vertices[i] == vertex => false,
            Some(i) => {
                self.facet_mut(facet).vertices.insert(i, vertex);
                true
            }
            None => {
                self.facet_mut(facet).vertices.push(vertex);
                true
            }
        }
    }

    /// Moves `facet1`'s neighbours to `facet2` (qhull's `qh_mergeneighbors`).
    fn merge_neighbors(&mut self, facet1: FacetId, facet2: FacetId) -> HullResult<()> {
        let visit = self.next_visit();
        for n in self.facet(facet2).neighbors.clone() {
            self.facet_mut(n).visit_id = visit;
        }
        for neighbor in self.facet(facet1).neighbors.clone() {
            if self.facet(neighbor).visit_id == visit {
                if self.facet(neighbor).simplicial {
                    self.make_ridges(neighbor)?;
                }
                if self.facet(neighbor).neighbors.first() == Some(&facet1) {
                    sets::delete(&mut self.facet_mut(neighbor).neighbors, facet2);
                    sets::replace(&mut self.facet_mut(neighbor).neighbors, facet1, facet2);
                } else {
                    sets::delete(&mut self.facet_mut(neighbor).neighbors, facet1);
                }
            } else if neighbor != facet2 {
                self.facet_mut(facet2).neighbors.push(neighbor);
                sets::replace(&mut self.facet_mut(neighbor).neighbors, facet1, facet2);
            }
        }
        sets::delete(&mut self.facet_mut(facet1).neighbors, facet2);
        sets::delete(&mut self.facet_mut(facet2).neighbors, facet1);
        Ok(())
    }

    /// Merges the sorted vertex sets of `facet1` into `facet2` (qhull's
    /// `qh_mergevertices`).
    fn merge_vertices(&mut self, facet1: FacetId, facet2: FacetId) {
        let first = &self.facet(facet1).vertices;
        let second = &self.facet(facet2).vertices;
        let mut merged = InlineVec::new();
        let mut j = 0;
        for &v in first {
            let id = v.index();
            while second.get(j).is_some_and(|w| w.index() > id) {
                merged.push(second[j]);
                j += 1;
            }
            if second.get(j) == Some(&v) {
                j += 1;
            }
            merged.push(v);
        }
        merged.extend(second[j..].iter().copied());
        self.facet_mut(facet2).vertices = merged;
    }

    /// Moves `facet1`'s ridges to `facet2`, deleting the ridges between
    /// them (qhull's `qh_mergeridges`).
    fn merge_ridges(&mut self, facet1: FacetId, facet2: FacetId) {
        let mut i = 0;
        while let Some(&r) = self.facet(facet2).ridges.get(i) {
            let ridge = self.ridge(r);
            if ridge.top == facet1 || ridge.bottom == facet1 {
                self.delete_ridge_merge(r);
            } else {
                i += 1;
            }
        }
        for r in self.facet(facet1).ridges.clone() {
            let ridge = self.ridge_mut(r);
            if ridge.top == facet1 {
                ridge.top = facet2;
            } else {
                ridge.bottom = facet2;
            }
            self.push_ridge(facet2, r);
        }
    }

    /// Replaces `facet1` by `facet2` in the neighbour sets of `facet1`'s
    /// vertices; vertices already in `facet2` just lose `facet1` and are
    /// deleted if left with one neighbour (qhull's
    /// `qh_mergevertex_neighbors`).
    fn merge_vertex_neighbors(&mut self, facet1: FacetId, facet2: FacetId) {
        let visit = self.vertex_visit;
        for v in self.facet(facet1).vertices.clone() {
            if self.vertex(v).visit_id == visit {
                sets::delete(&mut self.vertex_mut(v).neighbors, facet1);
                if self.vertex(v).neighbors.len() < 2 {
                    self.merge_vertex_delete(v, facet2);
                }
            } else {
                sets::replace(&mut self.vertex_mut(v).neighbors, facet1, facet2);
            }
        }
    }

    /// Deletes vertex `v` from merged `facet2` (qhull's `qh_mergevertex_del`).
    fn merge_vertex_delete(&mut self, v: VertexId, facet2: FacetId) {
        sets::delete_sorted(&mut self.facet_mut(facet2).vertices, v);
        self.vertex_mut(v).deleted = true;
        self.del_vertices.push(v);
    }

    /// Deletes a ridge removed by a merge, flagging its vertices and
    /// dropping vertex merges that refer to it (qhull's
    /// `qh_delridge_merge`).
    pub(super) fn delete_ridge_merge(&mut self, r: RidgeId) {
        if self.ridge(r).nonconvex {
            self.copy_nonconvex(r);
        }
        for v in self.ridge(r).vertices.clone() {
            self.vertex_mut(v).delridge = true;
        }
        let mut i = 0;
        while let Some(merge) = self.merge_sets().vertex.get(i).copied() {
            let (r1, r2) = merge.ridges;
            if r1 == r || r2 == r {
                self.merge_sets().vertex.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.unlink_ridge(r);
    }

    /// Passes the non-convex flag of deleted ridge `r` to another ridge
    /// between the same facets (qhull's `qh_copynonconvex`).
    fn copy_nonconvex(&mut self, r: RidgeId) {
        let (facet, other) = (self.ridge(r).top, self.ridge(r).bottom);
        self.ridge_mut(r).nonconvex = false;
        let twin = self.facet(facet).ridges.iter().copied().find(|&s| {
            let ridge = self.ridge(s);
            s != r && (ridge.top == other || ridge.bottom == other)
        });
        if let Some(s) = twin {
            self.ridge_mut(s).nonconvex = true;
        }
    }

    /// Makes the ridges of simplicial `facet` explicit and drops its
    /// merge-ridge markers (qhull's `qh_makeridges`).
    pub(super) fn make_ridges(&mut self, facet: FacetId) -> HullResult<()> {
        if !self.facet(facet).simplicial {
            return Ok(());
        }
        self.facet_mut(facet).simplicial = false;
        let neighbors = self.facet(facet).neighbors.clone();
        let merge_ridge = neighbors.contains(&FacetId::MERGE_RIDGE);
        for &n in neighbors.iter().filter(|&&n| n != FacetId::MERGE_RIDGE) {
            self.facet_mut(n).seen = false;
        }
        for r in self.facet(facet).ridges.clone() {
            let other = self.ridge(r).other(facet);
            self.facet_mut(other).seen = true;
        }
        for (i, &neighbor) in neighbors.iter().enumerate() {
            if neighbor == FacetId::MERGE_RIDGE || self.facet(neighbor).seen {
                continue;
            }
            if i >= 3 {
                return Err(HullError::Topology);
            }
            let vertices = sets::without_nth(&self.facet(facet).vertices, i);
            let (top, bottom) = if self.facet(facet).toporient ^ (i & 1 == 1) {
                (facet, neighbor)
            } else {
                (neighbor, facet)
            };
            let r = self.new_ridge(vertices, top, bottom);
            self.ridge_mut(r).tested = self.facet(facet).tested && !merge_ridge;
            self.push_ridge(facet, r);
            self.push_ridge(neighbor, r);
        }
        if merge_ridge {
            while sets::delete(&mut self.facet_mut(facet).neighbors, FacetId::MERGE_RIDGE) {}
        }
        Ok(())
    }
}
