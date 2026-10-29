//! Matching the new facets of a cone to each other across their ridges
//! (`qh_matchnewfacets`, `qh_matchneighbor`, `qh_matchdupridge`).
//!
//! qhull hashes ridges by vertex addresses; here vertex ids play that role.
//! The hash only orders the probe sequence, which decides nothing unless a
//! ridge is shared by more than two new facets.

use super::hull::{FacetId, Hull, HullError, HullResult, VertexId};
use super::precision::QhullMinMax;

/// Open-addressing table of new facets keyed by one of their ridges.
struct HashTable {
    slots: Vec<Option<FacetId>>,
}

impl HashTable {
    /// A table with at least `size` slots, sized like qhull's
    /// `qh_newhashtable`.
    fn new(size: usize) -> Self {
        let mut len = ((size + 1) * 2) | 1;
        while len.is_multiple_of(3) || len.is_multiple_of(5) {
            len += 2;
        }
        Self {
            slots: vec![None; len],
        }
    }

    /// The probe start for the ridge of `vertices` opposite `skip` (all
    /// vertices but the shared apex at index 0 and the skipped one).
    fn hash(&self, vertices: &[VertexId], skip: usize) -> usize {
        let sum: usize = vertices[1..].iter().map(|v| v.index()).sum();
        (sum - vertices[skip].index()) % self.slots.len()
    }

    /// The slot after `slot`, wrapping around.
    fn next(&self, slot: usize) -> usize {
        if slot + 1 >= self.slots.len() {
            0
        } else {
            slot + 1
        }
    }

    /// Inserts `facet` in the first free slot from `hash` unless it is
    /// already in the probe sequence (qhull's `qh_addhash`).
    fn add(&mut self, facet: FacetId, hash: usize) {
        let mut slot = hash;
        while let Some(f) = self.slots[slot] {
            if f == facet {
                return;
            }
            slot = self.next(slot);
        }
        self.slots[slot] = Some(facet);
    }
}

/// Whether two new facets sharing their first vertex share the ridge of
/// `a` opposite `skip_a`: returns the index skipped in `b` and whether the
/// ridge has the same orientation in both (qhull's `qh_matchvertices`).
fn match_vertices(a: &[VertexId], skip_a: usize, b: &[VertexId]) -> Option<(usize, bool)> {
    let other = a[3 - skip_a];
    let skip_b = if b[1] == other {
        2
    } else if b[2] == other {
        1
    } else {
        return None;
    };
    Some((skip_b, skip_a & 1 == skip_b & 1))
}

/// A facet slot: a facet and the index of the vertex opposite the ridge.
type Slot = (FacetId, usize);

/// The state of qhull's `qh_matchdupridge` while it ranks the pairings of
/// a duplicate ridge.
struct DupridgeSearch {
    /// Largest merge distance tolerated for a "good" pairing.
    max_good: f64,
    /// A vertex is duplicated across the ridge.
    is_duplicate: bool,
    /// `goodmatch`: the preferred pairing's first slot.
    good: Option<Slot>,
    /// `goodmatch2`: its second slot, consumed when merge ridges are made.
    good2: Option<Slot>,
    /// Merge distance of the good pairing.
    good_dist: f64,
    /// `maxmatch` and `maxmatch2`: the widest pairing.
    max: Option<(Slot, Slot)>,
    /// Width of the widest pairing.
    max_dist: f64,
    /// Width of the second widest pairing.
    max_dist2: f64,
}

impl DupridgeSearch {
    /// Makes `a`/`b` the preferred pairing, with merge distance `dist`.
    fn keep_good(&mut self, a: Slot, b: Slot, dist: f64) {
        self.good = Some(a);
        self.good2 = Some(b);
        self.good_dist = dist;
    }
}

impl Hull {
    /// Links the new facets to each other across their ridges; duplicate
    /// ridges are paired by `match_dupridge` (qhull's `qh_matchnewfacets`).
    /// Returns the widest distance of a duplicate-ridge merge.
    pub(super) fn match_new_facets(&mut self) -> HullResult<f64> {
        let newfacets = self.facets_from(self.newfacet_list);
        for &f in &newfacets {
            let neighbors = &mut self.facet_mut(f).neighbors;
            neighbors.truncate(1);
            neighbors.resize(3, FacetId::NONE);
        }
        let mut table = HashTable::new(newfacets.len() * 2);
        let mut unmatched: i64 = 0;
        for &f in &newfacets {
            for skip in 1..3 {
                self.match_neighbor(&mut table, f, skip, &mut unmatched)?;
            }
        }
        let mut max_dupdist = 0.0_f64;
        if unmatched != 0 {
            for &f in &newfacets {
                if !self.facet(f).dupridge {
                    continue;
                }
                for skip in 0..self.facet(f).neighbors.len() {
                    if self.facet(f).neighbors[skip] == FacetId::DUPLICATE_RIDGE {
                        let dist = self.match_dupridge(&table, (f, skip), &mut unmatched)?;
                        max_dupdist = max_dupdist.max_qhull(dist);
                    }
                }
            }
        }
        if unmatched != 0 {
            return Err(HullError::Topology);
        }
        Ok(max_dupdist)
    }

