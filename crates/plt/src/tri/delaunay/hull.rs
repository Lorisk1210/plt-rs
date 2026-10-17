//! Storage for the convex hull under construction (qhull's `qhT`, `facetT`,
//! `vertexT` and `ridgeT`).
//!
//! Facets, vertices and ridges live in arenas indexed by their qhull id, so
//! an id is also the creation order qhull uses to sort vertex sets and to
//! number facets. Facet 0 and vertex 0 are the sentinels that terminate the
//! facet and vertex lists (`qh.facet_tail`, `qh.vertex_tail`). The lists are
//! doubly linked through the arena, exactly like qhull's, because the order
//! in which facets are visited decides the order of the output triangles.

use super::precision::{Precision, QhullMinMax};

/// A facet, identified by its qhull id (its index in [`Hull::facets`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct FacetId(u32);

impl FacetId {
    /// The sentinel facet terminating the facet list (`qh.facet_tail`).
    pub(super) const TAIL: Self = Self(0);
    /// An unmatched neighbour slot of a new facet (a null set entry).
    pub(super) const NONE: Self = Self(u32::MAX);
    /// qhull's `qh_DUPLICATEridge`: the slot's ridge is shared by more than
    /// two new facets and is still waiting for a match.
    pub(super) const DUPLICATE_RIDGE: Self = Self(u32::MAX - 1);
    /// qhull's `qh_MERGEridge`: the slot's duplicate ridge will be resolved
    /// by merging the facets.
    pub(super) const MERGE_RIDGE: Self = Self(u32::MAX - 2);

    /// The arena index.
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }

    /// The qhull id (creation number).
    pub(super) fn id(self) -> u32 {
        self.0
    }
}

/// A vertex, identified by its qhull id (its index in [`Hull::vertices`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct VertexId(u32);

impl VertexId {
    /// The sentinel vertex terminating the vertex list (`qh.vertex_tail`).
    pub(super) const TAIL: Self = Self(0);

    /// The arena index.
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}

/// A ridge (an edge between two facets of the 3-d hull).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct RidgeId(u32);

impl RidgeId {
    /// The arena index.
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}

/// Why qhull gave up on the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HullError {
    /// The points are (nearly) collinear: no initial simplex exists.
    Singular,
    /// Roundoff made the hull inconsistent (a wide merge or a flipped facet).
    Precision,
    /// The facet structure could not be repaired (qhull's topology errors).
    Topology,
}

/// The result of an operation that qhull may abort.
pub(super) type HullResult<T> = Result<T, HullError>;

