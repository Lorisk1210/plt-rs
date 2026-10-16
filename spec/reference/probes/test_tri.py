"""Unstructured triangular grids (matplotlib.tri).

Covers `Triangulation` (Delaunay via qhull and explicit triangles, masks, edges,
neighbors, plane coefficients), the default `TrapezoidMapTriFinder`,
`LinearTriInterpolator`, `CubicTriInterpolator` (kinds 'geom', 'min_E', 'user'),
`UniformTriRefiner` and `TriAnalyzer`.

Common argument conventions
===========================
* ``x``, ``y``: point coordinates (lists of floats, equal length).
* ``triangles``: explicit triangles as a list of ``[i, j, k]`` point indices, in
  the order given to ``Triangulation(x, y, triangles)``. ``null`` (only where
  noted) means "compute the Delaunay triangulation of x, y".
* ``mask``: ``null`` or a list of bools, one per triangle of ``triangles``
  (passed to the ``Triangulation`` constructor).
* ``qx``, ``qy``: query point coordinates (flat lists, equal length).
* Index-based outputs for explicit triangulations refer to the input order of
  ``triangles`` (matplotlib keeps that order).

Orientation
-----------
Delaunay triangles from qhull are always anticlockwise (positive signed area).
Explicit triangles are stored as given at construction time; the first time the
C++ triangulation is created (lazily: by ``edges``, ``neighbors``,
``calculate_plane_coefficients``, ``get_trifinder`` or any interpolator)
matplotlib corrects every triangle ``[a, b, c]`` with
``cross(p_b - p_a, p_c - p_a) < 0`` (strictly; zero-area triangles are left
alone) *in place* to ``[a, c, b]``.

Canonical forms (applied by the adapters; Rust must apply the same)
-------------------------------------------------------------------
* canonical triple: a triangle's vertex-index triple (as stored by matplotlib,
  i.e. after orientation correction) rotated so that its smallest index comes
  first, preserving the cyclic order (hence orientation). ``[5, 2, 9]`` ->
  ``[2, 9, 5]``.
* canonical triangle list: canonical triples sorted lexicographically.
* canonical neighbors: a list aligned with the canonical triangle list; entry
  ``k`` is ``[n0, n1, n2]`` where ``n_i`` is the canonical triple of the triangle
  sharing edge ``(v_i, v_(i+1)%3)`` of canonical triple ``k = [v0, v1, v2]``, or
  ``null`` when there is none.
* xy-triangle: a triangle given by its vertex coordinates
  ``[[x0, y0], [x1, y1], [x2, y2]]`` in matplotlib's stored (anticlockwise)
  order, rotated so that the lexicographically smallest vertex (by x, then y;
  first one on ties) comes first. xy-triangle lists are sorted lexicographically
  on the flattened 6-tuple. Used for refined triangulations, whose point
  numbering is an artefact (and may contain duplicate points, see below).
* Masked interpolation results (outside / NaN) are ``null``.

Ops
===
``tri.Triangulation.delaunay`` args ``{x, y}`` (points in general position:
    every interior edge's in-circle test is far from zero, so the Delaunay
    triangulation is unique). Value ``{"triangles": canonical triangle list,
    "neighbors": canonical neighbors, "edges": edges, "is_delaunay": true}``.
    ``edges`` is matplotlib's exact ``Triangulation.edges``: every edge of an
    unmasked triangle once, as ``[j, i]`` with ``j > i``, rows sorted by
    ``(i, j)`` (this order is deterministic, it does not depend on triangle
    order). Duplicate points: only the first occurrence (lowest index) is used
    by the triangles, later duplicates are silently left out. Errors: fewer than
    3 points -> ValueError; fewer than 3 unique points -> ValueError; all points
    collinear -> RuntimeError (qhull "singular input"); NaN/inf coordinates ->
    RuntimeError; x, y of different length or not 1-D -> ValueError.

``tri.Triangulation.delaunay_counts`` args ``{x, y}`` with cocircular points
    (regular grids / polygons) where the Delaunay triangulation is NOT unique.
    Only choice-independent facts are recorded: ``{"ntri", "nedges",
    "points_used"}`` (``points_used`` = sorted list of point indices referenced
    by any triangle).

``tri.Triangulation.explicit`` args ``{x, y, triangles, mask, set_mask_after}``.
    Builds ``Triangulation(x, y, triangles, mask)`` -- or, if
    ``set_mask_after`` is true, builds it without mask, reads ``edges`` and
    ``neighbors`` (creating the C++ object), then calls ``set_mask(mask)``.
    Value ``{"triangles_input": triangles as stored right after construction
    (before orientation correction; int32 conversion truncates float indices),
    "triangles": triangles after orientation correction (input order),
    "masked_triangles": get_masked_triangles() after correction (the UNmasked
    triangles, input order), "edges": as above (unmasked triangles only),
    "neighbors": raw matplotlib neighbors (input order, -1 for none),
    "is_delaunay": false}``.
    ``neighbors[t][e]`` is the triangle across edge ``(tri[t][e],
    tri[t][(e+1)%3])`` (corrected triangles). Algorithm (defines degenerate
    cases too): masked triangles get ``[-1, -1, -1]`` and are ignored; a map
    ``directed edge -> (tri, e)`` starts empty; for t in index order, e in 0..2,
    with ``(s, f)`` the edge: if ``(f, s)`` is in the map, link both sides and
    remove it, otherwise insert ``(s, f) -> (t, e)``. Errors (ValueError):
    index >= npoints or < 0, shape not (N, 3) (incl. empty list, 1-D list,
    ragged, non-numeric), mask length != ntri, x/y length mismatch.

``tri.Triangulation.calculate_plane_coefficients`` args ``{x, y, triangles,
    mask, z}`` -> list (input order) of ``[a, b, c]`` with ``z = a*x + b*y + c``
    on each triangle; masked triangles give ``[0, 0, 0]``. Computed from the
    corrected triangle ``[p0, p1, p2]``: ``n = (p1 - p0) x (p2 - p0)`` (3-D with
    z); if ``n.z != 0``: ``a = -n.x/n.z, b = -n.y/n.z, c = n.p0/n.z``; else
    (collinear) Moore-Penrose: ``s = |d1.xy|^2 + |d2.xy|^2, a = (d1.x d1.z +
    d2.x d2.z)/s, b = (d1.y d1.z + d2.y d2.z)/s, c = p0.z - a p0.x - b p0.y``.
    z length != npoints -> ValueError. tol 1e-12.

``tri.TrapezoidMapTriFinder.find`` args ``{x, y, triangles (or null =
    Delaunay), mask, qx, qy}`` -> per query point the canonical triple of the
    containing triangle, or ``null`` (matplotlib -1). Rules (all verified
    against matplotlib, independent of its internal randomised edge order):
    * strictly inside an unmasked triangle -> that triangle; outside every
      unmasked triangle (incl. inside masked triangles, holes, far away, +-inf
      coordinates) -> null.
    * exactly on the interior of an edge: direct the edge from its
      lexicographically smaller endpoint (by x, then y) to the larger one; the
      result is the unmasked triangle on the LEFT of that directed edge if
      there is one, otherwise the one on the right.
    * exactly on a vertex: the lowest-index (input order) unmasked triangle
      containing that vertex; null if the vertex is only used by masked
      triangles (or by none). (Only probed with explicit triangles.)
    * NaN x (any y) -> null; NaN y with x strictly outside [min x, max x] of
      the triangulation's vertices -> null. NaN y with x inside that range is
      NOT probed: matplotlib's answer is an artefact of its randomised
      trapezoid map (mt19937/std::shuffle) and can be any triangle.
    Delaunay queries are only made in general position (well away from edges).
    Errors: qx/qy length mismatch -> ValueError; invalid triangulations
    (duplicate or overlapping triangles) -> RuntimeError.

``tri.LinearTriInterpolator.__call__`` args ``{x, y, triangles (or null), mask,
    z, qx, qy}`` -> list of interpolated values (``a*x + b*y + c`` of the
    triangle found by the TriFinder rules above), null outside/NaN. tol 1e-12.
``tri.LinearTriInterpolator.gradient`` same args -> ``{"dzdx": [...],
    "dzdy": [...]}`` = plane coefficients ``a``, ``b`` of the found triangle
    (on edges/vertices the triangle chosen by the TriFinder rules matters).
    NaN-y queries are not used here (see TriFinder). tol 1e-12.
    Errors: z length != npoints -> ValueError; qx/qy length mismatch ->
    ValueError.

``tri.CubicTriInterpolator.__call__`` args ``{x, y, triangles, mask, z, kind,
    dz, qx, qy}``; ``kind`` in 'geom', 'min_E', 'user' (others ValueError);
    ``dz`` is ``null`` or ``[dzdx_list, dzdy_list]`` (nodal derivatives, used for
    'user'; 'user' with dz null -> ValueError; dz lists shorter than npoints
    -> IndexError; z length != npoints -> ValueError). Value: list of
    interpolated values (reduced Hsieh-Clough-Tocher element, C1; the
    triangle chosen on shared edges/vertices only matters up to rounding),
    null outside the unmasked triangles and for any NaN coordinate.
``tri.CubicTriInterpolator.gradient`` same args -> ``{"dzdx", "dzdy"}``.
    tol 1e-10 for 'geom'/'user', 1e-6 for 'min_E' (PCG solve; the probes are
    asserted to be within tol/10 of the exact energy minimiser). NaN in z
    makes the nodal derivatives of every node sharing a triangle with it NaN
    (so values in triangles touching those nodes are null); with 'min_E' the
    PCG then does not iterate and the 'geom' derivatives are used unchanged.

``tri.UniformTriRefiner.refine_triangulation`` args ``{x, y, triangles, mask,
    subdiv, return_tri_index}``. Each triangle is split into 4 via edge
    midpoints ``(p + q) * 0.5`` (exactly this formula, coordinate-wise),
    recursively ``subdiv`` times (``subdiv <= 0`` -> the input triangulation
    unchanged). Value ``{"npoints": number of refined points (incl.
    duplicates), "points": all refined points ``[x, y]`` sorted (by x, y, then
    parent key when return_tri_index), "parents": only if return_tri_index:
    list aligned with "points" giving the canonical triple of the original
    ancestor triangle (null for points in no triangle), "triangles": sorted
    xy-triangles of the unmasked refined triangles, "masked_triangles": sorted
    xy-triangles of masked refined triangles, "has_mask": refined mask is not
    None}``. Parent of a point shared by several original triangles: the
    highest-index one among the unmasked ancestors containing it, or (if all
    are masked) the highest-index masked one. Sort key for parents: null before
    any triple, triples lexicographic. Quirk: the midpoint of an edge between a
    masked and an unmasked triangle is created twice (masked triangles have no
    neighbours), giving duplicate points.

``tri.UniformTriRefiner.refine_field`` args ``{x, y, triangles, mask, z,
    interpolator, dz, subdiv}``; ``interpolator`` is null (matplotlib default:
    CubicTriInterpolator kind 'min_E'), 'linear', 'cubic_geom', 'cubic_min_E' or
    'cubic_user' (with ``dz``). Value: list of ``[x, y, z]`` for every refined
    point, sorted by (x, y, z is null, z); z null where masked/NaN. Each point
    is evaluated in its parent triangle (see above) with the interpolator.
    Quirks for points whose parent is masked: 'linear' gives 0.0 (masked
    triangles have zero plane coefficients); cubic kinds evaluate the HCT
    patch of the LAST unmasked triangle (highest input index) at that point.
    tol: linear 1e-12, cubic geom/user 1e-10, min_E 1e-6.

``tri.TriAnalyzer.scale_factors`` args ``{x, y, triangles, mask}`` ->
    ``[1/ptp(x_used), 1/ptp(y_used)]`` over points used by unmasked triangles.
``tri.TriAnalyzer.circle_ratios`` args ``{x, y, triangles, mask, rescale}`` ->
    per triangle (input order) inradius/circumradius after optional rescaling
    by scale_factors; null for masked triangles; 0.0 for exactly flat
    triangles. tol 1e-12.
``tri.TriAnalyzer.get_flat_tri_mask`` args ``{x, y, triangles, mask,
    min_circle_ratio, rescale}`` -> list of bools (input order): the initial
    mask extended by repeatedly masking border triangles (unmasked with a
    missing neighbour, given the current mask) whose circle ratio <
    min_circle_ratio, until nothing changes.
"""