    /// Matches the ridge of `facet` opposite `skip` with a hashed facet, or
    /// hashes it; marks duplicate ridges (qhull's `qh_matchneighbor`).
    fn match_neighbor(
        &mut self,
        table: &mut HashTable,
        facet: FacetId,
        skip: usize,
        unmatched: &mut i64,
    ) -> HullResult<()> {
        let hash = table.hash(&self.facet(facet).vertices, skip);
        let mut found_self = false;
        let mut slot = hash;
        while let Some(other) = table.slots[slot] {
            if other == facet {
                found_self = true;
                slot = table.next(slot);
                continue;
            }
            let Some((other_skip, same)) = match_vertices(
                &self.facet(facet).vertices,
                skip,
                &self.facet(other).vertices,
            ) else {
                slot = table.next(slot);
                continue;
            };
            let is_match = same == (self.facet(facet).toporient ^ self.facet(other).toporient);
            let matched = self.facet(other).neighbors[other_skip];
            if is_match && matched == FacetId::NONE {
                self.facet_mut(other).neighbors[other_skip] = facet;
                self.facet_mut(facet).neighbors[skip] = other;
                *unmatched -= 1;
                return Ok(());
            }
            self.facet_mut(facet).neighbors[skip] = FacetId::DUPLICATE_RIDGE;
            self.facet_mut(facet).dupridge = true;
            table.add(facet, hash);
            *unmatched += 1;
            if matched != FacetId::DUPLICATE_RIDGE {
                self.facet_mut(other).neighbors[other_skip] = FacetId::DUPLICATE_RIDGE;
                self.facet_mut(other).dupridge = true;
                if matched != FacetId::NONE {
                    let back = self
                        .facet(matched)
                        .neighbors
                        .iter()
                        .position(|&n| n == other)
                        .ok_or(HullError::Topology)?;
                    self.facet_mut(matched).neighbors[back] = FacetId::DUPLICATE_RIDGE;
                    self.facet_mut(matched).dupridge = true;
                    table.add(matched, hash);
                    *unmatched += 2;
                }
            }
            return Ok(());
        }
        if !found_self {
            table.slots[slot] = Some(facet);
        }
        *unmatched += 1;
        Ok(())
    }

    /// Pairs the duplicate ridge at slot `at` with the facet giving the best
    /// merge and turns the remaining duplicates into merge ridges (qhull's
    /// `qh_matchdupridge`). Returns the merge distance.
    fn match_dupridge(
        &mut self,
        table: &HashTable,
        at: Slot,
        unmatched: &mut i64,
    ) -> HullResult<f64> {
        let hash = table.hash(&self.facet(at.0).vertices, at.1);
        let mut search = DupridgeSearch {
            max_good: self.prec.narrow_dupridge(),
            is_duplicate: false,
            good: None,
            good2: None,
            good_dist: 0.0,
            max: None,
            max_dist: -f64::MAX,
            max_dist2: 0.0,
        };
        for make_match in [false, true] {
            let visit = self.next_visit();
            let mut cursor = Some(at);
            while let Some((newfacet, newskip)) = cursor {
                cursor = None;
                self.facet_mut(newfacet).visit_id = visit;
                let mut slot = hash;
                while let Some(facet) = table.slots[slot] {
                    slot = table.next(slot);
                    if !self.facet(facet).dupridge || self.facet(facet).visit_id == visit {
                        continue;
                    }
                    let Some((skip, same)) = match_vertices(
                        &self.facet(newfacet).vertices,
                        newskip,
                        &self.facet(facet).vertices,
                    ) else {
                        continue;
                    };
                    if self.facet(newfacet).vertices[newskip] == self.facet(facet).vertices[skip] {
                        search.is_duplicate = true;
                    }
                    let is_match =
                        same == (self.facet(newfacet).toporient ^ self.facet(facet).toporient);
                    if self.facet(facet).neighbors[skip] != FacetId::DUPLICATE_RIDGE {
                        if !make_match {
                            return Err(HullError::Topology);
                        }
                    } else if !is_match {
                        cursor = Some((facet, skip));
                    } else if self.facet(newfacet).neighbors[newskip] == FacetId::DUPLICATE_RIDGE {
                        if make_match {
                            self.link_dupridge(&mut search, (facet, skip), (newfacet, newskip));
                            *unmatched -= 2;
                        } else if self.rank_dupridge(
                            &mut search,
                            (facet, skip),
                            (newfacet, newskip),
                        ) {
                            break;
                        }
                    }
                }
                if make_match && self.facet(newfacet).neighbors[newskip] == FacetId::DUPLICATE_RIDGE
                {
                    return Err(HullError::Topology);
                }
            }
            if !make_match {
                let (a, b) = match (search.good, search.good2) {
                    (Some(good), Some(good2)) => (good, good2),
                    _ => search.max.ok_or(HullError::Topology)?,
                };
                self.facet_mut(a.0).neighbors[a.1] = b.0;
                self.facet_mut(b.0).neighbors[b.1] = a.0;
                *unmatched -= 2;
            }
        }
        Ok(if search.good.is_some() {
            search.good_dist
        } else {
            search.max_dist2
        })
    }

