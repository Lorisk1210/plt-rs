//! Adding a point to the hull: its visible facets and horizon, the cone of
//! new facets, and the bookkeeping around them (`qh_buildhull`,
//! `qh_addpoint`, `qh_findhorizon`, `qh_makenewfacets`, `qh_deletevisible`,
//! `qh_resetlists`, ...).

use super::hull::{FacetId, Hull, HullError, HullResult, VertexId};
use super::precision::QhullMinMax;
use super::sets;

impl Hull {
    /// Adds the furthest outside point of each facet until none is left
    /// (qhull's `qh_buildhull`).
    pub(super) fn build_hull(&mut self) -> HullResult<()> {
        self.facet_next = Some(self.facet_list);
        while let Some((point, facet)) = self.next_furthest() {
            self.add_point(point, facet)?;
        }
        if self.narrow {
            self.out_coplanar()?;
        }
        Ok(())
    }

    /// Adds `point`, which is above `facet`, to the hull (qhull's
    /// `qh_addpoint`).
    fn add_point(&mut self, point: usize, facet: FacetId) -> HullResult<()> {
        self.max_outside_target = self
            .max_outside
            .max_qhull(self.prec.one_merge + self.prec.dist_round)
            .max_qhull(self.prec.min_outside);
        self.find_horizon(point, facet)?;
        self.make_new_facets(point)?;
        self.match_new_facets()?;
        self.make_new_planes();
        self.update_vertex_neighbors_cone();
        self.findbestnew = false;
        self.premerge()?;
        self.findbestnew = self.many_merges()
            || self
                .facets_from(self.newfacet_list)
                .into_iter()
                .any(|f| !self.facet(f).simplicial);
        self.partition_visible()?;
        self.findbestnew = false;
        self.findbest_notsharp = false;
        self.delete_visible();
        self.reset_lists(true);
        self.all_vertex_merges()?;
        self.merges = None;
        Ok(())
    }

    /// Moves the facets visible from `point` to the visible list, starting
    /// from `facet`, and marks coplanar horizon facets (qhull's
    /// `qh_findhorizon`).
    fn find_horizon(&mut self, point: usize, facet: FacetId) -> HullResult<()> {
        let p = self.points[point];
        self.move_to_end(facet);
        self.num_visible = 1;
        self.visible_list = Some(facet);
        self.facet_mut(facet).visible = true;
        self.facet_mut(facet).link = None;
        let visit = self.next_visit();
        let mut horizon = 0;
        let mut cursor = Some(facet);
        while let Some(visible) = cursor.filter(|&f| self.facet(f).visible) {
            if self.facet(visible).neighbors.is_empty() {
                return Err(HullError::Topology);
            }
            self.facet_mut(visible).visit_id = visit;
            for i in 0..self.facet(visible).neighbors.len() {
                let neighbor = self.facet(visible).neighbors[i];
                if self.facet(neighbor).visit_id == visit {
                    continue;
                }
                self.facet_mut(neighbor).visit_id = visit;
                let dist = self.dist(&p, neighbor);
                if dist > self.prec.min_visible {
                    self.move_to_end(neighbor);
                    let n = self.facet_mut(neighbor);
                    n.visible = true;
                    n.link = None;
                    self.num_visible += 1;
                } else {
                    let coplanar = dist >= -self.prec.max_coplanar;
                    self.facet_mut(neighbor).coplanarhorizon = coplanar;
                    if coplanar {
                        if dist > 0.0 {
                            self.max_outside = self.max_outside.max_qhull(dist);
                            let n = self.facet_mut(neighbor);
                            n.max_outside = n.max_outside.max_qhull(dist);
                        } else if self.min_vertex > dist {
                            self.min_vertex = dist;
                        }
                    }
                    horizon += 1;
                }
            }
            cursor = self.facet(visible).next;
        }
        if horizon == 0 {
            return Err(HullError::Topology);
        }
        Ok(())
    }