from __future__ import annotations

import math

import matplotlib.tri as mtri
import matplotlib.tri._triinterpolate as _triinterp
import numpy as np
import pytest

from spec_support import ProbeSet, rng

probes = ProbeSet("tri")

NAN = float("nan")
INF = float("inf")

TOL_LINEAR = 1e-12
TOL_CUBIC = 1e-10
TOL_MIN_E = 1e-6
CUBIC_TOL = {"geom": TOL_CUBIC, "user": TOL_CUBIC, "min_E": TOL_MIN_E}


# ---------------------------------------------------------------------------
# Canonicalisation helpers
# ---------------------------------------------------------------------------


def _canon_triple(t) -> list[int]:
    t = [int(v) for v in t]
    i = t.index(min(t))
    return t[i:] + t[:i]


def _canon_triangles(tris) -> list[list[int]]:
    return sorted(_canon_triple(t) for t in tris)


def _canon_neighbors(triangles, neighbors) -> list:
    """Canonical neighbors (see module docstring), aligned with _canon_triangles."""
    triangles = np.asarray(triangles)
    neighbors = np.asarray(neighbors)
    rows = []
    for t, nb in zip(triangles, neighbors):
        t = [int(v) for v in t]
        r = t.index(min(t))
        rotated_nb = [int(nb[(r + i) % 3]) for i in range(3)]
        rows.append(
            (
                _canon_triple(t),
                [None if n < 0 else _canon_triple(triangles[n]) for n in rotated_nb],
            )
        )
    rows.sort(key=lambda row: row[0])
    return [row[1] for row in rows]


def _canon_xy_triangle(pts) -> list[list[float]]:
    pts = [(float(p[0]), float(p[1])) for p in pts]
    i = min(range(3), key=lambda k: pts[k])
    return [list(pts[(i + k) % 3]) for k in range(3)]


def _xy_tri_key(t):
    return (t[0][0], t[0][1], t[1][0], t[1][1], t[2][0], t[2][1])


def _masked_list(values) -> list:
    arr = np.ma.masked_invalid(np.ma.asarray(values, dtype=float))
    return [None if m else float(v) for v, m in zip(arr.data.tolist(), np.ma.getmaskarray(arr).tolist())]


def _triangulation(x, y, triangles=None, mask=None) -> mtri.Triangulation:
    return mtri.Triangulation(x, y, triangles, mask)


# ---------------------------------------------------------------------------
# Adapters (one per op)
# ---------------------------------------------------------------------------


def _delaunay(x, y):
    t = mtri.Triangulation(x, y)
    return {
        "triangles": _canon_triangles(t.triangles),
        "neighbors": _canon_neighbors(t.triangles, t.neighbors),
        "edges": t.edges.tolist(),
        "is_delaunay": bool(t.is_delaunay),
    }


def _delaunay_counts(x, y):
    t = mtri.Triangulation(x, y)
    return {
        "ntri": int(t.triangles.shape[0]),
        "nedges": int(t.edges.shape[0]),
        "points_used": sorted({int(v) for v in t.triangles.ravel()}),
    }


def _explicit(x, y, triangles, mask, set_mask_after):
    if set_mask_after:
        t = mtri.Triangulation(x, y, triangles)
        triangles_input = t.triangles.tolist()
        _ = t.edges, t.neighbors
        t.set_mask(mask)
    else:
        t = mtri.Triangulation(x, y, triangles, mask)
        triangles_input = t.triangles.tolist()
    t.get_cpp_triangulation()
    return {
        "triangles_input": triangles_input,
        "triangles": t.triangles.tolist(),
        "masked_triangles": t.get_masked_triangles().tolist(),
        "edges": t.edges.tolist(),
        "neighbors": t.neighbors.tolist(),
        "is_delaunay": bool(t.is_delaunay),
    }


