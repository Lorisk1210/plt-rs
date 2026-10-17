//! Resolving duplicate ridges left by vertex renames by merging the nearly
//! adjacent vertices (`qh_all_vertexmerges`, `qh_merge_pinchedvertices`,
//! `qh_next_vertexmerge`, `qh_rename_adjacentvertex`).

use super::VertexMerge;
use crate::tri::delaunay::hull::{FacetId, Hull, HullError, HullResult, VertexId};
use crate::tri::delaunay::sets;

/// qhull's `qh_WIDEpinched`: vertices further apart than this many
/// `ONEmerge` are not merged.
const WIDE_PINCHED: f64 = 100.0;

impl Hull {
    /// Merges the queued pinched vertices and the facet merges they cause
    /// (qhull's `qh_all_vertexmerges`).
    pub(in crate::tri::delaunay) fn all_vertex_merges(&mut self) -> HullResult<()> {
        while !self.merge_sets().vertex.is_empty() {
            self.merge_pinched_vertices()?;
            self.update_vertex_neighbors();
            self.get_merge_set();
            self.all_merges(true)?;
            self.partition_visible()?;
            self.delete_visible();
            self.reset_lists(true);
        }
        Ok(())
    }

    /// Renames each queued vertex to its nearly adjacent partner, nearest
    /// pair first (qhull's `qh_merge_pinchedvertices`).
    fn merge_pinched_vertices(&mut self) -> HullResult<()> {
        self.vertex_neighbors_all();
        if self.visible_list.is_some()
            || self.newfacet_list.is_some()
            || self.newvertex_list.is_some()
        {
            return Err(HullError::Topology);
        }
        self.visible_list = Some(FacetId::TAIL);
        self.newfacet_list = Some(FacetId::TAIL);
        self.newvertex_list = Some(VertexId::TAIL);
        while let Some(merge) = self.next_vertex_merge()? {
            self.rename_adjacent_vertex(merge.vertex1, merge.vertex2)?;
            self.merge_degen_redundant()?;
        }
        Ok(())
    }

    /// Removes and returns the queued vertex merge with the nearest
    /// vertices, dropping merges of deleted vertices (qhull's
    /// `qh_next_vertexmerge`).
    fn next_vertex_merge(&mut self) -> HullResult<Option<VertexMerge>> {
        let mut best: Option<(usize, f64)> = None;
        let mut i = 0;
        while let Some(merge) = self.merge_sets().vertex.get(i).copied() {
            if self.vertex(merge.vertex1).deleted || self.vertex(merge.vertex2).deleted {
                self.merge_sets().vertex.swap_remove(i);
            } else {
                if merge.distance < best.map_or(f64::MAX, |(_, d)| d) {
                    best = Some((i, merge.distance));
                }
                i += 1;
            }
        }
        let Some((index, dist)) = best else {
            return Ok(None);
        };
        if dist / self.prec.one_merge > WIDE_PINCHED {
            return Err(HullError::Topology);
        }
        Ok(Some(self.merge_sets().vertex.swap_remove(index)))
    }

    /// Renames `oldvertex` to the nearby `newvertex` in all its facets
    /// (qhull's `qh_rename_adjacentvertex`).
    fn rename_adjacent_vertex(
        &mut self,
        oldvertex: VertexId,
        newvertex: VertexId,
    ) -> HullResult<()> {
        self.total_merges += 1;
        if self.vertex(oldvertex).deleted || self.vertex(newvertex).deleted {
            return Ok(());
        }
        if self.vertex(oldvertex).neighbors.is_empty()
            || self.vertex(newvertex).neighbors.is_empty()
        {
            return Err(HullError::Topology);
        }
        for neighbor in self.vertex(oldvertex).neighbors.clone() {
            if !self.facet(neighbor).simplicial {
                continue;
            }
            if self.facet(neighbor).vertices.contains(&newvertex) {
                self.make_ridges(neighbor)?;
            } else {
                self.replace_facet_vertex(neighbor, oldvertex, newvertex)?;
                sets::append_unique(&mut self.vertex_mut(newvertex).neighbors, neighbor);
                let vertices = self.facet(neighbor).vertices.clone();
                self.renew_vertices(&vertices);
            }
        }
        let ridges = self.vertex_ridges(oldvertex, true);
        for neighbor in self.vertex(oldvertex).neighbors.clone() {
            if self.facet(neighbor).simplicial {
                continue;
            }
            self.add_facet_vertex(neighbor, newvertex);
            sets::append_unique(&mut self.vertex_mut(newvertex).neighbors, neighbor);
            let vertices = self.facet(neighbor).vertices.clone();
            self.renew_vertices(&vertices);
            if self.newfacet_list == Some(FacetId::TAIL) {
                self.move_to_end(neighbor);
                self.facet_mut(neighbor).newfacet = true;
            }
        }
        self.rename_vertex(oldvertex, newvertex, &ridges, None)?;
        let vertex = self.vertex_mut(oldvertex);
        if vertex.deleted {
            vertex.partitioned = true;
        }
        Ok(())
    }

    /// Replaces `oldvertex` by `newvertex` in simplicial `facet`, keeping
    /// the vertices sorted and the neighbours opposite them (qhull's
    /// `qh_replacefacetvertex`).
    fn replace_facet_vertex(
        &mut self,
        facet: FacetId,
        oldvertex: VertexId,
        newvertex: VertexId,
    ) -> HullResult<()> {
        let f = self.facet(facet);
        if !f.simplicial || f.vertices.contains(&newvertex) {
            return Err(HullError::Topology);
        }
        let old_i = f
            .vertices
            .iter()
            .position(|&v| v == oldvertex)
            .ok_or(HullError::Topology)?;
        let mut new_i = f
            .vertices
            .iter()
            .position(|&v| v < newvertex)
            .unwrap_or(f.vertices.len());
        if old_i < new_i {
            new_i -= 1;
        }
        let f = self.facet_mut(facet);
        if old_i & 1 != new_i & 1 {
            f.toporient = !f.toporient;
        }
        f.vertices.remove(old_i);
        f.vertices.insert(new_i, newvertex);
        let neighbor = f.neighbors.remove(old_i);
        f.neighbors.insert(new_i, neighbor);
        Ok(())
    }
}
