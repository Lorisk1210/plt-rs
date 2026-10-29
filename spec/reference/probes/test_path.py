"""Path geometry: matplotlib.path, matplotlib.bezier, matplotlib.hatch (area "path").

Most results come from matplotlib's C++ ``_path`` module (point-in-path,
intersections, clipping, ``cleanup_path`` = transform/NaN-removal/clip/snap/
simplify/curve-flattening pipeline, polygon conversion); pltrs must
reproduce those algorithms exactly, including Agg's ``curve_div`` Bezier
flattening (approximation_scale 1, angle tolerance 0), Agg ``conv_contour``
for non-zero ``radius`` and the crossing-number (even-odd) inside test.

Shared conventions
==================
* **Path dict** (input and output): ``{"vertices": [[x, y], ...], "codes":
  [int, ...] | null}`` with STOP=0, MOVETO=1, LINETO=2, CURVE3=3, CURVE4=4,
  CLOSEPOLY=79.  As an *input* to any op other than ``path.Path`` it is
  decoded as ``Path(np.asarray(vertices, float).reshape(-1, 2), codes)``
  (so ``[]`` is the empty path), constructed under the reference rcParams
  (``path.simplify=True``, ``path.simplify_threshold=0.111111111111``)
  unless the op has an explicit ``rc`` arg.  As an *output* it is
  ``Path.vertices`` / ``Path.codes`` verbatim (``codes`` null iff None).
* **Transform** args: ``null`` (no transform) or a list (``[]`` = identity
  ``Affine2D()``) of operations applied
  in order to a fresh ``Affine2D()``: ``["rotate_deg", deg]``,
  ``["scale", sx, sy]``, ``["translate", tx, ty]``, ``["skew_deg", xdeg,
  ydeg]`` (e.g. ``[["scale", 2, 3], ["translate", 1, 0]]`` = scale first,
  then translate).
* **Bbox**: ``[[x0, y0], [x1, y1]]``; points: ``[[x, y], ...]``.
* **rc** args: dict of rcParams applied (``matplotlib.rc_context``) while the
  input Path is constructed *and* while the method runs.  Only
  ``path.simplify`` / ``path.simplify_threshold`` are used.  They set
  ``Path.simplify_threshold`` and ``Path.should_simplify`` (= threshold > 0
  and rc path.simplify and len(vertices) >= 128 and (codes is None or all
  codes <= LINETO)).
* **circle** args (bezier predicates): ``[cx, cy, r]`` meaning the predicate
  ``bezier.inside_circle(cx, cy, r)``: ``(x-cx)**2 + (y-cy)**2 < r**2``.
* ``tol`` on a probe: relative tolerance.  It is used for (a) results that go
  through numpy ``matmul`` (OpenBLAS dgemm/dgemv; summation order and FMA
  usage are BLAS-kernel details): BezierSegment evaluation, polynomial
  coefficients, axis_aligned_extrema, ``get_extents`` of curved paths; (b)
  bisection results (``find_bezier_t...``, ``split_bezier...``,
  ``split_path_inout``); (c) ``arc``/``wedge`` probes where numpy's SIMD
  ``tan`` differs from libm ``tan`` for that input (detected per probe).

Constructors (all return a path dict)
=====================================
* ``path.Path`` {vertices, codes, closed}: ``Path(vertices, codes,
  closed=closed)`` with the raw JSON lists (so ``vertices=[]`` has shape
  (0,) and raises ValueError).  Errors: ValueError for bad shapes, codes
  length mismatch, first code != MOVETO.
* ``path.Path.unit_rectangle`` {}, ``path.Path.unit_circle`` {},
  ``path.Path.unit_circle_righthalf`` {}.
* ``path.Path.circle`` {center, radius, readonly}: ``Path.circle``.
* ``path.Path.readonly`` {constructor}: bool ``.readonly`` of the path built
  by the named zero-arg constructor (``"unit_rectangle"``, ``"unit_circle"``,
  ``"unit_circle_righthalf"``) or ``"circle"`` (= ``Path.circle()``),
  ``"arc"`` (= ``Path.arc(0, 90)``), ``"unit_regular_polygon"`` (n=5).
* ``path.Path.arc`` {theta1, theta2, n, is_wedge} (degrees; ``n`` null =
  automatic ``int(2 ** ceil(span / (pi/2)))``); ``path.Path.wedge``
  {theta1, theta2, n}.
* ``path.Path.unit_regular_polygon`` {numVertices};
  ``path.Path.unit_regular_star`` {numVertices, innerCircle};
  ``path.Path.unit_regular_asterisk`` {numVertices}.
* ``path.Path.make_compound_path`` {paths: [path dict, ...]}.
* ``path.Path.make_compound_path_from_polys`` {XY}: ``XY`` decoded with
  ``np.asarray(XY, float)``; an empty list is passed as
  ``np.empty((0, nsides, 2))`` when the arg ``nsides`` is given.
* ``path.Path.hatch`` {hatchpattern, density}: ``Path.hatch`` (=
  ``matplotlib.hatch.get_path``); value is a path dict, or null for a null
  pattern.  Note: shape patterns (o O . *) raise ValueError in matplotlib
  when their row count (char count * int(density)) is odd.
* ``path.Path.hatch.get_extents`` {hatchpattern, density}: bbox of
  ``Path.hatch(hatchpattern, density).get_extents()``.

Queries
=======
* ``path.Path.contains_point`` {path, point, transform, radius} -> bool.
  matplotlib semantics: the transformed path is NaN-cleaned and flattened
  (Agg curve_div), offset with Agg ``conv_contour(width=radius)`` when
  radius != 0, every subpath is implicitly closed and tested on its own
  with the crossing-number rule, and the per-subpath results are OR-ed (a
  hole subpath does NOT exclude points).  Paths with < 3 vertices -> False.
  The contour offset is not orientation-normalised: radius > 0 grows a CCW
  path but shrinks a CW one (and vice versa).  Points with a non-finite
  coordinate -> False.
* ``path.Path.contains_points`` {path, points, transform, radius} -> [bool]
  (points decoded as ``np.asarray(points, float).reshape(-1, 2)``).
* ``path.Path.contains_path`` {path, other, transform} -> bool
  (``path.contains_path(other, transform)``; transform applies to *other*).
  Every vertex emitted by *other*'s flattened, NaN-cleaned iteration --
  including the stored coordinates of CLOSEPOLY vertices -- must be inside
  *path* (contains_point semantics, radius 0); empty *other* -> True.
* ``path.Path.get_extents`` {path, transform} -> bbox (empty -> null bbox
  ``[[inf, inf], [-inf, -inf]]``).
* ``path.Path.intersects_path`` {path, other, filled} -> bool.  Both paths
  are NaN-cleaned and flattened and walked as one polyline each: MOVETO
  jumps and CLOSEPOLY stored vertices act as ordinary segment endpoints;
  ~zero-length segments are skipped; with ``filled`` a full containment
  (contains_path either way) also counts.
* ``path.Path.intersects_bbox`` {path, bbox, filled} -> bool.
* ``path.Path.should_simplify`` {path, rc} -> bool; also
  ``path.Path.simplify_threshold`` {path, rc} -> float.

Conversions
===========
* ``path.Path.clip_to_bbox`` {path, bbox, inside} -> path dict, exact output
  of ``_path.clip_path_to_rect`` (Sutherland-Hodgman per subpath, in the
  order x<=xmax, x>=xmin, y<=ymax, y>=ymin; xmin/xmax swapped when
  ``inside`` is False) then ``make_compound_path`` of ``Path(poly)`` per
  polygon (so codes MOVETO/LINETO only).  No canonicalisation.
* ``path.Path.interpolated`` {path, steps} -> path dict.
* ``path.Path.to_polygons`` {path, transform, width, height, closed_only, rc}
  -> list of point lists, verbatim order.
* ``path.Path.cleaned`` {path, transform, remove_nans, clip, simplify, curves,
  stroke_width, snap, rc} -> path dict (includes the trailing STOP vertex
  that ``cleanup_path`` emits).  ``clip`` is null or ``[x1, y1, x2, y2]``
  (clipping is skipped unless x1 < x2 and y1 < y2); ``snap`` is
  true/false/null (null = auto).
* ``path.Path.iter_segments`` {path, transform, remove_nans, clip, snap,
  stroke_width, simplify, curves, rc} -> list of ``[flat_vertices, code]``
  where ``flat_vertices`` is ``[x0, y0, (x1, y1, (x2, y2))]``.
* ``path.Path.iter_bezier`` {path} -> list of ``[control_points, code]``
  (default iter_segments kwargs).
* ``path.Path.transformed`` {path, transform} -> path dict.
* ``path.get_path_collection_extents`` {master_transform, paths, transforms,
  offsets, offset_transform} -> ``{"bbox": bbox, "minpos": [x, y]}``;
  ``transforms`` is a list of 3x3 matrices decoded as
  ``np.asarray(transforms, float).reshape(-1, 3, 3)``.

matplotlib.bezier
=================
* ``bezier.BezierSegment.__call__`` {control_points, t} -> point (scalar t)
  or list of points (list t).
* ``bezier.BezierSegment.point_at_t`` {control_points, t} -> tuple
  (deprecated in 3.11 but still present).
* ``bezier.BezierSegment.properties`` {control_points} -> {"degree",
  "dimension", "control_points", "polynomial_coefficients"}.
* ``bezier.BezierSegment.axis_aligned_extrema`` {control_points} ->
  {"dims": [int], "dzeros": [float]} (verbatim order: grouped by dimension,
  roots ascending within a dimension).
* ``bezier.split_de_casteljau`` {beta, t} -> [left, right] control points.
* ``bezier.find_bezier_t_intersecting_with_closedpath`` {control_points,
  circle, t0, t1, tolerance} -> [t0, t1]; point function is
  ``tuple(BezierSegment(control_points)(t))``.  Error
  NonIntersectingPathException when both ends are on the same side.
* ``bezier.split_bezier_intersecting_with_closedpath`` {bezier, circle,
  tolerance} -> [left, right].
* ``bezier.split_path_inout`` {path, circle, tolerance, reorder_inout} ->
  [path_in dict, path_out dict]; inside predicate gets ``ctl_points[-2:]``.
* ``bezier.get_parallels`` {bezier2, width} -> [left, right] (3 points each).
* ``bezier.make_wedged_bezier2`` {bezier2, width, w1, wm, w2} -> [left,
  right].
* ``bezier.get_cos_sin`` {x0, y0, x1, y1} -> [cos, sin].
* ``bezier.check_if_parallel`` {dx1, dy1, dx2, dy2, tolerance} -> 1, -1 or
  false.
* ``bezier.inside_circle`` {cx, cy, r, xy} -> bool.
* ``bezier.get_intersection`` {cx1, cy1, cos_t1, sin_t1, cx2, cy2, cos_t2,
  sin_t2} -> [x, y]; ValueError when |det| < 1e-12.
* ``bezier.get_normal_points`` {cx, cy, cos_t, sin_t, length} ->
  [x1, y1, x2, y2].
* ``bezier.find_control_points`` {c1x, c1y, mmx, mmy, c2x, c2y} -> 3 points.
"""

from __future__ import annotations

import math

import matplotlib
import numpy as np
import pytest
from matplotlib import bezier as mbezier
from matplotlib.path import Path, get_path_collection_extents
from matplotlib.transforms import Affine2D, Bbox

from spec_support import ProbeSet, rng

probes = ProbeSet("path")