def _plane_coefficients(x, y, triangles, mask, z):
    return _triangulation(x, y, triangles, mask).calculate_plane_coefficients(z)


def _find(x, y, triangles, mask, qx, qy):
    t = _triangulation(x, y, triangles, mask)
    idx = t.get_trifinder()(qx, qy)
    return [None if i < 0 else _canon_triple(t.triangles[i]) for i in np.ravel(idx).tolist()]


def _linear(x, y, triangles, mask, z):
    return mtri.LinearTriInterpolator(_triangulation(x, y, triangles, mask), z)


def _linear_call(x, y, triangles, mask, z, qx, qy):
    return _masked_list(_linear(x, y, triangles, mask, z)(qx, qy))


def _linear_gradient(x, y, triangles, mask, z, qx, qy):
    gx, gy = _linear(x, y, triangles, mask, z).gradient(qx, qy)
    return {"dzdx": _masked_list(gx), "dzdy": _masked_list(gy)}


def _cubic(x, y, triangles, mask, z, kind, dz):
    t = _triangulation(x, y, triangles, mask)
    if dz is not None:
        dz = (np.asarray(dz[0], dtype=float), np.asarray(dz[1], dtype=float))
    return mtri.CubicTriInterpolator(t, z, kind=kind, dz=dz)


def _cubic_call(x, y, triangles, mask, z, kind, dz, qx, qy):
    return _masked_list(_cubic(x, y, triangles, mask, z, kind, dz)(qx, qy))


def _cubic_gradient(x, y, triangles, mask, z, kind, dz, qx, qy):
    gx, gy = _cubic(x, y, triangles, mask, z, kind, dz).gradient(qx, qy)
    return {"dzdx": _masked_list(gx), "dzdy": _masked_list(gy)}


def _refine_triangulation(x, y, triangles, mask, subdiv, return_tri_index):
    base = _triangulation(x, y, triangles, mask)
    refiner = mtri.UniformTriRefiner(base)
    if return_tri_index:
        refi, found = refiner.refine_triangulation(return_tri_index=True, subdiv=subdiv)
        parents = [None if i < 0 else _canon_triple(base.triangles[i]) for i in found.tolist()]
    else:
        refi = refiner.refine_triangulation(subdiv=subdiv)
        parents = None
    xs, ys = refi.x.tolist(), refi.y.tolist()
    order = list(range(len(xs)))
    if parents is None:
        order.sort(key=lambda i: (xs[i], ys[i]))
    else:
        order.sort(key=lambda i: (xs[i], ys[i], () if parents[i] is None else tuple(parents[i])))
    tri_mask = refi.mask if refi.mask is not None else np.zeros(len(refi.triangles), dtype=bool)
    unmasked, masked = [], []
    for t, m in zip(refi.triangles.tolist(), tri_mask.tolist()):
        xy = _canon_xy_triangle([(xs[v], ys[v]) for v in t])
        (masked if m else unmasked).append(xy)
    out = {
        "npoints": len(xs),
        "points": [[xs[i], ys[i]] for i in order],
        "triangles": sorted(unmasked, key=_xy_tri_key),
        "masked_triangles": sorted(masked, key=_xy_tri_key),
        "has_mask": refi.mask is not None,
    }
    if parents is not None:
        out["parents"] = [parents[i] for i in order]
    return out


def _make_interpolator(t, z, interpolator, dz):
    if interpolator is None:
        return None
    if interpolator == "linear":
        return mtri.LinearTriInterpolator(t, z)
    kind = {"cubic_geom": "geom", "cubic_min_E": "min_E", "cubic_user": "user"}[interpolator]
    if dz is not None:
        dz = (np.asarray(dz[0], dtype=float), np.asarray(dz[1], dtype=float))
    return mtri.CubicTriInterpolator(t, z, kind=kind, dz=dz)


def _refine_field(x, y, triangles, mask, z, interpolator, dz, subdiv):
    t = _triangulation(x, y, triangles, mask)
    interp = _make_interpolator(t, z, interpolator, dz)
    refi, rz = mtri.UniformTriRefiner(t).refine_field(z, triinterpolator=interp, subdiv=subdiv)
    zs = _masked_list(rz)
    rows = [[float(a), float(b), c] for a, b, c in zip(refi.x.tolist(), refi.y.tolist(), zs)]
    rows.sort(key=lambda r: (r[0], r[1], r[2] is None, 0.0 if r[2] is None else r[2]))
    return rows


def _scale_factors(x, y, triangles, mask):
    kx, ky = mtri.TriAnalyzer(_triangulation(x, y, triangles, mask)).scale_factors
    return [float(kx), float(ky)]


def _circle_ratios(x, y, triangles, mask, rescale):
    return _masked_list(mtri.TriAnalyzer(_triangulation(x, y, triangles, mask)).circle_ratios(rescale=rescale))


def _flat_tri_mask(x, y, triangles, mask, min_circle_ratio, rescale):
    analyzer = mtri.TriAnalyzer(_triangulation(x, y, triangles, mask))
    return [bool(v) for v in analyzer.get_flat_tri_mask(min_circle_ratio=min_circle_ratio, rescale=rescale)]


# ---------------------------------------------------------------------------
# Deterministic meshes
# ---------------------------------------------------------------------------


def _grid(n: int, m: int | None = None):
    """n x m integer grid, point index j*n + i at (i, j); alternating diagonals."""
    m = n if m is None else m
    xs = [float(i) for j in range(m) for i in range(n)]
    ys = [float(j) for j in range(m) for i in range(n)]
    tris = []
    for j in range(m - 1):
        for i in range(n - 1):
            a = j * n + i
            b, c = a + 1, a + n
            d = c + 1
            if (i + j) % 2 == 0:
                tris += [[a, b, d], [a, d, c]]
            else:
                tris += [[a, b, c], [b, d, c]]
    return xs, ys, tris


# 3x3 grid on [0, 2]^2, all diagonals (0,0)-(1,1) style.
G3X = [0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0]
G3Y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0]
G3T = [[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4], [3, 4, 7], [3, 7, 6], [4, 5, 8], [4, 8, 7]]
# Same triangles, several given clockwise and with rotated starting vertex.
G3T_MIXED = [[0, 4, 1], [4, 3, 0], [2, 5, 1], [5, 4, 1], [7, 3, 4], [3, 6, 7], [8, 5, 4], [7, 8, 4]]
G3MASK = [False, False, False, True, False, True, False, False]

G4X, G4Y, G4T = _grid(4)
# Square ring: the 4x4 grid without the central cell (a hole at (1, 2) x (1, 2)).
HOLE_T = [t for t in G4T if not all(v in (5, 6, 9, 10) for v in t)]
G4MASK = [i in (0, 7, 8, 17) for i in range(len(G4T))]


def _jittered_grid(seed: int, n: int, amp: float):
    r = rng(seed)
    xs, ys = [], []
    for j in range(n):
        for i in range(n):
            xs.append(i + amp * (r.next_f64() - 0.5))
            ys.append(j + amp * (r.next_f64() - 0.5))
    return xs, ys


JX, JY = _jittered_grid(21, 5, 0.35)
_r = rng(7)
RX = _r.uniform(30).tolist()
RY = _r.uniform(30).tolist()
_GOLDEN_ANGLE = math.pi * (3.0 - math.sqrt(5.0))
SX = [math.sqrt(i + 0.5) * math.cos(i * _GOLDEN_ANGLE) for i in range(24)]
SY = [math.sqrt(i + 0.5) * math.sin(i * _GOLDEN_ANGLE) for i in range(24)]
_r = rng(5)
DX = _r.uniform(8).tolist()
DY = _r.uniform(8).tolist()

# Mesh with flat (sliver) border triangles: a slightly concave bottom border.
_r = rng(31)
FX = [k / 10 for k in range(11)] + _r.uniform(14, 0.05, 0.95).tolist() + [0.0, 1.0]
FY = [0.02 * math.sin(math.pi * k / 10) * (1 + 0.04 * k) for k in range(11)] + _r.uniform(14, 0.2, 0.95).tolist() + [1.0, 1.0]

