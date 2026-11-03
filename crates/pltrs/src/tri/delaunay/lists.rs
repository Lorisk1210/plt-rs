//! The facet and vertex lists of the hull (qhull's `qh_appendfacet`,
//! `qh_removefacet`, `qh_prependfacet`, `qh_willdelete`, ...).
//!
//! qhull keeps facets in one doubly linked list and marks sublists by head
//! pointers: `facet_next` (the next facet to process), `newfacet_list`
//! (the cone of the current point, at the end) and `visible_list` (the
//! facets to delete, just before the new ones). Every operation keeps
//! these heads valid exactly as qhull does, since the list order becomes the
//! order of the output triangles.

use super::hull::{Facet, FacetId, Hull, RidgeId, VertexId};
use super::sets;

/// Which list head [`Hull::prepend_facet`] inserts before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListHead {
    /// `qh.facet_next`, the next facet to process.
    FacetNext,
    /// `qh.visible_list`, the facets to delete.
    Visible,
}

impl Hull {
    /// `f` if it is a facet of the list rather than the tail.
    pub(super) fn listed(&self, f: Option<FacetId>) -> Option<FacetId> {
        f.filter(|&f| self.facet(f).next.is_some())
    }

    /// The facet after `f` in the list, unless that is the tail: the step
    /// of [`facets_from`](Self::facets_from) for walks that leave the links
    /// alone.
    pub(super) fn next_listed(&self, f: FacetId) -> Option<FacetId> {
        self.listed(self.facet(f).next)
    }

    /// `v` if it is a vertex of the list rather than the tail.
    pub(super) fn listed_vertex(&self, v: Option<VertexId>) -> Option<VertexId> {
        v.filter(|&v| self.vertex(v).next.is_some())
    }

    /// The facets from `start` to the end of the list, snapshot first.
    pub(super) fn facets_from(&self, start: Option<FacetId>) -> Vec<FacetId> {
        let mut out = Vec::new();
        let mut cursor = self.listed(start);
        while let Some(f) = cursor {
            out.push(f);
            cursor = self.listed(self.facet(f).next);
        }
        out
    }

    /// The vertices from `start` to the end of the vertex list.
    pub(super) fn vertices_from(&self, start: Option<VertexId>) -> Vec<VertexId> {
        let mut out = Vec::new();
        let mut cursor = start;
        while let Some(v) = cursor {
            let Some(next) = self.vertex(v).next else {
                break;
            };
            out.push(v);
            cursor = Some(next);
        }
        out
    }

    /// The visible facets (`FORALLvisible_facets`).
    pub(super) fn visible_facets(&self) -> Vec<FacetId> {
        let mut out = Vec::new();
        let mut cursor = self.visible_list;
        while let Some(f) = cursor {
            if !self.facet(f).visible {
                break;
            }
            out.push(f);
            cursor = self.facet(f).next;
        }
        out
    }

    /// Appends `facet` to the end of the facet list (qhull's `qh_appendfacet`).
    pub(super) fn append_facet(&mut self, facet: FacetId) {
        let tail = FacetId::TAIL;
        if self.newfacet_list == Some(tail) {
            self.newfacet_list = Some(facet);
            if self.visible_list == Some(tail) {
                self.visible_list = Some(facet);
            }
        }
        if self.facet_next == Some(tail) {
            self.facet_next = Some(facet);
        }
        let previous = self.facet(tail).previous;
        self.facet_mut(facet).previous = previous;
        self.facet_mut(facet).next = Some(tail);
        match previous {
            Some(p) => self.facet_mut(p).next = Some(facet),
            None => self.facet_list = facet,
        }
        self.facet_mut(tail).previous = Some(facet);
        self.num_facets += 1;
    }

    /// Unlinks `facet` from the facet list (qhull's `qh_removefacet`).
    pub(super) fn remove_facet(&mut self, facet: FacetId) {
        let next = self
            .facet(facet)
            .next
            .expect("listed facets have a successor");
        let previous = self.facet(facet).previous;
        if self.newfacet_list == Some(facet) {
            self.newfacet_list = Some(next);
        }
        if self.facet_next == Some(facet) {
            self.facet_next = Some(next);
        }
        if self.visible_list == Some(facet) {
            self.visible_list = Some(next);
        }
        if let Some(p) = previous {
            self.facet_mut(p).next = Some(next);
            self.facet_mut(next).previous = Some(p);
        } else {
            self.facet_list = next;
            self.facet_mut(next).previous = None;
        }
        self.num_facets -= 1;
    }

