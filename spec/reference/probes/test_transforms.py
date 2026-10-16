"""Affine transforms, bounding boxes and interval helpers (matplotlib.transforms).

Shared input encodings
======================

``ops`` (affine operation list)
    A list of operations applied *in order* to a fresh ``Affine2D()``
    (identity), or to ``Affine2D(matrix)`` when a ``matrix`` arg is also
    given.  Every operation *post-multiplies* in the sense of matplotlib's
    chaining API: ``M <- Op @ M`` (the operation is applied *after* the ones
    before it).  Exact arithmetic (bit-for-bit, from matplotlib's source),
    with ``m`` the current matrix ``[[a, c, e], [b, d, f], [0, 0, 1]]`` and
    all right-hand sides using the *old* values:

    * ``["rotate", theta]`` (radians): ``k = cos(theta)``, ``s = sin(theta)``;
      for each column j: ``m[0][j] = k*m[0][j] - s*m[1][j]``,
      ``m[1][j] = s*m[0][j] + k*m[1][j]``.  A non-finite theta raises
      ``ValueError`` (Python ``math.cos``); NaN gives NaN entries.
    * ``["rotate_deg", deg]``: ``rotate(deg * (pi / 180))`` (``math.radians``).
    * ``["rotate_around", x, y, theta]``: ``translate(-x, -y)``,
      ``rotate(theta)``, ``translate(x, y)``.
    * ``["rotate_deg_around", x, y, deg]``: same with ``rotate_deg``.
    * ``["scale", s]`` or ``["scale", sx, sy]``: row 0 (all three entries)
      ``*= sx``, row 1 ``*= sy`` (``sy`` defaults to ``sx``).  Note
      ``0 * inf = NaN`` for the off-diagonal entries when scaling by inf.
    * ``["translate", tx, ty]``: ``m[0][2] += tx``, ``m[1][2] += ty``.
    * ``["skew", xs, ys]`` (radians): ``rx = tan(xs)``, ``ry = tan(ys)``;
      ``m[0][j] = m[0][j] + rx*m[1][j]``, ``m[1][j] = m[1][j] + ry*m[0][j]``.
    * ``["skew_deg", xs, ys]``: ``skew`` with both angles via ``math.radians``.
    * ``["clear"]``: reset to the identity.
    * ``["set_matrix", [[a, c, e], [b, d, f], [0, 0, 1]]]``: replace matrix.

Affine application to points (``affine_transform``, C++), bit-exact::

    x' = (a*x + c*y) + e
    y' = (b*x + d*y) + f

so e.g. the identity ``Affine2D`` maps ``(inf, 0)`` to ``(inf, NaN)``
(``0*inf``) and ``(NaN, 1)`` to ``(NaN, NaN)``.

``points``: ``[[x, y], ...]`` (an (N, 2) array; an empty list ``[]`` is
passed to matplotlib as an empty (0, 2) array and yields ``[]``).
``bbox``/``boxin``/``container``/...: ``[[x0, y0], [x1, y1]]`` passed to
``Bbox(points)`` (no reordering; "inverted" boxes have x0 > x1 or y0 > y1).
Matrices are returned as 3x3 nested lists (``get_matrix()``), bboxes as
``[[x0, y0], [x1, y1]]`` (``get_points()``), points as ``[[x, y], ...]``.
Optional args are *omitted* from ``args`` when not given (matplotlib default
applies).  Errors are recorded as the Python exception type name.

Ops
===

Affine2D
--------
``transforms.Affine2D.get_matrix`` {ops, matrix?} -> 3x3 matrix.
``transforms.Affine2D.from_values`` {a, b, c, d, e, f} -> 3x3 matrix
    ``[[a, c, e], [b, d, f], [0, 0, 1]]``.
``transforms.Affine2D.to_values`` {ops} -> ``[a, b, c, d, e, f]``.
``transforms.Affine2D.transform`` {ops, matrix?, points} -> points (formula
    above).  ``points`` may also be a single ``[x, y]`` (1-D in, 1-D out).
    A row length other than 2 (e.g. ``[[1, 2, 3]]``) raises ``ValueError``.
``transforms.Affine2D.transform_point`` {ops, point} -> ``[x, y]``;
    ``len(point) != 2`` raises ``ValueError``.
``transforms.Affine2D.is_separable`` {ops, matrix?} -> bool,
    ``m[0][1] == 0 and m[1][0] == 0`` (exact; rotate_deg(90) is NOT separable
    because cos(pi/2) = 6.1e-17).
``transforms.Affine2D.inverted`` {ops, matrix?} -> inverse 3x3 matrix
    (``numpy.linalg.inv`` of the full 3x3 matrix, LAPACK LU; recorded with
    ``tol`` -- the closed form ``det = a*d - b*c``, ``[[d, -c], [-b, a]] /
    det``, ``t' = -(A^-1 t)`` is within tol for every recorded case).
    Singular matrices raise ``LinAlgError`` (LAPACK exact-zero pivot; every
    recorded singular case has ``a*d - b*c == 0`` exactly).
``transforms.Affine2D.__add__`` {chain, points} -> {"matrix", "transformed",
    "is_separable"}: ``chain`` is a list of ``ops`` lists, composed left to
    right as ``T0 + T1 + ...`` ("T0 first, then T1"), i.e. matrix
    ``M_{n-1} @ ... @ M1 @ M0``.  ``transformed`` applies the *composed*
    matrix to ``points`` in one affine step (not sequentially).  The matrix
    product is ``numpy.dot`` (OpenBLAS), which on the reference machine
    computes ``C[i][j] = fma(B[i][2], A[2][j], fma(B[i][1], A[1][j],
    B[i][0]*A[0][j]))`` for ``C = B @ A`` (verified empirically); the naive
    ``(B[i][0]*A[0][j] + B[i][1]*A[1][j]) + B[i][2]*A[2][j]`` is within the
    recorded ``tol`` for every case, so either implementation passes.
``transforms.Affine2D.frozen`` {ops, post_ops} -> {"frozen", "original"}:
    matrix of ``t.frozen()`` taken after ``ops``, and the matrix of ``t``
    after further applying ``post_ops`` to ``t`` (the frozen copy is
    independent).
``transforms.IdentityTransform.transform`` {points} -> points unchanged
    (no arithmetic: NaN/inf are preserved, unlike an identity Affine2D).

Bbox construction and properties
--------------------------------
``transforms.Bbox.new`` {points} -> points; shape other than (2, 2) raises
    ``ValueError``.
``transforms.Bbox.from_bounds`` {x0, y0, width, height} -> points
    ``[[x0, y0], [x0 + width, y0 + height]]``.
``transforms.Bbox.from_extents`` {extents, minpos?} -> {"points", "minpos"};
    ``extents`` = ``[left, bottom, right, top]`` (the positional args); a
    count other than 4 raises ``ValueError``; ``minpos`` (scalar) sets both
    minpos components, default ``[inf, inf]``.
``transforms.Bbox.null`` {} -> ``[[inf, inf], [-inf, -inf]]``.
``transforms.Bbox.unit`` {} -> ``[[0, 0], [1, 1]]``.
``transforms.Bbox.properties`` {bbox} -> dict with keys x0, y0, x1, y1
    (raw), xmin/xmax/ymin/ymax (numpy min/max of the two coordinates; NaN
    propagates), width/height (signed ``x1 - x0``/``y1 - y0``), size
    ``[width, height]``, bounds ``[x0, y0, width, height]``, extents
    ``[x0, y0, x1, y1]``, min ``[xmin, ymin]``, max ``[xmax, ymax]``, p0
    ``[x0, y0]``, p1 ``[x1, y1]``, intervalx ``[x0, x1]``, intervaly
    ``[y0, y1]``, minpos (``[inf, inf]`` for a fresh Bbox).
``transforms.Bbox._is_finite`` {bbox} -> ``(width > 0 or height > 0) and
    width < inf and height < inf`` (signed width/height).

Bbox predicates
---------------
``transforms.Bbox.containsx`` {bbox, x}: ``x0 <= x <= x1 or x0 >= x >= x1``.
``transforms.Bbox.containsy`` {bbox, y}: same with y.
``transforms.Bbox.contains`` {bbox, x, y}: containsx and containsy.
``transforms.Bbox.fully_containsx`` {bbox, x}: ``x0 < x < x1 or x0 > x > x1``.
``transforms.Bbox.fully_containsy`` {bbox, y}; ``transforms.Bbox.fully_contains``
    {bbox, x, y}: open-interval versions.
``transforms.Bbox.overlaps`` {bbox, other}: after sorting each box's
    coordinates, ``ax1 <= bx2 and bx1 <= ax2 and ay1 <= by2 and by1 <= ay2``
    (touching edges overlap).
``transforms.Bbox.fully_overlaps`` {bbox, other}: same with strict ``<``.
``transforms.Bbox.count_contains`` {bbox, vertices} -> int: number of
    vertices with ``min < v < max`` strictly in both coordinates (min/max =
    the bbox's sorted corners); non-finite vertices never count; empty -> 0.
``transforms.Bbox.count_overlaps`` {bbox, bboxes} -> int: number of boxes
    in ``bboxes`` (list of bbox points) that overlap ``bbox`` *strictly*
    (both sorted; a box counts unless ``b.x2 <= a.x1 or b.y2 <= a.y1 or
    b.x1 >= a.x2 or b.y1 >= a.y2``), so touching edges do NOT count; since
    comparisons with NaN are false, a box containing NaN DOES count, and a
    null box (sorted to [-inf, inf]) counts too.

Bbox derived boxes (all return points)
--------------------------------------
``transforms.Bbox.union`` {bboxes}: ``[[min xmin, min ymin], [max xmax, max
    ymax]]`` (numpy min/max, NaN propagates); empty list -> ``ValueError``.
    A null box in the union yields the full plane (null has xmin = -inf...).
``transforms.Bbox.intersection`` {bbox1, bbox2}: ``x0 = max(xmin1, xmin2)``,
    ``x1 = min(xmax1, xmax2)``, same for y (numpy maximum/minimum: NaN
    propagates); returns ``[[x0, y0], [x1, y1]]`` if ``x0 <= x1 and y0 <=
    y1`` else ``null`` (JSON null).  Touching boxes give a degenerate box.
``transforms.Bbox.padded`` {bbox, w_pad, h_pad?}: ``[[x0 - w, y0 - h],
    [x1 + w, y1 + h]]`` (``h_pad`` defaults to ``w_pad``).
``transforms.Bbox.expanded`` {bbox, sw, sh}: ``dw = (sw*width - width)/2``,
    ``dh = (sh*height - height)/2``; ``[[x0 - dw, y0 - dh], [x1 + dw, y1 + dh]]``.
``transforms.Bbox.shrunk`` {bbox, mx, my}: ``[[x0, y0], [x0 + mx*width,
    y0 + my*height]]``.
``transforms.Bbox.shrunk_to_aspect`` {bbox, box_aspect, container?,
    fig_aspect?}: ``(w, h) = container.size`` (container defaults to bbox,
    fig_aspect to 1.0); ``H = w * box_aspect / fig_aspect`` (left to right);
    if ``H <= h``: ``W = w`` else ``W = h * fig_aspect / box_aspect``,
    ``H = h``; result ``[[x0, y0], [x0 + W, y0 + H]]``.  ``box_aspect <= 0``
    or ``fig_aspect <= 0`` raises ``ValueError``.
``transforms.Bbox.anchored`` {bbox, c, container}: ``c`` is an anchor code
    (C=(.5,.5) SW=(0,0) S=(.5,0) SE=(1,0) E=(1,.5) NE=(1,1) N=(.5,1)
    NW=(0,1) W=(0,.5)) or an ``[cx, cy]`` pair; with ``(l, b, w, h)`` the
    container bounds and ``(L, B, W, H)`` the bbox bounds the box is
    translated by ``[(l + cx*(w - W)) - L, (b + cy*(h - H)) - B]`` (added to
    both points).  Unknown codes raise ``KeyError``.
``transforms.Bbox.splitx`` {bbox, fractions} -> list of bboxes: with
    ``f = [0, *fractions, 1]``, ``w = x1 - x0``: boxes
    ``[[x0 + f[i]*w, y0], [x0 + f[i+1]*w, y1]]`` for consecutive pairs.
``transforms.Bbox.splity`` {bbox, fractions}: same along y.
``transforms.Bbox.translated`` {bbox, tx, ty}: both points + (tx, ty).
``transforms.Bbox.transformed`` {bbox, ops}: transforms only three corners
    ``ll = T(x0, y0)``, ``ul = T(x0, y1)``, ``lr = T(x1, y0)`` and returns
    ``[ll, [lr.x, ul.y]]`` -- NOT a bounding box under rotation.
``transforms.Bbox.rotated`` {bbox, radians}: corners (see below) rotated by
    ``Affine2D().rotate(radians)``, then the bounding box of the finite
    rotated corners (``update_from_data_xy(ignore=True)`` on a unit box).
``transforms.Bbox.corners`` {bbox}: ``[[x0, y0], [x0, y1], [x1, y0], [x1, y1]]``.
``transforms.TransformedBbox`` {bbox, ops}: transforms the four corners
    ``(x0,y0), (x1,y0), (x0,y1), (x1,y1)`` (in that order), then
    ``xs = (min(xs), max(xs))``, reversed if the input had ``x0 > x1``
    (likewise y), result ``[[xs0, ys0], [xs1, ys1]]``.  min/max are
    Python builtins (sequential scan keeping the current value unless
    ``candidate < current`` / ``candidate > current``; NaN-order dependent).

Bbox mutation
-------------
``transforms.Bbox.update`` {bbox, steps, minpos?} -> {"points", "minpos",
    "mutatedx", "mutatedy"}: start from ``Bbox(bbox)`` (optional initial
    ``minpos`` [mx, my], default [inf, inf]; ignore flag initially True) and
    apply each step dict in order:
      * ``{"method": "update_from_data_xy", "xy", "ignore"?, "updatex"?,
        "updatey"?}``
      * ``{"method": "update_from_data_x", "x", "ignore"?}`` (= xy update with
        updatey False)
      * ``{"method": "update_from_data_y", "y", "ignore"?}``
      * ``{"method": "update_from_path", "path", "ignore"?, "updatex"?,
        "updatey"?}`` (all vertices, codes ignored)
      * ``{"method": "ignore", "value": bool}`` (sets the default for
        ``ignore=None``).
    Update semantics (``update_from_path``): empty input or
    ``not (updatex or updatey)`` -> no-op.  ``ignore`` None -> the stored
    flag.  If ignore: start from points ``[[inf, inf], [-inf, -inf]]`` and
    minpos ``[inf, inf]``, else from the current ones.  Only vertices with
    BOTH coordinates finite are used.  For each updated axis:
    ``lo = min(lo, min(v))``, ``hi = max(hi, max(v))``, ``minpos =
    min(minpos, smallest v > 0)``.  Only the updated axes are written back
    (so ``ignore=True`` with ``updatey=False`` keeps the old y values).
    mutatedx/mutatedy: whether x0/x1 (y0/y1) differ from the initial points.
``transforms.Bbox.frozen`` {bbox, steps} -> {"points", "minpos"} of
    ``frozen()`` taken after ``steps`` (copies points and minpos).

Bbox-driven transforms (all return {"matrix", "transformed"})
-------------------------------------------------------------
``transforms.BboxTransform`` {boxin, boxout, points}: with bounds
    ``(inl, inb, inw, inh)``/``(outl, outb, outw, outh)``:
    ``sx = outw/inw``, ``sy = outh/inh``; matrix
    ``[[sx, 0, (-inl)*sx + outl], [0, sy, (-inb)*sy + outb], [0, 0, 1]]``
    (IEEE division: degenerate boxes give inf/NaN, no error).
``transforms.BboxTransformTo`` {boxout, points}: ``[[outw, 0, outl],
    [0, outh, outb], [0, 0, 1]]``.
``transforms.BboxTransformFrom`` {boxin, points}: ``sx = 1.0/inw``,
    ``sy = 1.0/inh``; ``[[sx, 0, (-inl)*sx], [0, sy, (-inb)*sy], [0, 0, 1]]``.

Other transforms (all return {"matrix", "transformed", ...})
------------------------------------------------------------
``transforms.blended_transform_factory`` {x_ops, y_ops, points} ->
    {"matrix", "transformed", "is_separable"}: matrix rows
    ``[x_m[0], y_m[1], [0, 0, 1]]``.  Both children are Affine2D; if either
    is not separable (``m[0][1] != 0 or m[1][0] != 0``) -> ``ValueError``.
``transforms.ScaledTranslation`` {xt, yt, scale_ops, points}: ``(tx, ty) =
    S.transform([xt, yt])`` with ``S`` built from ``scale_ops`` (the vector
    is transformed as a *point*, so a translation in S adds); matrix
    ``[[1, 0, tx], [0, 1, ty], [0, 0, 1]]``.  A figure's ``dpi_scale_trans``
    is ``[["scale", dpi]]``.
``transforms.offset_copy`` {base_ops, x, y, units?, dpi, points}: the base
    transform followed by an offset: units 'dots' -> ``(x, y)``; 'points'
    -> ``ScaledTranslation(x/72.0, y/72.0, dpi_scale_trans)``, i.e.
    ``dpi*(x/72.0)``; 'inches' (default) -> ``dpi*x``.  ``dpi`` null means
    no figure: allowed for 'dots', ``ValueError`` otherwise; unknown units
    -> ``ValueError``.  matrix = ``Translate @ base`` (adds the offset to
    e, f; exact).

Scalar helpers
--------------
``transforms.nonsingular`` {vmin, vmax, expander?, tiny?, increasing?} ->
    ``[vmin, vmax]`` (defaults 0.001, 1e-15, True).  Algorithm: any
    non-finite input -> ``[-expander, expander]``; swap if ``vmax < vmin``
    (remember swapped); ``M = max(|vmin|, |vmax|)``; if
    ``M < (1e6 / tiny) * 2.2250738585072014e-308`` -> ``[-expander,
    expander]``; elif ``vmax - vmin <= M * tiny``: if both are 0 ->
    ``[-expander, expander]`` else ``vmin -= expander*|vmin|``,
    ``vmax += expander*|vmax|``; finally swap back if swapped and not
    increasing.  (The public function is deprecated in 3.11; probes call
    the identical ``_nonsingular`` used internally.)
``transforms.interval_contains`` {interval, val}: sort ``(a, b)``, then
    ``a <= val <= b``.
``transforms.interval_contains_open`` {interval, val}: ``a < val < b or
    a > val > b`` (unsorted).
``transforms.interval_contains_close`` {interval, val, rtol?}: sort, ``r =
    (b - a) * rtol`` (default rtol 1e-10), ``a - r <= val <= b + r``.
"""

