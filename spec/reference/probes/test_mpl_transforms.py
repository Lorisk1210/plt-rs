"""Affine transforms, boxes and transform trees, ported from matplotlib's own
``tests/test_transforms.py`` with the tests' own inputs.

Area ``mpl_transforms``.

Arguments follow the ``transforms`` area (``test_transforms.py``): ``ops``
is an affine operation list applied to a fresh ``Affine2D()`` (or to
``Affine2D(matrix)``), ``points`` an ``(N, 2)`` list, boxes ``[[x0, y0],
[x1, y1]]``; matrices are returned as 3x3 nested lists.

Ops
---
``transforms.Affine2D.get_matrix`` {ops} -> 3x3 matrix.
``transforms.Affine2D.transform`` {ops, points} -> points.
``transforms.Affine2D.__add__`` {chain, points} -> {matrix, transformed,
    is_separable} of ``T0 + T1 + ...`` (see the ``transforms`` area).
``transforms.Affine2D.from_values.transform`` {values, points} ->
    ``Affine2D.from_values(*values).transform(points)``.
``transforms.Affine2D.transform_angles`` {ops, angles, points} -> the
    angles (degrees) of directions at ``points`` after the transform.
``transforms.AffineDeltaTransform.get_matrix`` {values} -> the matrix of
    ``AffineDeltaTransform(Affine2D.from_values(*values))``: the base matrix
    without its translation.
``transforms.Bbox._is_finite`` {bbox} -> bool.
``transforms.Bbox.frozen`` {extents, minpos} -> {points, minpos} of
    ``Bbox.from_extents(*extents, minpos=minpos).frozen()``.
``transforms.Bbox.intersection`` {bbox1, bbox2} -> points, or null when the
    boxes do not intersect.
``transforms.Bbox.overlaps`` {bbox, other} -> bool.
``_path.count_bboxes_overlapping_bbox`` {bbox, bboxes} -> how many of
    ``bboxes`` overlap ``bbox`` (touching edges do not count).
``transforms.TransformedBbox.contains`` / ``.fully_contains`` {ops, x, y}
    -> bool, for ``TransformedBbox(Bbox.unit(), Affine2D-from-ops)``.
``transforms.nonsingular`` {vmin, vmax} -> ``_nonsingular(vmin, vmax)``.
``transforms.interval_contains`` / ``interval_contains_open``
    {interval, val} -> bool.
``transforms.BlendedGenericTransform.transform_path_non_affine`` {path} ->
    the path (``{vertices, codes}``) through ``BlendedGenericTransform(
    Affine2D(), LogTransform(10, 'clip'))``: x unchanged, y as ``log10``
    with non-positive values clipped to -1000.
``path.Path.iter_segments`` {path, transform, remove_nans, clip, snap,
    stroke_width, simplify, curves, rc} -> ``[[flat_vertices, code], ...]``
    (the ``path`` area's op).
``axes.Axes.transData.transform`` {yscale, point} -> the display
    coordinates of ``point`` in a default ``Figure().subplots()`` axes after
    ``set_yscale(yscale)``.

Transform trees
...............
``TestBasicTransform``'s transforms: ``ta1 = Affine2D().rotate(pi / 2)``,
``ta2 = Affine2D().translate(10, 0)``, ``ta3 = Affine2D().scale(1, 2)``
and ``tn1``, ``tn2``, ``tn3``, three distinct non-affine transforms that
each translate by ``(1, 2)`` (``NonAffineForTest``). A tree is one of these
names, ``["+", a, b]`` (``a + b``) or ``["blend", x, y]``
(``BlendedGenericTransform(x, y)``).

``transforms.Transform.contains_branch`` {tree, branch} -> bool.
``transforms.Transform.contains_branch_separately`` {tree, branch} ->
    ``[x, y]``.
``transforms.Transform.transform_non_affine`` {tree, points} -> points
    through every step up to the last non-affine one.
``transforms.Transform.transform`` {tree, points} -> points.
``transforms.Transform.get_affine`` {tree} -> the matrix of the trailing
    affine part.

Data limits
...........
These ops add one artist to ``Figure().add_subplot(projection=projection)``
(rectilinear by default) and return ``ax.dataLim.get_points()``. Their
``transform`` is a tree over ``"data"`` (``ax.transData``), ``"axes"``
(``ax.transAxes``), ``["affine", ops]``, ``["+", a, b]``, ``["blend", x,
y]`` (``blended_transform_factory``) and ``["nonaffine", tree]`` (the
upstream ``NonAffineForTest``, a non-affine transform doing what ``tree``
does).

``axes.Axes.plot.dataLim`` {x?, y, transform, projection?}: ``plot(x, y)``,
    or ``plot(y)`` without ``x``.
``axes.Axes.add_patch.dataLim`` {vertices, transform}: a ``PathPatch`` of
    ``Path(vertices)``.
``axes.Axes.contourf.dataLim`` / ``axes.Axes.pcolor.dataLim`` /
``axes.Axes.pcolormesh.dataLim`` {transform}: ``xs, ys =
    np.meshgrid(np.linspace(15, 20, 15), np.linspace(12.4, 12.5, 20))``
    and ``z = np.log(xs * ys)`` (``z[:-1, :-1]`` for the pcolors).

Cases
-----
Named after the upstream tests; ``TestAffine2D``'s point is ``[1, 1]``, its
point list ``[[0, 2], [3, 3], [4, 0]]`` and its pivot ``(1, 1)``. A test
asserting that two matrices are equal records both.

Left out
--------
``test_init`` (numpy dtypes; plt-rs's ``Affine2D`` holds the six affine
coefficients, not the arbitrary bottom row the test passes),
``test_values``, ``test_clear`` (the ``transforms`` area probes
``from_values``, ``to_values`` and ``clear``), ``test_transform_single_point``
(the ``transforms`` area's 1-D points), ``test_offset_copy_errors`` (the
``transforms`` area's unknown units; plt-rs resolves units at draw time,
so there is no figure to miss), ``test_pre_transform_plotting`` and
``test_scale_swapping`` (figure cases of the ``mpl_small`` group);
``test_modify_inplace`` (numpy views of the matrix), ``test_copy``,
``test_deepcopy``, ``test_affine_inverted_invalidated``,
``TestAffineDeltaTransform``'s mutation half, ``test_transformed_path``,
``test_transformed_patch_path``, ``test_lockable_bbox`` and
``test_non_affine_caching`` (matplotlib's invalidation graph: plt-rs
transforms are values resolved against the figure at draw time, with no
cached state to go stale), ``test_invalid_transform``,
``test_transformwrapper`` and the error half of ``test_transform_angles``
and of ``test_count_bboxes_overlapping_bbox`` (array shape checks; plt-rs
takes typed points and boxes), ``test_external_transform_api``
(``_as_mpl_transform``), ``test_left_to_right_iteration`` (a private
generator), ``test_bbox_as_strings`` and ``test_str_transform`` (Python
``repr``/``str``), ``test_transform_depth`` and ``test_transform_shortcuts``
(``Transform.depth`` and the ``-`` operator, which plt-rs's transform
expressions do not have), ``test_scaledrotation_*`` (a private class driven by
mocks), ``test_pcolormesh_gouraud_nans`` (only checks that drawing does not
raise).
"""