# Explicit (canonical) copies of Delaunay triangulations for probes that need
# explicit triangles. The point sets are in general position (asserted in
# test_delaunay_meshes_are_unique), so these lists are platform independent.
JT = _canon_triangles(mtri.Triangulation(JX, JY).triangles)
# The Delaunay triangulation of the jittered grid has near-flat border slivers,
# which make the 'min_E' energy minimisation ill-conditioned; interpolation
# probes use the (valid, anticlockwise) grid connectivity on the same points.
JGT = _grid(5)[2]
RT = _canon_triangles(mtri.Triangulation(RX, RY).triangles)
FT = _canon_triangles(mtri.Triangulation(FX, FY).triangles)


def _field(xs, ys):
    """Smooth test field kept well away from zero."""
    return [2.0 + math.cos(0.9 * a) * math.sin(0.7 * b + 0.3) + 0.25 * a * b for a, b in zip(xs, ys)]


def _field_grad(xs, ys):
    dx = [-0.9 * math.sin(0.9 * a) * math.sin(0.7 * b + 0.3) + 0.25 * b for a, b in zip(xs, ys)]
    dy = [0.7 * math.cos(0.9 * a) * math.cos(0.7 * b + 0.3) + 0.25 * a for a, b in zip(xs, ys)]
    return [dx, dy]


def _linear_field(xs, ys):
    return [3.0 + 0.75 * a - 0.5 * b for a, b in zip(xs, ys)]


# Query sets ---------------------------------------------------------------

def _grid_queries():
    """Queries for the 3x3 grid G3: inside, edges, vertices, outside, non-finite."""
    inside = [(0.3, 0.1), (0.7, 0.2), (0.2, 0.6), (1.6, 0.3), (1.3, 0.8), (0.25, 1.6),
              (0.8, 1.1), (1.2, 1.7), (1.9, 1.95), (1.55, 1.2)]
    edges = [(0.5, 0.0), (1.5, 0.0), (0.0, 0.5), (2.0, 0.5), (0.0, 1.5), (2.0, 1.5),
             (0.5, 2.0), (1.5, 2.0), (0.5, 0.5), (1.5, 0.5), (0.5, 1.5), (1.5, 1.5),
             (1.0, 0.5), (1.0, 1.5), (0.5, 1.0), (1.5, 1.0), (0.25, 0.25), (1.75, 1.0)]
    vertices = list(zip(G3X, G3Y))
    outside = [(-0.5, 1.0), (2.5, 1.0), (1.0, -0.5), (1.0, 2.5), (3.0, 3.0), (-1.0, -1.0),
               (2.5, 0.0), (-0.5, 0.0), (0.0, -0.25), (1e300, 1.0), (-1e300, -1e300)]
    nonfinite = [(NAN, 1.0), (NAN, NAN), (NAN, -5.0), (3.0, NAN), (-1.0, NAN), (INF, 1.0),
                 (-INF, 1.0), (1.0, INF), (1.0, -INF), (0.5, INF), (INF, INF), (-INF, -INF)]
    return inside, edges, vertices, outside, nonfinite


def _split(points):
    return [p[0] for p in points], [p[1] for p in points]


def _segment_distance(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)


def _min_edge_distance(x, y, triangles, qx, qy):
    edges = {tuple(sorted((t[i], t[(i + 1) % 3]))) for t in triangles for i in range(3)}
    best = INF
    for px, py in zip(qx, qy):
        for a, b in edges:
            best = min(best, _segment_distance(px, py, x[a], y[a], x[b], y[b]))
    return best


def _incircle_margin(x, y, triangles):
    """Smallest normalised |incircle| determinant over interior edges."""
    t = mtri.Triangulation(x, y, triangles)
    tris, nbrs = t.triangles, t.neighbors
    worst = INF
    for i, (tri, nb) in enumerate(zip(tris, nbrs)):
        for e in range(3):
            j = nb[e]
            if j < 0:
                continue
            d = [v for v in tris[j] if v not in tri][0]
            a, b, c = tri
            rows = []
            for v in (a, b, c):
                ux, uy = x[v] - x[d], y[v] - y[d]
                rows.append([ux, uy, ux * ux + uy * uy])
            det = np.linalg.det(np.array(rows))
            scale = max(abs(r[2]) for r in rows) ** 2
            worst = min(worst, abs(det) / scale)
    return worst


# ---------------------------------------------------------------------------
# Triangulation
# ---------------------------------------------------------------------------

DELAUNAY_SETS = {
    "jittered_grid": (JX, JY),
    "random30": (RX, RY),
    "spiral": (SX, SY),
    "three_ccw": ([0.0, 1.0, 0.2], [0.0, 0.1, 1.0]),
    "three_cw": ([0.0, 0.2, 1.0], [0.0, 1.0, 0.1]),
    "four": ([0.0, 1.0, 0.1, 1.3], [0.0, 0.2, 1.0, 1.1]),
    "pentagon_irregular": ([0.0, 2.0, 2.5, 1.1, -0.4], [0.0, 0.3, 1.9, 2.6, 1.4]),
    "random8": (DX, DY),
    "random8_dup_last": (DX + [DX[3]], DY + [DY[3]]),
    "random8_dup_first": ([DX[5]] + DX, [DY[5]] + DY),
    "integers_as_ints": ([0, 3, 1, 4, 2], [0, 1, 3, 4, 7]),
}


def test_jittered_grid_connectivity_is_valid():
    for a, b, c in JGT:
        assert (JX[b] - JX[a]) * (JY[c] - JY[a]) - (JY[b] - JY[a]) * (JX[c] - JX[a]) > 0.1


def test_delaunay_meshes_are_unique():
    for name, (x, y) in {**DELAUNAY_SETS, "flat": (FX, FY)}.items():
        tris = mtri.Triangulation(x, y).triangles
        assert _incircle_margin(x, y, tris) > 1e-6, name


@pytest.mark.parametrize("name", sorted(DELAUNAY_SETS))
def test_delaunay(name):
    x, y = DELAUNAY_SETS[name]
    out = probes.probe("tri.Triangulation.delaunay", _delaunay, x=x, y=y)
    xs, ys = np.asarray(x, float), np.asarray(y, float)
    for a, b, c in out["triangles"]:
        assert (xs[b] - xs[a]) * (ys[c] - ys[a]) - (ys[b] - ys[a]) * (xs[c] - xs[a]) > 0
    # qhull's neighbours agree with the C++ recomputation.
    t = mtri.Triangulation(x, y)
    recomputed = mtri.Triangulation(x, y, t.triangles.copy())
    assert _canon_neighbors(recomputed.triangles, recomputed.neighbors) == out["neighbors"]


def test_delaunay_duplicates_keep_first():
    out = probes.probe("tri.Triangulation.delaunay", _delaunay, x=DX + [DX[3]], y=DY + [DY[3]])
    assert 8 not in {v for t in out["triangles"] for v in t}
    out = probes.probe("tri.Triangulation.delaunay", _delaunay, x=[DX[5]] + DX, y=[DY[5]] + DY)
    assert 6 not in {v for t in out["triangles"] for v in t}


@pytest.mark.parametrize(
    "x, y, err",
    [
        ([0.0, 1.0], [0.0, 1.0], ValueError),
        ([], [], ValueError),
        ([1.0, 1.0, 1.0], [2.0, 2.0, 2.0], ValueError),
        ([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], ValueError),
        ([0.0, 1.0, 2.0, 3.0], [0.0, 1.0, 2.0, 3.0], RuntimeError),
        ([0.0, 1.0, 2.0, 3.0, 5.0], [1.0, 1.0, 1.0, 1.0, 1.0], RuntimeError),
        ([0.0, 1.0, 0.0, NAN], [0.0, 0.0, 1.0, 1.0], RuntimeError),
        ([0.0, 1.0, 0.0, INF], [0.0, 0.0, 1.0, 1.0], RuntimeError),
        ([0.0, 1.0, 2.0], [0.0, 1.0], ValueError),
        ([[0.0, 1.0, 0.0]], [[0.0, 0.0, 1.0]], ValueError),
    ],
)
def test_delaunay_errors(x, y, err):
    assert isinstance(probes.probe("tri.Triangulation.delaunay", _delaunay, x=x, y=y), err)