import math

import numpy as np
import pytest

import matplotlib.transforms as mtransforms
from matplotlib.figure import Figure
from matplotlib.path import Path

from spec_support import ProbeSet

probes = ProbeSet("transforms")

# Non-finite inputs are part of the spec; numpy's RuntimeWarnings are noise.
pytestmark = pytest.mark.filterwarnings("ignore::RuntimeWarning")

INF = math.inf
NAN = math.nan

# Relative tolerances for outputs that go through numpy BLAS/LAPACK.
COMPOSE_TOL = 1e-14
INVERSE_TOL = 1e-12


# ---------------------------------------------------------------------------
# adapters
# ---------------------------------------------------------------------------

_SINGLE = {
    "rotate": 1,
    "rotate_deg": 1,
    "rotate_around": 3,
    "rotate_deg_around": 3,
    "translate": 2,
    "skew": 2,
    "skew_deg": 2,
    "clear": 0,
}


def _affine(ops, matrix=None):
    t = mtransforms.Affine2D(None if matrix is None else np.array(matrix, float))
    for op in ops:
        name, params = op[0], op[1:]
        if name == "set_matrix":
            (m,) = params
            t.set_matrix(np.array(m, float))
            continue
        if name == "scale":
            assert len(params) in (1, 2), op
        else:
            assert len(params) == _SINGLE[name], op
        getattr(t, name)(*params)
    return t