import math

import numpy as np
import pytest
from matplotlib import scale as mscale
import matplotlib.transforms as mtransforms
from matplotlib.figure import Figure
from matplotlib.patches import PathPatch
from matplotlib.path import Path
from matplotlib._path import count_bboxes_overlapping_bbox

from probes.test_path import REF_RC, _iter_segments, _path, _pd
from probes.test_transforms import _affine, _pts, affine_add, affine_get_matrix, affine_transform
from spec_support import ProbeSet

probes = ProbeSet("mpl_transforms")

pytestmark = pytest.mark.filterwarnings("ignore::RuntimeWarning")

INF = math.inf
NAN = math.nan

SINGLE = [[1.0, 1.0]]
MULTIPLE = [[0.0, 2.0], [3.0, 3.0], [4.0, 0.0]]
PIVOT = [1.0, 1.0]
SKEW = ["skew_deg", 26.5650512, 14.0362435]  # ~atan(0.5), ~atan(0.25)


# ---------------------------------------------------------------------------
# TestAffine2D
# ---------------------------------------------------------------------------


def matrix(*ops):
    probes.probe("transforms.Affine2D.get_matrix", affine_get_matrix, ops=list(ops))


def transformed(*ops):
    for points in (SINGLE, MULTIPLE):
        probes.probe("transforms.Affine2D.transform", affine_transform, ops=list(ops), points=points)


def chained(*chain):
    probes.probe("transforms.Affine2D.__add__", affine_add, chain=[list(c) for c in chain],
                 points=SINGLE + MULTIPLE)