def test_delaunay_cocircular_counts():
    octagon = [2 * math.pi * k / 8 for k in range(8)]
    hexagon_c = [2 * math.pi * k / 6 for k in range(6)]
    cases = [
        (G3X, G3Y, 8, 16),
        (*_grid(4, 3)[:2], 12, 23),
        ([math.cos(a) for a in octagon], [math.sin(a) for a in octagon], 6, 13),
        ([math.cos(a) for a in hexagon_c] + [0.0], [math.sin(a) for a in hexagon_c] + [0.0], 6, 12),
    ]
    for x, y, ntri, nedges in cases:
        out = probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=x, y=y)
        assert (out["ntri"], out["nedges"]) == (ntri, nedges)
        assert out["points_used"] == list(range(len(x)))


EXPLICIT_CASES = {
    "grid3": (G3X, G3Y, G3T, None),
    "grid3_mixed_orientation": (G3X, G3Y, G3T_MIXED, None),
    "grid3_masked": (G3X, G3Y, G3T, G3MASK),
    "grid3_mask_all_false": (G3X, G3Y, G3T, [False] * 8),
    "grid3_mask_all_true": (G3X, G3Y, G3T, [True] * 8),
    "grid3_mask_ints": (G3X, G3Y, G3T, [0, 1, 0, 0, 0, 0, 1, 0]),
    "hole": (G4X, G4Y, HOLE_T, None),
    "unused_points": ([0.0, 1.0, 0.0, 5.0, 1.0], [0.0, 0.0, 1.0, 5.0, 1.0], [[0, 1, 2], [1, 4, 2]], None),
    "collinear_triangle": ([0.0, 1.0, 2.0, 1.0], [0.0, 0.0, 0.0, 1.0], [[0, 2, 1], [0, 1, 3], [1, 2, 3]], None),
    "repeated_vertex": ([0.0, 1.0], [0.0, 1.0], [[0, 1, 1]], None),
    "duplicate_triangle": ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [[0, 1, 2], [0, 1, 2]], None),
    "float_indices_truncate": ([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0], [[0, 1.7, 2.2], [1, 3, 2]], None),
    "jittered_delaunay_explicit": (JX, JY, JT, None),
}


@pytest.mark.parametrize("name", sorted(EXPLICIT_CASES))
def test_explicit(name):
    x, y, tris, mask = EXPLICIT_CASES[name]
    out = probes.probe("tri.Triangulation.explicit", _explicit, x=x, y=y, triangles=tris, mask=mask,
                       set_mask_after=False)
    assert not isinstance(out, Exception)
    if mask is not None:
        after = probes.probe("tri.Triangulation.explicit", _explicit, x=x, y=y, triangles=tris, mask=mask,
                             set_mask_after=True)
        for key in ("triangles", "masked_triangles", "edges", "neighbors"):
            assert after[key] == out[key]


def test_explicit_orientation_is_corrected_lazily():
    out = probes.probe("tri.Triangulation.explicit", _explicit, x=G3X, y=G3Y, triangles=G3T_MIXED, mask=None,
                       set_mask_after=False)
    assert out["triangles_input"] == G3T_MIXED
    assert _canon_triangles(out["triangles"]) == _canon_triangles(G3T)
    for before, after in zip(G3T_MIXED, out["triangles"]):
        assert after in (before, [before[0], before[2], before[1]])
    reference = _explicit(G3X, G3Y, G3T, None, False)
    assert out["edges"] == reference["edges"]
    assert _canon_neighbors(out["triangles"], out["neighbors"]) == _canon_neighbors(
        reference["triangles"], reference["neighbors"])


def test_explicit_mask_effects():
    out = _explicit(G3X, G3Y, G3T, G3MASK, False)
    assert out["neighbors"][3] == [-1, -1, -1] and out["neighbors"][5] == [-1, -1, -1]
    assert [6, 3] not in out["edges"]  # edge 3-6 only belongs to masked triangle 5
    assert out["masked_triangles"] == [t for t, m in zip(G3T, G3MASK) if not m]


@pytest.mark.parametrize(
    "x, y, tris, mask",
    [
        (G3X, G3Y, [[0, 1, 9]], None),
        (G3X, G3Y, [[0, 1, -1]], None),
        (G3X, G3Y, [[0, 1]], None),
        (G3X, G3Y, [0, 1, 2], None),
        (G3X, G3Y, [], None),
        (G3X, G3Y, [[0, 1, 2], [1, 2]], None),
        (G3X, G3Y, [["a", "b", "c"]], None),
        (G3X, G3Y, [[0, 1, 2, 3]], None),
        (G3X, G3Y, G3T, [False, True]),
        (G3X, G3Y, G3T, True),
        ([0.0, 1.0, 0.0], [0.0, 1.0], [[0, 1, 2]], None),
    ],
)
def test_explicit_errors(x, y, tris, mask):
    out = probes.probe("tri.Triangulation.explicit", _explicit, x=x, y=y, triangles=tris, mask=mask,
                       set_mask_after=False)
    assert isinstance(out, ValueError)


def test_plane_coefficients():
    z3 = [1.0, 2.5, 3.0, 0.5, 4.0, 1.5, 2.0, 3.5, 5.0]
    cases = [
        (G3X, G3Y, G3T, None, z3),
        (G3X, G3Y, G3T_MIXED, None, z3),
        (G3X, G3Y, G3T, G3MASK, z3),
        (G3X, G3Y, G3T, None, _linear_field(G3X, G3Y)),
        (JX, JY, JT, None, _field(JX, JY)),
        ([0.0, 1.0, 2.0, 1.0], [0.0, 0.0, 0.0, 1.0], [[0, 2, 1], [0, 1, 3], [1, 2, 3]], None, [1.0, 2.0, 4.0, 3.0]),
        ([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], [[0, 1, 2]], None, [1.0, 2.0, 3.0]),
        (G3X, G3Y, G3T, None, [1.0, 2.0, 3.0, 4.0, NAN, 6.0, 7.0, 8.0, 9.0]),
    ]
    for x, y, tris, mask, z in cases:
        out = probes.probe("tri.Triangulation.calculate_plane_coefficients", _plane_coefficients,
                           x=x, y=y, triangles=tris, mask=mask, z=z, tol=TOL_LINEAR)
        assert np.shape(out) == (len(tris), 3)
    lin = probes.probe("tri.Triangulation.calculate_plane_coefficients", _plane_coefficients,
                       x=G3X, y=G3Y, triangles=G3T, mask=None, z=_linear_field(G3X, G3Y), tol=TOL_LINEAR)
    assert np.allclose(lin, [[0.75, -0.5, 3.0]] * 8, rtol=0, atol=1e-14)
    err = probes.probe("tri.Triangulation.calculate_plane_coefficients", _plane_coefficients,
                       x=G3X, y=G3Y, triangles=G3T, mask=None, z=[1.0, 2.0, 3.0], tol=TOL_LINEAR)
    assert isinstance(err, ValueError)


# ---------------------------------------------------------------------------
# TriFinder
# ---------------------------------------------------------------------------