def _pts(points):
    arr = np.asarray(points, float)
    if arr.size == 0:
        arr = arr.reshape(0, 2)
    return arr


def _bbox(points):
    return mtransforms.Bbox(points)


def _bb_out(bbox):
    return None if bbox is None else bbox.get_points()


def _errstate(fn):
    def wrapper(*args, **kwargs):
        with np.errstate(all="ignore"):
            return fn(*args, **kwargs)

    return wrapper


@_errstate
def affine_get_matrix(ops, matrix=None):
    return _affine(ops, matrix).get_matrix()


@_errstate
def affine_transform(ops, points, matrix=None):
    return _affine(ops, matrix).transform(_pts(points))


@_errstate
def affine_transform_point(ops, point):
    return _affine(ops).transform_point(np.asarray(point, float))


def affine_is_separable(ops, matrix=None):
    return bool(_affine(ops, matrix).is_separable)


def affine_inverted(ops, matrix=None):
    return _affine(ops, matrix).inverted().get_matrix()


@_errstate
def affine_add(chain, points):
    transforms = [_affine(ops) for ops in chain]
    total = transforms[0]
    for t in transforms[1:]:
        total = total + t
    return {
        "matrix": total.get_matrix(),
        "transformed": total.transform(_pts(points)),
        "is_separable": bool(total.is_separable),
    }


def affine_frozen(ops, post_ops):
    t = _affine(ops)
    frozen = t.frozen()
    for op in post_ops:
        getattr(t, op[0])(*op[1:])
    return {"frozen": frozen.get_matrix(), "original": t.get_matrix()}


def _matrix_and_points(t, points):
    return {"matrix": t.get_matrix(), "transformed": t.transform(_pts(points))}


# ---------------------------------------------------------------------------
# reference arithmetic used for hand-written assertions
# ---------------------------------------------------------------------------

def _naive_apply(m, points):
    return [[(m[0][0] * x + m[0][1] * y) + m[0][2], (m[1][0] * x + m[1][1] * y) + m[1][2]]
            for x, y in points]


def _naive_matmul(b, a):
    return [[(b[i][0] * a[0][j] + b[i][1] * a[1][j]) + b[i][2] * a[2][j] for j in range(3)]
            for i in range(3)]


def _naive_inverse(m):
    (a, c, e), (b, d, f), _ = m
    det = a * d - b * c
    ia, ic, ib, id_ = d / det, -c / det, -b / det, a / det
    return [[ia, ic, -(ia * e + ic * f)], [ib, id_, -(ib * e + id_ * f)], [0.0, 0.0, 1.0]]


def _assert_rel_close(actual, expected, tol):
    """Strict per-entry relative check (zero must match zero exactly)."""
    a = np.asarray(actual, float)
    e = np.asarray(expected, float)
    assert a.shape == e.shape
    for x, y in zip(a.ravel(), e.ravel()):
        if math.isnan(x) or math.isnan(y):
            assert math.isnan(x) and math.isnan(y)
        elif x != y:
            assert abs(x - y) <= tol * max(abs(x), abs(y)), (x, y)


def _same(a, b):
    """Exact equality with NaN == NaN."""
    a = np.asarray(a, float)
    b = np.asarray(b, float)
    return a.shape == b.shape and bool(np.all((a == b) | (np.isnan(a) & np.isnan(b))))


# ---------------------------------------------------------------------------
# Affine2D
# ---------------------------------------------------------------------------

AFFINE_OPS = [
    [],
    [["rotate", 0.0]],
    [["rotate", 0.5]],
    [["rotate", -1.25]],
    [["rotate", math.pi / 6]],
    [["rotate", math.pi]],
    [["rotate", 10.0]],
    [["rotate", NAN]],
    [["rotate_deg", 30]],
    [["rotate_deg", 45]],
    [["rotate_deg", 90]],
    [["rotate_deg", 180]],
    [["rotate_deg", 270]],
    [["rotate_deg", 360]],
    [["rotate_deg", -45]],
    [["rotate_deg", 720.5]],
    [["rotate_around", 1, 2, 0.5]],
    [["rotate_around", -3.5, 0.25, -2.0]],
    [["rotate_deg_around", 1, 2, 90]],
    [["rotate_deg_around", 10, -5, 33.3]],
    [["scale", 2]],
    [["scale", 2, 3]],
    [["scale", -1, 1]],
    [["scale", 0.5, -0.25]],
    [["scale", 0]],
    [["scale", 1e-300, 1e300]],
    [["scale", INF]],
    [["scale", 1, NAN]],
    [["translate", 1, 0]],
    [["translate", -2.5, 7.25]],
    [["translate", INF, 0]],
    [["skew", 0.1, 0.2]],
    [["skew", 0.0, -0.5]],
    [["skew_deg", 30, 0]],
    [["skew_deg", 0, 45]],
    [["skew_deg", 15, -20]],
    [["skew_deg", 90, 0]],
    [["rotate_deg", 30], ["scale", 2, 3], ["translate", 1, 0]],
    [["translate", 1, 0], ["scale", 2, 3], ["rotate_deg", 30]],
    [["scale", 2, 3], ["rotate_deg", 30], ["translate", 1, 0]],
    [["translate", 1, 2], ["rotate", 0.7]],
    [["rotate", 0.7], ["translate", 1, 2]],
    [["scale", 2], ["translate", 3, 4]],
    [["translate", 3, 4], ["scale", 2]],
    [["skew_deg", 10, 20], ["rotate_deg", 15], ["scale", 1.5, 0.5], ["translate", -1, 3]],
    [["translate", 2, 3], ["skew", 0.3, -0.4]],
    [["rotate", 0.3], ["rotate", 0.4]],
    [["rotate", 0.7]],
    [["rotate_deg", 30], ["rotate_deg", -30]],
    [["scale", 3, 5], ["clear"]],
    [["scale", 3, 5], ["clear"], ["translate", 1, 1]],
    [["rotate_deg", 10], ["set_matrix", [[1, 2, 3], [4, 5, 6], [0, 0, 1]]], ["scale", 2]],
    [["set_matrix", [[2, 0.5, -1], [0.25, 3, 4], [0, 0, 1]]]],
]


def test_affine_get_matrix():
    for ops in AFFINE_OPS:
        probes.probe("transforms.Affine2D.get_matrix", affine_get_matrix, ops=ops)

    # starting from an explicit matrix
    base = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [0.0, 0.0, 1.0]]
    for ops in ([], [["scale", 2, 3]], [["rotate_deg", 90]], [["translate", 1, -1]],
                [["skew", 0.25, 0.5]]):
        probes.probe("transforms.Affine2D.get_matrix", affine_get_matrix, ops=ops, matrix=base)

    m = probes.probe("transforms.Affine2D.get_matrix", affine_get_matrix, ops=[])
    assert m.tolist() == np.eye(3).tolist()
    m = affine_get_matrix(ops=[["scale", 2, 3], ["translate", 1, 0]])
    assert m.tolist() == [[2, 0, 1], [0, 3, 0], [0, 0, 1]]
    m = affine_get_matrix(ops=[["translate", 1, 0], ["scale", 2, 3]])
    assert m.tolist() == [[2, 0, 2], [0, 3, 0], [0, 0, 1]]
    # each op left-multiplies
    a = affine_get_matrix(ops=[["rotate", 0.7], ["translate", 1, 2]])
    assert np.allclose(a, np.array([[1, 0, 1], [0, 1, 2], [0, 0, 1]]) @ affine_get_matrix(ops=[["rotate", 0.7]]))
    # rotate_deg goes through math.radians
    assert _same(affine_get_matrix(ops=[["rotate_deg", 30]]),
                 affine_get_matrix(ops=[["rotate", math.radians(30)]]))
    # scale by inf: 0 * inf = NaN off the diagonal
    m = affine_get_matrix(ops=[["scale", INF]])
    assert m[0, 0] == INF and math.isnan(m[0, 1])


def test_affine_errors():
    for ops in ([["rotate", INF]], [["rotate_deg", -INF]], [["skew", INF, 0]],
                [["rotate_around", 0, 0, INF]]):
        r = probes.probe("transforms.Affine2D.get_matrix", affine_get_matrix, ops=ops)
        assert isinstance(r, ValueError)


def test_affine_from_values_and_to_values():
    for vals in ((1, 0, 0, 1, 0, 0), (1, 2, 3, 4, 5, 6), (0.5, -0.25, 1e-3, 2, -7, 8.5),
                 (NAN, 0, 0, INF, 1, 2)):
        a, b, c, d, e, f = vals
        m = probes.probe("transforms.Affine2D.from_values",
                         lambda **kw: mtransforms.Affine2D.from_values(**kw).get_matrix(),
                         a=a, b=b, c=c, d=d, e=e, f=f)
        assert _same(m, [[a, c, e], [b, d, f], [0, 0, 1]])
    for ops in ([], [["rotate_deg", 30], ["translate", 1, 2]], [["scale", 2, 3], ["skew", 0.1, 0.2]]):
        v = probes.probe("transforms.Affine2D.to_values", lambda ops: _affine(ops).to_values(), ops=ops)
        m = affine_get_matrix(ops=ops)
        assert list(v) == [m[0, 0], m[1, 0], m[0, 1], m[1, 1], m[0, 2], m[1, 2]]


POINTS = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [-2.5, 3.75], [1e6, -1e-6]]
NONFINITE_POINTS = [[NAN, 1.0], [1.0, NAN], [INF, 0.0], [0.0, -INF], [INF, INF], [NAN, NAN],
                    [2.0, 3.0]]


