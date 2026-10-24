"""matplotlib's own tests of ``matplotlib.tri``
(``tests/test_triangulation.py``), with the tests' own inputs:
triangulations and their masks, Delaunay triangulations of grids,
duplicates, lines and nearly degenerate points, point location, linear
and cubic interpolation (patch tests, C1 continuity, flat triangles,
invariance under rotation and stretching), mesh analysis, refinement and
the paths and errors of contours on triangulations.

Area ``mpl_triangulation``. Most ops are the ``tri`` area's
(``test_tri.py``, which documents their arguments and canonical forms) or
the ``contour`` area's (``test_contour.py``); 2-D query arrays are passed
flattened.

Ops
---
``tri.Triangulation.explicit``, ``tri.Triangulation.delaunay_counts``,
``tri.Triangulation.delaunay_order``, ``tri.LinearTriInterpolator.__call__``,
``tri.CubicTriInterpolator.__call__``, ``tri.CubicTriInterpolator.gradient``,
``tri.UniformTriRefiner.refine_triangulation``,
``tri.UniformTriRefiner.refine_field``, ``tri.TriAnalyzer.scale_factors``,
``tri.TriAnalyzer.circle_ratios``, ``tri.TriAnalyzer.get_flat_tri_mask``
    The ``tri`` area's.
``Axes.tricontourf``, ``ContourSet.get_paths``
    The ``contour`` area's.
``tri.TrapezoidMapTriFinder.__call__``  {x, y, triangles, mask, set_mask,
    qx, qy} -> the indices ``triang.get_trifinder()(qx, qy)`` returns
    (``-1`` outside), after ``triang.set_mask(set_mask)`` unless
    ``set_mask`` is null: the finder made before the new mask follows it.
``tri.Triangulation.tris_contain_point``  {x, y, points} -> for each
    point, how many triangles of the Delaunay triangulation of ``x``,
    ``y`` contain it (``Path(corners).contains_point``).
``tri.Triangulation.neighbors``  {x, y} -> whether the neighbours of the
    Delaunay triangulation of ``x``, ``y`` equal the neighbours computed
    again from its triangles.
``TriInterpolator._interpolate_multikeys``  {x, y, triangles, z,
    interpolator, qx, qy, tri} -> the values of the ``"linear"``,
    ``"cubic_min_E"`` or ``"cubic_geom"`` interpolator of ``z`` at the
    query points, all evaluated in triangle ``tri`` (``tri_index``).

Cases
-----
Named after the upstream tests. ``test_trifinder``,
``test_triinterp_colinear`` and the interpolation tests record the
values the tests compare with their expectations; ``test_triinterpcubic_C1_continuity``
records the values and gradients at its 10 places and the 24 points of
their stars for each of its 9 degrees of freedom (tolerance 1e-6, the
test's continuity bound being 1e-8); ``test_triinterp_transformations``
records the three interpolators at the rotated and stretched query points
(tolerance 1e-6) on the Delaunay triangles of the unrotated points.
``test_delaunay_duplicate_points`` draws its points from
``spec_support.rng(23)``.

Already covered
---------------
By the ``tri_behaviour`` area: ``test_tripcolor_color``,
``test_tripcolor_clim``, ``test_triplot_return``, ``test_triplot_label``
and ``test_qhull_large_offset``. By the ``mpl_misc`` figure group:
``test_tripcolor``, ``test_tri_smooth_contouring``,
``test_tri_smooth_gradient`` and ``test_triplot_with_ls``.
``test_trianalyzer_mismatched_indices`` compresses the triangulation of
``test_tritools``, as its ``scale_factors`` probe does.

Left out
--------
``test_extract_triangulation_params``,
``test_extract_triangulation_positional_mask`` (Python argument
parsing), ``test_tripcolor_warnings`` (warnings), ``test_no_modify`` and
``test_trirefiner_fortran_contiguous_triangles`` (numpy array ownership
and memory layout), ``test_internal_cpp_api`` (the private C++ extension),
``test_tricontourset_reuse`` (identity of private generator objects) and
the masked-array case of ``test_tricontour_non_finite_z`` (plt-rs takes
plain values). ``test_triinterpcubic_cg_solver`` and
``test_triinterpcubic_geom_weights`` test private helpers; they are
plt-rs unit tests of its conjugate gradient solver and angle weights
(``crates/plt/src/tri/interpolate/dof.rs``).
"""

