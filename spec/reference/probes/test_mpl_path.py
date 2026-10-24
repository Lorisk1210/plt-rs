"""matplotlib's own tests of ``matplotlib.path`` (``tests/test_path.py``),
with the tests' own inputs: the empty closed path, containment of paths
and points, exact extents of curves, compound paths, clipping to polygons,
segment intersections under rotation, full-turn arcs, NaN cleanup and
interpolation of closed subpaths.

Area ``mpl_path``. Arguments follow the ``path`` area (``test_path.py``):
a path is ``{"vertices": [[x, y], ...], "codes": [int, ...] | null}``
decoded as ``Path(np.asarray(vertices, float).reshape(-1, 2), codes)``, a
box ``[[x0, y0], [x1, y1]]``, a transform ``null`` or an ``Affine2D``
operation list.

Ops
---
``path.Path``  {vertices, closed} -> the path dict of ``Path(np.asarray(
    vertices, float).reshape(-1, 2), closed=closed)``: unlike the ``path``
    area's op, ``[]`` is the empty ``(0, 2)`` path.
``path.Path.get_extents``  {path, transform} -> box (the ``path`` area's).
``path.Path.contains_points``  {path, points, transform, radius} -> [bool]
    (the ``path`` area's).
``path.Path.contains_path``  {path, other, transform} -> bool (the ``path``
    area's).
``path.Path.make_compound_path``  {paths} -> path dict (the ``path``
    area's).
``path.Path.clip_to_bbox.to_polygons``  {path, bbox} -> the polygons of
    ``path.clip_to_bbox(Bbox(bbox)).to_polygons()``.
``path.Path.to_polygons``  {path, transform, width, height, closed_only,
    rc} -> list of point lists (the ``path`` area's).
``path.Path.intersects_path``  {path, other, filled} -> bool (the ``path``
    area's).
``path.Path.arc``  {theta1, theta2, n, is_wedge} -> path dict (the ``path``
    area's).
``path.Path.cleaned``  {path, transform, remove_nans, clip, simplify,
    curves, stroke_width, snap, rc} -> path dict with the trailing ``STOP``
    (the ``path`` area's).
``path.Path.interpolated``  {path, steps} -> path dict (the ``path``
    area's).
``axes.Axes.axvspan.contains_point``  {xscale, ylim, span, point} -> whether
    the path of ``ax.axvspan(*span)``, in a ``plt.subplots()`` axes set to
    ``xscale`` and ``ylim``, contains the display point of data ``point``
    through the span's (non-affine) transform.

Cases
-----
Named after the upstream tests. ``test_path_intersect_path`` rotates its
segments with ``Affine2D().rotate(np.deg2rad(phi))`` here and passes the
rotated vertices, and records each pair both ways; the arc tests record
both arcs they compare.

Already covered
---------------
By the ``path`` area: ``test_path_exceptions`` (bad shapes and codes),
``test_point_in_path``, ``test_extents_with_ignored_codes``,
``test_point_in_path_nan``, ``test_interpolated_closepoly`` and
``test_interpolated_empty_path`` (same behaviour, same or equivalent
inputs). By the ``mpl_lines`` figure group: ``test_path_clipping``,
``test_log_transform_with_zero``, ``test_xkcd``, ``test_xkcd_marker``,
``test_marker_paths_pdf`` and ``test_nan_isolated_points``.

Left out
--------
``test_readonly_path`` (Python attribute mutability: plt-rs paths are
values), ``test_path_deepcopy``, ``test_path_deepcopy_cycle`` and
``test_path_shallowcopy`` (Python copy semantics), and
``test_arrow_contains_point`` (see the ``mpl_lines`` group: it hit-tests
arrows before their first draw).
"""

import numpy as np
from matplotlib import transforms
from matplotlib.figure import Figure
from matplotlib.path import Path

from probes.test_path import (
    REF_RC, _arc, _cleaned, _compound, _const, _contains_path, _contains_points, _extents,
    _interpolated, _intersects_path, _path, _pd, _tan_differs, _to_polygons, pth,
)
from spec_support import ProbeSet

probes = ProbeSet("mpl_path")

M, L, C3, C4, CL, ST = 1, 2, 3, 4, 79, 0
NAN = float("nan")
EMPTY = {"vertices": [], "codes": None}


def _new_path(vertices, closed):
    return _pd(Path(np.asarray(vertices, float).reshape(-1, 2), closed=closed))


def test_empty_closed_path():
    r = probes.probe("path.Path", _new_path, vertices=[], closed=True)
    assert r["codes"] is None
    probes.probe("path.Path.get_extents", _extents, path=EMPTY, transform=None)


def _closed(vertices):
    """``Path(vertices, closed=True)`` as a path dict."""
    return pth(vertices, [M] + [L] * (len(vertices) - 2) + [CL])