def test_affine_transform():
    cases = [
        [],
        [["scale", 2, 3]],
        [["rotate_deg", 30], ["scale", 2, 3], ["translate", 1, 0]],
        [["translate", 1, 0], ["scale", 2, 3], ["rotate_deg", 30]],
        [["rotate_deg_around", 1, 1, 90]],
        [["skew_deg", 15, -20], ["translate", 0.5, 0.5]],
        [["scale", -1, 1], ["translate", 10, 0]],
    ]
    for ops in cases:
        r = probes.probe("transforms.Affine2D.transform", affine_transform, ops=ops, points=POINTS)
        assert _same(r, _naive_apply(affine_get_matrix(ops=ops), POINTS))
        r = probes.probe("transforms.Affine2D.transform", affine_transform, ops=ops,
                         points=NONFINITE_POINTS)
        assert _same(r, _naive_apply(affine_get_matrix(ops=ops), NONFINITE_POINTS))

    # identity Affine2D still does arithmetic: 0 * inf = NaN, NaN contaminates
    r = affine_transform(ops=[], points=[[INF, 0.0], [NAN, 1.0]])
    assert r[0, 0] == INF and math.isnan(r[0, 1]) and math.isnan(r[1, 1])

    # empty inputs
    for ops in ([], [["rotate_deg", 30], ["translate", 1, 2]]):
        r = probes.probe("transforms.Affine2D.transform", affine_transform, ops=ops, points=[])
        assert r.shape == (0, 2)
    # a single 1-D point
    r = probes.probe("transforms.Affine2D.transform", affine_transform,
                     ops=[["rotate_deg", 90], ["translate", 1, 2]], points=[3.0, 4.0])
    assert r.shape == (2,)
    # explicit matrix
    probes.probe("transforms.Affine2D.transform", affine_transform, ops=[],
                 matrix=[[1, 2, 3], [4, 5, 6], [0, 0, 1]], points=POINTS)
    # bad shape
    r = probes.probe("transforms.Affine2D.transform", affine_transform, ops=[],
                     points=[[1.0, 2.0, 3.0]])
    assert isinstance(r, ValueError)

    for point in ([1.0, 2.0], [-3.0, 0.5], [NAN, 0.0]):
        probes.probe("transforms.Affine2D.transform_point", affine_transform_point,
                     ops=[["rotate_deg", 30], ["translate", 1, 0]], point=point)
    r = probes.probe("transforms.Affine2D.transform_point", affine_transform_point,
                     ops=[], point=[1.0, 2.0, 3.0])
    assert isinstance(r, ValueError)


def test_affine_is_separable():
    expect = {
        "[]": True,
        "[['rotate', 0.0]]": True,
        "[['rotate_deg', 90]]": False,
        "[['rotate_deg', 180]]": False,
        "[['scale', -1, 1]]": True,
        "[['skew', 0.1, 0.0]]": False,
    }
    cases = [
        [], [["rotate", 0.0]], [["rotate_deg", 90]], [["rotate_deg", 180]], [["rotate_deg", 360]],
        [["rotate_deg", 30], ["rotate_deg", -30]], [["scale", -1, 1]], [["scale", 2, 3], ["translate", 4, 5]],
        [["skew", 0.1, 0.0]], [["skew", 0.0, 0.1]], [["skew_deg", 0, 0]],
        [["rotate_deg_around", 1, 1, 0]],
    ]
    for ops in cases:
        r = probes.probe("transforms.Affine2D.is_separable", affine_is_separable, ops=ops)
        if str(ops) in expect:
            assert r is expect[str(ops)], ops
    probes.probe("transforms.Affine2D.is_separable", affine_is_separable, ops=[],
                 matrix=[[1, 0, 5], [0, 2, 6], [0, 0, 1]])
    probes.probe("transforms.Affine2D.is_separable", affine_is_separable, ops=[],
                 matrix=[[1, 0, 5], [1e-300, 2, 6], [0, 0, 1]])


def test_affine_inverted():
    cases = [
        [],
        [["scale", 2, 4]],
        [["scale", 3, -0.5]],
        [["translate", 1.5, -2.5]],
        [["scale", 2, 4], ["translate", 1, 3]],
        [["rotate_deg", 30]],
        [["rotate", 1.1], ["translate", 3, 7]],
        [["rotate_deg", 30], ["scale", 2, 3], ["translate", 1, 0.5]],
        [["skew", 0.3, 0.1], ["scale", 1.5, 2.5], ["translate", -4, 9]],
        [["rotate_deg_around", 2, 5, 45]],
        [["scale", 1e-5, 1e5], ["translate", 2, 3]],
    ]
    for ops in cases:
        r = probes.probe("transforms.Affine2D.inverted", affine_inverted, tol=INVERSE_TOL, ops=ops)
        _assert_rel_close(_naive_inverse(affine_get_matrix(ops=ops).tolist()), r, INVERSE_TOL)
        assert np.allclose(r @ affine_get_matrix(ops=ops), np.eye(3))
    r = probes.probe("transforms.Affine2D.inverted", affine_inverted, tol=INVERSE_TOL, ops=[],
                     matrix=[[2, 1, 3], [1, 3, -2], [0, 0, 1]])
    _assert_rel_close(_naive_inverse([[2, 1, 3], [1, 3, -2], [0, 0, 1]]), r, INVERSE_TOL)

    singular = [
        dict(ops=[["scale", 0]]),
        dict(ops=[["scale", 1, 0]]),
        dict(ops=[["scale", 0, 1], ["rotate_deg", 30]]),
        dict(ops=[], matrix=[[1, 2, 0], [2, 4, 0], [0, 0, 1]]),
        dict(ops=[], matrix=[[0, 0, 5], [0, 0, 6], [0, 0, 1]]),
    ]
    for kw in singular:
        r = probes.probe("transforms.Affine2D.inverted", affine_inverted, **kw)
        assert type(r).__name__ == "LinAlgError", kw


def test_affine_add():
    pts = [[0.0, 0.0], [1.0, 2.0], [-3.0, 0.5], [10.0, -7.0]]
    chains = [
        [[["scale", 2, 3]], [["translate", 1, 0]]],
        [[["translate", 1, 0]], [["scale", 2, 3]]],
        [[["rotate_deg", 30]], [["scale", 2, 3]], [["translate", 1, 0]]],
        [[["rotate", 0.4], ["translate", 1, 2]], [["skew", 0.2, 0.1], ["scale", 1.5, 0.75]]],
        [[["scale", 1.25, 0.8], ["translate", -2, 3]], [["rotate_deg", 50], ["translate", 0.5, 0.25]]],
        [[["translate", 5, 5]], []],
        [[], [["rotate_deg", 60]]],
        [[["rotate_deg", 10]], [["rotate_deg", 20]], [["rotate_deg", 30]]],
    ]
    for chain in chains:
        r = probes.probe("transforms.Affine2D.__add__", affine_add, tol=COMPOSE_TOL, chain=chain,
                         points=pts)
        mats = [affine_get_matrix(ops=ops).tolist() for ops in chain]
        expected = mats[0]
        for m in mats[1:]:
            expected = _naive_matmul(m, expected)
        _assert_rel_close(r["matrix"], expected, COMPOSE_TOL)
        _assert_rel_close(r["transformed"], _naive_apply(expected, pts), COMPOSE_TOL)
        # (A + B) applies A first: same as chaining the ops
        flat = [op for ops in chain for op in ops]
        assert np.allclose(r["matrix"], affine_get_matrix(ops=flat))


def test_affine_frozen():
    for ops, post in (([["scale", 2, 3]], [["translate", 1, 1]]),
                      ([["rotate_deg", 30], ["translate", 1, 2]], [["clear"]]),
                      ([], [["rotate", 0.5]])):
        r = probes.probe("transforms.Affine2D.frozen", affine_frozen, ops=ops, post_ops=post)
        assert _same(r["frozen"], affine_get_matrix(ops=ops))
        assert _same(r["original"], affine_get_matrix(ops=ops + post))


def test_identity_transform():
    for pts in (POINTS, NONFINITE_POINTS):
        r = probes.probe("transforms.IdentityTransform.transform",
                         lambda points: mtransforms.IdentityTransform().transform(_pts(points)),
                         points=pts)
        assert _same(r, pts)


# ---------------------------------------------------------------------------
# Bbox construction and properties
# ---------------------------------------------------------------------------

def bbox_properties(bbox):
    b = _bbox(bbox)
    return {
        "x0": b.x0, "y0": b.y0, "x1": b.x1, "y1": b.y1,
        "xmin": b.xmin, "xmax": b.xmax, "ymin": b.ymin, "ymax": b.ymax,
        "width": b.width, "height": b.height, "size": b.size,
        "bounds": b.bounds, "extents": b.extents,
        "min": b.min, "max": b.max, "p0": b.p0, "p1": b.p1,
        "intervalx": b.intervalx, "intervaly": b.intervaly,
        "minpos": b.minpos,
    }


BOXES = [
    [[0.0, 0.0], [1.0, 1.0]],
    [[1.0, 2.0], [4.0, 6.0]],
    [[4.0, 2.0], [1.0, 6.0]],      # inverted x
    [[1.0, 6.0], [4.0, 2.0]],      # inverted y
    [[4.0, 6.0], [1.0, 2.0]],      # inverted both
    [[2.0, 3.0], [2.0, 3.0]],      # point
    [[-1.5, 0.0], [2.5, 0.0]],     # horizontal line
    [[-3.0, -2.0], [-1.0, -0.5]],
    [[INF, INF], [-INF, -INF]],    # null
    [[NAN, 0.0], [1.0, 1.0]],
    [[0.0, -INF], [1.0, INF]],
]