def test_rotate():
    for radians, degrees in ((math.pi / 2, 90), (math.pi, 180), (3 * math.pi / 2, 270)):
        matrix(["rotate", radians])
        matrix(["rotate_deg", degrees])
        transformed(["rotate_deg", degrees])
    chained([["rotate_deg", 90]], [["rotate_deg", 90]])
    chained([["rotate_deg", 90]], [["rotate_deg", 180]])


def test_rotate_around():
    for radians, degrees in ((math.pi / 2, 90), (math.pi, 180), (3 * math.pi / 2, 270)):
        matrix(["rotate_around", *PIVOT, radians])
        matrix(["rotate_deg_around", *PIVOT, degrees])
        transformed(["rotate_deg_around", *PIVOT, degrees])
    chained([["rotate_deg_around", *PIVOT, 90]], [["rotate_deg_around", *PIVOT, 90]])
    chained([["rotate_deg_around", *PIVOT, 90]], [["rotate_deg_around", *PIVOT, 180]])


def test_scale():
    matrix(["scale", 3, -2])
    transformed(["scale", 3, -2])
    chained([["scale", 3, 1]], [["scale", 1, -2]])


def test_skew():
    matrix(["skew", math.pi / 8, math.pi / 12])
    matrix(["skew_deg", 22.5, 15])
    transformed(SKEW)


def test_translate():
    matrix(["translate", 23, 42])
    transformed(["translate", 23, 42])
    chained([["translate", 23, 0]], [["translate", 0, 42]])


OTHERS = {
    "rotate": ["rotate_deg", 90],
    "rotate_around": ["rotate_deg_around", *PIVOT, 180],
    "scale": ["scale", 3, -2],
    "skew": SKEW,
    "translate": ["translate", 23, 42],
}


def _plus_other(first, others):
    """``first`` followed by each other operation, chained in place and as
    the sum of two transforms."""
    for other in others:
        transformed(first, other)
        matrix(first, other)
        chained([first], [other])


def test_rotate_plus_other():
    _plus_other(OTHERS["rotate"], [["rotate_deg_around", *PIVOT, 180], OTHERS["scale"], SKEW,
                                   OTHERS["translate"]])


def test_rotate_around_plus_other():
    _plus_other(["rotate_deg_around", *PIVOT, 90], [["rotate_deg", 180], OTHERS["scale"], SKEW,
                                                     OTHERS["translate"]])


def test_scale_plus_other():
    _plus_other(OTHERS["scale"], [OTHERS["rotate"], ["rotate_deg_around", *PIVOT, 90], SKEW,
                                  OTHERS["translate"]])


def test_skew_plus_other():
    _plus_other(SKEW, [OTHERS["rotate"], ["rotate_deg_around", *PIVOT, 90], OTHERS["scale"],
                       OTHERS["translate"]])


def test_translate_plus_other():
    _plus_other(OTHERS["translate"], [OTHERS["rotate"], ["rotate_deg_around", *PIVOT, 90],
                                      OTHERS["scale"], SKEW])


def _from_values_transform(values, points):
    return mtransforms.Affine2D.from_values(*values).transform(_pts(points))


def test_Affine2D_from_values():
    points = [[0, 0], [10, 20], [-1, 0]]
    for k in range(6):
        values = [0.0] * 6
        values[k] = k + 1.0
        probes.probe("transforms.Affine2D.from_values.transform", _from_values_transform,
                     values=values, points=points)


# TestAffineDeltaTransform.test_invalidate
def test_affine_delta_invalidate():
    probes.probe("transforms.AffineDeltaTransform.get_matrix",
                 lambda values: mtransforms.AffineDeltaTransform(
                     mtransforms.Affine2D.from_values(*values)).get_matrix(),
                 values=[1, 5, 4, 1, 2, 3])


def test_transform_angles():
    probes.probe("transforms.Affine2D.transform_angles",
                 lambda ops, angles, points: _affine(ops).transform_angles(
                     np.asarray(angles, float), np.asarray(points, float)),
                 ops=[], angles=[20, 45, 60], points=[[0, 0], [1, 1], [2, 2]])


# ---------------------------------------------------------------------------
# TestBasicTransform
# ---------------------------------------------------------------------------