def _expected_find(x, y, triangles, mask, px, py):
    """Reference implementation of the documented TriFinder rules."""
    corrected = []
    for a, b, c in triangles:
        cross = (x[b] - x[a]) * (y[c] - y[a]) - (y[b] - y[a]) * (x[c] - x[a])
        corrected.append([a, c, b] if cross < 0 else [a, b, c])
    live = [i for i in range(len(corrected)) if mask is None or not mask[i]]
    if math.isnan(px):
        return None
    if math.isnan(py):
        used = {v for i in live for v in corrected[i]}
        xs = [x[v] for v in used]
        assert px < min(xs) or px > max(xs)
        return None
    for v in range(len(x)):
        if (x[v], y[v]) == (px, py):
            cands = [i for i in live if v in corrected[i]]
            return _canon_triple(corrected[cands[0]]) if cands else None
    best_left = best_right = inside = None
    for i in live:
        t = corrected[i]
        sides = []
        for e in range(3):
            a, b = t[e], t[(e + 1) % 3]
            sides.append((x[b] - x[a]) * (py - y[a]) - (y[b] - y[a]) * (px - x[a]))
        if all(s > 0 for s in sides):
            inside = i
        elif all(s >= 0 for s in sides) and sum(s == 0 for s in sides) == 1:
            e = sides.index(0)
            a, b = t[e], t[(e + 1) % 3]
            lo, hi = sorted([(x[a], y[a]), (x[b], y[b])])
            # Triangle is on the left of a -> b (anticlockwise); left of lo -> hi iff a -> b == lo -> hi.
            if (x[a], y[a]) == lo:
                best_left = i
            else:
                best_right = i
    if inside is not None:
        return _canon_triple(corrected[inside])
    hit = best_left if best_left is not None else best_right
    return None if hit is None else _canon_triple(corrected[hit])


def _check_find(x, y, triangles, mask, qx, qy):
    out = probes.probe("tri.TrapezoidMapTriFinder.find", _find, x=x, y=y, triangles=triangles, mask=mask,
                       qx=qx, qy=qy)
    expected = [_expected_find(x, y, triangles, mask, a, b) for a, b in zip(qx, qy)]
    assert out == expected
    return out


@pytest.mark.parametrize("tris", ["ccw", "mixed"])
@pytest.mark.parametrize("group", range(5))
def test_find_grid(tris, group):
    triangles = G3T if tris == "ccw" else G3T_MIXED
    qx, qy = _split(_grid_queries()[group])
    _check_find(G3X, G3Y, triangles, None, qx, qy)


@pytest.mark.parametrize("group", range(5))
def test_find_grid_masked(group):
    qx, qy = _split(_grid_queries()[group])
    out = _check_find(G3X, G3Y, G3T, G3MASK, qx, qy)
    if group == 2:  # vertex 6 (0, 2) only belongs to masked triangle 5
        assert out[6] is None


def test_find_hole():
    pts = [(1.5, 1.5), (1.2, 1.7), (1.9, 1.1), (1.5, 1.0), (1.0, 1.5), (2.0, 1.5), (1.5, 2.0),
           (1.0, 1.0), (2.0, 2.0), (1.0, 2.0), (2.0, 1.0), (0.5, 1.5), (2.5, 2.5), (0.2, 0.9),
           (3.0, 1.5), (1.5, 3.0), (3.5, 1.5), (NAN, 1.5), (4.0, NAN)]
    _check_find(G4X, G4Y, HOLE_T, None, *_split(pts))


def test_find_unused_points():
    x, y = [0.0, 1.0, 0.0, 5.0, 1.0], [0.0, 0.0, 1.0, 5.0, 1.0]
    tris = [[0, 1, 2], [1, 4, 2]]
    _check_find(x, y, tris, None, [5.0, 0.2, 0.8, 3.0, 0.5, 0.5], [5.0, 0.2, 0.8, 3.0, 0.5, 1.0])


@pytest.mark.parametrize("which", ["jittered_grid", "random30"])
def test_find_delaunay_general_position(which):
    x, y = DELAUNAY_SETS[which]
    lo, hi = min(x + y) - 0.5, max(x + y) + 0.5
    r = rng(100 if which == "jittered_grid" else 101)
    qx = r.uniform(40, lo, hi).tolist()
    qy = r.uniform(40, lo, hi).tolist()
    tris = mtri.Triangulation(x, y).triangles.tolist()
    assert _min_edge_distance(x, y, tris, qx, qy) > 1e-6
    out = probes.probe("tri.TrapezoidMapTriFinder.find", _find, x=x, y=y, triangles=None, mask=None, qx=qx, qy=qy)
    assert out == [_expected_find(x, y, tris, None, a, b) for a, b in zip(qx, qy)]
    assert any(v is None for v in out) and any(v is not None for v in out)


def test_find_errors():
    out = probes.probe("tri.TrapezoidMapTriFinder.find", _find, x=G3X, y=G3Y, triangles=G3T, mask=None,
                       qx=[0.5, 1.0], qy=[0.5])
    assert isinstance(out, ValueError)
    sq_x, sq_y = [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]
    for tris in ([[0, 1, 2], [0, 1, 2]], [[0, 1, 3], [0, 1, 2]]):
        out = probes.probe("tri.TrapezoidMapTriFinder.find", _find, x=sq_x, y=sq_y, triangles=tris, mask=None,
                           qx=[0.2], qy=[0.2])
        assert isinstance(out, RuntimeError)


# ---------------------------------------------------------------------------
# Interpolators
# ---------------------------------------------------------------------------


def _g4_queries(include_nan_y: bool):
    r = rng(3)
    inside = list(zip(r.uniform(16, 0.01, 2.99).tolist(), r.uniform(16, 0.01, 2.99).tolist()))
    edges = [(0.5, 0.0), (1.5, 1.0), (2.0, 2.5), (0.5, 0.5), (2.5, 0.5), (1.5, 2.5), (3.0, 1.25), (0.0, 2.75)]
    vertices = [(0.0, 0.0), (1.0, 1.0), (2.0, 1.0), (3.0, 3.0), (0.0, 3.0), (3.0, 0.0)]
    outside = [(-0.5, 1.0), (3.5, 2.0), (1.5, -0.25), (2.0, 4.0), (5.0, 5.0)]
    nonfinite = [(NAN, 1.0), (4.0, NAN), (INF, 1.0), (1.0, -INF)]
    if include_nan_y:
        nonfinite.append((1.5, NAN))
    return _split(inside + edges + vertices + outside + nonfinite)


def _general_queries(x, y, seed, n=30):
    lo_x, hi_x = min(x), max(x)
    lo_y, hi_y = min(y), max(y)
    r = rng(seed)
    qx = r.uniform(n, lo_x - 0.2 * (hi_x - lo_x), hi_x + 0.2 * (hi_x - lo_x)).tolist()
    qy = r.uniform(n, lo_y - 0.2 * (hi_y - lo_y), hi_y + 0.2 * (hi_y - lo_y)).tolist()
    return qx + [NAN], qy + [0.5 * (lo_y + hi_y)]


LINEAR_CASES = {
    "grid4": (G4X, G4Y, G4T, None, _field(G4X, G4Y)),
    "grid4_masked": (G4X, G4Y, G4T, G4MASK, _field(G4X, G4Y)),
    "grid4_linear_field": (G4X, G4Y, G4T, None, _linear_field(G4X, G4Y)),
    "grid4_nan_z": (G4X, G4Y, G4T, None, [NAN if i == 5 else v for i, v in enumerate(_field(G4X, G4Y))]),
    "hole": (G4X, G4Y, HOLE_T, None, _field(G4X, G4Y)),
}


@pytest.mark.parametrize("name", sorted(LINEAR_CASES))
def test_linear_interpolator(name):
    x, y, tris, mask, z = LINEAR_CASES[name]
    qx, qy = _g4_queries(include_nan_y=True)
    vals = probes.probe("tri.LinearTriInterpolator.__call__", _linear_call, x=x, y=y, triangles=tris, mask=mask,
                        z=z, qx=qx, qy=qy, tol=TOL_LINEAR)
    qx, qy = _g4_queries(include_nan_y=False)
    grads = probes.probe("tri.LinearTriInterpolator.gradient", _linear_gradient, x=x, y=y, triangles=tris,
                         mask=mask, z=z, qx=qx, qy=qy, tol=TOL_LINEAR)
    assert vals[-1] is None  # NaN y inside the x-range -> masked value
    if name == "grid4_linear_field":
        for a, b, v in zip(qx, qy, vals):
            if v is not None:
                assert v == pytest.approx(3.0 + 0.75 * a - 0.5 * b, rel=1e-14)
        assert all(g is None or g == pytest.approx(0.75) for g in grads["dzdx"])
    # gradient = plane coefficients of the triangle chosen by the TriFinder rules
    planes = _plane_coefficients(x, y, tris, mask, z)
    t = _triangulation(x, y, tris, mask)
    for i, (a, b) in enumerate(zip(qx, qy)):
        found = int(t.get_trifinder()(a, b))
        expected = None if found < 0 or math.isnan(planes[found][0]) else float(planes[found][0])
        assert grads["dzdx"][i] == expected