import numpy as np
import matplotlib.tri as mtri
from matplotlib.path import Path

from probes.test_contour import raw_paths_op, tricontourf_op
from probes.test_tri import (
    _circle_ratios, _cubic_call, _cubic_gradient, _delaunay_counts, _delaunay_order,
    _explicit, _flat_tri_mask, _linear_call, _masked_list, _refine_field,
    _refine_triangulation, _scale_factors,
)
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_triangulation")


def _flat(values):
    return np.ravel(np.asarray(values, dtype=float)).tolist()


def meshgrid_triangles(n):
    """``(2 * (n - 1) ** 2, 3)`` triangles meshing an ``(n, n)`` meshgrid."""
    tri = []
    for i in range(n - 1):
        for j in range(n - 1):
            a, b = i + j * n, (i + 1) + j * n
            c, d = i + (j + 1) * n, (i + 1) + (j + 1) * n
            tri += [[a, b, d], [a, d, c]]
    return tri


# --- Triangulation ---------------------------------------------------------

X4 = [-1.0, 0.0, 1.0, 0.0]
Y4 = [0.0, -1.0, 0.0, 1.0]


def test_triangulation_init():
    probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=X4, y=[1.0, 2.0])
    for triangles in ([0, 1, 2], "other", [[0, 1, 99]], [[0, 1, -1]]):
        probes.probe("tri.Triangulation.explicit", _explicit, x=X4, y=Y4,
                     triangles=triangles, mask=None, set_mask_after=False)


def test_triangulation_set_mask():
    triangles = [[0, 1, 2], [2, 3, 0]]
    for mask in (None, [False, True], [False, True, False], [False], [True], False, True):
        probes.probe("tri.Triangulation.explicit", _explicit, x=X4, y=Y4, triangles=triangles,
                     mask=mask, set_mask_after=True)


def test_delaunay():
    x, y = np.meshgrid(np.linspace(0.0, 1.0, 5), np.linspace(0.0, 1.0, 4))
    x, y = _flat(x), _flat(y)
    r = probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=x, y=y)
    assert r == {"ntri": 24, "nedges": 43, "points_used": list(range(20))}
    probes.probe("tri.Triangulation.delaunay_order", _delaunay_order, x=x, y=y)


def test_delaunay_duplicate_points():
    r = rng(23)
    x, y = r.uniform(10), r.uniform(10)
    x[7], y[7] = x[3], y[3]
    used = probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts,
                        x=x.tolist(), y=y.tolist())["points_used"]
    assert used == [i for i in range(10) if i != 7]
    probes.probe("tri.Triangulation.delaunay_order", _delaunay_order, x=x.tolist(), y=y.tolist())


def test_delaunay_points_in_line():
    x = np.linspace(0.0, 10.0, 11).tolist()
    probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=x, y=x)
    probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=x + [2.0],
                 y=x + [8.0])


def test_delaunay_insufficient_points():
    for x, y in [([], []), ([1], [5]), ([1, 2], [5, 6]), ([1, 2, 1], [5, 6, 5]),
                 ([1, 2, 2], [5, 6, 6]), ([1, 1, 1, 2, 1, 2], [5, 5, 5, 6, 5, 6])]:
        probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=x, y=y)


ROBUST_POINTS = [
    [0.8660254037844384, -0.5000000000000004],
    [0.7577722283113836, -0.5000000000000004],
    [0.6495190528383288, -0.5000000000000003],
    [0.5412658773652739, -0.5000000000000003],
    [0.811898816047911, -0.40625000000000044],
    [0.7036456405748561, -0.4062500000000004],
    [0.5953924651018013, -0.40625000000000033]]
ROBUST_QUERIES = [[0.58, -0.46], [0.65, -0.46], [0.65, -0.42], [0.7, -0.48], [0.7, -0.44],
                  [0.75, -0.44], [0.8, -0.48]]