class NonAffineForTest(mtransforms.Transform):
    """The upstream helper: looks non-affine, does what ``real_trans`` does."""

    is_affine = False
    output_dims = 2
    input_dims = 2

    def __init__(self, real_trans, *args, **kwargs):
        self.real_trans = real_trans
        super().__init__(*args, **kwargs)

    def transform_non_affine(self, values):
        return self.real_trans.transform(values)

    def transform_path_non_affine(self, path):
        return self.real_trans.transform_path(path)


def _basic_leaves():
    return {
        "ta1": mtransforms.Affine2D().rotate(np.pi / 2),
        "ta2": mtransforms.Affine2D().translate(10, 0),
        "ta3": mtransforms.Affine2D().scale(1, 2),
        **{name: NonAffineForTest(mtransforms.Affine2D().translate(1, 2))
           for name in ("tn1", "tn2", "tn3")},
    }


def _basic_tree(spec, leaves):
    if isinstance(spec, str):
        return leaves[spec]
    kind, a, b = spec
    a, b = _basic_tree(a, leaves), _basic_tree(b, leaves)
    return a + b if kind == "+" else mtransforms.BlendedGenericTransform(a, b)


def _basic(fn):
    def evaluate(tree, **kw):
        leaves = _basic_leaves()
        other = {k: _basic_tree(v, leaves) if k == "branch" else v for k, v in kw.items()}
        return fn(_basic_tree(tree, leaves), **other)

    return evaluate


def plus(*items):
    """``items[0] + items[1] + ...`` as a tree (left-associative)."""
    tree = items[0]
    for item in items[1:]:
        tree = ["+", tree, item]
    return tree


STACK1 = plus(plus("ta1", plus("tn1", "ta2")), "ta3")
STACK2 = plus("ta1", "tn1", "ta2", "ta3")
STACK2_SUBSET = plus("tn1", "ta2", "ta3")


def test_contains_branch():
    contains = _basic(lambda t, branch: bool(t.contains_branch(branch)))
    r1 = plus("ta2", "ta1")
    for tree, branch in [(r1, r1), (r1, "ta1"), (r1, "ta2"), (r1, plus("ta2", "ta2")),
                         (STACK1, "ta3"), (STACK2, "ta3"), (STACK1, STACK2_SUBSET),
                         (STACK2, STACK2_SUBSET), (STACK2_SUBSET, STACK1), (STACK2_SUBSET, STACK2),
                         (STACK1, plus("ta2", "ta3")), (STACK2, plus("ta2", "ta3")),
                         (STACK1, plus("tn1", "ta2"))]:
        probes.probe("transforms.Transform.contains_branch", contains, tree=tree, branch=branch)
    separately = _basic(lambda t, branch: list(t.contains_branch_seperately(branch)))
    blend = ["blend", "tn2", STACK2]
    for tree in (blend, plus("tn3", blend)):
        probes.probe("transforms.Transform.contains_branch_separately", separately, tree=tree,
                     branch=STACK2_SUBSET)


def test_affine_simplification():
    points = [[0, 0], [10, 20], [NAN, 1], [-1, 0]]
    probes.probe("transforms.Transform.transform_non_affine",
                 _basic(lambda t, points: t.transform_non_affine(_pts(points))),
                 tree=STACK1, points=points)
    probes.probe("transforms.Transform.transform", _basic(lambda t, points: t.transform(_pts(points))),
                 tree=STACK1, points=points)
    for tree in (STACK1, STACK2):
        probes.probe("transforms.Transform.get_affine", _basic(lambda t: t.get_affine().get_matrix()),
                     tree=tree)


# ---------------------------------------------------------------------------
# TestTransformPlotInterface and the pre-transform limits
# ---------------------------------------------------------------------------


def _plot_tree(spec, ax):
    if spec == "data":
        return ax.transData
    if spec == "axes":
        return ax.transAxes
    kind, *rest = spec
    if kind == "affine":
        return _affine(rest[0])
    if kind == "nonaffine":
        return NonAffineForTest(_plot_tree(rest[0], ax))
    a, b = (_plot_tree(r, ax) for r in rest)
    return a + b if kind == "+" else mtransforms.blended_transform_factory(a, b)


def _data_lim(add):
    def evaluate(transform, projection=None, **kw):
        ax = Figure().add_subplot(projection=projection)
        add(ax, _plot_tree(transform, ax), **kw)
        return ax.dataLim.get_points()

    return evaluate


def _plot(ax, transform, y, x=None):
    if x is None:
        ax.plot(y, transform=transform)
    else:
        ax.plot(x, y, transform=transform)