# get_parallels warns (and falls back to a straight line) for collinear control
# points; the fallback geometry is what the probes record.
pytestmark = pytest.mark.filterwarnings("ignore:Lines do not intersect:UserWarning")

REF_RC = {"path.simplify": True, "path.simplify_threshold": 0.111111111111}
MATMUL_TOL = 1e-12
BISECT_TOL = 1e-9

# --------------------------------------------------------------------------
# decoding / encoding helpers
# --------------------------------------------------------------------------


def _path(d):
    return Path(np.asarray(d["vertices"], dtype=float).reshape(-1, 2), d["codes"])


def _pd(p):
    return {"vertices": np.asarray(p.vertices).reshape(-1, 2),
            "codes": None if p.codes is None else np.asarray(p.codes)}


def _tr(ops):
    if ops is None:
        return None
    t = Affine2D()
    for name, *a in ops:
        if name == "rotate_deg":
            t.rotate_deg(*a)
        elif name == "scale":
            t.scale(*a)
        elif name == "translate":
            t.translate(*a)
        elif name == "skew_deg":
            t.skew_deg(*a)
        else:
            raise KeyError(name)
    return t


def _bbox(b):
    return Bbox(np.asarray(b, dtype=float))


def _pts(points):
    return np.asarray(points, dtype=float).reshape(-1, 2)


def _const(p):
    """Plain-JSON path dict of a matplotlib-built path (for use as an input)."""
    return {"vertices": np.asarray(p.vertices, dtype=float).tolist(),
            "codes": None if p.codes is None else [int(c) for c in p.codes]}


def pth(vertices, codes=None):
    return {"vertices": [list(map(float, v)) for v in vertices],
            "codes": None if codes is None else list(codes)}


M, L, C3, C4, CL, ST = 1, 2, 3, 4, 79, 0

# --------------------------------------------------------------------------
# shared test geometry (all plain JSON)
# --------------------------------------------------------------------------

SQ = pth([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]], [M, L, L, L, CL])
SQ_CW = pth([[0, 0], [0, 1], [1, 1], [1, 0], [0, 0]], [M, L, L, L, CL])
SQ_OPEN = pth([[0, 0], [1, 0], [1, 1], [0, 1]])
SQ_OPEN_CODES = pth([[0, 0], [1, 0], [1, 1], [0, 1]], [M, L, L, L])
SQ_GARBAGE_CLOSE = pth([[0, 0], [1, 0], [1, 1], [0, 1], [9, 9]], [M, L, L, L, CL])
TRI = pth([[0, 0], [4, 0], [0, 3]])
BOWTIE = pth([[0, 0], [2, 2], [2, 0], [0, 2], [0, 0]], [M, L, L, L, CL])
L_SHAPE = pth([[0, 0], [3, 0], [3, 1], [1, 1], [1, 3], [0, 3], [0, 0]],
              [M, L, L, L, L, L, CL])
DONUT = pth([[0, 0], [4, 0], [4, 4], [0, 4], [0, 0],
             [1, 1], [1, 3], [3, 3], [3, 1], [1, 1]],
            [M, L, L, L, CL, M, L, L, L, CL])
DONUT_SAME = pth([[0, 0], [4, 0], [4, 4], [0, 4], [0, 0],
                  [1, 1], [3, 1], [3, 3], [1, 3], [1, 1]],
                 [M, L, L, L, CL, M, L, L, L, CL])
TWO_SQUARES = pth([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0],
                   [3, 0], [4, 0], [4, 1], [3, 1], [3, 0]],
                  [M, L, L, L, CL, M, L, L, L, CL])
LINE2 = pth([[0, 0], [1, 1]])
QUAD_OPEN = pth([[0, 0], [1, 2], [2, 0]], [M, C3, C3])
QUAD_CLOSED = pth([[0, 0], [1, 2], [2, 0], [0, 0]], [M, C3, C3, CL])
CUBIC_OPEN = pth([[0, 0], [0, 1], [1, 1], [1, 0]], [M, C4, C4, C4])
CUBIC_S = pth([[0, 0], [3, 2], [-1, 2], [2, 0]], [M, C4, C4, C4])
CUBIC_LOOP_CLOSED = pth([[0, 0], [4, 4], [-2, 4], [2, 0], [0, 0]], [M, C4, C4, C4, CL])
CIRCLE = _const(Path.unit_circle())
CIRCLE_BIG = _const(Path.circle((2.0, 1.0), 3.0))
CIRCLE_HALF = _const(Path.unit_circle_righthalf())
ARC90 = _const(Path.arc(0, 90))
WEDGE = _const(Path.wedge(30, 120))
STAR5 = _const(Path.unit_regular_star(5))
MALFORMED = pth([[0, 0], [2, 0], [3, 1], [2, 2], [1, 3], [0, 2], [0, 0]],
                [M, L, C3, C3, C4, C4, CL])  # CURVE4 with only 2 vertices before CLOSEPOLY
MIXED_OK = pth([[0, 0], [2, 0], [3, 1], [2, 2], [1, 3], [0, 3], [0, 2], [0, 0]],
               [M, L, C3, C3, C4, C4, C4, CL])
EMPTY = {"vertices": [], "codes": None}

NAN = float("nan")
POLY_NAN = pth([[0, 0], [1, 1], [NAN, 2], [3, 3], [4, 4], [5, NAN], [6, 6]])
POLY_NAN_CODES = pth([[0, 0], [1, 1], [NAN, 2], [3, 3], [4, 4], [5, 5]],
                     [M, L, L, L, L, L])
POLY_NAN_FIRST = pth([[NAN, NAN], [1, 1], [2, 0], [3, 1]])
CURVE_NAN = pth([[0, 0], [1, 1], [1, NAN], [2, 0], [3, 3], [4, 0], [5, 1]],
                [M, L, C4, C4, C4, L, L])
QUAD_NAN = pth([[0, 0], [1, 2], [2, 0], [NAN, 1], [4, 0], [5, 1]],
               [M, C3, C3, C3, C3, L])
SQ_NAN_CLOSED = pth([[0, 0], [1, 0], [NAN, 1], [0, 1], [0, 0]], [M, L, L, L, CL])


def _long_polyline(n, seed, noise):
    # inputs are rounded to 3 decimals only to keep the golden small (they are explicit args)
    r = rng(seed)
    u = r.uniform(n, -noise, noise)
    return pth([[float(i), round(float(10.0 * math.sin(i / 10.0) + u[i]), 3)] for i in range(n)])


LONG_SMOOTH = _long_polyline(132, 7, 0.02)    # >= 128 vertices: should_simplify is True
LONG_NOISY = _long_polyline(130, 8, 2.0)      # noise above the threshold


def _cheap_long(n, codes=None):
    return pth([[float(i), float(i % 3)] for i in range(n)], codes)


LONG_127 = _cheap_long(127)                   # just below the 128-vertex rule
LONG_128 = _cheap_long(128)


def _grid(lo, hi, n):
    xs = np.linspace(lo, hi, n)
    return [[float(x), float(y)] for y in xs for x in xs]


GRID_SQ = _grid(-0.25, 1.25, 7)       # dyadic: includes edges and vertices
GRID_DONUT = _grid(0.0, 4.0, 9)       # step 0.5
GRID_TRI = _grid(-1.0, 4.0, 11)       # step 0.5
GRID_BOWTIE = _grid(-0.5, 2.5, 7)     # step 0.5


def _rays(radii):
    """Points on rays at 0, 11.25, ..., 90 degrees (covers Bezier joints and mid-segments)."""
    out = []
    for k in range(9):
        a = math.pi / 2 * k / 8
        for r in radii:
            out.append([r * math.cos(a), r * math.sin(a)])
    return out


CIRCLE_PTS = _rays([0.5, 0.95, 0.99, 1.0, 1.01])

# ==========================================================================
# Constructors
# ==========================================================================


def _new_path(vertices, codes, closed):
    return _pd(Path(vertices, codes, closed=closed))


def test_path_constructor():
    op = "path.Path"
    r = probes.probe(op, _new_path, vertices=[[0, 0], [1, 0], [1, 1]], codes=None, closed=False)
    assert r["codes"] is None
    r = probes.probe(op, _new_path, vertices=[[0, 0], [1, 0], [1, 1]], codes=None, closed=True)
    assert list(r["codes"]) == [M, L, CL]
    probes.probe(op, _new_path, vertices=[[0, 0], [1, 0], [1, 1], [0, 0]], codes=None, closed=True)
    probes.probe(op, _new_path, vertices=[[0, 0], [1, 0]], codes=None, closed=True)
    r = probes.probe(op, _new_path, vertices=[[0, 0]], codes=None, closed=True)
    assert list(r["codes"]) == [CL]  # single vertex: CLOSEPOLY overrides MOVETO
    probes.probe(op, _new_path, vertices=[[0, 0], [1, 0], [1, 1]], codes=[M, L, L], closed=True)
    probes.probe(op, _new_path, vertices=[[0, 0], [1, 0], [1, 1], [0, 0]], codes=[M, L, L, CL], closed=False)
    probes.probe(op, _new_path, vertices=[[0, 0], [1, 2], [2, 0]], codes=[M, C3, C3], closed=False)
    probes.probe(op, _new_path, vertices=[[0, 0], [1, 2], [2, 0]], codes=[M, 5, 7], closed=False)
    probes.probe(op, _new_path, vertices=[[0, NAN], [1, 2], [float("inf"), 0]], codes=None, closed=False)
    probes.probe(op, _new_path, vertices=[[1, 2]], codes=[M], closed=False)
    probes.probe(op, _new_path, vertices=[[1, 2]], codes=None, closed=False)
    # empty: a flat [] has shape (0,) and is rejected; [[]] has shape (1, 0)
    assert isinstance(probes.probe(op, _new_path, vertices=[], codes=None, closed=False), ValueError)
    assert isinstance(probes.probe(op, _new_path, vertices=[[]], codes=None, closed=False), ValueError)
    # errors
    assert isinstance(probes.probe(op, _new_path, vertices=[[0, 0], [1, 0], [1, 1]], codes=[M, L], closed=False), ValueError)
    assert isinstance(probes.probe(op, _new_path, vertices=[[0, 0], [1, 0]], codes=[L, L], closed=False), ValueError)
    assert isinstance(probes.probe(op, _new_path, vertices=[[0, 0], [1, 0]], codes=[[M, L]], closed=False), ValueError)
    assert isinstance(probes.probe(op, _new_path, vertices=[[0, 0, 0], [1, 0, 0]], codes=None, closed=False), ValueError)
    assert isinstance(probes.probe(op, _new_path, vertices=[0, 1, 2, 3], codes=None, closed=False), ValueError)


def test_fixed_constructors():
    r = probes.probe("path.Path.unit_rectangle", lambda: _pd(Path.unit_rectangle()))
    assert np.array_equal(r["vertices"], [[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]])
    assert list(r["codes"]) == [M, L, L, L, CL]
    r = probes.probe("path.Path.unit_circle", lambda: _pd(Path.unit_circle()))
    assert len(r["vertices"]) == 26 and r["codes"][-1] == CL
    probes.probe("path.Path.unit_circle_righthalf", lambda: _pd(Path.unit_circle_righthalf()))


def _circle(center, radius, readonly):
    return _pd(Path.circle(center=center, radius=radius, readonly=readonly))