/// A facet of the lifted 3-d hull (qhull's `facetT`).
#[derive(Clone, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors qhull's facetT bit flags, one per role"
)]
pub(super) struct Facet {
    /// Vertices, sorted by decreasing id.
    pub(super) vertices: Vec<VertexId>,
    /// Neighbours; for simplicial facets neighbour `i` is opposite vertex `i`.
    pub(super) neighbors: Vec<FacetId>,
    /// Explicit ridges (non-simplicial facets and their neighbours).
    pub(super) ridges: Vec<RidgeId>,
    /// qhull has allocated the ridge set (it may have become empty since);
    /// a visible facet with a ridge set takes the non-simplicial path when
    /// its cone is built.
    pub(super) ridge_set: bool,
    /// Points above the facet; the last one is the furthest.
    pub(super) outside: Vec<usize>,
    /// Coplanar and interior points assigned to the facet.
    pub(super) coplanar: Vec<usize>,
    /// Unit outer normal, once the hyperplane is set.
    pub(super) normal: Option<[f64; 3]>,
    /// Offset of the hyperplane (`normal . p + offset = 0` on the facet).
    pub(super) offset: f64,
    /// The facet's centrum, cached for convexity tests.
    pub(super) center: Option<[f64; 3]>,
    /// Distance of the furthest outside point.
    pub(super) furthest_dist: f64,
    /// Maximum distance of a point above the facet.
    pub(super) max_outside: f64,
    /// qhull's `f` union: the replacement of a visible facet, the next facet
    /// of a cycle of new facets (`samecycle`), the current cycle of a horizon
    /// facet (`newcycle`) or the owner of a triangulated facet.
    pub(super) link: Option<FacetId>,
    /// Next facet in the facet list (`None` only for the tail).
    pub(super) next: Option<FacetId>,
    /// Previous facet in the facet list.
    pub(super) previous: Option<FacetId>,
    /// Visit marker compared with [`Hull::visit_id`].
    pub(super) visit_id: u32,
    /// Number of merges into this facet.
    pub(super) nummerge: u16,
    /// Created with top orientation (decides the orientation of the normal).
    pub(super) toporient: bool,
    /// Has exactly three vertices and three neighbours, no explicit ridges.
    pub(super) simplicial: bool,
    /// Part of the upper hull of the lifted points (not a Delaunay triangle).
    pub(super) upperdelaunay: bool,
    /// Will be deleted (on the visible list).
    pub(super) visible: bool,
    /// On the new facet list.
    pub(super) newfacet: bool,
    /// Newly merged, vertices still to be reduced.
    pub(super) newmerge: bool,
    /// Convexity with the neighbours has been tested.
    pub(super) tested: bool,
    /// Scratch flag.
    pub(super) seen: bool,
    /// The interior point is above the hyperplane.
    pub(super) flipped: bool,
    /// Horizon facet coplanar with the point being added.
    pub(super) coplanarhorizon: bool,
    /// New facet that will merge into its coplanar horizon facet.
    pub(super) mergehorizon: bool,
    /// Its merge cycle has been processed.
    pub(super) cycledone: bool,
    /// Has a duplicate ridge.
    pub(super) dupridge: bool,
    /// Has (or a neighbour has) a [`FacetId::MERGE_RIDGE`] neighbour.
    pub(super) mergeridge: bool,
    /// A neighbour has a [`FacetId::MERGE_RIDGE`] neighbour.
    pub(super) mergeridge2: bool,
    /// Keep the centrum after merging.
    pub(super) keepcentrum: bool,
    /// Degenerate (too few neighbours), queued for merging.
    pub(super) degenerate: bool,
    /// Redundant (vertices contained in a neighbour), queued for merging.
    pub(super) redundant: bool,
    /// A triangle of a triangulated non-simplicial facet.
    pub(super) tricoplanar: bool,
}

/// A vertex of the hull (qhull's `vertexT`).
#[derive(Clone, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors qhull's vertexT bit flags, one per role"
)]
pub(super) struct Vertex {
    /// Index of the point in [`Hull::points`].
    pub(super) point: usize,
    /// Neighbouring facets, once [`Hull::vertex_neighbors`] is set.
    pub(super) neighbors: Vec<FacetId>,
    /// Next vertex in the vertex list (`None` only for the tail).
    pub(super) next: Option<VertexId>,
    /// Previous vertex in the vertex list.
    pub(super) previous: Option<VertexId>,
    /// Visit marker compared with [`Hull::vertex_visit`].
    pub(super) visit_id: u32,
    /// Scratch flag.
    pub(super) seen: bool,
    /// Will be deleted.
    pub(super) deleted: bool,
    /// Belonged to a deleted ridge.
    pub(super) delridge: bool,
    /// On the new vertex list.
    pub(super) newfacet: bool,
    /// A deleted vertex whose point has been repartitioned.
    pub(super) partitioned: bool,
}

/// A ridge between two facets (qhull's `ridgeT`).
#[derive(Clone, Debug)]
pub(super) struct Ridge {
    /// The two vertices, sorted by decreasing id.
    pub(super) vertices: Vec<VertexId>,
    /// The facet that sees the ridge with top orientation.
    pub(super) top: FacetId,
    /// The other facet.
    pub(super) bottom: FacetId,
    /// Tested for convexity.
    pub(super) tested: bool,
    /// The facets were found non-convex at this ridge.
    pub(super) nonconvex: bool,
}