def _tris_contain_point(x, y, points):
    t = mtri.Triangulation(x, y)
    return [sum(bool(Path(np.column_stack((t.x[tri], t.y[tri]))).contains_point(p))
                for tri in t.triangles) for p in points]


def test_delaunay_robust():
    x, y = [p[0] for p in ROBUST_POINTS], [p[1] for p in ROBUST_POINTS]
    counts = probes.probe("tri.Triangulation.tris_contain_point", _tris_contain_point,
                          x=x, y=y, points=ROBUST_QUERIES)
    assert counts == [1] * 7
    probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=x[1:], y=y[1:])


# --- TriFinder -------------------------------------------------------------

GRID_X, GRID_Y = (_flat(v) for v in np.meshgrid(np.arange(4), np.arange(4)))
GRID_T = [[0, 1, 4], [1, 5, 4], [1, 2, 5], [2, 6, 5], [2, 3, 6], [3, 7, 6], [4, 5, 8],
          [5, 9, 8], [5, 6, 9], [6, 10, 9], [6, 7, 10], [7, 11, 10], [8, 9, 12], [9, 13, 12],
          [9, 10, 13], [10, 14, 13], [10, 11, 14], [11, 15, 14]]
GRID_MASK = [8 <= i < 10 for i in range(18)]


def _trifinder(x, y, triangles, mask, set_mask, qx, qy):
    t = mtri.Triangulation(x, y, triangles, mask)
    finder = t.get_trifinder()
    if set_mask is not None:
        t.set_mask(set_mask)
    return np.ravel(finder(qx, qy)).tolist()


def _find(qx, qy, x=GRID_X, y=GRID_Y, triangles=GRID_T, mask=GRID_MASK, set_mask=None):
    return probes.probe("tri.TrapezoidMapTriFinder.__call__", _trifinder, x=x, y=y,
                        triangles=triangles, mask=mask, set_mask=set_mask, qx=_flat(qx),
                        qy=_flat(qy))


def test_trifinder():
    xs, ys = np.meshgrid([0.25, 1.25, 2.25, 3.25], [0.25, 1.25, 2.25, 3.25])
    assert _find(xs, ys) == [0, 2, 4, -1, 6, -1, 10, -1, 12, 14, 16, -1, -1, -1, -1, -1]
    assert _find(xs - 0.5, ys - 0.5) == [-1, -1, -1, -1, -1, 1, 3, 5, -1, 7, -1, 11, -1, 13,
                                         15, 17]
    _find([0.5, 1.5, 2.5, 0.5, 1.5, 2.5, 1.5, 1.5, 0.0, 1.0, 2.0, 3.0],
          [0.0, 0.0, 0.0, 3.0, 3.0, 3.0, 1.0, 2.0, 1.5, 1.5, 1.5, 1.5])
    _find([0.0, 3.0], [0.0, 3.0])
    # Horizontal colinear points.
    xs, ys = np.meshgrid([-0.1, 0.4, 0.9, 1.4, 1.9, 2.4, 2.9], [-0.1, 0.1])
    _find(xs, ys, x=[1.5, 0, 1, 2, 3, 1.5, 1.5], y=[-1, 0, 0, 0, 0, 0.0, 1],
          triangles=[[0, 2, 1], [0, 3, 2], [0, 4, 3], [1, 2, 5], [2, 3, 5], [3, 4, 5],
                     [1, 5, 6], [4, 6, 5]], mask=None)
    # Vertical colinear points.
    xs, ys = np.meshgrid([-0.1, 0.1], [-0.1, 0.4, 0.9, 1.4, 1.9, 2.4, 2.9])
    _find(xs, ys, x=[-1, -0.0, 0, 0, 0, 0, 1], y=[1.5, 1.5, 0, 1, 2, 3, 1.5],
          triangles=[[0, 1, 2], [0, 1, 5], [1, 2, 3], [1, 3, 4], [1, 4, 5], [2, 6, 3],
                     [3, 6, 4], [4, 6, 5]], mask=None)
    # A new mask reaches a finder made before it.
    square = dict(x=[0, 1, 0, 1], y=[0, 0, 1, 1], triangles=[[0, 1, 2], [1, 3, 2]], mask=None)
    qx, qy = [-0.2, 0.2, 0.8, 1.2], [0.5, 0.5, 0.5, 0.5]
    _find(qx, qy, **square)
    _find(qx, qy, **square, set_mask=[True, False])