def test_circle():
    for center, radius in [([0.0, 0.0], 1.0), ([2.0, 1.0], 3.0), ([-1.5, 0.25], 0.5),
                           ([0.0, 0.0], 0.0), ([0.0, 0.0], -2.0), ([1e6, -1e6], 1e-3)]:
        probes.probe("path.Path.circle", _circle, center=center, radius=radius, readonly=False)
    probes.probe("path.Path.circle", _circle, center=[0.0, 0.0], radius=1.0, readonly=True)


def _readonly(constructor):
    build = {
        "unit_rectangle": Path.unit_rectangle,
        "unit_circle": Path.unit_circle,
        "unit_circle_righthalf": Path.unit_circle_righthalf,
        "circle": Path.circle,
        "arc": lambda: Path.arc(0, 90),
        "unit_regular_polygon": lambda: Path.unit_regular_polygon(5),
    }[constructor]
    return build().readonly


def test_readonly():
    for name in ["unit_rectangle", "unit_circle", "unit_circle_righthalf", "circle", "arc",
                 "unit_regular_polygon"]:
        probes.probe("path.Path.readonly", _readonly, constructor=name)
    assert _readonly("circle") is False and _readonly("unit_circle") is True


def _tan_differs(theta1, theta2, n):
    """True if numpy's tan and libm's tan disagree on this arc's half-step."""
    try:
        n_turns = (theta2 - theta1) / 360
        nearest = np.rint(n_turns)
        full = nearest != 0 and abs(n_turns - nearest) <= 1e-12
        eta2 = theta1 + 360 if full else theta2 - 360 * np.floor(n_turns)
        e1, e2 = np.deg2rad([theta1, eta2])
        if n is None:
            n = int(2 ** np.ceil((e2 - e1) / (np.pi * 0.5)))
        x = 0.5 * ((e2 - e1) / n)
        return float(np.tan(x)) != math.tan(float(x))
    except Exception:  # noqa: BLE001
        return False


def _arc(theta1, theta2, n, is_wedge):
    return _pd(Path.arc(theta1, theta2, n, is_wedge))


def _wedge(theta1, theta2, n):
    return _pd(Path.wedge(theta1, theta2, n))


ARC_CASES = [
    (0, 90, None), (0, 45, None), (0, 10, None), (0, 180, None), (0, 270, None),
    (0, 360, None), (0, 720, None), (45, 405, None), (0, 360 + 1e-13, None),
    (0, 360 - 1e-13, None), (0, 360 + 1e-9, None), (0, -360, None), (90, 0, None),
    (350, 10, None), (30, 400, None), (-90, 90, None), (720, 810, None),
    (12.5, 77.25, None), (0, 0, None), (10, 10, None), (0, 1e-9, None), (-30, -60, None),
    (0, 90, 1), (0, 90, 3), (0, 360, 8), (0, 360, 4), (0, 360, 1), (0, 359, None),
    (0, 361, None),
]


def test_arc_and_wedge():
    for t1, t2, n in ARC_CASES:
        tol = 1e-14 if _tan_differs(t1, t2, n) else None
        for wedge in (False, True):
            probes.probe("path.Path.arc", _arc, theta1=t1, theta2=t2, n=n, is_wedge=wedge, tol=tol)
    for t1, t2, n in ARC_CASES[:4] + [(90, 0, None), (0, 360, None), (0, 90, 3)]:
        # Path.wedge(t1, t2, n) == Path.arc(t1, t2, n, is_wedge=True)
        probes.probe("path.Path.wedge", _wedge, theta1=t1, theta2=t2, n=n,
                     tol=1e-14 if _tan_differs(t1, t2, n) else None)
    r = probes.probe("path.Path.arc", _arc, theta1=0, theta2=90, n=None, is_wedge=False)
    assert list(r["codes"]) == [M, C4, C4, C4, C4, C4, C4]
    r = probes.probe("path.Path.arc", _arc, theta1=0, theta2=360, n=None, is_wedge=False)
    assert len(r["codes"]) == 16 * 3 + 1
    r = probes.probe("path.Path.wedge", _wedge, theta1=0, theta2=90, n=None)
    assert list(r["codes"][:2]) == [M, L] and list(r["codes"][-2:]) == [L, CL]
    for bad_n in (0, -1):
        assert isinstance(probes.probe("path.Path.arc", _arc, theta1=0, theta2=90, n=bad_n, is_wedge=False),
                          ValueError)
    probes.probe("path.Path.arc", _arc, theta1=0, theta2=NAN, n=None, is_wedge=False)
    probes.probe("path.Path.arc", _arc, theta1=0, theta2=NAN, n=4, is_wedge=False)


def _polygon(numVertices):
    return _pd(Path.unit_regular_polygon(numVertices))


def _star(numVertices, innerCircle):
    return _pd(Path.unit_regular_star(numVertices, innerCircle))


def _asterisk(numVertices):
    return _pd(Path.unit_regular_asterisk(numVertices))


def test_regular_shapes():
    for n in [1, 2, 3, 4, 5, 6, 8, 16, 17, 24, -3]:
        probes.probe("path.Path.unit_regular_polygon", _polygon, numVertices=n)
    assert isinstance(probes.probe("path.Path.unit_regular_polygon", _polygon, numVertices=0),
                      ZeroDivisionError)
    r = probes.probe("path.Path.unit_regular_polygon", _polygon, numVertices=4)
    assert r["vertices"][0][0] == pytest.approx(0) and r["vertices"][0][1] == 1.0
    for n, inner in [(5, 0.5), (5, 0.3), (3, 0.5), (8, 0.7), (6, 1.0), (4, 1.5), (20, 0.5), (2, 0.25)]:
        probes.probe("path.Path.unit_regular_star", _star, numVertices=n, innerCircle=inner)
    for n in [2, 3, 4, 6, 8]:
        probes.probe("path.Path.unit_regular_asterisk", _asterisk, numVertices=n)


def _compound(paths):
    return _pd(Path.make_compound_path(*[_path(p) for p in paths]))


def _compound_from_polys(XY, nsides=None):
    arr = np.asarray(XY, dtype=float)
    if arr.size == 0 and nsides is not None:
        arr = np.empty((0, nsides, 2))
    return _pd(Path.make_compound_path_from_polys(arr))


def test_compound_paths():
    op = "path.Path.make_compound_path"
    r = probes.probe(op, _compound, paths=[])
    assert len(r["vertices"]) == 0 and r["codes"] is None
    probes.probe(op, _compound, paths=[SQ])
    probes.probe(op, _compound, paths=[SQ, TRI])
    probes.probe(op, _compound, paths=[TRI, LINE2, SQ_OPEN])
    probes.probe(op, _compound, paths=[QUAD_CLOSED, EMPTY, CUBIC_OPEN])
    r = probes.probe(op, _compound, paths=[pth([[0, 0], [1, 1], [2, 2]], [M, L, ST]), pth([[5, 5], [6, 6]], [M, L])])
    assert list(r["codes"]) == [M, L, M, L]
    probes.probe(op, _compound, paths=[EMPTY])
    probes.probe(op, _compound, paths=[pth([[0, 0]], [M]), pth([[1, 1]])])

    op = "path.Path.make_compound_path_from_polys"
    probes.probe(op, _compound_from_polys, XY=[[[0, 0], [1, 0], [1, 1]], [[2, 2], [3, 2], [3, 3]]])
    probes.probe(op, _compound_from_polys, XY=[[[0, 0], [1, 0], [1, 1], [0, 1]]])
    probes.probe(op, _compound_from_polys, XY=[[[0, 0]], [[1, 1]]])
    probes.probe(op, _compound_from_polys, XY=[], nsides=3)
    assert isinstance(probes.probe(op, _compound_from_polys, XY=[[[0, 0, 0], [1, 0, 0]]]), ValueError)
    assert isinstance(probes.probe(op, _compound_from_polys, XY=[[0, 0], [1, 1]]), ValueError)


HATCH_PATTERNS = ["/", "\\", "|", "-", "+", "x", "X", "o", "O", ".", "*"]
HATCH_COMBOS = ["//", "\\\\\\", "/\\", "+x", "|-", "-|/\\", "xxx", "+.*", "a/", ""]


def _hatch(hatchpattern, density):
    p = Path.hatch(hatchpattern, density)
    return None if p is None else _pd(p)


def test_hatch():
    op = "path.Path.hatch"
    for pat in HATCH_PATTERNS:
        for d in (1, 2, 3):
            probes.probe(op, _hatch, hatchpattern=pat, density=d)
        if pat in "/\\|-+xX":
            probes.probe(op, _hatch, hatchpattern=pat, density=6)
    probes.probe(op, _hatch, hatchpattern="*", density=4)
    for pat in HATCH_COMBOS:
        probes.probe(op, _hatch, hatchpattern=pat, density=2)
    for pat in ["/", "+", "o", "*"]:
        probes.probe(op, _hatch, hatchpattern=pat, density=0)
    # shape hatches with an odd number of rows (count * density) fail inside matplotlib
    assert isinstance(probes.probe(op, _hatch, hatchpattern="o", density=1), ValueError)
    assert isinstance(probes.probe(op, _hatch, hatchpattern="oo", density=1), dict)
    probes.probe(op, _hatch, hatchpattern="/", density=2.7)
    probes.probe(op, _hatch, hatchpattern="-", density=-1)
    assert probes.probe(op, _hatch, hatchpattern=None, density=6) is None
    r = probes.probe(op, _hatch, hatchpattern="-", density=2)
    assert np.allclose(r["vertices"], [[0, 0.25], [1, 0.25], [0, 0.75], [1, 0.75]])
    assert list(r["codes"]) == [M, L, M, L]
    for pat, d in [("/", 2), ("x", 3), ("-", 1), ("o", 2), ("O", 2), (".", 2), ("O", 1), ("*", 2), ("+", 3),
                   ("Oo.*", 2), ("", 2)]:
        probes.probe("path.Path.hatch.get_extents", _hatch_extents, hatchpattern=pat, density=d,
                     tol=MATMUL_TOL if any(c in pat for c in "oO.") else None)


def _hatch_extents(hatchpattern, density):
    return Path.hatch(hatchpattern, density).get_extents().get_points()


# ==========================================================================
# Containment
# ==========================================================================


def _contains_point(path, point, transform, radius):
    return bool(_path(path).contains_point(point, _tr(transform), radius))


def _contains_points(path, points, transform, radius):
    return _path(path).contains_points(_pts(points), _tr(transform), radius)


def test_contains_points_square():
    op = "path.Path.contains_points"
    for path in (SQ, SQ_CW, SQ_OPEN, SQ_OPEN_CODES, SQ_GARBAGE_CLOSE):
        for radius in (0.0, 0.1, -0.1, 0.5, -0.5):
            probes.probe(op, _contains_points, path=path, points=GRID_SQ, transform=None, radius=radius)
    for radius in (0.25, -0.25, 1e-9, -1e-9):
        probes.probe(op, _contains_points, path=SQ, points=GRID_SQ, transform=None, radius=radius)
        probes.probe(op, _contains_points, path=SQ_CW, points=GRID_SQ, transform=None, radius=radius)
    r = probes.probe(op, _contains_points, path=SQ, points=[[0.5, 0.5], [1.5, 0.5], [-0.5, 0.5]],
                     transform=None, radius=0.0)
    assert list(r) == [True, False, False]
    # transformed squares (points are in the transformed space)
    grid_big = _grid(-2.0, 4.0, 9)  # step 0.75
    for tr in ([["scale", 2, 3], ["translate", 1, -1]], [["rotate_deg", 45]],
               [["rotate_deg", 30], ["scale", 2, 1], ["translate", 0.5, 0.5]],
               [["skew_deg", 15, 0]], [["scale", -1, 1]]):
        for radius in (0.0, 0.3):
            probes.probe(op, _contains_points, path=SQ, points=grid_big, transform=tr, radius=radius)
    probes.probe(op, _contains_points, path=SQ, points=[], transform=None, radius=0.0)
    probes.probe(op, _contains_points, path=SQ, points=[[NAN, 0.5], [0.5, NAN], [0.5, 0.5]],
                 transform=None, radius=0.0)
    probes.probe(op, _contains_points, path=SQ_NAN_CLOSED, points=GRID_SQ, transform=None, radius=0.0)
    probes.probe(op, _contains_points, path=SQ, points=[[0.5, 0.5], [2, 2], [1e300, 0.5], [-1e300, 0.5]],
                 transform=None, radius=0.0)


