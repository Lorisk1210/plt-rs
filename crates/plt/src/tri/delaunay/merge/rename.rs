//! Removing redundant vertices of merged facets by renaming them to a
//! neighbouring vertex (`qh_reducevertices`, `qh_rename_sharedvertex`,
//! `qh_find_newvertex`, `qh_renamevertex`, `qh_renameridgevertex`).

use super::VertexMerge;
use crate::tri::delaunay::hull::{FacetId, Hull, HullError, HullResult, RidgeId, VertexId};
use crate::tri::delaunay::sets;

impl Hull {
    /// Removes extra vertices of merged facets and renames vertices shared
    /// by exactly two facets (qhull's `qh_reducevertices` for 3-d); returns
    /// whether a degenerate or redundant facet was merged.
    pub(super) fn reduce_vertices(&mut self) -> HullResult<bool> {
        let mut degen_redundant = self.merge_degen_redundant()? > 0;
        'restart: loop {
            for f in self.facets_from(self.newfacet_list) {
                if self.facet(f).newmerge && self.remove_extra_vertices(f) {
                    self.degen_redundant_facet(f);
                    if self.merge_degen_redundant()? > 0 {
                        degen_redundant = true;
                        continue 'restart;
                    }
                }
            }
            for f in self.facets_from(self.newfacet_list) {
                if !self.facet(f).newmerge {
                    continue;
                }
                self.facet_mut(f).newmerge = false;
                let mut i = 0;
                while let Some(&v) = self.facet(f).vertices.get(i) {
                    if self.vertex(v).delridge && self.rename_shared_vertex(v, f)?.is_some() {
                        if self.merge_degen_redundant()? > 0 {
                            degen_redundant = true;
                            continue 'restart;
                        }
                    } else {
                        i += 1;
                    }
                }
            }
            break;
        }
        for v in self.vertices_from(self.newvertex_list) {
            if self.vertex(v).delridge && !self.vertex(v).deleted {
                self.vertex_mut(v).delridge = false;
            }
        }
        Ok(degen_redundant)
    }

    /// Renames `vertex`, shared by `facet` and one other facet, to another
    /// vertex of both if no duplicate ridge results (qhull's
    /// `qh_rename_sharedvertex` for 3-d).
    fn rename_shared_vertex(
        &mut self,
        vertex: VertexId,
        facet: FacetId,
    ) -> HullResult<Option<VertexId>> {
        let neighbors = &self.vertex(vertex).neighbors;
        if neighbors.len() != 2 {
            return Ok(None);
        }
        let neighbor = if neighbors[0] == facet {
            neighbors[1]
        } else {
            neighbors[0]
        };
        let visit = self.next_visit();
        self.facet_mut(neighbor).visit_id = visit;
        let mut ridges = Vec::new();
        self.vertex_ridges_facet(vertex, facet, &mut ridges);
        let mut vertices =
            intersect_sorted(&self.facet(facet).vertices, &self.facet(neighbor).vertices);
        sets::delete(&mut vertices, vertex);
        let found = self.find_new_vertex(vertex, vertices, &ridges);
        if let Some(newvertex) = found {
            self.rename_vertex(vertex, newvertex, &ridges, Some((facet, neighbor)))?;
        }
        Ok(found)
    }

    /// The first of `vertices` (by fewest shared ridges) that `oldvertex` can
    /// be renamed to without duplicating a ridge (qhull's
    /// `qh_find_newvertex`).
    fn find_new_vertex(
        &mut self,
        oldvertex: VertexId,
        mut vertices: Vec<VertexId>,
        ridges: &[RidgeId],
    ) -> Option<VertexId> {
        for &r in ridges {
            for v in self.ridge(r).vertices.clone() {
                self.vertex_mut(v).seen = false;
            }
        }
        for &v in &vertices {
            let vertex = self.vertex_mut(v);
            vertex.visit_id = 0;
            vertex.seen = true;
        }
        for &r in ridges {
            for v in self.ridge(r).vertices.clone() {
                if self.vertex(v).seen {
                    self.vertex_mut(v).visit_id += 1;
                }
            }
        }
        let mut i = 0;
        while let Some(&v) = vertices.get(i) {
            if self.vertex(v).visit_id == 0 {
                vertices.swap_remove(i);
            } else {
                i += 1;
            }
        }
        let max_visit = u32::try_from(ridges.len()).expect("fewer than 2^32 ridges");
        if self.vertex_visit < max_visit {
            self.vertex_visit = max_visit;
        }
        vertices.sort_by_key(|&v| self.vertex(v).visit_id);
        for v in vertices {
            let duplicates = self.vertex_ridges(v, false).into_iter().any(|r| {
                ridges.iter().any(|&old| {
                    old != r
                        && sets::equal_except(
                            &self.ridge(r).vertices,
                            v,
                            &self.ridge(old).vertices,
                            oldvertex,
                        )
                })
            });
            if !duplicates {
                return Some(v);
            }
        }
        None
    }

    /// The ridges of `vertex`'s neighbouring facets that contain it
    /// (qhull's `qh_vertexridges`); with `all_neighbors` false the last
    /// neighbour is skipped, as its ridges were found from the others.
    pub(super) fn vertex_ridges(&mut self, vertex: VertexId, all_neighbors: bool) -> Vec<RidgeId> {
        self.visit_id += 2;
        let visit = self.visit_id;
        let neighbors = self.vertex(vertex).neighbors.clone();
        for &n in &neighbors {
            self.facet_mut(n).visit_id = visit;
        }
        let mut ridges = Vec::new();
        for (i, &n) in neighbors.iter().enumerate() {
            if i + 1 < neighbors.len() || all_neighbors {
                self.vertex_ridges_facet(vertex, n, &mut ridges);
            }
        }
        ridges
    }

    /// Appends the ridges of `facet` containing `vertex` whose other facet
    /// is marked with the current visit (qhull's `qh_vertexridges_facet`).
    fn vertex_ridges_facet(&mut self, vertex: VertexId, facet: FacetId, ridges: &mut Vec<RidgeId>) {
        let visit = self.visit_id;
        for &r in &self.facet(facet).ridges {
            let ridge = self.ridge(r);
            if self.facet(ridge.other(facet)).visit_id == visit && ridge.vertices.contains(&vertex)
            {
                ridges.push(r);
            }
        }
        self.facet_mut(facet).visit_id = visit - 1;
    }

    /// Renames `oldvertex` to `newvertex` in `ridges` and its facets
    /// (qhull's `qh_renamevertex`); `shared` gives the facet and neighbour
    /// of a shared vertex, `None` renames it everywhere.
    pub(super) fn rename_vertex(
        &mut self,
        oldvertex: VertexId,
        newvertex: VertexId,
        ridges: &[RidgeId],
        shared: Option<(FacetId, FacetId)>,
    ) -> HullResult<()> {
        for &r in ridges {
            if !self.rename_ridge_vertex(r, oldvertex, newvertex)? {
                continue;
            }
            let (top, bottom) = (self.ridge(r).top, self.ridge(r).bottom);
            if !self.shrunk_by_rename(top, newvertex) && !self.shrunk_by_rename(bottom, newvertex) {
                self.maybe_duplicate_ridge(r);
            }
        }
        match shared {
            None => self.rename_everywhere(oldvertex)?,
            Some((oldfacet, neighbor)) => {
                if self.vertex(oldvertex).neighbors.len() == 2 {
                    for n in self.vertex(oldvertex).neighbors.clone() {
                        sets::delete_sorted(&mut self.facet_mut(n).vertices, oldvertex);
                        self.degen_redundant_facet(n);
                    }
                    self.vertex_mut(oldvertex).deleted = true;
                    self.del_vertices.push(oldvertex);
                } else {
                    sets::delete_sorted(&mut self.facet_mut(oldfacet).vertices, oldvertex);
                    sets::delete(&mut self.vertex_mut(oldvertex).neighbors, oldfacet);
                    if self.remove_extra_vertices(neighbor) {
                        self.degen_redundant_facet(neighbor);
                    }
                }
                self.degen_redundant_facet(oldfacet);
            }
        }
        Ok(())
    }

    /// Whether `facet` lost a vertex to a rename (fewer than three vertices,
    /// or three with the new vertex in a non-simplicial facet).
    fn shrunk_by_rename(&self, facet: FacetId, newvertex: VertexId) -> bool {
        let f = self.facet(facet);
        f.vertices.len() < 3
            || (f.vertices.len() == 3 && !f.simplicial && f.vertices.contains(&newvertex))
    }

    /// Drops renamed `oldvertex` from all its facets and queues the facets
    /// that became degenerate or redundant (the `!oldfacet` case of
    /// `qh_renamevertex`).
    fn rename_everywhere(&mut self, oldvertex: VertexId) -> HullResult<()> {
        let mut i = 0;
        while let Some(&neighbor) = self.vertex(oldvertex).neighbors.get(i) {
            if self.facet(neighbor).simplicial {
                self.degen_redundant_facet(neighbor);
                i += 1;
                continue;
            }
            self.may_drop_neighbor(neighbor)?;
            sets::delete_sorted(&mut self.facet_mut(neighbor).vertices, oldvertex);
            let removed = self.remove_extra_vertices(neighbor);
            self.degen_redundant_facet(neighbor);
            self.test_redundant_neighbors(neighbor);
            self.test_degen_neighbors(neighbor);
            if !removed {
                i += 1;
            }
        }
        if !self.vertex(oldvertex).deleted {
            self.vertex_mut(oldvertex).deleted = true;
            self.del_vertices.push(oldvertex);
        }
        Ok(())
    }

    /// Renames `oldvertex` to `newvertex` in ridge `r`, deleting the ridge
    /// if it already has `newvertex` (qhull's `qh_renameridgevertex`).
    fn rename_ridge_vertex(
        &mut self,
        r: RidgeId,
        oldvertex: VertexId,
        newvertex: VertexId,
    ) -> HullResult<bool> {
        let old_nth = self
            .ridge(r)
            .vertices
            .iter()
            .position(|&v| v == oldvertex)
            .ok_or(HullError::Topology)?;
        self.ridge_mut(r).vertices.remove(old_nth);
        let mut nth = 0;
        for &v in &self.ridge(r).vertices {
            if v == newvertex {
                self.delete_ridge_merge(r);
                return Ok(false);
            }
            if v < newvertex {
                break;
            }
            nth += 1;
        }
        let ridge = self.ridge_mut(r);
        ridge.vertices.insert(nth, newvertex);
        if old_nth.abs_diff(nth) % 2 == 1 {
            std::mem::swap(&mut ridge.top, &mut ridge.bottom);
        }
        Ok(true)
    }

    /// Queues a vertex merge if renamed ridge `ridge` now duplicates another
    /// ridge of a merged facet (qhull's `qh_maybe_duplicateridge`).
    fn maybe_duplicate_ridge(&mut self, ridge: RidgeId) {
        for facet in [self.ridge(ridge).top, self.ridge(ridge).bottom] {
            if self.facet(facet).simplicial || self.facet(facet).nummerge == 0 {
                continue;
            }
            for other in self.facet(facet).ridges.clone() {
                if other == ridge || self.ridge(other).vertices != self.ridge(ridge).vertices {
                    continue;
                }
                let (dist, pair) = self.vertex_bestdist(&self.ridge(other).vertices.clone());
                if let Some((vertex, pinched)) = pair {
                    self.merge_sets().vertex.push(VertexMerge {
                        vertex1: pinched,
                        vertex2: vertex,
                        distance: dist,
                        ridges: (ridge, other),
                    });
                }
            }
        }
    }
}

/// The common elements of two vertex sets sorted by decreasing id (qhull's
/// `qh_vertexintersect_new`).
fn intersect_sorted(a: &[VertexId], b: &[VertexId]) -> Vec<VertexId> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Greater => i += 1,
            std::cmp::Ordering::Less => j += 1,
        }
    }
    out
}