def _patch(ax, transform, vertices):
    ax.add_patch(PathPatch(Path(vertices), transform=transform))


def _grid():
    xs, ys = np.meshgrid(np.linspace(15, 20, 15), np.linspace(12.4, 12.5, 20))
    return xs, ys, np.log(xs * ys)


def _contourf(ax, transform):
    ax.contourf(*_grid(), transform=transform)


def _pcolor(name):
    def add(ax, transform):
        xs, ys, z = _grid()
        getattr(ax, name)(xs, ys, z[:-1, :-1], transform=transform)

    return add


def plot_lim(**kw):
    probes.probe("axes.Axes.plot.dataLim", _data_lim(_plot), **kw)


TENTH = ["affine", [["scale", 0.1]]]
TIMES10 = ["affine", [["scale", 10]]]
OFFSET = ["affine", [["translate", 10, 10]]]


PRE_TENTH = ["+", TENTH, "data"]


def test_contour_pre_transform_limits():
    probes.probe("axes.Axes.contourf.dataLim", _data_lim(_contourf), transform=PRE_TENTH)


def test_pcolor_pre_transform_limits():
    probes.probe("axes.Axes.pcolor.dataLim", _data_lim(_pcolor("pcolor")), transform=PRE_TENTH)


def test_pcolormesh_pre_transform_limits():
    probes.probe("axes.Axes.pcolormesh.dataLim", _data_lim(_pcolor("pcolormesh")),
                 transform=PRE_TENTH)


LINE_X = [0.1, 1.2, 0.8]
LINE_Y = [35, -5, 18]
TENS = list(range(10))
NONAFFINE_OFFSET = ["+", ["+", OFFSET, ["nonaffine", OFFSET]], "data"]


class TestTransformPlotInterface:
    def test_line_extent_axes_coords(self):
        plot_lim(x=LINE_X, y=[0.9, 0.5, 0.8], transform="axes")

    def test_line_extent_data_coords(self):
        plot_lim(x=LINE_X, y=[0.9, 0.5, 0.8], transform="data")

    def test_line_extent_compound_coords1(self):
        plot_lim(x=LINE_X, y=LINE_Y, transform=["blend", "axes", "data"])

    def test_line_extent_predata_transform_coords(self):
        plot_lim(x=LINE_X, y=LINE_Y, transform=["+", TIMES10, "data"])

    def test_line_extent_compound_coords2(self):
        plot_lim(x=LINE_X, y=LINE_Y, transform=["blend", "axes", ["+", TIMES10, "data"]])

    def test_line_extents_affine(self):
        plot_lim(y=TENS, transform=["+", OFFSET, "data"])

    def test_line_extents_non_affine(self):
        plot_lim(y=TENS, transform=NONAFFINE_OFFSET)

    def test_pathc_extents_non_affine(self):
        _patch_lim(NONAFFINE_OFFSET)

    def test_pathc_extents_affine(self):
        _patch_lim(["+", OFFSET, "data"])

    def test_line_extents_for_non_affine_transData(self):
        plot_lim(y=TENS, transform=["+", ["affine", [["translate", 0, 10]]], "data"],
                 projection="polar")


def _patch_lim(transform):
    probes.probe("axes.Axes.add_patch.dataLim", _data_lim(_patch),
                 vertices=[[0, 0], [0, 10], [10, 10], [10, 0]], transform=transform)


# ---------------------------------------------------------------------------
# Paths through a log transform
# ---------------------------------------------------------------------------


def _blended_log_clip(path):
    trans = mtransforms.BlendedGenericTransform(mtransforms.Affine2D(), mscale.LogTransform(10, "clip"))
    return _pd(trans.transform_path_non_affine(_path(path)))


def test_clipping_of_log():
    path = Path._create_closed([(0.2, -99), (0.4, -99), (0.4, 20), (0.2, 20)])
    path = {"vertices": path.vertices.tolist(), "codes": path.codes.tolist()}
    tpath = probes.probe("transforms.BlendedGenericTransform.transform_path_non_affine",
                         _blended_log_clip, path=path)
    tpath = {"vertices": tpath["vertices"].tolist(), "codes": tpath["codes"].tolist()}
    segments = probes.probe("path.Path.iter_segments", _iter_segments, path=tpath, transform=None,
                            remove_nans=True, clip=[0, 0, 100, 100], snap=False, stroke_width=1.0,
                            simplify=False, curves=True, rc=REF_RC)
    assert [code for _, code in segments] == path["codes"][:-1]