def test_contains_points_shapes():
    op = "path.Path.contains_points"
    for path, grid in ((TRI, GRID_TRI), (DONUT, GRID_DONUT), (DONUT_SAME, GRID_DONUT),
                       (L_SHAPE, GRID_TRI), (BOWTIE, GRID_BOWTIE), (TWO_SQUARES, GRID_DONUT),
                       (STAR5, _grid(-1.0, 1.0, 9)), (WEDGE, _grid(-1.0, 1.0, 9))):
        for radius in (0.0, 0.2, -0.2):
            probes.probe(op, _contains_points, path=path, points=grid, transform=None, radius=radius)
    r = probes.probe(op, _contains_points, path=DONUT, points=[[0.5, 0.5], [2, 2], [3.5, 2]],
                     transform=None, radius=0.0)
    # each subpath is tested on its own and the results are OR-ed: holes are NOT excluded
    assert list(r) == [True, True, True]
    # fewer than 3 vertices: never inside
    for radius in (0.0, 0.5):
        probes.probe(op, _contains_points, path=LINE2, points=[[0.5, 0.5], [0, 0], [2, 2]],
                     transform=None, radius=radius)
    probes.probe(op, _contains_points, path=EMPTY, points=[[0, 0]], transform=None, radius=0.0)
    # collinear (no area)
    probes.probe(op, _contains_points, path=pth([[0, 0], [1, 1], [2, 2]]),
                 points=[[0.5, 0.5], [1, 1], [0.5, 0.6]], transform=None, radius=0.0)


def test_contains_points_curves():
    op = "path.Path.contains_points"
    for path in (CIRCLE, CIRCLE_HALF):
        for radius in (0.0, 0.05, -0.05):
            probes.probe(op, _contains_points, path=path, points=CIRCLE_PTS, transform=None, radius=radius)
    # flattening tolerance depends on the transformed size
    big_pts = [[100 * x, 100 * y] for x, y in CIRCLE_PTS]
    probes.probe(op, _contains_points, path=CIRCLE, points=big_pts, transform=[["scale", 100, 100]], radius=0.0)
    probes.probe(op, _contains_points, path=CIRCLE, points=big_pts, transform=[["scale", 100, 100]], radius=5.0)
    probes.probe(op, _contains_points, path=CIRCLE_BIG,
                 points=[[2 + 3 * x, 1 + 3 * y] for x, y in CIRCLE_PTS], transform=None, radius=0.0)
    quad_pts = _grid(-0.5, 2.5, 13)
    for path in (QUAD_OPEN, QUAD_CLOSED, CUBIC_OPEN, CUBIC_S, CUBIC_LOOP_CLOSED, MIXED_OK, ARC90):
        for radius in (0.0, 0.2):
            probes.probe(op, _contains_points, path=path, points=quad_pts, transform=None, radius=radius)
    probes.probe(op, _contains_points, path=QUAD_CLOSED, points=quad_pts, transform=[["scale", 50, 50]],
                 radius=0.0)
    r = probes.probe(op, _contains_points, path=CIRCLE, points=[[0, 0], [0.5, 0.5], [1, 1]],
                     transform=None, radius=0.0)
    assert list(r) == [True, True, False]


def test_contains_point():
    op = "path.Path.contains_point"
    cases = [
        (SQ, [0.5, 0.5]), (SQ, [1.5, 0.5]), (SQ, [0.0, 0.5]), (SQ, [1.0, 0.5]), (SQ, [0.5, 0.0]),
        (SQ, [0.5, 1.0]), (SQ, [0.0, 0.0]), (SQ, [1.0, 1.0]), (SQ, [1.0, 0.0]), (SQ, [0.0, 1.0]),
        (SQ, [0.999, 0.5]), (SQ, [1.001, 0.5]), (SQ_CW, [0.0, 0.5]), (SQ_CW, [1.0, 0.5]),
        (SQ_OPEN, [0.5, 0.5]), (TRI, [1.0, 1.0]), (TRI, [2.0, 1.5]), (TRI, [2.0, 1.4]),
        (DONUT, [2.0, 2.0]), (DONUT, [0.5, 2.0]), (DONUT, [1.0, 2.0]), (DONUT, [3.0, 2.0]),
        (BOWTIE, [1.0, 1.0]), (BOWTIE, [0.5, 1.0]), (BOWTIE, [1.0, 0.5]),
        (CIRCLE, [0.0, 0.0]), (CIRCLE, [0.99, 0.0]), (CIRCLE, [1.0, 0.0]),
        (CIRCLE, [0.9238795325112867 * 0.999, 0.3826834323650898 * 0.999]),
        (CIRCLE, [0.9238795325112867 * 1.001, 0.3826834323650898 * 1.001]),
        (QUAD_OPEN, [1.0, 0.5]), (QUAD_OPEN, [1.0, 0.99]), (QUAD_OPEN, [1.0, 1.01]),
        (LINE2, [0.5, 0.5]), (EMPTY, [0.0, 0.0]), (SQ, [NAN, 0.5]),
    ]
    for path, point in cases:
        for radius in (0.0, 0.1, -0.1):
            probes.probe(op, _contains_point, path=path, point=point, transform=None, radius=radius)
    for tr, point in (([["scale", 2, 3], ["translate", 1, -1]], [2.0, 0.5]),
                      ([["scale", 2, 3], ["translate", 1, -1]], [0.5, 0.5]),
                      ([["rotate_deg", 45]], [0.0, 0.7]), ([["rotate_deg", 45]], [0.5, 0.2]),
                      ([["scale", 100, 100]], [92.0, 38.0]), ([["scale", 100, 100]], [92.3, 38.2])):
        for path in (SQ, CIRCLE):
            probes.probe(op, _contains_point, path=path, point=point, transform=tr, radius=0.0)
            probes.probe(op, _contains_point, path=path, point=point, transform=tr, radius=2.0)
    assert probes.probe(op, _contains_point, path=SQ, point=[0.5, 0.5], transform=None, radius=0.0) is True
    assert probes.probe(op, _contains_point, path=DONUT, point=[2.0, 2.0], transform=None, radius=0.0) is True
    assert probes.probe(op, _contains_point, path=BOWTIE, point=[0.5, 1.0], transform=None, radius=0.0) is True


def _contains_path(path, other, transform):
    return bool(_path(path).contains_path(_path(other), _tr(transform)))


def test_contains_path():
    op = "path.Path.contains_path"
    big = pth([[-1, -1], [5, -1], [5, 5], [-1, 5], [-1, -1]], [M, L, L, L, CL])
    small = pth([[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75], [0.25, 0.25]], [M, L, L, L, CL])
    cases = [
        (SQ, small, None), (small, SQ, None), (SQ, SQ, None), (SQ, SQ_CW, None),
        (SQ, small, [["translate", 0.5, 0]]), (SQ, small, [["translate", 2, 0]]),
        (SQ, small, [["scale", 0.5, 0.5]]), (big, DONUT, None), (DONUT, small, None),
        (DONUT, small, [["translate", 0.25, 0.25]]), (DONUT, small, [["translate", 2, 2]]),
        (big, CIRCLE, None), (CIRCLE, small, None), (CIRCLE, SQ, None),
        (CIRCLE, pth([[0.6, 0.6], [0.65, 0.6]]), None), (CIRCLE, pth([[0.7, 0.7], [0.71, 0.71]]), None),
        (big, QUAD_OPEN, None), (QUAD_CLOSED, pth([[1, 0.2], [1, 0.9]]), None),
        (QUAD_CLOSED, pth([[1, 0.2], [1, 1.1]]), None), (TRI, LINE2, None), (LINE2, LINE2, None),
        (SQ, EMPTY, None), (EMPTY, SQ, None),
        # CLOSEPOLY vertex coordinates of *other* are tested as points too
        (big, pth([[1, 1], [2, 1], [2, 2], [99, 99]], [M, L, L, CL]), None),
        (big, pth([[1, 1], [2, 1], [2, 2], [1, 1]], [M, L, L, CL]), None),
        (big, pth([[1, 1], [2, 1], [2, 2], [0, 0]], [M, L, L, CL]), None),
        (big, pth([[1, 1], [2, 1], [NAN, 2], [3, 3]]), None),
        (L_SHAPE, pth([[0.5, 2.5], [2.5, 0.5]]), None), (L_SHAPE, pth([[0.5, 0.5], [2.5, 0.5]]), None),
    ]
    for path, other, tr in cases:
        probes.probe(op, _contains_path, path=path, other=other, transform=tr)
    assert probes.probe(op, _contains_path, path=SQ, other=small, transform=None) is True
    assert probes.probe(op, _contains_path, path=small, other=SQ, transform=None) is False


# ==========================================================================
# Extents
# ==========================================================================


def _extents(path, transform):
    return _path(path).get_extents(_tr(transform)).get_points()


def test_get_extents():
    op = "path.Path.get_extents"
    for path, tr in [
        (SQ, None), (SQ_GARBAGE_CLOSE, None), (TRI, None), (DONUT, None), (LINE2, None),
        (pth([[1, 2]], [M]), None), (pth([[1, 2]]), None), (EMPTY, None),
        (POLY_NAN, None), (POLY_NAN_CODES, None), (pth([[0, 0], [1, 1], [5, 5]], [M, L, ST]), None),
        (pth([[0, 0], [1, 1], [5, 5], [-3, 7]], [M, L, M, CL]), None),
        (SQ, [["rotate_deg", 30]]), (SQ, [["scale", 2, -3], ["translate", 1, 1]]),
    ]:
        probes.probe(op, _extents, path=path, transform=tr)
    r = probes.probe(op, _extents, path=EMPTY, transform=None)
    assert np.all(np.isinf(r))
    for path, tr in [
        (CIRCLE, None), (CIRCLE_BIG, None), (CIRCLE_HALF, None), (QUAD_OPEN, None), (QUAD_CLOSED, None),
        (CUBIC_OPEN, None), (CUBIC_S, None), (CUBIC_LOOP_CLOSED, None), (MIXED_OK, None), (ARC90, None),
        (WEDGE, None), (_const(Path.arc(10, 100)), None), (_const(Path.arc(200, 300, 1)), None),
        (_const(Path.arc(0, 90, 1)), None), (CIRCLE, [["rotate_deg", 30]]),
        (CIRCLE, [["scale", 2, 3], ["translate", 1, 0]]), (QUAD_OPEN, [["rotate_deg", 90]]),
        (CUBIC_S, [["skew_deg", 20, 10]]), (CURVE_NAN, None), (QUAD_NAN, None),
        (pth([[0, 0], [1, 1], [1, 1], [1, 1]], [M, C4, C4, C4]), None),
        (pth([[0, 0], [1, 0], [2, 0], [3, 0]], [M, C4, C4, C4]), None),
        (pth([[0, 0], [2, 0], [1, 0]], [M, C3, C3]), None),
        (pth([[0, 0], [1, 1], [2, 2], [3, 3], [9, 9]], [M, C4, C4, C4, ST]), None), (MALFORMED, None),
    ]:
        probes.probe(op, _extents, path=path, transform=tr, tol=MATMUL_TOL)
    r = _extents(CIRCLE, None)
    assert np.allclose(r, [[-1, -1], [1, 1]])
    r = _extents(QUAD_OPEN, None)
    assert np.allclose(r, [[0, 0], [2, 1]])
    r = _extents(CUBIC_OPEN, None)
    assert np.allclose(r, [[0, 0], [1, 0.75]])