def test_contains_path():
    path = _closed([(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)])
    for other in [
            _closed([(0.25, 0.25), (0.25, 0.75), (0.75, 0.75), (0.75, 0.25), (0.25, 0.25)]),
            _closed([(-0.25, -0.25), (-0.25, 1.75), (1.75, 1.75), (1.75, -0.25), (-0.25, -0.25)]),
            _closed([(-0.25, -0.25), (-0.25, 1.75), (0.5, 0.5), (1.75, 1.75), (1.75, -0.25),
                     (-0.25, -0.25)]),
            _closed([(0.25, 0.25), (0.25, 1.25), (1.25, 1.25), (1.25, 0.25), (0.25, 0.25)]),
            _closed([(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]),
            _closed([(2, 2), (2, 3), (3, 3), (3, 2), (2, 2)])]:
        probes.probe("path.Path.contains_path", _contains_path, path=path, other=other,
                     transform=None)
        probes.probe("path.Path.contains_path", _contains_path, path=other, other=path,
                     transform=None)


def test_contains_points_negative_radius():
    r = probes.probe("path.Path.contains_points", _contains_points,
                     path=_const(Path.unit_circle()), points=[[0, 0], [1.25, 0], [0.9, 0.9]],
                     transform=None, radius=-0.5)
    assert list(r) == [True, False, False]


def test_exact_extents():
    for path, extents in [
            (pth([[0, 0], [1, 0], [1, 1], [0, 1]], [M, C4, C4, C4]), (0, 0, 0.75, 1)),
            (pth([[0, 0], [0, 1], [1, 0]], [M, C3, C3]), (0, 0, 1, 0.5)),
            (pth([[0, 1], [1, 1]], [M, L]), (0, 1, 1, 1))]:
        r = probes.probe("path.Path.get_extents", _extents, path=path, transform=None)
        assert np.array_equal(np.ravel(r), extents)


def _span_contains(xscale, ylim, span, point):
    ax = Figure().subplots()
    ax.set(xscale=xscale, ylim=ylim)
    polygon = ax.axvspan(*span)
    return bool(polygon.get_path().contains_point(ax.transData.transform(point),
                                                  polygon.get_transform()))


def test_nonlinear_containment():
    for point, inside in [([5, 0.5], True), ([0.5, 0.5], False), ([50, 0.5], False)]:
        r = probes.probe("axes.Axes.axvspan.contains_point", _span_contains, xscale="log",
                         ylim=[0, 1], span=[1, 10], point=point)
        assert r is inside


def test_make_compound_path_empty():
    r = probes.probe("path.Path.make_compound_path", _compound, paths=[EMPTY, EMPTY])
    assert np.shape(r["codes"]) == (0,)
    r = probes.probe("path.Path.make_compound_path", _compound, paths=[pth([[0, 0]]), EMPTY])
    assert np.shape(r["codes"]) == (1,)


def test_make_compound_path_stops():
    r = probes.probe("path.Path.make_compound_path", _compound,
                     paths=3 * [pth([[0, 0], [0, 0]], [M, ST])])
    assert ST not in list(r["codes"])


HAND = [
    [1.64516129, 1.16145833], [1.64516129, 1.59375], [1.35080645, 1.921875],
    [1.375, 2.18229167], [1.68548387, 1.9375], [1.60887097, 2.55208333],
    [1.68548387, 2.69791667], [1.76209677, 2.56770833], [1.83064516, 1.97395833],
    [1.89516129, 2.75], [1.9516129, 2.84895833], [2.01209677, 2.76041667],
    [1.99193548, 1.99479167], [2.11290323, 2.63020833], [2.2016129, 2.734375],
    [2.25403226, 2.60416667], [2.14919355, 1.953125], [2.30645161, 2.36979167],
    [2.39112903, 2.36979167], [2.41532258, 2.1875], [2.1733871, 1.703125],
    [2.07782258, 1.16666667]]


def _clipped_polygons(path, bbox):
    clipped = _path(path).clip_to_bbox(transforms.Bbox(np.asarray(bbox, float)))
    return [np.asarray(poly) for poly in clipped.to_polygons()]


def test_path_no_doubled_point_in_to_polygon():
    poly = _closed([[y, x] for x, y in HAND])
    r = probes.probe("path.Path.clip_to_bbox.to_polygons", _clipped_polygons, path=poly,
                     bbox=[[1.0, 1.5], [2.1, 2.5]])
    assert np.all(r[0][-2] != r[0][-1]) and np.all(r[0][-1] == r[0][0])


def test_path_to_polygons():
    for data in ([[10, 10], [20, 20]], [[10, 10], [20, 20], [30, 30]]):
        for size in (40, 0):
            for closed_only in (True, False):
                probes.probe("path.Path.to_polygons", _to_polygons, path=pth(data),
                             transform=None, width=size, height=size, closed_only=closed_only,
                             rc=REF_RC)


def _rotated(transform, vertices):
    return _const(transform.transform_path(Path(vertices)))


def _intersect_both(a, b):
    for path, other in ((a, b), (b, a)):
        probes.probe("path.Path.intersects_path", _intersects_path, path=path, other=other,
                     filled=True)


def test_path_intersect_path():
    phis = np.concatenate([np.array([0, 15, 30, 45, 60, 75, 90, 105, 120, 135]) + delta
                           for delta in [-1, 0, 1]])
    for phi in phis:
        t = transforms.Affine2D().rotate(np.deg2rad(phi))
        rot = lambda vertices: _rotated(t, vertices)  # noqa: E731
        _intersect_both(pth([(-2, 0), (2, 0)]), rot([(-2, 0), (2, 0)]))
        _intersect_both(pth([(0, 0), (2, 0)]), rot([(0, 0), (2, 0)]))
        pairs = [([(0, 1), (0, 3)], [(1, 3), (0, 3)]),
                 ([(0, 1), (0, 3)], [(0, 5), (0, 3)]),
                 ([(0, 0), (5, 5)], [(1, 1), (3, 3)]),
                 ([(0, 1), (0, 5)], [(3, 0), (3, 3)]),
                 ([(0, 1), (0, 5)], [(0, 6), (0, 7)]),
                 ([(0, 1), (0, 5)], [(0, 1), (0, 2), (0, 5)]),
                 ([(1, -1), (0, -1)], [(0, 1), (0.9, 1)]),
                 ([(0., -5.), (1., -5.)], [(1., 5.), (0., 5.)])]
        for eps in [1e-5, 1e-8, 1e-10, 1e-12]:
            pairs += [([(0, 1), (0, 5)], [(0 + eps, 1), (0 + eps, 5)]),
                      ([(0, 1), (0, 5)], [(0, 5 + eps), (0, 7)]),
                      ([(0, 1), (0, 5)], [(0, 5 - eps), (0, 7)])]
        for a, b in pairs:
            _intersect_both(rot(a), rot(b))
        collinear = rot([(0, 1), (0, 3)])
        probes.probe("path.Path.intersects_path", _intersects_path, path=collinear,
                     other=collinear, filled=True)


def _probe_arc(theta1, theta2):
    tol = 1e-14 if _tan_differs(theta1, theta2, None) else None
    probes.probe("path.Path.arc", _arc, theta1=theta1, theta2=theta2, n=None, is_wedge=False,
                 tol=tol)


def test_full_arc():
    for offset in range(-720, 361, 45):
        _probe_arc(offset, 360 + offset)


def test_arc_full_circle_snap():
    for theta2 in [360, 720, 360 * 5, np.nextafter(360, 1e6), np.nextafter(360, 0),
                   np.nextafter(720, 1e6)]:
        _probe_arc(0, float(theta2))


def test_arc_negative_full_circle():
    for theta1, theta2 in [(0, -360), (0, -720), (360, 0), (10, -350)]:
        _probe_arc(theta1, theta2)
        _probe_arc(theta1, theta1 + 360)


def test_arc_unwrap_partial_turn():
    for theta2, equivalent in [(410, 50), (540, 180), (360.001, 0.001)]:
        _probe_arc(0, theta2)
        _probe_arc(0, equivalent)


def test_disjoint_zero_length_segment():
    this_path = pth([[824.85064295, 2056.26489203], [861.69033931, 2041.00539016],
                     [868.57864109, 2057.63522175], [831.73894473, 2072.89472361],
                     [824.85064295, 2056.26489203]], [M, L, L, L, CL])
    outline_path = pth([[859.91051028, 2165.38461538], [859.06772495, 2149.30331334],
                        [859.06772495, 2181.46591743], [859.91051028, 2165.38461538],
                        [859.91051028, 2165.38461538]], [M, L, L, L, L])
    _intersect_both(outline_path, this_path)


def test_intersect_zero_length_segment():
    _intersect_both(pth([[1, 0], [.5, .5], [.5, .5], [0, 1]]), pth([[0, 0], [1, 1]]))


def test_cleanup_closepoly():
    for path in [pth([[NAN, NAN], [NAN, NAN]], [M, CL]),
                 pth([[NAN, NAN]] * 4, [M, C3, C3, CL])]:
        r = probes.probe("path.Path.cleaned", _cleaned, path=path, transform=None,
                         remove_nans=True, clip=None, simplify=False, curves=False,
                         stroke_width=1.0, snap=False, rc=REF_RC)
        assert list(r["codes"]) == [ST]


def test_interpolated_moveto():
    path = pth([[0, 0], [0, 1], [1, 2], [4, 4], [4, 5], [5, 5]], [M, L, L] * 2)
    r = probes.probe("path.Path.interpolated", _interpolated, path=path, steps=3)
    assert list(r["codes"]) == ([M] + [L] * 6) * 2


def test_interpolated_moveto_closepoly():
    path = pth([(4, 3), (5, 4), (5, 3), (0, 0), (8, 6), (10, 8), (10, 6), (0, 0)],
               ([M] + [L] * 2 + [CL]) * 2)
    r = probes.probe("path.Path.interpolated", _interpolated, path=path, steps=2)
    assert list(r["codes"]) == ([M] + [L] * 5 + [CL]) * 2