# --- interpolation ---------------------------------------------------------

def _values(op, fn, tol=None, **args):
    return probes.probe(op, fn, tol=tol, **args)


def _interpolate(x, y, triangles, mask, z, qx, qy, kinds=("linear", "min_E", "geom"),
                 tol=None):
    for kind in kinds:
        common = dict(x=x, y=y, triangles=triangles, mask=mask, z=z, qx=_flat(qx),
                      qy=_flat(qy))
        if kind == "linear":
            _values("tri.LinearTriInterpolator.__call__", _linear_call, tol=tol, **common)
        else:
            _values("tri.CubicTriInterpolator.__call__", _cubic_call, tol=tol, kind=kind,
                    dz=None, **common)


def _quad(x, y):
    a, b, c = 1.23, -4.79, 0.6
    return a * (x - 0.5) ** 2 + b * (y - 0.5) ** 2 + c * x * y


def _gradient_quad(x, y):
    a, b, c = 1.23, -4.79, 0.6
    return (2 * a * (x - 0.5) + c * y, 2 * b * (y - 0.5) + c * x)


PATCH_X = [0.2, 0.33367, 0.669, 0., 1., 1., 0.]
PATCH_Y = [0.3, 0.80755, 0.4335, 0., 0., 1., 1.]
PATCH_T = [[0, 1, 2], [3, 0, 4], [4, 0, 2], [4, 2, 5], [1, 5, 2], [6, 5, 1], [6, 1, 0], [6, 0, 3]]


def test_triinterp():
    z = (1.23 * np.asarray(GRID_X) - 4.79 * np.asarray(GRID_Y)).tolist()
    grid = dict(x=GRID_X, y=GRID_Y, triangles=GRID_T, mask=GRID_MASK, z=z)
    inside = np.meshgrid(np.linspace(0.25, 2.75, 6), [0.25, 0.75, 2.25, 2.75])
    _interpolate(**grid, qx=inside[0], qy=inside[1])
    outside = np.meshgrid([-0.25, 1.25, 1.75, 3.25], [-0.25, 1.25, 1.75, 3.25])
    _interpolate(**grid, qx=outside[0], qy=outside[1])
    mixed = np.meshgrid(np.linspace(0.25, 1.75, 6), [0.25, 0.75, 1.25, 1.75])
    _interpolate(**grid, qx=mixed[0], qy=mixed[1])
    # Second order patch test of the cubic interpolator given the gradients.
    x, y = np.asarray(PATCH_X), np.asarray(PATCH_Y)
    dz = [list(g) for g in _gradient_quad(x, y)]
    xs, ys = np.meshgrid(np.linspace(0., 1., 5), np.linspace(0., 1., 5))
    user = dict(x=PATCH_X, y=PATCH_Y, triangles=PATCH_T, mask=None, z=_quad(x, y).tolist(),
                kind="user", dz=dz)
    _values("tri.CubicTriInterpolator.__call__", _cubic_call, **user, qx=_flat(xs), qy=_flat(ys))
    _values("tri.CubicTriInterpolator.gradient", _cubic_gradient, **user, qx=PATCH_X, qy=PATCH_Y)
    # Cubic interpolation does better than linear on a dense mesh.
    n = 11
    x, y = (np.ravel(v) for v in np.meshgrid(np.linspace(0., 1., n + 1),
                                             np.linspace(0., 1., n + 1)))
    xs, ys = np.meshgrid(np.linspace(0.1, 0.9, 5), np.linspace(0.1, 0.9, 5))
    _interpolate(x.tolist(), y.tolist(), meshgrid_triangles(n + 1), None, _quad(x, y).tolist(),
                 xs, ys)