def _collection_extents(master_transform, paths, transforms, offsets, offset_transform):
    bb = get_path_collection_extents(
        _tr(master_transform), [_path(p) for p in paths],
        np.asarray(transforms, dtype=float).reshape(-1, 3, 3), _pts(offsets),
        _tr(offset_transform))
    return {"bbox": bb.get_points(), "minpos": bb.minpos}


def test_path_collection_extents():
    op = "path.get_path_collection_extents"
    scale2 = [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]]
    shift = [[1.0, 0.0, -3.0], [0.0, 1.0, 0.5], [0.0, 0.0, 1.0]]
    probes.probe(op, _collection_extents, master_transform=[], paths=[SQ], transforms=[],
                 offsets=[[0, 0]], offset_transform=[])
    probes.probe(op, _collection_extents, master_transform=[["translate", 1, 1]], paths=[SQ, CIRCLE],
                 transforms=[scale2], offsets=[[1, 1], [5, 5], [-2, 0]], offset_transform=[["scale", 10, 10]])
    probes.probe(op, _collection_extents, master_transform=[], paths=[_const(Path.arc(0, 90, 1))],
                 transforms=[], offsets=[[0, 0]], offset_transform=[])
    probes.probe(op, _collection_extents, master_transform=[], paths=[SQ, TRI], transforms=[scale2, shift],
                 offsets=[[0, 0], [1, 1], [2, 2]], offset_transform=[])
    probes.probe(op, _collection_extents, master_transform=[], paths=[POLY_NAN], transforms=[],
                 offsets=[[0, 0]], offset_transform=[])
    probes.probe(op, _collection_extents, master_transform=[], paths=[pth([[-1, -2], [-0.5, -1]])],
                 transforms=[], offsets=[[0, 0]], offset_transform=[])
    assert isinstance(probes.probe(op, _collection_extents, master_transform=[], paths=[], transforms=[],
                                   offsets=[[0, 0]], offset_transform=[]), ValueError)
    assert isinstance(probes.probe(op, _collection_extents, master_transform=[], paths=[SQ], transforms=[],
                                   offsets=[], offset_transform=[]), ValueError)


# ==========================================================================
# Intersections
# ==========================================================================


def _intersects_path(path, other, filled):
    return bool(_path(path).intersects_path(_path(other), filled))


def _intersects_bbox(path, bbox, filled):
    return bool(_path(path).intersects_bbox(_bbox(bbox), filled))


def _sq_at(x0, y0, s=1.0):
    return pth([[x0, y0], [x0 + s, y0], [x0 + s, y0 + s], [x0, y0 + s], [x0, y0]], [M, L, L, L, CL])


def test_intersects_path():
    op = "path.Path.intersects_path"
    cases = [
        (SQ, _sq_at(0.5, 0.5)),            # overlap
        (SQ, _sq_at(2, 2)),                # disjoint
        (SQ, _sq_at(0.25, 0.25, 0.5)),     # contained
        (_sq_at(0.25, 0.25, 0.5), SQ),     # container second
        (SQ, _sq_at(1, 1)),                # corner touch
        (SQ, _sq_at(1, 0)),                # shared edge
        (SQ, _sq_at(1, 0.5)),              # partially shared edge
        (SQ, _sq_at(1 + 1e-12, 0)),        # near touch (isclose)
        (SQ, _sq_at(1 + 1e-9, 0)),         # small gap
        (SQ, SQ),
        (pth([[0, 0], [1, 0]]), pth([[2, 0], [3, 0]])),   # collinear disjoint
        (pth([[0, 0], [2, 0]]), pth([[1, 0], [3, 0]])),   # collinear overlap
        (pth([[0, 0], [1, 0]]), pth([[1, 0], [3, 0]])),   # collinear touching
        (pth([[0, 0], [0, 1]]), pth([[0, 2], [0, 3]])),   # vertical collinear disjoint
        (pth([[0, 0], [0, 2]]), pth([[0, 1], [0, 3]])),   # vertical collinear overlap
        (pth([[0, 0], [1, 0]]), pth([[0, 1], [1, 1]])),   # parallel
        (pth([[0, 0], [2, 2]]), pth([[0, 2], [2, 0]])),   # X crossing
        (pth([[0, 0], [2, 0]]), pth([[1, 0], [1, 2]])),   # T junction
        (pth([[0, 0], [2, 0]]), pth([[1, 1e-14], [1, 2]])),
        (pth([[0, 0], [2, 0]]), pth([[1, 1e-6], [1, 2]])),
        (pth([[0, 0], [1, 1], [2, 0]]), pth([[0, 1], [2, 1]])),  # touches apex
        (pth([[0, 0], [0, 0], [1, 1]]), pth([[0, 1], [1, 0]])),  # zero-length first segment
        (pth([[0, 0], [1, 1], [NAN, NAN], [2, 2], [3, 1]]), pth([[0, 1], [1, 0]])),
        (pth([[0, 0], [1, 1], [NAN, NAN], [2, 2], [3, 1]]), pth([[1.2, 2], [1.8, 1]])),
        # subpaths: the MOVETO jump from (1, 0) to (3, 0) crosses the other path
        (pth([[0, 0], [1, 0], [3, 0], [4, 0]], [M, L, M, L]), pth([[2, -1], [2, 1]])),
        (TWO_SQUARES, pth([[2, -1], [2, 2]])),
        # CLOSEPOLY vertex coordinates
        (pth([[0, 0], [1, 0], [1, 1], [5, 5]], [M, L, L, CL]), pth([[3, 0], [3, 10]])),
        (LINE2, pth([[5, 5]])), (pth([[0, 0]]), pth([[0, 0]])), (EMPTY, SQ),
        # curves (flattened)
        (CIRCLE, pth([[-2, 0], [2, 0]])), (CIRCLE, pth([[-2, 1], [2, 1]])),
        (CIRCLE, pth([[-2, 0.999], [2, 0.999]])), (CIRCLE, pth([[-2, 1.001], [2, 1.001]])),
        (CIRCLE, pth([[0, 0], [0.1, 0.1]])), (CIRCLE, _sq_at(-0.5, -0.5)), (CIRCLE, _sq_at(-2, -2, 4)),
        (CIRCLE, CIRCLE_BIG), (CIRCLE, _const(Path.circle((2.0, 0.0), 1.0))),
        (QUAD_OPEN, pth([[0, 0.99], [2, 0.99]])), (QUAD_OPEN, pth([[0, 1.01], [2, 1.01]])),
        (QUAD_OPEN, pth([[0, 1.0], [2, 1.0]])), (CUBIC_S, CUBIC_OPEN), (ARC90, pth([[0, 0], [1, 1]])),
        (DONUT, _sq_at(1.5, 1.5)), (DONUT, _sq_at(0.5, 1.5)), (DONUT, _sq_at(-1, -1, 6)),
        (BOWTIE, pth([[1, 0], [1, 0.9]])), (BOWTIE, pth([[0.5, 1], [1.5, 1]])),
    ]
    for a, b in cases:
        for filled in (True, False):
            probes.probe(op, _intersects_path, path=a, other=b, filled=filled)
    assert probes.probe(op, _intersects_path, path=SQ, other=_sq_at(0.25, 0.25, 0.5), filled=True) is True
    assert probes.probe(op, _intersects_path, path=SQ, other=_sq_at(0.25, 0.25, 0.5), filled=False) is False
    assert probes.probe(op, _intersects_path, path=SQ, other=_sq_at(2, 2), filled=True) is False


def test_intersects_bbox():
    op = "path.Path.intersects_bbox"
    cases = [
        (SQ, [[0.25, 0.25], [0.75, 0.75]]),   # bbox inside path
        (SQ, [[-1, -1], [2, 2]]),             # path inside bbox
        (SQ, [[0.5, 0.5], [2, 2]]),           # crossing
        (SQ, [[2, 2], [3, 3]]),               # disjoint
        (SQ, [[1, 0], [2, 1]]),               # shared edge
        (SQ, [[1, 1], [2, 2]]),               # corner touch
        (SQ, [[2, 2], [0.5, 0.5]]),           # inverted bbox
        (SQ, [[0.5, 0.5], [0.5, 0.5]]),       # degenerate bbox inside
        (SQ, [[1.5, 0.5], [1.5, 0.5]]),
        (SQ_OPEN, [[0.25, 0.25], [0.75, 0.75]]),
        (LINE2, [[0.4, 0.4], [0.6, 0.6]]), (LINE2, [[0.6, 0.0], [0.9, 0.3]]), (LINE2, [[0.5, 0.0], [0.9, 0.5]]),
        (pth([[0, 0], [2, 0]]), [[0.5, 0], [1, 1]]), (pth([[0, 0], [2, 0]]), [[0.5, -1], [1, 0]]),
        (pth([[5, 5]]), [[0, 0], [1, 1]]), (pth([[0.5, 0.5]]), [[0, 0], [1, 1]]), (EMPTY, [[0, 0], [1, 1]]),
        (DONUT, [[1.5, 1.5], [2.5, 2.5]]), (DONUT, [[0.25, 0.25], [0.75, 0.75]]),
        (DONUT, [[0.5, 1.5], [1.5, 2.5]]),
        (CIRCLE, [[-0.1, -0.1], [0.1, 0.1]]), (CIRCLE, [[0.7, 0.7], [0.8, 0.8]]),
        (CIRCLE, [[0.69, 0.69], [0.71, 0.71]]), (CIRCLE, [[0.72, 0.72], [0.8, 0.8]]),
        (CIRCLE, [[1, -0.1], [2, 0.1]]), (CIRCLE, [[1.001, -0.1], [2, 0.1]]),
        (QUAD_OPEN, [[0.9, 0.9], [1.1, 1.1]]), (QUAD_OPEN, [[0.9, 0.2], [1.1, 0.4]]),
        (pth([[NAN, NAN], [0.5, 0.5], [2, 2]]), [[0, 0], [1, 1]]),
        (TWO_SQUARES, [[1.5, 0.25], [2.5, 0.75]]),
    ]
    for path, bbox in cases:
        for filled in (True, False):
            probes.probe(op, _intersects_bbox, path=path, bbox=bbox, filled=filled)
    assert probes.probe(op, _intersects_bbox, path=SQ, bbox=[[0.25, 0.25], [0.75, 0.75]], filled=True) is True
    assert probes.probe(op, _intersects_bbox, path=SQ, bbox=[[0.25, 0.25], [0.75, 0.75]], filled=False) is False
    assert probes.probe(op, _intersects_bbox, path=SQ, bbox=[[2, 2], [3, 3]], filled=True) is False