def test_linear_interpolator_delaunay():
    for x, y, seed in ((JX, JY, 40), (RX, RY, 41)):
        z = _field(x, y)
        qx, qy = _general_queries(x, y, seed)
        tris = mtri.Triangulation(x, y).triangles.tolist()
        assert _min_edge_distance(x, y, tris, qx[:-1], qy[:-1]) > 1e-6
        probes.probe("tri.LinearTriInterpolator.__call__", _linear_call, x=x, y=y, triangles=None, mask=None,
                     z=z, qx=qx, qy=qy, tol=TOL_LINEAR)
        probes.probe("tri.LinearTriInterpolator.gradient", _linear_gradient, x=x, y=y, triangles=None,
                     mask=None, z=z, qx=qx, qy=qy, tol=TOL_LINEAR)


def test_linear_interpolator_errors():
    out = probes.probe("tri.LinearTriInterpolator.__call__", _linear_call, x=G3X, y=G3Y, triangles=G3T, mask=None,
                       z=[1.0, 2.0], qx=[0.5], qy=[0.5], tol=TOL_LINEAR)
    assert isinstance(out, ValueError)
    out = probes.probe("tri.LinearTriInterpolator.__call__", _linear_call, x=G3X, y=G3Y, triangles=G3T, mask=None,
                       z=G3X, qx=[0.5, 0.2], qy=[0.5], tol=TOL_LINEAR)
    assert isinstance(out, ValueError)


CUBIC_CASES = {
    "grid4": (G4X, G4Y, G4T, None, _field(G4X, G4Y)),
    "grid4_masked": (G4X, G4Y, G4T, G4MASK, _field(G4X, G4Y)),
    "hole": (G4X, G4Y, HOLE_T, None, _field(G4X, G4Y)),
    "jittered": (JX, JY, JGT, None, _field(JX, JY)),
}


def _exact_min_e(monkeypatch, fn, **args):
    """Re-run a min_E probe with a (practically) exact PCG solve."""
    original = _triinterp._cg

    def tight(A, b, x0=None, tol=1e-10, maxiter=1000):
        return original(A, b, x0=x0, tol=1e-15, maxiter=100000)

    with monkeypatch.context() as m:
        m.setattr(_triinterp, "_cg", tight)
        return fn(**args)


def _assert_close(a, b, rel):
    if isinstance(a, dict):
        for k in a:
            _assert_close(a[k], b[k], rel)
        return
    for u, v in zip(a, b):
        assert (u is None) == (v is None)
        if u is not None:
            assert abs(u - v) <= rel * abs(v)


@pytest.mark.parametrize("kind", ["geom", "min_E", "user"])
@pytest.mark.parametrize("name", sorted(CUBIC_CASES))
def test_cubic_interpolator(name, kind, monkeypatch):
    x, y, tris, mask, z = CUBIC_CASES[name]
    dz = _field_grad(x, y) if kind == "user" else None
    if name == "jittered":
        qx, qy = _general_queries(x, y, 50)
    else:
        qx, qy = _g4_queries(include_nan_y=True)
    args = dict(x=x, y=y, triangles=tris, mask=mask, z=z, kind=kind, dz=dz, qx=qx, qy=qy)
    vals = probes.probe("tri.CubicTriInterpolator.__call__", _cubic_call, **args, tol=CUBIC_TOL[kind])
    grads = probes.probe("tri.CubicTriInterpolator.gradient", _cubic_gradient, **args, tol=CUBIC_TOL[kind])
    assert vals[-1] is None and grads["dzdx"][-1] is None
    if name != "jittered":
        # Values at vertices reproduce z (indices 24..29 of the query list).
        for i in range(24, 30):
            v = round(qy[i]) * 4 + round(qx[i])
            assert vals[i] == pytest.approx(z[v], rel=1e-12)
    if kind == "user":
        for i in range(24, 30) if name != "jittered" else []:
            v = round(qy[i]) * 4 + round(qx[i])
            assert grads["dzdx"][i] == pytest.approx(dz[0][v], rel=1e-9, abs=1e-12)
            assert grads["dzdy"][i] == pytest.approx(dz[1][v], rel=1e-9, abs=1e-12)
    if kind == "min_E":
        _assert_close(vals, _exact_min_e(monkeypatch, _cubic_call, **args), TOL_MIN_E / 10)
        _assert_close(grads, _exact_min_e(monkeypatch, _cubic_gradient, **args), TOL_MIN_E / 10)


@pytest.mark.parametrize("kind", ["geom", "min_E", "user"])
def test_cubic_interpolator_linear_patch(kind):
    """All kinds reproduce a linear field (user: with its exact gradient)."""
    z = _linear_field(JX, JY)
    dz = [[0.75] * len(JX), [-0.5] * len(JX)] if kind == "user" else None
    qx, qy = _general_queries(JX, JY, 60)
    vals = probes.probe("tri.CubicTriInterpolator.__call__", _cubic_call, x=JX, y=JY, triangles=JGT, mask=None, z=z,
                        kind=kind, dz=dz, qx=qx, qy=qy, tol=CUBIC_TOL[kind])
    grads = probes.probe("tri.CubicTriInterpolator.gradient", _cubic_gradient, x=JX, y=JY, triangles=JGT, mask=None,
                         z=z, kind=kind, dz=dz, qx=qx, qy=qy, tol=CUBIC_TOL[kind])
    for a, b, v, gx, gy in zip(qx, qy, vals, grads["dzdx"], grads["dzdy"]):
        if v is not None:
            assert v == pytest.approx(3.0 + 0.75 * a - 0.5 * b, rel=1e-9)
            assert gx == pytest.approx(0.75, rel=1e-7) and gy == pytest.approx(-0.5, rel=1e-7)


@pytest.mark.parametrize("kind", ["geom", "min_E"])
def test_cubic_interpolator_nan_z(kind):
    """NaN in z poisons the nodal derivatives of the node's neighbours (min_E: PCG
    does not iterate and returns its 'geom' starting guess)."""
    z = [NAN if i == 15 else v for i, v in enumerate(_field(G4X, G4Y))]
    qx, qy = _g4_queries(include_nan_y=True)
    vals = probes.probe("tri.CubicTriInterpolator.__call__", _cubic_call, x=G4X, y=G4Y, triangles=G4T, mask=None,
                        z=z, kind=kind, dz=None, qx=qx, qy=qy, tol=CUBIC_TOL[kind])
    assert any(v is None for v in vals[:16]) and any(v is not None for v in vals[:16])
    assert vals == _cubic_call(G4X, G4Y, G4T, None, z, "geom", None, qx, qy)


@pytest.mark.parametrize(
    "kind, dz, z",
    [
        ("cubic", None, G3X),
        ("user", None, G3X),
        ("geom", None, [1.0, 2.0]),
        ("user", [[0.0] * 3, [0.0] * 3], G3X),
    ],
)
def test_cubic_interpolator_errors(kind, dz, z):
    out = probes.probe("tri.CubicTriInterpolator.__call__", _cubic_call, x=G3X, y=G3Y, triangles=G3T, mask=None,
                       z=z, kind=kind, dz=dz, qx=[0.5], qy=[0.25], tol=TOL_CUBIC)
    assert isinstance(out, Exception)


# ---------------------------------------------------------------------------
# UniformTriRefiner
# ---------------------------------------------------------------------------

ONE_X, ONE_Y, ONE_T = [0.0, 1.0, 0.25], [0.0, 0.0, 1.0], [[0, 1, 2]]