    /// Creates the cone of new facets from `point` to the horizon (qhull's
    /// `qh_makenewfacets`). Like qhull, a visible facet that creates no new
    /// facet takes the last new facet of an earlier one as its replacement.
    fn make_new_facets(&mut self, point: usize) -> HullResult<()> {
        self.newfacet_list = Some(FacetId::TAIL);
        self.newvertex_list = Some(VertexId::TAIL);
        let apex = self.new_vertex(point);
        self.append_vertex(apex);
        let (mut newfacet, mut newfacet2) = (None, None);
        for visible in self.visible_facets() {
            for n in self.facet(visible).neighbors.clone() {
                self.facet_mut(n).seen = false;
            }
            if self.facet(visible).ridge_set {
                newfacet2 = self.make_new_nonsimplicial(visible, apex)?;
            }
            if self.facet(visible).simplicial {
                newfacet = self.make_new_simplicial(visible, apex)?;
            }
            if newfacet2.is_some() {
                newfacet = newfacet2;
            }
            let facet = self.facet_mut(visible);
            if newfacet.is_some() {
                facet.link = newfacet;
            }
            facet.ridges.clear();
            facet.neighbors.clear();
        }
        Ok(())
    }

    /// Creates a new facet with `vertices` attached to `horizon` (qhull's
    /// `qh_makenewfacet`).
    pub(super) fn make_new_facet(
        &mut self,
        vertices: Vec<VertexId>,
        toporient: bool,
        horizon: FacetId,
    ) -> FacetId {
        for &v in &vertices {
            if !self.vertex(v).newfacet {
                self.remove_vertex(v);
                self.append_vertex(v);
            }
        }
        let f = self.new_facet();
        let facet = self.facet_mut(f);
        facet.vertices = vertices;
        facet.toporient = toporient;
        facet.neighbors.push(horizon);
        self.append_facet(f);
        f
    }

    /// New facets from `apex` to the horizon neighbours of simplicial
    /// `visible` (qhull's `qh_makenew_simplicial`); returns the last one.
    fn make_new_simplicial(
        &mut self,
        visible: FacetId,
        apex: VertexId,
    ) -> HullResult<Option<FacetId>> {
        let mut newfacet = None;
        for neighbor in self.facet(visible).neighbors.clone() {
            let n = self.facet(neighbor);
            if n.seen || n.visible {
                continue;
            }
            let horizon_skip = n
                .neighbors
                .iter()
                .position(|&f| f == visible)
                .ok_or(HullError::Topology)?;
            let mut vertices = Vec::with_capacity(3);
            vertices.push(apex);
            vertices.extend(sets::without_nth(&n.vertices, horizon_skip));
            let odd = horizon_skip & 1 == 1;
            let toporient = if n.toporient { odd } else { !odd };
            let coplanar = n.coplanarhorizon;
            let f = self.make_new_facet(vertices, toporient, neighbor);
            if coplanar {
                let facet = self.facet_mut(f);
                facet.link = Some(f);
                facet.mergehorizon = true;
            }
            self.facet_mut(neighbor).neighbors[horizon_skip] = f;
            newfacet = Some(f);
        }
        Ok(newfacet)
    }

    /// New facets from `apex` to the horizon ridges of `visible`, attached
    /// to the horizon (qhull's `qh_makenew_nonsimplicial`); returns the
    /// last one.
    pub(super) fn make_new_nonsimplicial(
        &mut self,
        visible: FacetId,
        apex: VertexId,
    ) -> HullResult<Option<FacetId>> {
        let mut newfacet = None;
        for ridge in self.facet(visible).ridges.clone() {
            let neighbor = self.ridge(ridge).other(visible);
            if !self.facet(neighbor).visible {
                let toporient = self.ridge(ridge).top == visible;
                let mut vertices = Vec::with_capacity(3);
                vertices.push(apex);
                vertices.extend_from_slice(&self.ridge(ridge).vertices);
                let f = self.make_new_facet(vertices, toporient, neighbor);
                newfacet = Some(f);
                if self.facet(neighbor).coplanarhorizon {
                    self.facet_mut(f).mergehorizon = true;
                    if self.facet(neighbor).seen {
                        let cycle = self
                            .facet(neighbor)
                            .link
                            .expect("a seen coplanar horizon has a cycle");
                        self.facet_mut(f).link = self.facet(cycle).link;
                        self.facet_mut(cycle).link = Some(f);
                    } else {
                        self.facet_mut(f).link = Some(f);
                        self.facet_mut(neighbor).link = Some(f);
                    }
                }
                if self.facet(neighbor).seen {
                    if self.facet(neighbor).simplicial {
                        return Err(HullError::Topology);
                    }
                    self.facet_mut(neighbor).neighbors.push(f);
                } else {
                    sets::replace(&mut self.facet_mut(neighbor).neighbors, visible, f);
                }
                if self.facet(neighbor).simplicial {
                    sets::delete(&mut self.facet_mut(neighbor).ridges, ridge);
                } else {
                    self.push_ridge(f, ridge);
                    let r = self.ridge_mut(ridge);
                    if toporient {
                        r.top = f;
                    } else {
                        r.bottom = f;
                    }
                }
            }
            self.facet_mut(neighbor).seen = true;
        }
        Ok(newfacet)
    }