# ==========================================================================
# Clipping / interpolation / polygons
# ==========================================================================


def _clip_to_bbox(path, bbox, inside):
    return _pd(_path(path).clip_to_bbox(_bbox(bbox), inside))


def test_clip_to_bbox():
    op = "path.Path.clip_to_bbox"
    big_sq = _sq_at(0, 0, 4)
    cases = [
        (big_sq, [[1, 1], [3, 5]]), (SQ, [[0.5, 0.5], [2, 2]]), (SQ, [[-1, -1], [2, 2]]),
        (SQ, [[2, 2], [3, 3]]), (SQ, [[0.25, 0.25], [0.75, 0.75]]), (SQ, [[2, 2], [0.5, 0.5]]),
        (TRI, [[1, 0.5], [3, 2]]), (SQ_OPEN, [[0.5, -1], [2, 2]]), (BOWTIE, [[0.5, 0.5], [1.5, 1.5]]),
        (DONUT, [[0.5, 0.5], [2, 2]]), (DONUT, [[1.5, 1.5], [2.5, 2.5]]), (L_SHAPE, [[0.5, 0.5], [2, 2]]),
        (TWO_SQUARES, [[0.5, -1], [3.5, 2]]), (CIRCLE, [[-0.5, -2], [2, 2]]), (CIRCLE, [[0, 0], [2, 2]]),
        (QUAD_CLOSED, [[0, 0.5], [2, 2]]), (WEDGE, [[-1, 0.5], [1, 1]]), (EMPTY, [[0, 0], [1, 1]]),
        (pth([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]), [[0.5, 0.5], [2, 2]]),
        (pth([[0, 0], [2, 0], [2, 2]], [M, L, L]), [[1, -1], [3, 3]]),
    ]
    for path, bbox in cases:
        for inside in (True, False):
            probes.probe(op, _clip_to_bbox, path=path, bbox=bbox, inside=inside)
    r = probes.probe(op, _clip_to_bbox, path=SQ, bbox=[[0.5, 0.5], [2, 2]], inside=True)
    assert np.array_equal(r["vertices"], [[0.5, 0.5], [1, 0.5], [1, 1], [0.5, 1], [0.5, 0.5]])


def _interpolated(path, steps):
    return _pd(_path(path).interpolated(steps))


def test_interpolated():
    op = "path.Path.interpolated"
    for path in (SQ, SQ_GARBAGE_CLOSE, TRI, SQ_OPEN_CODES, DONUT, TWO_SQUARES, LINE2, QUAD_OPEN,
                 QUAD_CLOSED, CUBIC_OPEN, MIXED_OK, POLY_NAN, pth([[1, 2]]), pth([[1, 2]], [M])):
        for steps in (1, 2, 3, 5):
            probes.probe(op, _interpolated, path=path, steps=steps)
    probes.probe(op, _interpolated, path=EMPTY, steps=3)
    probes.probe(op, _interpolated, path=pth([[0, 0], [1, 1], [5, 5]], [M, L, M]), steps=2)
    probes.probe(op, _interpolated, path=pth([[0, 0], [1, 1], [5, 5], [6, 6]], [M, L, CL, L]), steps=2)
    assert isinstance(probes.probe(op, _interpolated, path=SQ, steps=0), ValueError)
    probes.probe(op, _interpolated, path=TRI, steps=0)
    r = probes.probe(op, _interpolated, path=LINE2, steps=4)
    assert np.allclose(r["vertices"], [[0, 0], [0.25, 0.25], [0.5, 0.5], [0.75, 0.75], [1, 1]])
    r = probes.probe(op, _interpolated, path=SQ_GARBAGE_CLOSE, steps=2)
    assert list(r["vertices"][-1]) == [0, 0]


def _to_polygons(path, transform, width, height, closed_only, rc):
    with matplotlib.rc_context(rc):
        p = _path(path)
        return [np.asarray(poly) for poly in p.to_polygons(_tr(transform), width, height, closed_only)]


def test_to_polygons():
    op = "path.Path.to_polygons"
    paths = [SQ, SQ_OPEN, SQ_OPEN_CODES, TRI, LINE2, pth([[0, 0], [1, 0], [1, 1], [0, 0]]), DONUT,
             TWO_SQUARES, BOWTIE, CIRCLE, QUAD_OPEN, QUAD_CLOSED, CUBIC_S, ARC90, MIXED_OK, POLY_NAN,
             POLY_NAN_CODES, POLY_NAN_FIRST, CURVE_NAN, SQ_NAN_CLOSED, EMPTY,
             pth([[0, 0], [1, 1], [2, 0], [5, 5], [6, 5]], [M, L, L, M, L]),
             pth([[0, 0], [1, 1], [2, 0], [0, 0], [5, 5], [6, 5], [6, 6]], [M, L, L, CL, M, L, L])]
    for path in paths:
        for closed_only in (True, False):
            probes.probe(op, _to_polygons, path=path, transform=None, width=0, height=0,
                         closed_only=closed_only, rc=REF_RC)
    for path in (SQ, TRI, SQ_OPEN, CIRCLE, DONUT):
        for closed_only in (True, False):
            probes.probe(op, _to_polygons, path=path, transform=[["scale", 10, 10], ["translate", -2, -3]],
                         width=0, height=0, closed_only=closed_only, rc=REF_RC)
            # width/height non-zero: clip to (0, 0)-(width, height) (padded) in the C++ path
            probes.probe(op, _to_polygons, path=path, transform=[["scale", 10, 10], ["translate", -2, -3]],
                         width=5, height=4, closed_only=closed_only, rc=REF_RC)
    probes.probe(op, _to_polygons, path=pth([[-50, 1], [50, 2], [60, 3]]), transform=None, width=10, height=10,
                 closed_only=False, rc=REF_RC)
    probes.probe(op, _to_polygons, path=CIRCLE, transform=[["scale", 100, 100]], width=0, height=0,
                 closed_only=True, rc=REF_RC)
    for path, rc in ((LONG_SMOOTH, REF_RC), (LONG_NOISY, REF_RC), (LONG_128, REF_RC),
                     (LONG_SMOOTH, {"path.simplify": False, "path.simplify_threshold": 0.111111111111}),
                     (LONG_SMOOTH, {"path.simplify": True, "path.simplify_threshold": 1.0})):
        probes.probe(op, _to_polygons, path=path, transform=None, width=0, height=0, closed_only=False, rc=rc)
    probes.probe(op, _to_polygons, path=LONG_SMOOTH, transform=None, width=100, height=100,
                 closed_only=False, rc=REF_RC)
    probes.probe(op, _to_polygons, path=LONG_SMOOTH, transform=None, width=100, height=100,
                 closed_only=True, rc=REF_RC)
    r = probes.probe(op, _to_polygons, path=TRI, transform=None, width=0, height=0, closed_only=True, rc=REF_RC)
    assert np.array_equal(r[0], [[0, 0], [4, 0], [0, 3], [0, 0]])
    r = probes.probe(op, _to_polygons, path=LINE2, transform=None, width=0, height=0, closed_only=True, rc=REF_RC)
    assert r == []


# ==========================================================================
# cleaned / iter_segments / iter_bezier / transformed
# ==========================================================================


def _cleaned(path, transform, remove_nans, clip, simplify, curves, stroke_width, snap, rc):
    with matplotlib.rc_context(rc):
        p = _path(path)
        return _pd(p.cleaned(_tr(transform), remove_nans, None if clip is None else tuple(clip),
                             simplify=simplify, curves=curves, stroke_width=stroke_width, snap=snap))


def probe_cleaned(path, transform=None, remove_nans=False, clip=None, simplify=False, curves=False,
                  stroke_width=1.0, snap=False, rc=REF_RC):
    return probes.probe("path.Path.cleaned", _cleaned, path=path, transform=transform, remove_nans=remove_nans,
                        clip=clip, simplify=simplify, curves=curves, stroke_width=stroke_width, snap=snap, rc=rc)


def test_cleaned_basic():
    for path in (SQ, TRI, SQ_OPEN_CODES, DONUT, QUAD_OPEN, CUBIC_S, MIXED_OK, CIRCLE, EMPTY, pth([[1, 2]])):
        probe_cleaned(path)
        probe_cleaned(path, curves=True)
    r = probe_cleaned(SQ)
    assert list(r["codes"]) == [M, L, L, L, CL, ST]
    probe_cleaned(SQ, transform=[["scale", 2, 3], ["translate", 1, 0]])
    probe_cleaned(CIRCLE, transform=[["scale", 100, 100]])
    probe_cleaned(CIRCLE, transform=[["scale", 100, 100]], curves=True)
    probe_cleaned(CIRCLE, transform=[["scale", 400, 150]])
    probe_cleaned(QUAD_OPEN, transform=[["scale", 100, 100]])
    probe_cleaned(CUBIC_S, transform=[["scale", 50, 50]])
    probe_cleaned(CUBIC_LOOP_CLOSED, transform=[["scale", 20, 20]])
    probe_cleaned(ARC90, transform=[["scale", 300, 300]])
    probe_cleaned(pth([[0, 0], [1, 1], [1, 1], [1, 1]], [M, C4, C4, C4]), transform=[["scale", 10, 10]])
    probe_cleaned(pth([[0, 0], [1, 0], [2, 0], [3, 0]], [M, C4, C4, C4]), transform=[["scale", 10, 10]])
    probe_cleaned(pth([[0, 0], [1, 0], [0, 0]], [M, C3, C3]), transform=[["scale", 10, 10]])
    # malformed: the CLOSEPOLY vertex is consumed as the last CURVE4 point
    probe_cleaned(MALFORMED, curves=True)
    probe_cleaned(MALFORMED, curves=False)


def test_cleaned_nans():
    for path in (POLY_NAN, POLY_NAN_CODES, POLY_NAN_FIRST, CURVE_NAN, QUAD_NAN, SQ_NAN_CLOSED,
                 pth([[NAN, NAN], [NAN, NAN]]), pth([[0, 0], [1, 1], [float("inf"), 2], [3, 3]])):
        for curves in (True, False):
            probe_cleaned(path, remove_nans=True, curves=curves)
        probe_cleaned(path, remove_nans=False, curves=True)


def test_cleaned_snap():
    paths = [pth([[0.3, 0.3], [10.6, 0.3], [10.6, 5.2], [0.3, 5.2], [0.3, 0.3]], [M, L, L, L, CL]),
             pth([[0.3, 0.3], [10.6, 7.7]]),          # diagonal: not snapped in auto mode
             pth([[0.5, 0.5], [1.5, 0.5], [2.5, 0.5], [-0.5, -1.5]]),
             QUAD_OPEN]
    for path in paths:
        for snap in (True, False, None):
            for stroke_width in (1.0, 2.0, 0.4, 1.5, 2.5):
                probe_cleaned(path, snap=snap, stroke_width=stroke_width, curves=True)
    r = probe_cleaned(paths[0], snap=True, stroke_width=1.0, curves=True)
    assert list(r["vertices"][0]) == [0.5, 0.5]


def test_cleaned_clip():
    paths = [pth([[-10, 5], [20, 5]]), pth([[-10, -10], [20, 20], [30, -5]]), SQ, _sq_at(-5, -5, 20),
             pth([[5, 5], [6, 6], [7, 5]]), pth([[-100, 50], [100, 50], [200, 60]]), CIRCLE, POLY_NAN,
             pth([[-10, 5], [20, 5], [20, 50], [-10, 50]], [M, L, M, L])]
    for path in paths:
        for clip in ([0, 0, 10, 10], [2, 2, 3, 3]):
            probe_cleaned(path, clip=clip, curves=True)
            probe_cleaned(path, clip=clip, remove_nans=True, curves=False)
    for path in paths[:2]:
        probe_cleaned(path, clip=[10, 10, 0, 0], curves=True)  # inverted rect: clipping disabled
    probe_cleaned(CIRCLE, clip=[0, 0, 10, 10], transform=[["scale", 30, 30]], curves=False)