    /// Ranks the pairing of `facet` and `newfacet` by the vertex distances
    /// across the ridge (the first pass of `qh_matchdupridge`); returns
    /// `true` to stop scanning.
    fn rank_dupridge(&mut self, search: &mut DupridgeSearch, facet: Slot, newfacet: Slot) -> bool {
        for f in [facet.0, newfacet.0] {
            if self.facet(f).normal.is_none() {
                self.set_facet_plane(f);
            }
        }
        let mut dupdist = self.get_distance(facet.0, newfacet.0).2;
        let dupdist2 = self.get_distance(newfacet.0, facet.0).2;
        let (f_flipped, n_flipped) = (self.facet(facet.0).flipped, self.facet(newfacet.0).flipped);
        let good_flipped = search.good.is_some_and(|(g, _)| self.facet(g).flipped);
        let max_good = search.max_good;
        if search.is_duplicate {
            search.good = None;
            search.max_dist = dupdist.min_qhull(dupdist2);
            search.max_dist2 = f64::MAX / 2.0;
            search.max = Some((facet, newfacet));
            return true;
        }
        if f_flipped && !n_flipped && dupdist < max_good {
            if search.good.is_none() || !good_flipped || dupdist < search.good_dist {
                search.keep_good(facet, newfacet, dupdist);
            }
        } else if n_flipped && !f_flipped && dupdist2 < max_good {
            if search.good.is_none() || !good_flipped || dupdist2 < search.good_dist {
                search.keep_good(newfacet, facet, dupdist2);
            }
        } else if dupdist < max_good && (!n_flipped || f_flipped) {
            if search.good.is_none() || (!good_flipped && dupdist < search.good_dist) {
                search.keep_good(facet, newfacet, dupdist);
            }
        } else if dupdist2 < max_good && (!f_flipped || n_flipped) {
            if search.good.is_none() || (!good_flipped && dupdist2 < search.good_dist) {
                search.keep_good(newfacet, facet, dupdist2);
            }
        } else if search.good.is_none() {
            if !n_flipped || f_flipped {
                dupdist = dupdist.min_qhull(dupdist2);
            }
            if dupdist > search.max_dist {
                search.max_dist2 = search.max_dist;
                search.max_dist = dupdist;
                search.max = Some((facet, newfacet));
            } else if dupdist > search.max_dist2 {
                search.max_dist2 = dupdist;
            }
        }
        false
    }

    /// Links a remaining duplicate-ridge pairing in the second pass of
    /// `qh_matchdupridge`, turning one side into a merge ridge.
    fn link_dupridge(&mut self, search: &mut DupridgeSearch, facet: Slot, newfacet: Slot) {
        if self.facet(newfacet.0).tricoplanar {
            self.facet_mut(facet.0).neighbors[facet.1] = newfacet.0;
            self.facet_mut(newfacet.0).neighbors[newfacet.1] = facet.0;
        } else if let (Some(_), Some(good2)) = (search.good, search.good2) {
            self.facet_mut(good2.0).neighbors[good2.1] = FacetId::MERGE_RIDGE;
            self.facet_mut(facet.0).neighbors[facet.1] = newfacet.0;
            self.facet_mut(newfacet.0).neighbors[newfacet.1] = facet.0;
            search.good2 = None;
        } else {
            self.facet_mut(facet.0).neighbors[facet.1] = newfacet.0;
            self.facet_mut(newfacet.0).neighbors[newfacet.1] = FacetId::MERGE_RIDGE;
        }
    }
}