impl Ridge {
    /// The facet across the ridge from `facet` (qhull's `otherfacet_`).
    pub(super) fn other(&self, facet: FacetId) -> FacetId {
        if self.top == facet {
            self.bottom
        } else {
            self.top
        }
    }
}

/// The hull under construction and qhull's global state for it.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors qhull's independent global flags"
)]
pub(super) struct Hull {
    /// The lifted points (input sites and the point at infinity).
    pub(super) points: Vec<[f64; 3]>,
    /// Facet arena, indexed by qhull id.
    pub(super) facets: Vec<Facet>,
    /// Vertex arena, indexed by qhull id.
    pub(super) vertices: Vec<Vertex>,
    /// Ridge arena.
    pub(super) ridges: Vec<Ridge>,
    /// First facet of the facet list.
    pub(super) facet_list: FacetId,
    /// First facet still to process for outside points.
    pub(super) facet_next: Option<FacetId>,
    /// First new facet (`qh.newfacet_list`).
    pub(super) newfacet_list: Option<FacetId>,
    /// First facet to delete (`qh.visible_list`).
    pub(super) visible_list: Option<FacetId>,
    /// First new vertex (`qh.newvertex_list`).
    pub(super) newvertex_list: Option<VertexId>,
    /// Number of facets in the facet list (visible ones included).
    pub(super) num_facets: usize,
    /// Number of facets on the visible list.
    pub(super) num_visible: usize,
    /// Current facet visit marker.
    pub(super) visit_id: u32,
    /// Current vertex visit marker.
    pub(super) vertex_visit: u32,
    /// Roundoff-derived constants.
    pub(super) prec: Precision,
    /// Centre of the initial simplex, inside the hull.
    pub(super) interior_point: [f64; 3],
    /// Maximum distance of a point above its facet (`qh.max_outside`).
    pub(super) max_outside: f64,
    /// Minimum distance of a vertex below a facet (`qh.min_vertex`).
    pub(super) min_vertex: f64,
    /// Target for "clearly outside" ratios (`qh.MAXoutside`).
    pub(super) max_outside_target: f64,
    /// The initial simplex is very narrow (`qh.NARROWhull`).
    pub(super) narrow: bool,
    /// Vertex neighbour sets are maintained (`qh.VERTEXneighbors`).
    pub(super) vertex_neighbors: bool,
    /// Partition into new facets by exhaustive search (`qh.findbestnew`).
    pub(super) findbestnew: bool,
    /// The new facets are known not to be sharp (`qh.findbest_notsharp`).
    pub(super) findbest_notsharp: bool,
    /// Total number of merges (qhull's `Ztotmerge` statistic).
    pub(super) total_merges: u32,
    /// Facet id guarding against infinite repartitioning.
    pub(super) repart_facetid: u32,
    /// Vertices to delete once their facets are gone (`qh.del_vertices`).
    pub(super) del_vertices: Vec<VertexId>,
    /// Pending facet and vertex merges, while merging is active.
    pub(super) merges: Option<super::merge::MergeSets>,
    /// Scratch set of coplanar facets for `qh_findbesthorizon`.
    pub(super) coplanar_facets: Vec<FacetId>,
}