SIMPLIFY_RCS = [REF_RC, {"path.simplify": True, "path.simplify_threshold": 0.0},
                {"path.simplify": True, "path.simplify_threshold": 1.0},
                {"path.simplify": False, "path.simplify_threshold": 0.5}]


def test_cleaned_simplify():
    small = [pth([[0, 0], [1, 0.01], [2, 0], [3, 0.02], [4, 0], [5, 3], [6, 0]]),
             pth([[0, 0], [1, 0], [2, 0], [3, 0], [2, 0], [1, 0], [5, 0]]),
             pth([[0, 0], [1, 1], [2, 2], [3, 3], [4, 4.05], [5, 5]]),
             pth([[0, 0], [0, 1], [0, 2], [0, 3], [0, 1], [1, 1]]),
             pth([[0, 0], [10, 0], [10, 0.01], [20, 0.02], [20, 10], [20.04, 20]])]
    for path in small + [POLY_NAN]:
        for rc in SIMPLIFY_RCS:
            probe_cleaned(path, simplify=True, rc=rc)
    for rc in SIMPLIFY_RCS[:3]:
        probe_cleaned(LONG_SMOOTH, simplify=True, rc=rc)
    probe_cleaned(LONG_NOISY, simplify=True)
    probe_cleaned(LONG_128, simplify=True)
    probe_cleaned(LONG_SMOOTH, simplify=True, transform=[["scale", 3, 3]])
    probe_cleaned(LONG_SMOOTH, simplify=True, transform=[["scale", 0.1, 0.1]])
    probe_cleaned(LONG_SMOOTH, simplify=True, clip=[20, -20, 80, 20])
    probe_cleaned(LONG_SMOOTH, simplify=True, snap=True)
    probe_cleaned(DONUT, simplify=True)
    probe_cleaned(CIRCLE, simplify=True, transform=[["scale", 100, 100]], curves=False)
    probe_cleaned(CIRCLE, simplify=True, transform=[["scale", 100, 100]], curves=True)
    probe_cleaned(LONG_SMOOTH, simplify=False)


def _iter_segments(path, transform, remove_nans, clip, snap, stroke_width, simplify, curves, rc):
    with matplotlib.rc_context(rc):
        p = _path(path)
        return [[np.asarray(v), int(c)] for v, c in p.iter_segments(
            _tr(transform), remove_nans=remove_nans, clip=None if clip is None else tuple(clip), snap=snap,
            stroke_width=stroke_width, simplify=simplify, curves=curves)]


def probe_iter_segments(path, transform=None, remove_nans=True, clip=None, snap=False, stroke_width=1.0,
                        simplify=None, curves=True, rc=REF_RC):
    return probes.probe("path.Path.iter_segments", _iter_segments, path=path, transform=transform,
                        remove_nans=remove_nans, clip=clip, snap=snap, stroke_width=stroke_width,
                        simplify=simplify, curves=curves, rc=rc)


def test_iter_segments():
    for path in (SQ, TRI, DONUT, QUAD_OPEN, CUBIC_S, MIXED_OK, CIRCLE, POLY_NAN, CURVE_NAN, EMPTY,
                 pth([[0, 0], [1, 1], [2, 2]], [M, L, ST]), pth([[0, 0], [1, 1], [2, 2], [3, 3]], [M, L, ST, L])):
        probe_iter_segments(path)
        probe_iter_segments(path, curves=False)
    r = probe_iter_segments(SQ)
    assert [c for _, c in r] == [M, L, L, L, CL]
    assert list(r[0][0]) == [0, 0]
    r = probe_iter_segments(QUAD_OPEN)
    assert [c for _, c in r] == [M, C3] and list(r[1][0]) == [1, 2, 2, 0]
    for rc in SIMPLIFY_RCS:
        probe_iter_segments(LONG_SMOOTH, rc=rc)        # simplify=None -> Path.should_simplify
    probe_iter_segments(LONG_SMOOTH, simplify=False)
    probe_iter_segments(LONG_NOISY)
    for path in (LONG_127, LONG_128):
        probe_iter_segments(path)
        probe_iter_segments(path, simplify=True)
    probe_iter_segments(SQ, transform=[["rotate_deg", 30], ["scale", 2, 3]])
    probe_iter_segments(SQ, snap=True, transform=[["scale", 10.3, 10.3]])
    probe_iter_segments(SQ, snap=None, stroke_width=2.0, transform=[["scale", 10.3, 10.3]])
    probe_iter_segments(POLY_NAN, remove_nans=False)
    probe_iter_segments(MALFORMED)
    probe_iter_segments(pth([[-10, 5], [20, 5], [25, 30]]), clip=[0, 0, 10, 10])


def _should_simplify(path, rc):
    with matplotlib.rc_context(rc):
        return bool(_path(path).should_simplify)


def _simplify_threshold(path, rc):
    with matplotlib.rc_context(rc):
        return _path(path).simplify_threshold


def test_should_simplify():
    op = "path.Path.should_simplify"
    long_curve = _cheap_long(129, [M] + [C3] * 128)
    long_close = _cheap_long(130, [M] + [L] * 128 + [CL])
    long_stop = _cheap_long(130, [M] + [L] * 128 + [ST])
    long_lines = _cheap_long(130, [M] + [L] * 129)
    for path in (SQ, LONG_127, LONG_128):
        for rc in SIMPLIFY_RCS:
            probes.probe(op, _should_simplify, path=path, rc=rc)
    for path in (long_curve, long_close, long_stop, long_lines):
        probes.probe(op, _should_simplify, path=path, rc=REF_RC)
    assert probes.probe(op, _should_simplify, path=LONG_128, rc=REF_RC) is True
    assert probes.probe(op, _should_simplify, path=LONG_127, rc=REF_RC) is False
    for rc in SIMPLIFY_RCS:
        probes.probe("path.Path.simplify_threshold", _simplify_threshold, path=SQ, rc=rc)


def _iter_bezier(path):
    return [[b.control_points, int(c)] for b, c in _path(path).iter_bezier()]


def test_iter_bezier():
    op = "path.Path.iter_bezier"
    for path in (SQ, TRI, QUAD_CLOSED, CUBIC_S, MIXED_OK, CIRCLE, DONUT, POLY_NAN, EMPTY,
                 pth([[0, 0], [1, 1], [5, 5], [6, 6]], [M, L, CL, L])):
        probes.probe(op, _iter_bezier, path=path)
    assert isinstance(probes.probe(op, _iter_bezier, path=pth([[0, 0], [1, 1]], [M, L])), list)


def _transformed(path, transform):
    return _pd(_path(path).transformed(_tr(transform)))


def test_transformed():
    op = "path.Path.transformed"
    for path in (SQ, TRI, CIRCLE, EMPTY, POLY_NAN):
        for tr in ([["scale", 2, 3], ["translate", 1, 0]], [["rotate_deg", 90]], [["rotate_deg", 30]],
                   [["skew_deg", 10, 20], ["translate", -1, 2]], []):
            probes.probe(op, _transformed, path=path, transform=tr)
    r = probes.probe(op, _transformed, path=SQ, transform=[["scale", 2, 3], ["translate", 1, 0]])
    assert np.array_equal(r["vertices"], [[1, 0], [3, 0], [3, 3], [1, 3], [1, 0]])


# ==========================================================================
# matplotlib.bezier
# ==========================================================================

BEZ = {
    "point": [[1.5, -2.0]],
    "line": [[0.0, 0.0], [4.0, 2.0]],
    "quad": [[0.0, 0.0], [1.0, 2.0], [2.0, 0.0]],
    "cubic": [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
    "cubic_s": [[0.0, 0.0], [3.0, 2.0], [-1.0, 2.0], [2.0, 0.0]],
    "cubic_generic": [[0.1, 0.7], [1.3, -0.4], [2.9, 3.1], [-0.6, 1.7]],
    "cubic_degenerate": [[1.0, 1.0], [1.0, 1.0], [1.0, 1.0], [1.0, 1.0]],
    "cubic_collinear": [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0]],
    "cubic_backtrack": [[0.0, 0.0], [3.0, 0.0], [-1.0, 0.0], [2.0, 0.0]],
    "quad_linear_deriv": [[0.0, 0.0], [1.0, 1.0], [2.0, 0.5]],
    "quartic": [[0.0, 0.0], [1.0, 3.0], [2.0, -2.0], [3.0, 2.0], [4.0, 0.0]],
    "cubic_3d": [[0.0, 0.0, 0.0], [1.0, 2.0, -1.0], [2.0, -1.0, 3.0], [3.0, 0.0, 1.0]],
    "circle_seg": [list(v) for v in Path.unit_circle().vertices[0:4].tolist()],
}


def _bz_call(control_points, t):
    return mbezier.BezierSegment(np.asarray(control_points, float))(t)


def _bz_point_at_t(control_points, t):
    return mbezier.BezierSegment(np.asarray(control_points, float)).point_at_t(t)


def _bz_properties(control_points):
    b = mbezier.BezierSegment(np.asarray(control_points, float))
    return {"degree": b.degree, "dimension": b.dimension, "control_points": b.control_points,
            "polynomial_coefficients": b.polynomial_coefficients}


def _bz_extrema(control_points):
    dims, dzeros = mbezier.BezierSegment(np.asarray(control_points, float)).axis_aligned_extrema()
    return {"dims": [int(d) for d in dims], "dzeros": dzeros}


def test_bezier_segment():
    ts = [0.0, 0.25, 0.5, 0.75, 1.0]
    for name, cp in BEZ.items():
        probes.probe("bezier.BezierSegment.__call__", _bz_call, control_points=cp, t=ts, tol=MATMUL_TOL)
        probes.probe("bezier.BezierSegment.__call__", _bz_call, control_points=cp, t=0.3, tol=MATMUL_TOL)
        probes.probe("bezier.BezierSegment.properties", _bz_properties, control_points=cp, tol=MATMUL_TOL)
        tol = 1e-9 if name == "quartic" else MATMUL_TOL  # quartic: np.roots (eigenvalues)
        probes.probe("bezier.BezierSegment.axis_aligned_extrema", _bz_extrema, control_points=cp, tol=tol)
    probes.probe("bezier.BezierSegment.__call__", _bz_call, control_points=BEZ["cubic"],
                 t=[-0.5, 1.5, 0.1, 0.9], tol=MATMUL_TOL)
    probes.probe("bezier.BezierSegment.__call__", _bz_call, control_points=BEZ["cubic"], t=[], tol=MATMUL_TOL)
    probes.probe("bezier.BezierSegment.point_at_t", _bz_point_at_t, control_points=BEZ["cubic"], t=0.5,
                 tol=MATMUL_TOL)
    probes.probe("bezier.BezierSegment.point_at_t", _bz_point_at_t, control_points=BEZ["cubic_generic"], t=0.3,
                 tol=MATMUL_TOL)
    r = _bz_call(BEZ["cubic"], ts)
    assert np.allclose(r[2], [0.5, 0.75])
    r = _bz_extrema(BEZ["cubic"])
    assert r["dims"] == [0, 0, 1] and np.allclose(r["dzeros"], [0, 1, 0.5])  # x' = 6t(1-t): roots 0 and 1
    r = _bz_properties(BEZ["cubic"])
    assert r["degree"] == 3 and r["dimension"] == 2
    assert np.allclose(r["polynomial_coefficients"], [[0, 0], [0, 3], [3, -3], [-2, 0]])


