//! Triangulating the merged (non-simplicial) facets, option `Qt`
//! (qhull's `qh_triangulate`, `qh_triangulate_facet`, `qh_triangulate_null`,
//! `qh_triangulate_mirror`, `qh_triangulate_link`).
//!
//! Each non-simplicial facet is replaced by a fan of triangles from its
//! highest-numbered vertex to its ridges; the fan triangles are appended to
//! the facet list, which fixes their position in the output. Ridges through
//! the apex give null triangles that are unlinked again. Vertex neighbour
//! sets are not maintained here: nothing reads them after the build.

use super::hull::{FacetId, Hull, HullError, HullResult};
use super::merge::{MergeKind, MergeSets};
use super::sets;

impl Hull {
    /// Replaces every non-simplicial facet by triangles (qhull's
    /// `qh_triangulate`). Without merges every facet is simplicial, and
    /// the facets need not be walked.
    pub(super) fn triangulate(&mut self) -> HullResult<()> {
        if !self.merged {
            self.visible_list = None;
            self.num_visible = 0;
            return Ok(());
        }
        self.merges = Some(MergeSets::default());
        let mut first_triangulated = None;
        let mut cursor = Some(self.facet_list);
        while let Some(facet) = self.listed(cursor) {
            cursor = self.facet(facet).next;
            let f = self.facet(facet);
            if f.visible || f.simplicial {
                continue;
            }
            first_triangulated.get_or_insert(facet);
            self.triangulate_facet(facet)?;
        }
        let mut cursor = first_triangulated;
        while let Some(facet) = self.listed(cursor) {
            cursor = self.facet(facet).next;
            let f = self.facet(facet);
            if f.visible {
                continue;
            }
            if !f.ridges.is_empty() {
                return Err(HullError::Topology);
            }
            if f.vertices[0] == f.vertices[1] {
                self.triangulate_null(facet)?;
            }
        }
        while let Some(merge) = self.merge_sets().degen.pop() {
            if merge.kind == MergeKind::Mirror {
                self.triangulate_mirror(merge.facet1, merge.facet2)?;
            }
        }
        self.merges = None;
        let mut cursor = first_triangulated;
        while let Some(facet) = self.listed(cursor) {
            cursor = self.facet(facet).next;
            if self.facet(facet).visible {
                self.delete_facet(facet);
            }
        }
        self.visible_list = None;
        self.num_visible = 0;
        Ok(())
    }

    /// Replaces non-simplicial `facet` by a fan of triangles from its first
    /// vertex (qhull's `qh_triangulate_facet`). The triangles share the
    /// facet's hyperplane.
    fn triangulate_facet(&mut self, facet: FacetId) -> HullResult<()> {
        for n in self.facet(facet).neighbors.clone() {
            let neighbor = self.facet_mut(n);
            neighbor.seen = false;
            neighbor.coplanarhorizon = false;
        }
        self.visible_list = Some(FacetId::TAIL);
        self.newfacet_list = Some(FacetId::TAIL);
        let apex = self.facet(facet).vertices[0];
        self.make_new_nonsimplicial(facet, apex)?;
        self.will_delete(facet, None);
        let source = self.facet(facet);
        let (upper, normal, offset) = (source.upperdelaunay, source.normal(), source.offset);
        for f in self.facets_from(self.newfacet_list) {
            let triangle = self.facet_mut(f);
            triangle.tricoplanar = true;
            triangle.link = Some(facet);
            triangle.upperdelaunay = upper;
            triangle.set_normal(normal);
            triangle.offset = offset;
        }
        self.match_new_facets()?;
        self.newvertex_list = None;
        self.visible_list = None;
        self.reset_lists(false);
        Ok(())
    }

    /// Unlinks the null triangle `facet` (its first two vertices are the
    /// same) by linking its first two neighbours (qhull's
    /// `qh_triangulate_null`).
    fn triangulate_null(&mut self, facet: FacetId) -> HullResult<()> {
        let neighbors = &self.facet(facet).neighbors;
        let (first, second) = (neighbors[0], neighbors[1]);
        self.triangulate_link(facet, first, facet, second)?;
        self.will_delete(facet, None);
        Ok(())
    }

    /// Unlinks two mirrored triangles by linking their neighbours (qhull's
    /// `qh_triangulate_mirror`).
    fn triangulate_mirror(&mut self, facet_a: FacetId, facet_b: FacetId) -> HullResult<()> {
        let neighbors_a = self.facet(facet_a).neighbors.clone();
        for (i, &neighbor) in neighbors_a.iter().enumerate() {
            let neighbor_b = self.facet(facet_b).neighbors[i];
            if neighbor == facet_b && neighbor_b == facet_a {
                continue;
            }
            if self.facet(neighbor).redundant
                && self.facet(neighbor_b).redundant
                && self.has_mirror_merge(neighbor, neighbor_b)
            {
                continue;
            }
            if self.facet(neighbor).visible && self.facet(neighbor_b).visible {
                continue;
            }
            self.triangulate_link(facet_a, neighbor, facet_b, neighbor_b)?;
        }
        self.will_delete(facet_a, None);
        self.will_delete(facet_b, None);
        Ok(())
    }

    /// Links `facet_a` and `facet_b` in place of `old_a` and `old_b`,
    /// queueing them as mirrors if already linked (qhull's
    /// `qh_triangulate_link`).
    fn triangulate_link(
        &mut self,
        old_a: FacetId,
        facet_a: FacetId,
        old_b: FacetId,
        facet_b: FacetId,
    ) -> HullResult<()> {
        let a_has_b = self.facet(facet_a).neighbors.contains(&facet_b);
        let b_has_a = self.facet(facet_b).neighbors.contains(&facet_a);
        if a_has_b != b_has_a {
            return Err(HullError::Topology);
        }
        let (a, b) = (self.facet(facet_a), self.facet(facet_b));
        if a_has_b && !b.redundant {
            if a.redundant || (b.flipped && !a.flipped) || a.vertices != b.vertices {
                return Err(HullError::Topology);
            }
            self.append_merge(facet_a, facet_b, MergeKind::Mirror, 0.0);
        }
        sets::replace(&mut self.facet_mut(facet_b).neighbors, old_b, facet_a);
        sets::replace(&mut self.facet_mut(facet_a).neighbors, old_a, facet_b);
        Ok(())
    }

    /// Whether a mirror merge of `a` and `b` is queued (qhull's
    /// `qh_hasmerge`).
    fn has_mirror_merge(&mut self, a: FacetId, b: FacetId) -> bool {
        self.merge_sets().degen.iter().any(|m| {
            m.kind == MergeKind::Mirror
                && ((m.facet1 == a && m.facet2 == b) || (m.facet1 == b && m.facet2 == a))
        })
    }
}