    /// Sets the hyperplanes of the new facets that will not merge into their
    /// horizon (qhull's `qh_makenewplanes`).
    fn make_new_planes(&mut self) {
        for f in self.facets_from(self.newfacet_list) {
            if !self.facet(f).mergehorizon {
                self.set_facet_plane(f);
            }
        }
    }

    /// Updates vertex neighbours for the cone and queues the vertices
    /// interior to the visible region for deletion (qhull's
    /// `qh_update_vertexneighbors_cone`).
    fn update_vertex_neighbors_cone(&mut self) {
        if self.vertex_neighbors {
            for v in self.vertices_from(self.newvertex_list) {
                let mut i = 0;
                while let Some(&n) = self.vertex(v).neighbors.get(i) {
                    if self.facet(n).visible {
                        self.vertex_mut(v).neighbors.swap_remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
            for f in self.facets_from(self.newfacet_list) {
                for v in self.facet(f).vertices.clone() {
                    self.vertex_mut(v).neighbors.push(f);
                }
            }
        }
        self.delete_interior_vertices();
    }

    /// Updates vertex neighbours after vertex merges, when no cone of new
    /// facets is pending (qhull's `qh_update_vertexneighbors` as called by
    /// `qh_all_vertexmerges`).
    pub(super) fn update_vertex_neighbors(&mut self) {
        if self.vertex_neighbors {
            for v in self.vertices_from(self.newvertex_list) {
                let facets = &self.facets;
                self.vertices[v.index()]
                    .neighbors
                    .retain(|n| !facets[n.index()].visible);
            }
            for f in self.facets_from(self.newfacet_list) {
                for v in self.facet(f).vertices.clone() {
                    sets::append_unique(&mut self.vertex_mut(v).neighbors, f);
                }
            }
        }
        self.delete_interior_vertices();
    }

    /// Queues the vertices of visible facets that are in no remaining facet
    /// for deletion; others just lose the visible facet as a neighbour.
    fn delete_interior_vertices(&mut self) {
        for visible in self.visible_facets() {
            for v in self.facet(visible).vertices.clone() {
                let vertex = self.vertex(v);
                if vertex.newfacet || vertex.deleted {
                    continue;
                }
                let kept = self.vertex_neighbors
                    && vertex.neighbors.iter().any(|&n| !self.facet(n).visible);
                if kept {
                    sets::delete(&mut self.vertex_mut(v).neighbors, visible);
                } else {
                    self.vertex_mut(v).deleted = true;
                    self.del_vertices.push(v);
                }
            }
        }
    }

    /// Builds the vertex neighbour sets (qhull's `qh_vertexneighbors`).
    pub(super) fn vertex_neighbors_all(&mut self) {
        if self.vertex_neighbors {
            return;
        }
        let visit = self.next_vertex_visit();
        for f in self.facets_from(Some(self.facet_list)) {
            if self.facet(f).visible {
                continue;
            }
            for v in self.facet(f).vertices.clone() {
                let vertex = self.vertex_mut(v);
                if vertex.visit_id != visit {
                    vertex.visit_id = visit;
                    vertex.neighbors.clear();
                }
                vertex.neighbors.push(f);
            }
        }
        self.vertex_neighbors = true;
    }

    /// Deletes the visible facets and the queued vertices (qhull's
    /// `qh_deletevisible`).
    pub(super) fn delete_visible(&mut self) {
        for visible in self.visible_facets() {
            self.delete_facet(visible);
        }
        self.num_visible = 0;
        for v in std::mem::take(&mut self.del_vertices) {
            self.delete_vertex(v);
        }
    }

    /// Clears the new and visible lists and their flags (qhull's
    /// `qh_resetlists`).
    pub(super) fn reset_lists(&mut self, reset_visible: bool) {
        for v in self.vertices_from(self.newvertex_list) {
            self.vertex_mut(v).newfacet = false;
        }
        self.newvertex_list = None;
        for f in self.facets_from(self.newfacet_list) {
            let facet = self.facet_mut(f);
            facet.newfacet = false;
            facet.dupridge = false;
        }
        self.newfacet_list = None;
        if reset_visible {
            for f in self.visible_facets() {
                let facet = self.facet_mut(f);
                facet.link = None;
                facet.visible = false;
            }
            self.num_visible = 0;
        }
        self.visible_list = None;
    }
}