def _split_de_casteljau(beta, t):
    left, right = mbezier.split_de_casteljau(np.asarray(beta, float), t)
    return [np.asarray(left), np.asarray(right)]


def test_split_de_casteljau():
    op = "bezier.split_de_casteljau"
    # (a single control point would loop forever in matplotlib: not probed)
    for name in ("line", "quad", "cubic", "cubic_generic", "quartic", "cubic_3d", "circle_seg"):
        for t in (0.0, 0.5, 0.3, 1.0, 1.5):
            probes.probe(op, _split_de_casteljau, beta=BEZ[name], t=t)
    r = _split_de_casteljau(BEZ["line"], 0.5)
    assert np.array_equal(r[0], [[0, 0], [2, 1]]) and np.array_equal(r[1], [[2, 1], [4, 2]])


def _find_t(control_points, circle, t0, t1, tolerance):
    bz = mbezier.BezierSegment(np.asarray(control_points, float))
    return list(mbezier.find_bezier_t_intersecting_with_closedpath(
        lambda t: tuple(bz(t)), mbezier.inside_circle(*circle), t0=t0, t1=t1, tolerance=tolerance))


def _split_bezier(bezier, circle, tolerance):
    left, right = mbezier.split_bezier_intersecting_with_closedpath(
        np.asarray(bezier, float), mbezier.inside_circle(*circle), tolerance=tolerance)
    return [np.asarray(left), np.asarray(right)]


def test_bezier_closedpath_split():
    op = "bezier.find_bezier_t_intersecting_with_closedpath"
    for name, circle in (("line", [0, 0, 1]), ("line", [4, 2, 2]), ("quad", [0, 0, 1.5]),
                         ("cubic", [1, 0, 0.5]), ("cubic_generic", [0.1, 0.7, 1.0]),
                         ("cubic_s", [2, 0, 1.2])):
        for tolerance in (0.01, 1e-4, 0.5):
            probes.probe(op, _find_t, control_points=BEZ[name], circle=circle, t0=0.0, t1=1.0,
                         tolerance=tolerance, tol=BISECT_TOL)
    probes.probe(op, _find_t, control_points=BEZ["line"], circle=[0, 0, 1], t0=0.1, t1=0.9, tolerance=0.01,
                 tol=BISECT_TOL)
    probes.probe(op, _find_t, control_points=BEZ["cubic_degenerate"], circle=[0, 0, 1], t0=0.0, t1=1.0,
                 tolerance=0.01, tol=BISECT_TOL)
    probes.probe(op, _find_t, control_points=[[1e17, 0.0], [1e17 + 1e3, 0.0]], circle=[0, 0, 1e17 + 512],
                 t0=0.0, t1=1.0, tolerance=1e-30, tol=BISECT_TOL)
    e = probes.probe(op, _find_t, control_points=BEZ["line"], circle=[10, 10, 1], t0=0.0, t1=1.0, tolerance=0.01)
    assert type(e).__name__ == "NonIntersectingPathException"
    probes.probe(op, _find_t, control_points=BEZ["line"], circle=[2, 1, 10], t0=0.0, t1=1.0, tolerance=0.01)

    op = "bezier.split_bezier_intersecting_with_closedpath"
    for name, circle in (("line", [0, 0, 1]), ("quad", [0, 0, 1.5]), ("cubic", [1, 0, 0.5]),
                         ("cubic_generic", [0.1, 0.7, 1.0]), ("circle_seg", [0, -1, 0.5])):
        for tolerance in (0.01, 1e-5):
            probes.probe(op, _split_bezier, bezier=BEZ[name], circle=circle, tolerance=tolerance, tol=BISECT_TOL)
    probes.probe(op, _split_bezier, bezier=BEZ["quad"], circle=[10, 10, 1], tolerance=0.01)


def _split_path_inout(path, circle, tolerance, reorder_inout):
    a, b = mbezier.split_path_inout(_path(path), mbezier.inside_circle(*circle), tolerance, reorder_inout)
    return [_pd(a), _pd(b)]


def test_split_path_inout():
    op = "bezier.split_path_inout"
    cases = [
        (pth([[0, 0], [1, 0], [2, 0], [3, 0]]), [0, 0, 1.5]),
        (pth([[0, 0], [1, 0], [2, 0], [3, 0]], [M, L, L, L]), [0, 0, 1.5]),
        (pth([[3, 0], [2, 0], [1, 0], [0, 0]]), [0, 0, 1.5]),
        (QUAD_OPEN, [0, 0, 1.0]), (CUBIC_S, [2, 0, 1.0]), (MIXED_OK, [0, 0, 2.5]),
        (pth([[0, 0], [1, 1], [2, 2], [3, 3]], [M, L, L, L]), [3, 3, 2]),
    ]
    for path, circle in cases:
        for reorder in (False, True):
            probes.probe(op, _split_path_inout, path=path, circle=circle, tolerance=0.01, reorder_inout=reorder,
                         tol=BISECT_TOL)
    probes.probe(op, _split_path_inout, path=QUAD_OPEN, circle=[0, 0, 1.0], tolerance=1e-6, reorder_inout=False,
                 tol=BISECT_TOL)
    assert isinstance(probes.probe(op, _split_path_inout, path=pth([[0, 0], [1, 0]]), circle=[10, 10, 1],
                                   tolerance=0.01, reorder_inout=False), ValueError)


def test_bezier_helpers():
    op = "bezier.get_parallels"
    for b2, width in (([[0, 0], [1, 1], [2, 0]], 0.1), ([[0, 0], [1, 1], [2, 0]], 0.0),
                      ([[0, 0], [1, 0], [2, 0]], 0.5), ([[0, 0], [1, 0], [0.5, 0]], 0.5),
                      ([[0, 0], [0, 1], [1, 1]], 0.25), ([[1, 2], [3, 5], [6, 1]], 1.0),
                      ([[0, 0], [0, 0], [1, 1]], 0.2)):
        probes.probe(op, lambda bezier2, width: mbezier.get_parallels(bezier2, width), bezier2=b2, width=width)
    op = "bezier.make_wedged_bezier2"
    for b2, width, w1, wm, w2 in (([[0, 0], [1, 1], [2, 0]], 0.2, 1.0, 0.5, 0.0),
                                  ([[0, 0], [1, 1], [2, 0]], 0.2, 1.0, 1.0, 1.0),
                                  ([[1, 2], [3, 5], [6, 1]], 1.0, 0.5, 0.3, 0.1),
                                  ([[0, 0], [1, 0], [2, 0]], 0.5, 1.0, 0.5, 0.0),
                                  ([[0, 0], [0, 0], [1, 1]], 0.2, 1.0, 0.5, 0.0)):
        probes.probe(op, lambda bezier2, width, w1, wm, w2: mbezier.make_wedged_bezier2(bezier2, width, w1, wm, w2),
                     bezier2=b2, width=width, w1=w1, wm=wm, w2=w2)
    op = "bezier.get_cos_sin"
    for x0, y0, x1, y1 in ((0, 0, 1, 0), (0, 0, 3, 4), (1, 1, 1, 1), (2, 3, -1, -1), (0, 0, 0, -2)):
        probes.probe(op, lambda x0, y0, x1, y1: mbezier.get_cos_sin(x0, y0, x1, y1), x0=x0, y0=y0, x1=x1, y1=y1)
    assert mbezier.get_cos_sin(1, 1, 1, 1) == (0.0, 0.0)
    op = "bezier.check_if_parallel"
    for dx1, dy1, dx2, dy2, tolerance in ((1, 0, 2, 0, 1e-5), (1, 0, -2, 0, 1e-5), (1, 0, 0, 1, 1e-5),
                                          (1, 1, 2, 2.00001, 1e-5), (1, 1, 2, 2.001, 1e-5), (1, 1, 2, 2.001, 0.01),
                                          (0, 0, 0, 0, 1e-5), (0, 1, 0, -1, 1e-5), (-1, 1e-20, -1, -1e-20, 1e-5)):
        probes.probe(op, lambda dx1, dy1, dx2, dy2, tolerance: mbezier.check_if_parallel(dx1, dy1, dx2, dy2,
                                                                                         tolerance),
                     dx1=dx1, dy1=dy1, dx2=dx2, dy2=dy2, tolerance=tolerance)
    assert mbezier.check_if_parallel(1, 0, 2, 0) == 1 and mbezier.check_if_parallel(1, 0, -2, 0) == -1
    op = "bezier.inside_circle"
    for cx, cy, r, xy in ((0, 0, 1, [0.5, 0.5]), (0, 0, 1, [1, 0]), (0, 0, 1, [0.6, 0.8]), (1, 1, 2, [3, 1]),
                          (1, 1, 2, [2.9, 1]), (0, 0, 0, [0, 0]), (0, 0, -1, [0.5, 0])):
        probes.probe(op, lambda cx, cy, r, xy: mbezier.inside_circle(cx, cy, r)(xy), cx=cx, cy=cy, r=r, xy=xy)
    op = "bezier.get_intersection"
    s2 = math.sqrt(0.5)
    for args in ((0, 0, 1, 0, 1, -1, 0, 1), (0, 0, s2, s2, 2, 0, -s2, s2), (1, 2, 0.6, 0.8, -3, 4, 0.8, -0.6),
                 (0, 0, 1, 0, 0, 1, 1, 0), (0, 0, 1, 0, 0, 1, -1, 0), (0, 0, 1, 0, 5, 5, 1, 1e-13)):
        keys = ("cx1", "cy1", "cos_t1", "sin_t1", "cx2", "cy2", "cos_t2", "sin_t2")
        probes.probe(op, lambda **kw: mbezier.get_intersection(*[kw[k] for k in keys]), **dict(zip(keys, args)))
    assert isinstance(probes.probe(op, lambda **kw: mbezier.get_intersection(*[kw[k] for k in keys]),
                                   cx1=0, cy1=0, cos_t1=1, sin_t1=0, cx2=0, cy2=1, cos_t2=1, sin_t2=0), ValueError)
    op = "bezier.get_normal_points"
    for cx, cy, c, s, length in ((0, 0, 1, 0, 1), (1, 2, 0.6, 0.8, 0.5), (1, 2, 0.6, 0.8, 0.0), (0, 0, 0, 0, 1),
                                 (3, -1, -s2, s2, -2)):
        probes.probe(op, lambda cx, cy, cos_t, sin_t, length: mbezier.get_normal_points(cx, cy, cos_t, sin_t, length),
                     cx=cx, cy=cy, cos_t=c, sin_t=s, length=length)
    op = "bezier.find_control_points"
    for a in ((0, 0, 1, 1, 2, 0), (0, 0, 1, 0.5, 2, 2), (-1, 3, 0.25, 0.75, 4, -2)):
        keys2 = ("c1x", "c1y", "mmx", "mmy", "c2x", "c2y")
        probes.probe(op, lambda **kw: mbezier.find_control_points(*[kw[k] for k in keys2]), **dict(zip(keys2, a)))
    assert mbezier.find_control_points(0, 0, 1, 1, 2, 0)[1] == (1.0, 2.0)