def test_bbox_construction():
    for pts in ([[0, 0], [1, 1]], [[3, 4], [1, 2]], [[NAN, 1], [INF, -INF]]):
        r = probes.probe("transforms.Bbox.new", lambda points: _bbox(points).get_points(), points=pts)
        assert _same(r, pts)
    for pts in ([[1, 2, 3]], [1, 2, 3, 4], [[1, 2], [3, 4], [5, 6]]):
        r = probes.probe("transforms.Bbox.new", lambda points: _bbox(points).get_points(), points=pts)
        assert isinstance(r, ValueError)

    for x0, y0, w, h in ((0, 0, 1, 1), (1, 2, 3, 4), (1, 2, -3, -4), (0.1, 0.2, 0.7, 0.3),
                         (5, 5, 0, 0), (1e16, 0, 1, 1)):
        r = probes.probe("transforms.Bbox.from_bounds",
                         lambda **kw: mtransforms.Bbox.from_bounds(**kw).get_points(),
                         x0=x0, y0=y0, width=w, height=h)
        assert _same(r, [[x0, y0], [x0 + w, y0 + h]])

    def from_extents(extents, minpos=None):
        b = mtransforms.Bbox.from_extents(*extents, minpos=minpos)
        return {"points": b.get_points(), "minpos": b.minpos}

    for ext in ([1, 2, 3, 4], [3, 4, 1, 2], [0.5, -0.5, 0.5, -0.5]):
        r = probes.probe("transforms.Bbox.from_extents", from_extents, extents=ext)
        assert r["minpos"].tolist() == [INF, INF]
    r = probes.probe("transforms.Bbox.from_extents", from_extents, extents=[1, 2, 3, 4], minpos=0.5)
    assert r["minpos"].tolist() == [0.5, 0.5]
    for ext in ([1, 2, 3], [1, 2, 3, 4, 5]):
        assert isinstance(probes.probe("transforms.Bbox.from_extents", from_extents, extents=ext),
                          ValueError)

    r = probes.probe("transforms.Bbox.null", lambda: mtransforms.Bbox.null().get_points())
    assert r.tolist() == [[INF, INF], [-INF, -INF]]
    r = probes.probe("transforms.Bbox.unit", lambda: mtransforms.Bbox.unit().get_points())
    assert r.tolist() == [[0, 0], [1, 1]]


def test_bbox_properties():
    for bbox in BOXES:
        r = probes.probe("transforms.Bbox.properties", bbox_properties, bbox=bbox)
        assert r["width"] == bbox[1][0] - bbox[0][0] or math.isnan(r["width"])
    r = bbox_properties([[4.0, 6.0], [1.0, 2.0]])
    assert (r["width"], r["height"]) == (-3.0, -4.0)
    assert r["min"].tolist() == [1.0, 2.0] and r["max"].tolist() == [4.0, 6.0]
    assert r["p0"].tolist() == [4.0, 6.0]
    r = bbox_properties([[INF, INF], [-INF, -INF]])
    assert r["width"] == -INF and r["min"].tolist() == [-INF, -INF]

    for bbox in BOXES + [[[0, 0], [0, 1]], [[0, 0], [-1, 1]], [[0, 0], [-1, -1]]]:
        probes.probe("transforms.Bbox._is_finite", lambda bbox: bool(_bbox(bbox)._is_finite()),
                     bbox=bbox)


# ---------------------------------------------------------------------------
# Bbox predicates
# ---------------------------------------------------------------------------

def test_bbox_contains():
    boxes = [
        [[0.0, 0.0], [2.0, 1.0]],
        [[2.0, 1.0], [0.0, 0.0]],      # inverted
        [[1.0, 1.0], [1.0, 1.0]],      # point
        [[INF, INF], [-INF, -INF]],    # null
    ]
    values = [-1.0, 0.0, 0.5, 1.0, 1.5, 2.0, 3.0, NAN, INF]
    for bbox in boxes:
        b = _bbox(bbox)
        for v in values:
            for name in ("containsx", "containsy", "fully_containsx", "fully_containsy"):
                arg = "x" if name.endswith("x") else "y"
                r = probes.probe(f"transforms.Bbox.{name}",
                                 lambda bbox, _n=name, **kw: bool(getattr(_bbox(bbox), _n)(*kw.values())),
                                 bbox=bbox, **{arg: v})
                assert r == bool(getattr(b, name)(v))
        for x, y in ([0.0, 0.0], [1.0, 0.5], [2.0, 1.0], [0.5, 1.0], [1.0, 1.0], [-0.5, 0.5],
                     [1.0, 2.0], [NAN, 0.5]):
            for name in ("contains", "fully_contains"):
                probes.probe(f"transforms.Bbox.{name}",
                             lambda bbox, x, y, _n=name: bool(getattr(_bbox(bbox), _n)(x, y)),
                             bbox=bbox, x=x, y=y)
    unit = _bbox([[0, 0], [1, 1]])
    assert unit.contains(1, 1) and not unit.fully_contains(1, 1)
    assert _bbox([[1, 1], [0, 0]]).contains(0.5, 0.5)


def test_bbox_overlaps():
    base = [[0.0, 0.0], [2.0, 2.0]]
    others = [
        [[1.0, 1.0], [3.0, 3.0]],
        [[2.0, 0.0], [3.0, 2.0]],      # touching edge
        [[2.0, 2.0], [3.0, 3.0]],      # touching corner
        [[2.5, 0.0], [3.0, 2.0]],      # disjoint
        [[0.5, 0.5], [1.0, 1.0]],      # inside
        [[-1.0, -1.0], [5.0, 5.0]],    # containing
        [[3.0, 3.0], [1.0, 1.0]],      # inverted overlapping
        [[1.0, 1.0], [1.0, 1.0]],      # point inside
        [[INF, INF], [-INF, -INF]],    # null
        [[NAN, 0.0], [1.0, 1.0]],
    ]
    for other in others:
        for name in ("overlaps", "fully_overlaps"):
            probes.probe(f"transforms.Bbox.{name}",
                         lambda bbox, other, _n=name: bool(getattr(_bbox(bbox), _n)(_bbox(other))),
                         bbox=base, other=other)
            probes.probe(f"transforms.Bbox.{name}",
                         lambda bbox, other, _n=name: bool(getattr(_bbox(bbox), _n)(_bbox(other))),
                         bbox=[[2.0, 2.0], [0.0, 0.0]], other=other)
    b = _bbox(base)
    assert b.overlaps(_bbox(others[1])) and not b.fully_overlaps(_bbox(others[1]))

    def count_overlaps(bbox, bboxes):
        return int(_bbox(bbox).count_overlaps([_bbox(o) for o in bboxes]))

    for bboxes in ([], others[:1], others[:6], others[1:3], others[6:8], others):
        probes.probe("transforms.Bbox.count_overlaps", count_overlaps, bbox=base, bboxes=bboxes)
    assert count_overlaps(base, others[1:3]) == 0
    probes.probe("transforms.Bbox.count_overlaps", count_overlaps, bbox=[[2.0, 2.0], [0.0, 0.0]],
                 bboxes=others[:6])


def test_bbox_count_contains():
    def count_contains(bbox, vertices):
        return int(_bbox(bbox).count_contains(vertices))

    verts = [[0.5, 0.5], [0.0, 0.5], [1.0, 1.0], [0.999, 0.001], [1.5, 0.5], [NAN, 0.5],
             [INF, 0.5], [-0.5, -0.5], [0.25, 0.75]]
    for bbox in ([[0.0, 0.0], [1.0, 1.0]], [[1.0, 1.0], [0.0, 0.0]], [[-1.0, -1.0], [2.0, 2.0]],
                 [[0.5, 0.5], [0.5, 0.5]], [[INF, INF], [-INF, -INF]]):
        probes.probe("transforms.Bbox.count_contains", count_contains, bbox=bbox, vertices=verts)
    r = probes.probe("transforms.Bbox.count_contains", count_contains, bbox=[[0.0, 0.0], [1.0, 1.0]],
                     vertices=[])
    assert r == 0
    assert count_contains([[0.0, 0.0], [1.0, 1.0]], verts) == 3


# ---------------------------------------------------------------------------
# Bbox derived boxes
# ---------------------------------------------------------------------------

def bbox_union(bboxes):
    return mtransforms.Bbox.union([_bbox(b) for b in bboxes]).get_points()


def bbox_intersection(bbox1, bbox2):
    return _bb_out(mtransforms.Bbox.intersection(_bbox(bbox1), _bbox(bbox2)))


NULL = [[INF, INF], [-INF, -INF]]


def test_bbox_union_intersection():
    unions = [
        [[[0, 0], [1, 1]]],
        [[[0, 0], [1, 1]], [[2, 3], [4, 5]]],
        [[[0, 0], [1, 1]], [[0.5, -1], [0.75, 0.5]]],
        [[[1, 1], [0, 0]], [[3, 2], [2, 3]]],
        [[[0, 0], [0, 0]], NULL],
        [NULL],
        [NULL, NULL],
        [[[0, 0], [1, 1]], [[NAN, 0], [2, 2]]],
        [[[0, 0], [1, 1]], [[-INF, 0], [0, 0]]],
    ]
    for bboxes in unions:
        probes.probe("transforms.Bbox.union", bbox_union, bboxes=bboxes)
    assert isinstance(probes.probe("transforms.Bbox.union", bbox_union, bboxes=[]), ValueError)
    assert bbox_union([[[0, 0], [0, 0]], NULL]).tolist() == [[-INF, -INF], [INF, INF]]

    pairs = [
        ([[0, 0], [2, 2]], [[1, 1], [3, 3]]),
        ([[0, 0], [2, 2]], [[3, 3], [4, 4]]),         # disjoint
        ([[0, 0], [2, 2]], [[0.5, 3], [1, 4]]),       # disjoint in y only
        ([[0, 0], [2, 2]], [[2, 0], [3, 2]]),         # touching edge
        ([[0, 0], [2, 2]], [[2, 2], [3, 3]]),         # touching corner
        ([[0, 0], [2, 2]], [[0.5, 0.5], [1, 1]]),     # nested
        ([[2, 2], [0, 0]], [[3, 3], [1, 1]]),         # both inverted
        ([[0, 0], [2, 2]], NULL),
        (NULL, NULL),
        ([[0, 0], [2, 2]], [[NAN, 0], [1, 1]]),
        ([[0, 0], [2, 2]], [[-INF, -INF], [INF, 1]]),
    ]
    for b1, b2 in pairs:
        probes.probe("transforms.Bbox.intersection", bbox_intersection, bbox1=b1, bbox2=b2)
    assert bbox_intersection([[0, 0], [2, 2]], [[3, 3], [4, 4]]) is None
    assert bbox_intersection([[0, 0], [2, 2]], NULL).tolist() == [[0, 0], [2, 2]]
    assert bbox_intersection(NULL, NULL).tolist() == [[-INF, -INF], [INF, INF]]