def _log_trans_data(yscale, point):
    ax = Figure().subplots()
    ax.set_yscale(yscale)
    return ax.transData.transform(point)


def test_log_transform():
    probes.probe("axes.Axes.transData.transform", _log_trans_data, yscale="log", point=[1, 1])


# ---------------------------------------------------------------------------
# Boxes and intervals
# ---------------------------------------------------------------------------


def _box(bbox):
    return mtransforms.Bbox(bbox)


def test_bbox_is_finite():
    for bbox in ([[1, 1], [1, 1]], [[0, 0], [INF, 1]], [[-INF, 0], [2, 2]], [[NAN, 0], [2, 2]],
                 [[0, 0], [0, 2]], [[0, 0], [2, 0]], [[0, 0], [1, 2]]):
        probes.probe("transforms.Bbox._is_finite", lambda bbox: bool(_box(bbox)._is_finite()),
                     bbox=bbox)


def _frozen(extents, minpos):
    frozen = mtransforms.Bbox.from_extents(*extents, minpos=minpos).frozen()
    return {"points": frozen.get_points(), "minpos": frozen.minpos}


def test_bbox_frozen_copies_minpos():
    probes.probe("transforms.Bbox.frozen", _frozen, extents=[0.0, 0.0, 1.0, 1.0], minpos=1.0)


def _intersection(bbox1, bbox2):
    result = mtransforms.Bbox.intersection(_box(bbox1), _box(bbox2))
    return None if result is None else result.get_points()


def test_bbox_intersection():
    r1 = [[0, 0], [1, 1]]
    for other in (r1, [[0.5, 0.5], [1.5, 1.5]], [[0.5, 0], [0.75, 0.75]], [[0.5, 1.5], [1, 2.5]],
                  [[1, 1], [2, 2]]):
        probes.probe("transforms.Bbox.intersection", _intersection, bbox1=r1, bbox2=other)


def test_count_bboxes_overlapping_bbox():
    corners = [[[0, 0], [2, 2]], [[8, 0], [10, 2]], [[0, 8], [2, 10]], [[8, 8], [10, 10]]]
    center = [[4, 4], [6, 6]]
    for bboxes in (corners, [center], [center, *corners]):
        probes.probe("_path.count_bboxes_overlapping_bbox",
                     lambda bbox, bboxes: int(count_bboxes_overlapping_bbox(bbox, bboxes)),
                     bbox=[[0, 0], [10, 10]], bboxes=bboxes)


def test_nan_overlap():
    probes.probe("transforms.Bbox.overlaps", lambda bbox, other: bool(_box(bbox).overlaps(_box(other))),
                 bbox=[[0, 0], [1, 1]], other=[[0, 0], [1, NAN]])


def _transformed_bbox(method):
    def evaluate(ops, x, y):
        return bool(getattr(mtransforms.TransformedBbox(mtransforms.Bbox.unit(), _affine(ops)), method)(x, y))

    return evaluate


def test_transformedbbox_contains():
    for method, ops, points in [
        ("contains", [["rotate_deg", 30]], [(0.8, 0.5), (-0.4, 0.85), (0.9, 0.5)]),
        ("contains", [["translate", 0.25, 0.5]], [(1.25, 1.5)]),
        ("fully_contains", [["translate", 0.25, 0.5]], [(1.25, 1.5), (0.1, 0.1)]),
    ]:
        for x, y in points:
            probes.probe(f"transforms.TransformedBbox.{method}", _transformed_bbox(method), ops=ops,
                         x=x, y=y)


def test_nonsingular():
    for vmin, vmax in ((0, NAN), (0, 0), (0, 7.9e-317)):
        probes.probe("transforms.nonsingular",
                     lambda vmin, vmax: list(mtransforms._nonsingular(vmin, vmax)), vmin=vmin, vmax=vmax)


def test_interval_contains():
    for name, fn in (("interval_contains", mtransforms._interval_contains),
                     ("interval_contains_open", mtransforms._interval_contains_open)):
        for interval, val in [((0, 1), 0.5), ((0, 1), 0), ((0, 1), 1), ((0, 1), -1), ((0, 1), 2),
                              ((1, 0), 0.5)]:
            probes.probe(f"transforms.{name}", lambda interval, val, _fn=fn: bool(_fn(interval, val)),
                         interval=list(interval), val=val)