impl Hull {
    /// An empty hull over `points` with the sentinel facet and vertex.
    pub(super) fn new(points: Vec<[f64; 3]>, prec: Precision) -> Self {
        let mut hull = Self {
            points,
            facets: Vec::new(),
            vertices: Vec::new(),
            ridges: Vec::new(),
            facet_list: FacetId::TAIL,
            facet_next: None,
            newfacet_list: Some(FacetId::TAIL),
            visible_list: None,
            newvertex_list: Some(VertexId::TAIL),
            num_facets: 0,
            num_visible: 0,
            visit_id: 0,
            vertex_visit: 0,
            interior_point: [0.0; 3],
            max_outside: 0.0,
            min_vertex: -prec.dist_round,
            max_outside_target: 0.0_f64
                .max_qhull(prec.one_merge + prec.dist_round)
                .max_qhull(prec.min_outside),
            prec,
            narrow: false,
            vertex_neighbors: false,
            findbestnew: false,
            findbest_notsharp: false,
            total_merges: 0,
            repart_facetid: 0,
            del_vertices: Vec::new(),
            merges: None,
            coplanar_facets: Vec::new(),
        };
        hull.new_facet();
        hull.new_vertex(usize::MAX);
        hull
    }

    /// The facet with id `f`.
    pub(super) fn facet(&self, f: FacetId) -> &Facet {
        &self.facets[f.index()]
    }

    /// The facet with id `f`, mutably.
    pub(super) fn facet_mut(&mut self, f: FacetId) -> &mut Facet {
        &mut self.facets[f.index()]
    }

    /// The vertex with id `v`.
    pub(super) fn vertex(&self, v: VertexId) -> &Vertex {
        &self.vertices[v.index()]
    }

    /// The vertex with id `v`, mutably.
    pub(super) fn vertex_mut(&mut self, v: VertexId) -> &mut Vertex {
        &mut self.vertices[v.index()]
    }

    /// The ridge with id `r`.
    pub(super) fn ridge(&self, r: RidgeId) -> &Ridge {
        &self.ridges[r.index()]
    }

    /// The ridge with id `r`, mutably.
    pub(super) fn ridge_mut(&mut self, r: RidgeId) -> &mut Ridge {
        &mut self.ridges[r.index()]
    }

    /// Coordinates of vertex `v`.
    pub(super) fn vertex_point(&self, v: VertexId) -> [f64; 3] {
        self.points[self.vertex(v).point]
    }

    /// The normal of facet `f`; every facet that is tested has one.
    pub(super) fn normal(&self, f: FacetId) -> [f64; 3] {
        self.facet(f)
            .normal
            .expect("facets are given a hyperplane before distance tests")
    }

    /// The id the next facet will get (qhull's `qh.facet_id`).
    pub(super) fn next_facet_id(&self) -> u32 {
        u32::try_from(self.facets.len()).expect("fewer than 2^32 facets")
    }

    /// Allocates a facet with the next id (qhull's `qh_newfacet`).
    pub(super) fn new_facet(&mut self) -> FacetId {
        let id = FacetId(self.next_facet_id());
        self.facets.push(Facet {
            max_outside: self.prec.dist_round,
            simplicial: true,
            newfacet: true,
            ..Facet::default()
        });
        id
    }

    /// Allocates a vertex for `point` with the next id (qhull's `qh_newvertex`).
    pub(super) fn new_vertex(&mut self, point: usize) -> VertexId {
        let id = VertexId(u32::try_from(self.vertices.len()).expect("fewer than 2^32 vertices"));
        self.vertices.push(Vertex {
            point,
            ..Vertex::default()
        });
        id
    }

    /// Allocates a ridge (qhull's `qh_newridge`).
    pub(super) fn new_ridge(
        &mut self,
        vertices: Vec<VertexId>,
        top: FacetId,
        bottom: FacetId,
    ) -> RidgeId {
        let id = RidgeId(u32::try_from(self.ridges.len()).expect("fewer than 2^32 ridges"));
        self.ridges.push(Ridge {
            vertices,
            top,
            bottom,
            tested: false,
            nonconvex: false,
        });
        id
    }

    /// Increments and returns the facet visit marker.
    pub(super) fn next_visit(&mut self) -> u32 {
        self.visit_id += 1;
        self.visit_id
    }

    /// Increments and returns the vertex visit marker.
    pub(super) fn next_vertex_visit(&mut self) -> u32 {
        self.vertex_visit += 1;
        self.vertex_visit
    }
}