def test_bbox_padded_expanded_shrunk_translated():
    def padded(bbox, w_pad, h_pad=None):
        return _bbox(bbox).padded(w_pad, h_pad).get_points()

    for bbox in ([[0, 0], [1, 1]], [[4, 6], [1, 2]], [[2, 2], [2, 2]]):
        probes.probe("transforms.Bbox.padded", padded, bbox=bbox, w_pad=0.5)
        probes.probe("transforms.Bbox.padded", padded, bbox=bbox, w_pad=0.5, h_pad=-0.25)
        probes.probe("transforms.Bbox.padded", padded, bbox=bbox, w_pad=-2.0)
    assert padded([[0, 0], [1, 1]], 0.5).tolist() == [[-0.5, -0.5], [1.5, 1.5]]

    def expanded(bbox, sw, sh):
        return _bbox(bbox).expanded(sw, sh).get_points()

    for bbox in ([[0, 0], [1, 1]], [[1, 2], [4, 6]], [[4, 6], [1, 2]], [[2, 2], [2, 2]], NULL):
        for sw, sh in ((2, 2), (0.5, 1.5), (1, 1), (0, 0), (-1, 3)):
            probes.probe("transforms.Bbox.expanded", expanded, bbox=bbox, sw=sw, sh=sh)
    assert expanded([[0, 0], [1, 1]], 2, 2).tolist() == [[-0.5, -0.5], [1.5, 1.5]]

    def shrunk(bbox, mx, my):
        return _bbox(bbox).shrunk(mx, my).get_points()

    for bbox in ([[0, 0], [1, 1]], [[1, 2], [4, 6]], [[4, 6], [1, 2]]):
        for mx, my in ((0.5, 0.5), (0.25, 0.8), (1, 1), (2, 0), (-0.5, 1.5)):
            probes.probe("transforms.Bbox.shrunk", shrunk, bbox=bbox, mx=mx, my=my)
    assert shrunk([[1, 2], [4, 6]], 0.5, 0.25).tolist() == [[1, 2], [2.5, 3]]

    def translated(bbox, tx, ty):
        return _bbox(bbox).translated(tx, ty).get_points()

    for bbox in ([[0, 0], [1, 1]], [[4, 6], [1, 2]], NULL):
        for tx, ty in ((1, 2), (-0.5, 0.25), (INF, 0)):
            probes.probe("transforms.Bbox.translated", translated, bbox=bbox, tx=tx, ty=ty)


def test_bbox_shrunk_to_aspect():
    def shrunk_to_aspect(bbox, box_aspect, **kw):
        if "container" in kw:
            kw["container"] = _bbox(kw["container"])
        return _bbox(bbox).shrunk_to_aspect(box_aspect, **kw).get_points()

    boxes = ([[0, 0], [4, 2]], [[1, 1], [2, 4]], [[0.1, 0.2], [0.9, 0.8]], [[3, 3], [1, 1]])
    for bbox in boxes:
        for aspect in (1.0, 0.5, 2.0, 0.75):
            probes.probe("transforms.Bbox.shrunk_to_aspect", shrunk_to_aspect, bbox=bbox,
                         box_aspect=aspect)
            probes.probe("transforms.Bbox.shrunk_to_aspect", shrunk_to_aspect, bbox=bbox,
                         box_aspect=aspect, fig_aspect=0.6)
            probes.probe("transforms.Bbox.shrunk_to_aspect", shrunk_to_aspect, bbox=bbox,
                         box_aspect=aspect, container=[[0, 0], [10, 3]])
            probes.probe("transforms.Bbox.shrunk_to_aspect", shrunk_to_aspect, bbox=bbox,
                         box_aspect=aspect, container=[[0, 0], [1, 5]], fig_aspect=1.5)
    assert shrunk_to_aspect([[0, 0], [4, 2]], 1.0).tolist() == [[0, 0], [2, 2]]
    assert shrunk_to_aspect([[0, 0], [4, 2]], 0.25).tolist() == [[0, 0], [4, 1]]
    for kw in (dict(box_aspect=0.0), dict(box_aspect=-1.0), dict(box_aspect=1.0, fig_aspect=0.0),
               dict(box_aspect=1.0, fig_aspect=-2.0)):
        r = probes.probe("transforms.Bbox.shrunk_to_aspect", shrunk_to_aspect, bbox=[[0, 0], [4, 2]],
                         **kw)
        assert isinstance(r, ValueError)


ANCHORS = ["C", "SW", "S", "SE", "E", "NE", "N", "NW", "W"]


def test_bbox_anchored():
    def anchored(bbox, c, container):
        return _bbox(bbox).anchored(c if isinstance(c, str) else tuple(c), _bbox(container)).get_points()

    cases = [
        ([[0, 0], [1, 1]], [[0, 0], [4, 3]]),         # smaller than container
        ([[5, 5], [6, 7]], [[0, 0], [4, 3]]),         # elsewhere, smaller
        ([[0, 0], [6, 5]], [[1, 1], [3, 2]]),         # larger than container
        ([[0.1, 0.1], [0.3, 0.6]], [[0.1, 0.1], [0.9, 0.9]]),
        ([[2, 2], [1, 1]], [[0, 0], [4, 3]]),         # inverted bbox
        ([[0, 0], [1, 1]], [[4, 3], [0, 0]]),         # inverted container
    ]
    for bbox, container in cases:
        for c in ANCHORS:
            probes.probe("transforms.Bbox.anchored", anchored, bbox=bbox, c=c, container=container)
        for c in ([0.25, 0.75], [0, 0], [1, 1], [-0.5, 1.5]):
            probes.probe("transforms.Bbox.anchored", anchored, bbox=bbox, c=c, container=container)
    assert anchored([[0, 0], [1, 1]], "C", [[0, 0], [4, 3]]).tolist() == [[1.5, 1], [2.5, 2]]
    assert anchored([[0, 0], [1, 1]], "NE", [[0, 0], [4, 3]]).tolist() == [[3, 2], [4, 3]]
    for c in ("X", "c", "ne"):
        r = probes.probe("transforms.Bbox.anchored", anchored, bbox=[[0, 0], [1, 1]], c=c,
                         container=[[0, 0], [4, 3]])
        assert isinstance(r, KeyError)


def test_bbox_split():
    def split(name):
        def fn(bbox, fractions):
            return [b.get_points() for b in getattr(_bbox(bbox), name)(*fractions)]
        return fn

    for bbox in ([[0, 0], [1, 1]], [[1, 2], [5, 4]], [[5, 4], [1, 2]]):
        for fr in ([], [0.5], [0.25, 0.75], [0.1, 0.2, 0.7], [0.75, 0.25], [-0.5, 1.5], [1 / 3]):
            for name in ("splitx", "splity"):
                probes.probe(f"transforms.Bbox.{name}", split(name), bbox=bbox, fractions=fr)
    r = split("splitx")([[1, 2], [5, 4]], [0.25, 0.75])
    assert [b.tolist() for b in r] == [[[1, 2], [2, 4]], [[2, 2], [4, 4]], [[4, 2], [5, 4]]]


def test_bbox_transformed_rotated_corners():
    def transformed(bbox, ops):
        return _bbox(bbox).transformed(_affine(ops)).get_points()

    def transformed_bbox(bbox, ops):
        return mtransforms.TransformedBbox(_bbox(bbox), _affine(ops)).get_points()

    opss = [
        [],
        [["scale", 2, 3], ["translate", 1, -1]],
        [["scale", -1, 1]],
        [["scale", 1, -2], ["translate", 0, 5]],
        [["rotate_deg", 30]],
        [["rotate_deg", 90]],
        [["rotate_deg", 180], ["translate", 1, 1]],
        [["skew_deg", 20, 10]],
        [["rotate_deg_around", 1, 1, 45], ["scale", 0.5]],
    ]
    boxes = [[[0, 0], [1, 1]], [[1, 2], [4, 3]], [[4, 3], [1, 2]], [[1, 3], [4, 2]],
             [[2, 2], [2, 2]]]
    for bbox in boxes:
        for ops in opss:
            probes.probe("transforms.Bbox.transformed", transformed, bbox=bbox, ops=ops)
            probes.probe("transforms.TransformedBbox", transformed_bbox, bbox=bbox, ops=ops)
    for bbox in ([[0, 0], [1, 1]], [[NAN, 0], [1, 1]], [[0, 0], [INF, 1]], NULL):
        probes.probe("transforms.TransformedBbox", transformed_bbox, bbox=bbox,
                     ops=[["scale", 2, 3]])
        probes.probe("transforms.Bbox.transformed", transformed, bbox=bbox, ops=[["scale", 2, 3]])
    # transformed() uses three corners only; TransformedBbox bounds all four
    r = transformed([[0, 0], [1, 1]], [["rotate_deg", 45]])
    assert r[1, 0] == pytest.approx(math.sqrt(0.5)) and r[0, 1] == 0
    r = transformed_bbox([[0, 0], [1, 1]], [["rotate_deg", 45]])
    assert r[0, 0] == pytest.approx(-math.sqrt(0.5)) and r[1, 1] == pytest.approx(math.sqrt(2))
    # orientation of the input is preserved by TransformedBbox
    r = transformed_bbox([[4, 3], [1, 2]], [["scale", 2, 2]])
    assert r.tolist() == [[8, 6], [2, 4]]

    def rotated(bbox, radians):
        return _bbox(bbox).rotated(radians).get_points()

    for bbox in ([[0, 0], [1, 1]], [[1, 2], [4, 3]], [[4, 3], [1, 2]], [[0, 0], [INF, 1]]):
        for rad in (0.0, 0.3, math.pi / 4, math.pi / 2, math.pi, -1.0):
            probes.probe("transforms.Bbox.rotated", rotated, bbox=bbox, radians=rad)

    for bbox in ([[0, 0], [1, 1]], [[4, 3], [1, 2]], NULL):
        r = probes.probe("transforms.Bbox.corners", lambda bbox: _bbox(bbox).corners(), bbox=bbox)
        (x0, y0), (x1, y1) = bbox
        assert _same(r, [[x0, y0], [x0, y1], [x1, y0], [x1, y1]])