    /// Inserts `facet` before the facet at `head` and makes it the new head
    /// (qhull's `qh_prependfacet`).
    pub(super) fn prepend_facet(&mut self, facet: FacetId, head: ListHead) {
        let list = match head {
            ListHead::FacetNext => self.facet_next,
            ListHead::Visible => self.visible_list,
        }
        .unwrap_or(FacetId::TAIL);
        let previous = self.facet(list).previous;
        self.facet_mut(facet).previous = previous;
        if let Some(p) = previous {
            self.facet_mut(p).next = Some(facet);
        }
        self.facet_mut(list).previous = Some(facet);
        self.facet_mut(facet).next = Some(list);
        if self.facet_list == list {
            self.facet_list = facet;
        }
        if self.facet_next == Some(list) {
            self.facet_next = Some(facet);
        }
        match head {
            ListHead::FacetNext => self.facet_next = Some(facet),
            ListHead::Visible => self.visible_list = Some(facet),
        }
        self.num_facets += 1;
    }

    /// Moves `facet` to the end of the facet list.
    pub(super) fn move_to_end(&mut self, facet: FacetId) {
        self.remove_facet(facet);
        self.append_facet(facet);
    }

    /// Appends `vertex` to the vertex list and marks it new
    /// (qhull's `qh_appendvertex`).
    pub(super) fn append_vertex(&mut self, vertex: VertexId) {
        let tail = VertexId::TAIL;
        if self.newvertex_list == Some(tail) {
            self.newvertex_list = Some(vertex);
        }
        let previous = self.vertex(tail).previous;
        let v = self.vertex_mut(vertex);
        v.newfacet = true;
        v.previous = previous;
        v.next = Some(tail);
        if let Some(p) = previous {
            self.vertex_mut(p).next = Some(vertex);
        }
        self.vertex_mut(tail).previous = Some(vertex);
    }

    /// Unlinks `vertex` from the vertex list (qhull's `qh_removevertex`).
    pub(super) fn remove_vertex(&mut self, vertex: VertexId) {
        let next = self
            .vertex(vertex)
            .next
            .expect("listed vertices have a successor");
        let previous = self.vertex(vertex).previous;
        if self.newvertex_list == Some(vertex) {
            self.newvertex_list = Some(next);
        }
        if let Some(p) = previous {
            self.vertex_mut(p).next = Some(next);
        }
        self.vertex_mut(next).previous = previous;
    }

    /// Moves the vertices of `vertices` that are not new to the end of the
    /// vertex list (qhull's `qh_newvertices`).
    pub(super) fn renew_vertices(&mut self, vertices: &[VertexId]) {
        for &v in vertices {
            if !self.vertex(v).newfacet {
                self.remove_vertex(v);
                self.append_vertex(v);
            }
        }
    }

    /// Deletes `facet`: unlinks it and frees its slot for the next new
    /// facet (qhull's `qh_delfacet`).
    pub(super) fn delete_facet(&mut self, facet: FacetId) {
        self.remove_facet(facet);
        self.facets[facet.index()] = Facet::default();
        self.free_facets.push(facet);
    }

    /// Deletes `vertex` (qhull's `qh_delvertex`).
    pub(super) fn delete_vertex(&mut self, vertex: VertexId) {
        self.remove_vertex(vertex);
        self.vertex_mut(vertex).neighbors = Vec::new();
    }

    /// Appends ridge `r` to the ridge set of `facet`, allocating the set.
    pub(super) fn push_ridge(&mut self, facet: FacetId, r: RidgeId) {
        let f = self.facet_mut(facet);
        f.ridges.push(r);
        f.ridge_set = true;
    }

    /// Removes ridge `r` from both facets' ridge sets (qhull's `qh_setdel`
    /// on both sides before `qh_delridge`); the arena slot stays unused.
    pub(super) fn unlink_ridge(&mut self, r: RidgeId) {
        let (top, bottom) = (self.ridge(r).top, self.ridge(r).bottom);
        sets::delete(&mut self.facet_mut(top).ridges, r);
        sets::delete(&mut self.facet_mut(bottom).ridges, r);
    }

    /// Marks `facet` for deletion and moves it to the head of the visible
    /// list (qhull's `qh_willdelete`).
    pub(super) fn will_delete(&mut self, facet: FacetId, replace: Option<FacetId>) {
        self.remove_facet(facet);
        self.prepend_facet(facet, ListHead::Visible);
        self.num_visible += 1;
        let f = self.facet_mut(facet);
        f.visible = true;
        f.link = replace;
        f.ridges.clear();
        f.neighbors.clear();
    }

    /// The live replacement of a visible facet, following replacement chains
    /// (qhull's `qh_getreplacement`).
    pub(super) fn replacement(&self, visible: FacetId) -> Option<FacetId> {
        let mut result = Some(visible);
        while let Some(f) = result {
            if !self.facet(f).visible {
                break;
            }
            result = self.facet(f).link;
        }
        result
    }
}