REFINE_CASES = [
    ("single", ONE_X, ONE_Y, ONE_T, None, [-1, 0, 1, 2, 3]),
    ("single_cw", ONE_X, ONE_Y, [[0, 2, 1]], None, [1]),
    ("grid3", G3X, G3Y, G3T, None, [1, 2]),
    ("grid3_mixed", G3X, G3Y, G3T_MIXED, None, [1]),
    ("grid3_masked", G3X, G3Y, G3T, G3MASK, [0, 1, 2]),
    ("unused_points", [0.0, 1.0, 0.0, 5.0, 1.0], [0.0, 0.0, 1.0, 5.0, 1.0], [[0, 1, 2], [1, 4, 2]], None, [0, 1]),
    ("collinear", [0.0, 1.0, 2.0, 1.0], [0.0, 0.0, 0.0, 1.0], [[0, 2, 1], [0, 1, 3], [1, 2, 3]], None, [1]),
    ("jittered", JX, JY, JT, None, [1]),
]


@pytest.mark.parametrize("return_tri_index", [False, True])
@pytest.mark.parametrize("case", REFINE_CASES, ids=[c[0] for c in REFINE_CASES])
def test_refine_triangulation(case, return_tri_index):
    name, x, y, tris, mask, subdivs = case
    for subdiv in subdivs:
        out = probes.probe("tri.UniformTriRefiner.refine_triangulation", _refine_triangulation, x=x, y=y,
                           triangles=tris, mask=mask, subdiv=subdiv, return_tri_index=return_tri_index)
        ntri = len(out["triangles"]) + len(out["masked_triangles"])
        assert ntri == len(tris) * 4 ** max(subdiv, 0)
        assert out["has_mask"] == (mask is not None)
        if return_tri_index:
            assert len(out["parents"]) == out["npoints"]


def test_refine_masked_duplicates_and_parents():
    out = _refine_triangulation(G3X, G3Y, G3T, G3MASK, 1, True)
    pts = [tuple(p) for p in out["points"]]
    # midpoints of the 4 edges shared by a masked and an unmasked triangle are duplicated
    assert len(pts) - len(set(pts)) == 4
    # vertex (1, 0) belongs to triangles 0, 2, 3(masked): highest unmasked -> 2
    i = pts.index((1.0, 0.0))
    assert out["parents"][i] == _canon_triple(G3T[2])


REFINE_FIELD_CASES = {
    "grid4": (G4X, G4Y, G4T, None, _field(G4X, G4Y)),
    "grid3_masked": (G3X, G3Y, G3T, G3MASK, _field(G3X, G3Y)),
    "jittered": (JX, JY, JGT, None, _field(JX, JY)),
}
REFINE_FIELD_TOL = {None: TOL_MIN_E, "linear": TOL_LINEAR, "cubic_geom": TOL_CUBIC, "cubic_min_E": TOL_MIN_E,
                    "cubic_user": TOL_CUBIC}


@pytest.mark.parametrize("interpolator", [None, "linear", "cubic_geom", "cubic_min_E", "cubic_user"])
@pytest.mark.parametrize("name", sorted(REFINE_FIELD_CASES))
def test_refine_field(name, interpolator, monkeypatch):
    x, y, tris, mask, z = REFINE_FIELD_CASES[name]
    dz = _field_grad(x, y) if interpolator == "cubic_user" else None
    subdivs = [1] if name == "jittered" else [0, 1, 2]
    for subdiv in subdivs:
        args = dict(x=x, y=y, triangles=tris, mask=mask, z=z, interpolator=interpolator, dz=dz, subdiv=subdiv)
        rows = probes.probe("tri.UniformTriRefiner.refine_field", _refine_field, **args,
                            tol=REFINE_FIELD_TOL[interpolator])
        assert all(r[2] is not None for r in rows)
        if interpolator in (None, "cubic_min_E"):
            exact = _exact_min_e(monkeypatch, _refine_field, **args)
            _assert_close([r[2] for r in rows], [r[2] for r in exact], TOL_MIN_E / 10)
    if name == "grid3_masked" and interpolator == "linear":
        # points of masked-only children evaluate the zero plane of their masked parent
        assert sum(r[2] == 0.0 for r in rows) > 0


def test_refine_field_linear_patch():
    z = _linear_field(G4X, G4Y)
    for interpolator in ("linear", "cubic_geom"):
        rows = probes.probe("tri.UniformTriRefiner.refine_field", _refine_field, x=G4X, y=G4Y, triangles=G4T,
                            mask=None, z=z, interpolator=interpolator, dz=None, subdiv=2,
                            tol=REFINE_FIELD_TOL[interpolator])
        for a, b, v in rows:
            assert v == pytest.approx(3.0 + 0.75 * a - 0.5 * b, rel=1e-12)


# ---------------------------------------------------------------------------
# TriAnalyzer
# ---------------------------------------------------------------------------

FAX = [50.0 * v for v in FX]  # anisotropic copy of the flat-border mesh
ANALYZER_CASES = {
    "grid3": (G3X, G3Y, G3T, None),
    "grid3_masked": (G3X, G3Y, G3T, G3MASK),
    "flat_border": (FX, FY, FT, None),
    "flat_border_anisotropic": (FAX, FY, FT, None),
    "flat_border_masked": (FX, FY, FT, [i % 7 == 3 for i in range(len(FT))]),
    "collinear": ([0.0, 1.0, 2.0, 1.0], [0.0, 0.0, 0.0, 1.0], [[0, 2, 1], [0, 1, 3], [1, 2, 3]], None),
    "interior_flat": ([0.0, 2.0, 1.0, 1.0, 1.0], [0.0, 0.0, 2.0, 0.6, 0.61], [[0, 1, 3], [1, 4, 3], [1, 2, 4],
                                                                         [2, 0, 4], [0, 3, 4]], None),
    "unused_points": ([0.0, 1.0, 0.0, 5.0, 1.0], [0.0, 0.0, 1.0, 9.0, 1.0], [[0, 1, 2], [1, 4, 2]], None),
}
MIN_RATIOS = [0.01, 0.05, 0.1, 0.2, 0.45]


@pytest.mark.parametrize("name", sorted(ANALYZER_CASES))
def test_analyzer(name):
    x, y, tris, mask = ANALYZER_CASES[name]
    probes.probe("tri.TriAnalyzer.scale_factors", _scale_factors, x=x, y=y, triangles=tris, mask=mask)
    for rescale in (True, False):
        ratios = probes.probe("tri.TriAnalyzer.circle_ratios", _circle_ratios, x=x, y=y, triangles=tris, mask=mask,
                              rescale=rescale, tol=TOL_LINEAR)
        assert all(r is None or 0.0 <= r <= 0.5 + 1e-15 for r in ratios)
        for m in MIN_RATIOS:
            # thresholds are not within rounding distance of any ratio
            assert all(r is None or abs(r - m) > 1e-9 for r in ratios)
            new_mask = probes.probe("tri.TriAnalyzer.get_flat_tri_mask", _flat_tri_mask, x=x, y=y, triangles=tris,
                                    mask=mask, min_circle_ratio=m, rescale=rescale)
            if mask is not None:
                assert all(n for n, o in zip(new_mask, mask) if o)


def test_analyzer_known_values():
    sf = _scale_factors(G3X, G3Y, G3T, None)
    assert sf == [0.5, 0.5]
    assert _scale_factors([0.0, 1.0, 0.0, 5.0, 1.0], [0.0, 0.0, 1.0, 9.0, 1.0], [[0, 1, 2], [1, 4, 2]], None) == [1, 1]
    ratios = _circle_ratios(G3X, G3Y, G3T, None, True)
    assert ratios == pytest.approx([math.sqrt(2) - 1] * 8, rel=1e-12)
    collinear = _circle_ratios([0.0, 1.0, 2.0, 1.0], [0.0, 0.0, 0.0, 1.0], [[0, 2, 1], [0, 1, 3], [1, 2, 3]], None,
                               True)
    assert collinear[0] == 0.0
    flat = _flat_tri_mask(FX, FY, FT, None, 0.01, True)
    assert sum(flat) == 9
    # interior slivers are never removed while they do not touch the border
    interior = ANALYZER_CASES["interior_flat"][:3]
    assert min(_circle_ratios(*interior, None, True)) < 0.01
    assert _flat_tri_mask(*interior, None, 0.01, True) == [False] * 5