# ---------------------------------------------------------------------------
# Bbox mutation
# ---------------------------------------------------------------------------

def _apply_steps(b, steps):
    for step in steps:
        kw = dict(step)
        method = kw.pop("method")
        if method == "ignore":
            b.ignore(kw["value"])
        elif method == "update_from_data_xy":
            xy = kw.pop("xy")
            b.update_from_data_xy(np.asarray(xy, float) if len(xy) else xy, **kw)
        elif method == "update_from_data_x":
            b.update_from_data_x(np.asarray(kw.pop("x"), float), **kw)
        elif method == "update_from_data_y":
            b.update_from_data_y(np.asarray(kw.pop("y"), float), **kw)
        elif method == "update_from_path":
            p = kw.pop("path")
            path = Path(np.asarray(p["vertices"], float).reshape(-1, 2),
                        None if p["codes"] is None else np.asarray(p["codes"], np.uint8))
            b.update_from_path(path, **kw)
        else:
            raise AssertionError(method)


@_errstate
def bbox_update(bbox, steps, minpos=None):
    b = _bbox(bbox)
    if minpos is not None:
        b.minpos = minpos
    _apply_steps(b, steps)
    return {"points": b.get_points(), "minpos": b.minpos,
            "mutatedx": bool(b.mutatedx()), "mutatedy": bool(b.mutatedy())}


def _xy(xy, ignore="default", **kw):
    step = {"method": "update_from_data_xy", "xy": xy}
    if ignore != "default":
        step["ignore"] = ignore
    step.update(kw)
    return step


def test_bbox_update():
    data = [[1.0, 2.0], [3.0, -1.0], [-0.5, 4.0]]
    positive = [[0.25, 8.0], [2.0, 0.125]]
    nonfinite = [[NAN, 5.0], [10.0, INF], [-INF, 0.5], [2.0, 2.0]]
    all_bad = [[NAN, 1.0], [INF, 2.0]]
    start_boxes = [[[0.0, 0.0], [1.0, 1.0]], NULL, [[5.0, 5.0], [-5.0, -5.0]]]
    for start in start_boxes:
        for ignore in (True, False, None, "default"):
            for xy in (data, positive, nonfinite, all_bad, [], [[2.0, 3.0]]):
                probes.probe("transforms.Bbox.update", bbox_update, bbox=start,
                             steps=[_xy(xy, ignore)])
            for upd in (dict(updatex=False), dict(updatey=False), dict(updatex=False, updatey=False)):
                probes.probe("transforms.Bbox.update", bbox_update, bbox=start,
                             steps=[_xy(data, ignore, **upd)])

    r = bbox_update(bbox=[[0.0, 0.0], [1.0, 1.0]], steps=[_xy(data, True)])
    assert r["points"].tolist() == [[-0.5, -1.0], [3.0, 4.0]]
    assert r["minpos"].tolist() == [1.0, 2.0]
    r = bbox_update(bbox=[[0.0, 0.0], [1.0, 1.0]], steps=[_xy(data, False)])
    assert r["points"].tolist() == [[-0.5, -1.0], [3.0, 4.0]]
    r = bbox_update(bbox=[[0.0, 0.0], [1.0, 1.0]], steps=[_xy(all_bad, True)])
    assert r["points"].tolist() == NULL
    r = bbox_update(bbox=[[0.0, 0.0], [1.0, 1.0]], steps=[_xy(data, True, updatey=False)])
    assert r["points"].tolist() == [[-0.5, 0.0], [3.0, 1.0]]

    # sequences, the stored ignore flag, minpos accumulation
    sequences = [
        [_xy([[1, 1]], True), _xy([[2, 3], [3, 2]], False)],
        [_xy([[1, 1]], True), _xy([[2, 3], [3, 2]], True)],
        [{"method": "ignore", "value": False}, _xy([[2, 3]])],
        [{"method": "ignore", "value": False}, _xy([[2, 3]], None)],
        [{"method": "ignore", "value": False}, _xy([[2, 3]], True)],
        [{"method": "ignore", "value": True}, _xy([[2, 3]], None)],
        [_xy([[-1, -2], [-3, -4]], True), _xy([[0.5, 0.25]], False)],
        [_xy([[4, 8]], True), _xy([[2, 16]], False), _xy([[-1, 1]], False)],
        [_xy([[4, 8]], True), _xy([[0.5, 0.5]], True)],
        [{"method": "update_from_data_x", "x": [3.0, -2.0, 7.5], "ignore": True}],
        [{"method": "update_from_data_x", "x": [3.0, NAN, 7.5], "ignore": False}],
        [{"method": "update_from_data_y", "y": [0.5, 9.0], "ignore": True}],
        [{"method": "update_from_data_y", "y": [], "ignore": True}],
        [{"method": "update_from_data_x", "x": [2.0, 4.0], "ignore": True},
         {"method": "update_from_data_y", "y": [-1.0, 0.5], "ignore": False}],
        [{"method": "update_from_path", "ignore": True,
          "path": {"vertices": [[0, 0], [2, 1], [1, 3], [0, 0]], "codes": [1, 2, 2, 79]}}],
        [{"method": "update_from_path", "ignore": False, "updatey": False,
          "path": {"vertices": [[-4, 9], [0.75, 2]], "codes": None}}],
    ]
    for start in ([[0.0, 0.0], [1.0, 1.0]], NULL):
        for steps in sequences:
            probes.probe("transforms.Bbox.update", bbox_update, bbox=start, steps=steps)
    r = bbox_update(bbox=NULL, steps=sequences[0])
    assert r["points"].tolist() == [[1, 1], [3, 3]]

    # explicit initial minpos
    for minpos in ([0.5, 0.25], [10.0, 10.0]):
        for ignore in (True, False):
            probes.probe("transforms.Bbox.update", bbox_update, bbox=[[0.0, 0.0], [1.0, 1.0]],
                         minpos=minpos, steps=[_xy(positive, ignore)])

    def frozen(bbox, steps):
        b = _bbox(bbox)
        _apply_steps(b, steps)
        f = b.frozen()
        return {"points": f.get_points(), "minpos": f.minpos}

    for steps in ([], [_xy(positive, True)], sequences[7]):
        r = probes.probe("transforms.Bbox.frozen", frozen, bbox=[[0.0, 0.0], [1.0, 1.0]], steps=steps)
        ref = bbox_update(bbox=[[0.0, 0.0], [1.0, 1.0]], steps=steps)
        assert _same(r["points"], ref["points"]) and _same(r["minpos"], ref["minpos"])


# ---------------------------------------------------------------------------
# Bbox-driven transforms
# ---------------------------------------------------------------------------

PTS_UNIT = [[0.0, 0.0], [1.0, 1.0], [0.5, 0.25], [-1.0, 2.0]]


def test_bbox_transforms():
    @_errstate
    def bbox_transform(boxin, boxout, points):
        return _matrix_and_points(mtransforms.BboxTransform(_bbox(boxin), _bbox(boxout)), points)

    @_errstate
    def bbox_transform_to(boxout, points):
        return _matrix_and_points(mtransforms.BboxTransformTo(_bbox(boxout)), points)

    @_errstate
    def bbox_transform_from(boxin, points):
        return _matrix_and_points(mtransforms.BboxTransformFrom(_bbox(boxin)), points)

    boxes = [
        [[0.0, 0.0], [1.0, 1.0]],
        [[1.0, 2.0], [4.0, 6.0]],
        [[0.125, 0.11], [0.9, 0.88]],
        [[4.0, 6.0], [1.0, 2.0]],    # inverted
        [[-3.0, 10.0], [7.0, 10.0]],  # zero height
        [[2.0, 2.0], [2.0, 2.0]],    # point
    ]
    for boxin in boxes:
        for boxout in boxes[:4] + [[[0.0, 0.0], [640.0, 480.0]]]:
            probes.probe("transforms.BboxTransform", bbox_transform, boxin=boxin, boxout=boxout,
                         points=PTS_UNIT)
        probes.probe("transforms.BboxTransformTo", bbox_transform_to, boxout=boxin, points=PTS_UNIT)
        probes.probe("transforms.BboxTransformFrom", bbox_transform_from, boxin=boxin,
                     points=PTS_UNIT)
    r = bbox_transform(boxin=[[1.0, 2.0], [4.0, 6.0]], boxout=[[0.0, 0.0], [1.0, 1.0]],
                       points=[[1.0, 2.0], [4.0, 6.0]])
    assert r["transformed"].tolist() == [[0, 0], [1, 1]]
    r = bbox_transform_to(boxout=[[1.0, 2.0], [4.0, 6.0]], points=[[0.0, 0.0], [1.0, 1.0]])
    assert r["matrix"].tolist() == [[3, 0, 1], [0, 4, 2], [0, 0, 1]]
    r = bbox_transform_from(boxin=[[2.0, 2.0], [2.0, 2.0]], points=[])
    assert r["matrix"][0, 0] == INF


# ---------------------------------------------------------------------------
# blended, ScaledTranslation, offset_copy
# ---------------------------------------------------------------------------