def _star(loc):
    n_star, epsilon = 24, 1.e-10
    angles = np.linspace(0., 2 * np.pi, n_star)
    return ([loc[0]] + (loc[0] + epsilon * np.cos(angles)).tolist(),
            [loc[1]] + (loc[1] + epsilon * np.sin(angles)).tolist())


def test_triinterpcubic_C1_continuity():
    (ax, ay), (bx, by), (cx, cy) = (PATCH_X[0], PATCH_Y[0]), (PATCH_X[1], PATCH_Y[1]), \
        (PATCH_X[2], PATCH_Y[2])
    places = [(ax, ay), (bx, by), (cx, cy), ((ax + bx) * 0.5, (ay + by) * 0.5),
              ((ax + cx) * 0.5, (ay + cy) * 0.5), ((cx + bx) * 0.5, (cy + by) * 0.5),
              ((ax + bx + cx) / 3., (ay + by + cy) / 3.),
              ((4. * ax + bx + cx) / 6., (4. * ay + by + cy) / 6.),
              ((ax + 4. * bx + cx) / 6., (ay + 4. * by + cy) / 6.),
              ((ax + bx + 4. * cx) / 6., (ay + by + 4. * cy) / 6.)]
    for idof in range(9):
        z, dzx, dzy = np.zeros(7), np.zeros(7), np.zeros(7)
        [z, dzx, dzy][idof // 3][idof % 3] = 1.0
        user = dict(x=PATCH_X, y=PATCH_Y, triangles=PATCH_T, mask=None, z=z.tolist(),
                    kind="user", dz=[dzx.tolist(), dzy.tolist()])
        for place in places:
            qx, qy = _star(place)
            _values("tri.CubicTriInterpolator.__call__", _cubic_call, tol=1e-6, **user,
                    qx=qx, qy=qy)
            _values("tri.CubicTriInterpolator.gradient", _cubic_gradient, tol=1e-6, **user,
                    qx=qx, qy=qy)


COLINEAR_X = np.array([1.5, 0, 1, 2, 3, 1.5, 1.5])
COLINEAR_Y = np.array([-1, 0, 0, 0, 0, 0.0, 1])
COLINEAR_T = [[0, 2, 1], [0, 3, 2], [0, 4, 3], [1, 2, 5], [2, 3, 5], [3, 4, 5], [1, 5, 6],
              [4, 6, 5]]
MULTIKEYS = {"linear": mtri.LinearTriInterpolator,
             "cubic_min_E": lambda t, z: mtri.CubicTriInterpolator(t, z),
             "cubic_geom": lambda t, z: mtri.CubicTriInterpolator(t, z, kind="geom")}


def _multikeys(x, y, triangles, z, interpolator, qx, qy, tri):
    interp = MULTIKEYS[interpolator](mtri.Triangulation(x, y, triangles), z)
    zs, = interp._interpolate_multikeys(np.asarray(qx), np.asarray(qy),
                                        tri_index=tri * np.ones(len(qx), dtype=np.int32))
    return _masked_list(zs)


def test_triinterp_colinear():
    for a, b in [[1, 0], [0, 1], [1, 1], [1, 2], [-2, -1], [-2, 1]]:
        x = (a * COLINEAR_X + b * COLINEAR_Y)
        y = (-b * COLINEAR_X + a * COLINEAR_Y)
        z = (1.23 * x - 4.79 * y).tolist()
        xs, ys = np.meshgrid(np.linspace(np.min(x), np.max(x), 20),
                             np.linspace(np.min(y), np.max(y), 20))
        _interpolate(x.tolist(), y.tolist(), COLINEAR_T, None, z, xs, ys)
        t = mtri.Triangulation(x, y, COLINEAR_T)
        pt1, pt2 = t.triangles[4, 0], t.triangles[4, 1]
        qx = np.linspace(t.x[pt1], t.x[pt2], 10).tolist()
        qy = np.linspace(t.y[pt1], t.y[pt2], 10).tolist()
        for interpolator in MULTIKEYS:
            probes.probe("TriInterpolator._interpolate_multikeys", _multikeys, x=x.tolist(),
                         y=y.tolist(), triangles=COLINEAR_T, z=z, interpolator=interpolator,
                         qx=qx, qy=qy, tri=4)


def _transformations_field(x, y):
    r1 = np.hypot(0.5 - x, 0.5 - y)
    theta1 = np.arctan2(0.5 - x, 0.5 - y)
    r2 = np.hypot(-x - 0.2, -y - 0.2)
    theta2 = np.arctan2(-x - 0.2, -y - 0.2)
    z = -(2 * (np.exp((r1 / 10) ** 2) - 1) * 30. * np.cos(7. * theta1) +
          (np.exp((r2 / 10) ** 2) - 1) * 30. * np.cos(11. * theta2) +
          0.7 * (x ** 2 + y ** 2))
    return (np.max(z) - z) / (np.max(z) - np.min(z))


def test_triinterp_transformations():
    n_angles, n_radii = 20, 10
    radii = np.linspace(0.15, 0.95, n_radii)
    angles = np.linspace(0 + n_angles, 2 * np.pi + n_angles, n_angles, endpoint=False)
    angles = np.repeat(angles[..., np.newaxis], n_radii, axis=1)
    angles[:, 1::2] += np.pi / n_angles
    x0 = (radii * np.cos(angles)).flatten()
    y0 = (radii * np.sin(angles)).flatten()
    triangles = mtri.Triangulation(x0, y0).triangles.tolist()
    z0 = _transformations_field(x0, y0).tolist()
    xs0, ys0 = (np.ravel(v) for v in np.meshgrid(np.linspace(-1., 1., 23),
                                                 np.linspace(-1., 1., 23)))
    theta = 2 * np.pi / n_angles
    configurations = [
        (x0, y0, xs0, ys0),
        (np.cos(theta) * x0 + np.sin(theta) * y0, -np.sin(theta) * x0 + np.cos(theta) * y0,
         np.cos(theta) * xs0 + np.sin(theta) * ys0, -np.sin(theta) * xs0 + np.cos(theta) * ys0),
        (987654.3210 * x0, y0, 987654.3210 * xs0, ys0),
        (x0, 987654.3210 * y0, xs0, 987654.3210 * ys0),
    ]
    for x, y, xs, ys in configurations:
        _interpolate(x.tolist(), y.tolist(), triangles, None, z0, xs, ys, tol=1e-6)


# --- TriAnalyzer -----------------------------------------------------------

def test_tritools():
    tools = dict(x=[0., 1., 0.5, 0., 2.], y=[0., 0., 0.5 * np.sqrt(3.), -1., 1.],
                 triangles=[[0, 1, 2], [0, 1, 3], [1, 2, 4]], mask=[False, False, True])
    probes.probe("tri.TriAnalyzer.scale_factors", _scale_factors, **tools)
    probes.probe("tri.TriAnalyzer.circle_ratios", _circle_ratios, **tools, rescale=False)
    probes.probe("tri.TriAnalyzer.circle_ratios", _circle_ratios, x=[0., 1., 2.],
                 y=[1., 4., 7.], triangles=[[0, 1, 2]], mask=None, rescale=True)
    n = 9
    v = np.linspace(-1., 1., n + 1)
    x, y = np.meshgrid(np.abs(v) ** 2. * np.sign(v), np.abs(v) ** 0.25 * np.sign(v))
    mesh = dict(x=_flat(x), y=_flat(y), triangles=meshgrid_triangles(n + 1))
    flat = probes.probe("tri.TriAnalyzer.get_flat_tri_mask", _flat_tri_mask, **mesh, mask=None,
                        min_circle_ratio=0.2, rescale=True)
    corners = [0, 1, 2, 3, 14, 15, 16, 17, 18, 19, 34, 35, 126, 127, 142, 143, 144, 145, 146,
               147, 158, 159, 160, 161]
    assert [i for i, m in enumerate(flat) if m] == corners
    probes.probe("tri.TriAnalyzer.get_flat_tri_mask", _flat_tri_mask, **mesh,
                 mask=[i == 80 for i in range(162)], min_circle_ratio=0.2, rescale=True)


# --- UniformTriRefiner -----------------------------------------------------

def test_trirefine():
    n = 3
    x, y = np.meshgrid(np.linspace(-1., 1., n + 1), np.linspace(-1., 1., n + 1))
    probes.probe("tri.UniformTriRefiner.refine_triangulation", _refine_triangulation,
                 x=_flat(x), y=_flat(y), triangles=meshgrid_triangles(n + 1),
                 mask=[i >= n ** 2 for i in range(2 * n ** 2)], subdiv=2, return_tri_index=False)
    x, y = [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]
    z = np.hypot(np.asarray(x) - 0.3, np.asarray(y) - 0.4).tolist()
    fields = [probes.probe("tri.UniformTriRefiner.refine_field", _refine_field, tol=1e-6, x=x,
                           y=y, triangles=triangles, mask=None, z=z, interpolator=None, dz=None,
                           subdiv=1)
              for triangles in ([[0, 1, 3], [3, 2, 0]], [[0, 1, 3], [2, 0, 3]])]
    np.testing.assert_array_almost_equal(fields[0], fields[1])


def test_trirefine_masked():
    x, y = np.mgrid[:2, :2]
    x, y = np.repeat(x.flatten(), 2).tolist(), np.repeat(y.flatten(), 2).tolist()
    for interpolator in ("linear", "cubic_min_E"):
        probes.probe("tri.UniformTriRefiner.refine_field", _refine_field, x=x, y=y,
                     triangles=None, mask=None, z=[0.0] * 8, interpolator=interpolator, dz=None,
                     subdiv=2)


# --- Delaunay neighbours ---------------------------------------------------

def _neighbors_match(x, y):
    t = mtri.Triangulation(x, y)
    qhull = t.neighbors.copy()
    t._neighbors = None
    return bool(np.array_equal(qhull, t.neighbors))


def test_qhull_triangle_orientation():
    xi = np.linspace(-2, 2, 100)
    x, y = map(np.ravel, np.meshgrid(xi, xi))
    w = (x > y - 1) & (x < -1.95) & (y > -1.2)
    x, y = x[w], y[w]
    theta = np.radians(25)
    x1 = (x * np.cos(theta) - y * np.sin(theta)).tolist()
    y1 = (x * np.sin(theta) + y * np.cos(theta)).tolist()
    probes.probe("tri.Triangulation.neighbors", _neighbors_match, x=x1, y=y1)
    probes.probe("tri.Triangulation.delaunay_counts", _delaunay_counts, x=x1, y=y1)


# --- contours --------------------------------------------------------------

def _delaunay_triangles(x, y):
    return mtri.Triangulation(x, y).triangles.tolist()


def test_tricontourf_decreasing_levels():
    x, y = [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]
    probes.probe("Axes.tricontourf", tricontourf_op, x=x, y=y,
                 triangles=_delaunay_triangles(x, y), z=[0.2, 0.4, 0.6], levels=[1.0, 0.0])


def test_tricontour_non_finite_z():
    x, y = [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]
    for bad in (np.inf, -np.inf, np.nan):
        probes.probe("Axes.tricontourf", tricontourf_op, x=x, y=y,
                     triangles=_delaunay_triangles(x, y), z=[0.0, 1.0, 2.0, bad])


SQUARE_X = [0.0, 4.0, 4.0, 0.0, 2.0]
SQUARE_Y = [0.0, 0.0, 4.0, 4.0, 2.0]


def _paths(func, z, levels):
    return probes.probe("ContourSet.get_paths", raw_paths_op, func=func, x=SQUARE_X, y=SQUARE_Y,
                        triangles=_delaunay_triangles(SQUARE_X, SQUARE_Y), z=z, levels=levels)


def test_tricontour_path():
    _paths("tricontour", [1.0, 0.0, 0.0, 0.0, 0.0], [0.5])
    _paths("tricontour", [0.0, 0.0, 0.0, 0.0, 1.0], [0.5])


def test_tricontourf_path():
    _paths("tricontourf", [0.0, 0.0, 0.0, 0.0, 1.0], [0.5, 1.5])
    _paths("tricontourf", [1.0, 0.0, 0.0, 0.0, 0.0], [0.5, 1.5])
    _paths("tricontourf", [0.0, 0.0, 0.0, 0.0, 1.0], [-0.5, 0.5])