def test_blended():
    @_errstate
    def blended(x_ops, y_ops, points):
        t = mtransforms.blended_transform_factory(_affine(x_ops), _affine(y_ops))
        out = _matrix_and_points(t, points)
        out["is_separable"] = bool(t.is_separable)
        return out

    pts = [[0.0, 0.0], [1.0, 2.0], [-3.0, 0.5], [NAN, 1.0], [INF, 0.0]]
    cases = [
        ([], []),
        ([["scale", 2, 3], ["translate", 1, 4]], []),
        ([], [["scale", 2, 3], ["translate", 1, 4]]),
        ([["scale", 2, 3], ["translate", 1, 4]], [["scale", 5, 7], ["translate", -1, -2]]),
        ([["scale", -1, 0.5]], [["scale", 0.25, -4], ["translate", 0, 10]]),
        ([["scale", 2, 3]], [["scale", 2, 3]]),
        ([["rotate", 0.0], ["translate", 1, 1]], [["translate", 2, 2]]),
    ]
    for x_ops, y_ops in cases:
        r = probes.probe("transforms.blended_transform_factory", blended, x_ops=x_ops, y_ops=y_ops,
                         points=pts)
        mx, my = affine_get_matrix(ops=x_ops), affine_get_matrix(ops=y_ops)
        assert _same(r["matrix"], [mx[0], my[1], [0, 0, 1]])
        assert r["is_separable"] is True
    for x_ops, y_ops in (([["rotate_deg", 30]], []), ([], [["rotate_deg", 90]]),
                         ([["skew", 0.1, 0.0]], []), ([["rotate_deg", 180]], [["scale", 2]])):
        r = probes.probe("transforms.blended_transform_factory", blended, x_ops=x_ops, y_ops=y_ops,
                         points=pts)
        assert isinstance(r, ValueError)


def test_scaled_translation():
    @_errstate
    def scaled_translation(xt, yt, scale_ops, points):
        return _matrix_and_points(mtransforms.ScaledTranslation(xt, yt, _affine(scale_ops)), points)

    pts = [[0.0, 0.0], [1.0, 2.0], [-3.5, 0.25]]
    for xt, yt in ((0.1, 0.2), (1.0, -0.5), (0.0, 0.0), (1 / 72, 5 / 72), (INF, 0.0)):
        for scale_ops in ([["scale", 72]], [["scale", 100]], [["scale", 96.5]], [["scale", 2, 3]],
                          [["scale", 100], ["translate", 5, 7]], [["rotate_deg", 90]], []):
            probes.probe("transforms.ScaledTranslation", scaled_translation, xt=xt, yt=yt,
                         scale_ops=scale_ops, points=pts)
    r = scaled_translation(0.1, 0.2, [["scale", 100]], [[0.0, 0.0]])
    assert r["matrix"].tolist() == [[1, 0, 100 * 0.1], [0, 1, 100 * 0.2], [0, 0, 1]]
    r = scaled_translation(1.0, 1.0, [["scale", 100], ["translate", 5, 7]], [[0.0, 0.0]])
    assert r["matrix"][:2, 2].tolist() == [105, 107]


def test_offset_copy():
    @_errstate
    def offset_copy(base_ops, x, y, dpi, points, **kw):
        fig = None if dpi is None else Figure(dpi=dpi)
        t = mtransforms.offset_copy(_affine(base_ops), fig, x, y, **kw)
        return {"matrix": t.get_affine().get_matrix(), "transformed": t.transform(_pts(points))}

    pts = [[0.0, 0.0], [1.0, 2.0], [-3.5, 0.25]]
    bases = [[], [["scale", 2, 3], ["translate", 1, 1]], [["rotate_deg", 30], ["translate", 10, 20]]]
    for base in bases:
        for units in ("inches", "points", "dots"):
            for dpi in (72, 100, 150.5):
                for x, y in ((0.1, -0.2), (5.0, 3.0)):
                    probes.probe("transforms.offset_copy", offset_copy, base_ops=base, x=x, y=y,
                                 units=units, dpi=dpi, points=pts)
        probes.probe("transforms.offset_copy", offset_copy, base_ops=base, x=0.25, y=0.5,
                     dpi=100, points=pts)  # default units: inches
    r = offset_copy([], 0.1, -0.2, 100, pts, units="inches")
    assert r["matrix"].tolist() == [[1, 0, 100 * 0.1], [0, 1, 100 * -0.2], [0, 0, 1]]
    r = offset_copy([], 36.0, 72.0, 100, pts, units="points")
    assert r["matrix"].tolist() == [[1, 0, 50], [0, 1, 100], [0, 0, 1]]
    r = probes.probe("transforms.offset_copy", offset_copy, base_ops=[["scale", 2]], x=3.0, y=4.0,
                     units="dots", dpi=None, points=pts)
    assert r["matrix"].tolist() == [[2, 0, 3], [0, 2, 4], [0, 0, 1]]
    for units in ("inches", "points"):
        r = probes.probe("transforms.offset_copy", offset_copy, base_ops=[], x=1.0, y=1.0,
                         units=units, dpi=None, points=pts)
        assert isinstance(r, ValueError)
    r = probes.probe("transforms.offset_copy", offset_copy, base_ops=[], x=1.0, y=1.0, units="cm",
                     dpi=100, points=pts)
    assert isinstance(r, ValueError)


# ---------------------------------------------------------------------------
# nonsingular / interval helpers
# ---------------------------------------------------------------------------

def nonsingular(vmin, vmax, **kw):
    return list(mtransforms._nonsingular(vmin, vmax, **kw))


def test_nonsingular():
    pairs = [
        (0.0, 1.0), (1.0, 0.0), (0.0, 0.0), (1.0, 1.0), (-1.0, -1.0), (-2.0, -2.0),
        (5.0, 5.0), (1e-10, 1e-10), (1e-300, 1e-300), (1e-286, 1e-286), (0.0, 1e-300),
        (0.0, 1e-20), (1e-20, 2e-20), (1.0, 1.0000000000000002), (1.0000000000000002, 1.0),
        (1.0, 1.000000000000001), (1.0, 1.00000000000001), (1e10, 1e10 + 1e-5),
        (100.0, 100.0 + 1e-13), (-3.0, 3.0), (1e308, 1e308), (-1e308, 1e308),
        (1.7976931348623157e308, 1.7976931348623157e308), (INF, 1.0), (1.0, -INF), (NAN, 1.0),
        (0.0, NAN), (INF, INF), (2, 2), (0, 0), (7, 3),
    ]
    for vmin, vmax in pairs:
        probes.probe("transforms.nonsingular", nonsingular, vmin=vmin, vmax=vmax)
    for vmin, vmax in ((5.0, 2.0), (2.0, 5.0), (3.0, 3.0), (3.0 + 1e-15, 3.0), (0.0, 0.0),
                       (-1.0, -1.0 - 1e-16), (INF, 0.0)):
        for increasing in (True, False):
            probes.probe("transforms.nonsingular", nonsingular, vmin=vmin, vmax=vmax,
                         increasing=increasing)
    for expander in (0.05, 0.5, 1.0, 2.0):
        for vmin, vmax in ((0.0, 0.0), (1.0, 1.0), (-4.0, -4.0), (NAN, 0.0), (10.0, 1.0)):
            probes.probe("transforms.nonsingular", nonsingular, vmin=vmin, vmax=vmax,
                         expander=expander)
    for tiny in (1e-10, 1e-5, 0.1, 0.5, 1e-300):
        for vmin, vmax in ((1.0, 1.05), (1.0, 1.0 + 1e-8), (100.0, 100.001), (1e-280, 1e-280),
                           (0.0, 1e-290)):
            probes.probe("transforms.nonsingular", nonsingular, vmin=vmin, vmax=vmax, tiny=tiny)
    probes.probe("transforms.nonsingular", nonsingular, vmin=6.0, vmax=6.0, expander=0.1,
                 tiny=1e-3, increasing=False)
    probes.probe("transforms.nonsingular", nonsingular, vmin=6.0 + 1e-6, vmax=6.0, expander=0.1,
                 tiny=1e-3, increasing=False)

    assert nonsingular(0.0, 0.0) == [-0.001, 0.001]
    assert nonsingular(5.0, 2.0) == [2.0, 5.0]
    assert nonsingular(5.0, 2.0, increasing=False) == [5.0, 2.0]
    assert nonsingular(1.0, 1.0) == [1.0 - 0.001, 1.0 + 0.001]
    assert nonsingular(NAN, 1.0) == [-0.001, 0.001]
    assert nonsingular(1e-300, 1e-300) == [-0.001, 0.001]
    # public (deprecated) wrapper is the same function
    with pytest.warns(DeprecationWarning):
        assert list(mtransforms.nonsingular(3.0, 3.0)) == nonsingular(3.0, 3.0)


def test_interval_contains():
    intervals = [[0.0, 1.0], [1.0, 0.0], [1.0, 1.0], [-2.0, 3.0], [-INF, 0.0]]
    vals = [-1.0, 0.0, 0.5, 1.0, 1.5, 1.0 + 1e-11, -1e-11, 1.0 + 1e-9, NAN, INF, -INF]
    for interval in intervals:
        for val in vals:
            probes.probe("transforms.interval_contains",
                         lambda interval, val: bool(mtransforms._interval_contains(interval, val)),
                         interval=interval, val=val)
            probes.probe("transforms.interval_contains_open",
                         lambda interval, val: bool(mtransforms._interval_contains_open(interval, val)),
                         interval=interval, val=val)
            probes.probe("transforms.interval_contains_close",
                         lambda interval, val: bool(mtransforms._interval_contains_close(interval, val)),
                         interval=interval, val=val)
    for rtol in (0.0, 1e-3, 0.5):
        for val in (-0.4, -0.0005, 1.0004, 1.002, 1.4, 1.6):
            probes.probe("transforms.interval_contains_close",
                         lambda interval, val, rtol: bool(
                             mtransforms._interval_contains_close(interval, val, rtol=rtol)),
                         interval=[1.0, 0.0], val=val, rtol=rtol)
    assert mtransforms._interval_contains((1.0, 0.0), 1.0)
    assert not mtransforms._interval_contains_open((0.0, 1.0), 1.0)
    assert mtransforms._interval_contains_open((1.0, 0.0), 0.5)
    assert mtransforms._interval_contains_close((0.0, 1.0), 1.0 + 1e-11)
    assert not mtransforms._interval_contains_close((0.0, 1.0), 1.0 + 1e-9)
    with pytest.warns(DeprecationWarning):
        assert mtransforms.interval_contains((0, 1), 0.5)
    with pytest.warns(DeprecationWarning):
        assert mtransforms.interval_contains_open((0, 1), 0.5)
