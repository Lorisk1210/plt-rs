"""Bezier utilities, ported from matplotlib's own ``tests/test_bezier.py``:
the real polynomial roots in [0, 1] that the extrema of Bezier curves come
from, and the splitting of a curve at a closed path for coordinates so
large that the bisection runs out of floating-point precision.

Area ``mpl_bezier``.

Ops
---
``bezier._real_roots_in_01``  {coeffs} -> the sorted real roots in [0, 1]
    of ``sum(c[k] * t**k)`` (ascending coefficients).
``bezier.split_bezier_intersecting_with_closedpath``  {bezier, circle,
    tolerance} -> ``[left, right]`` control points of the curve split where
    it leaves ``inside_circle(*circle)``.

Cases
-----
``test_real_roots_in_01`` records the roots of the coefficients
``np.poly(roots)[::-1].real`` of each upstream root set;
``test_real_roots_in_01_no_roots`` its two coefficient lists and an empty one;
``test_split_bezier_with_large_values`` the split it must complete
(gh-27753).
"""

import numpy as np
from matplotlib import bezier as mbezier

from spec_support import ProbeSet

probes = ProbeSet("mpl_bezier")


def _roots(coeffs):
    return mbezier._real_roots_in_01(np.asarray(coeffs, float)).tolist()


def _split(bezier, circle, tolerance):
    left, right = mbezier.split_bezier_intersecting_with_closedpath(
        np.asarray(bezier, float), mbezier.inside_circle(*circle), tolerance=tolerance)
    return [np.asarray(left), np.asarray(right)]


def test_real_roots_in_01():
    for roots in ([0.5], [0.25, 0.75], [0.2, 0.5, 0.8], [0.1, 0.2, 0.3, 0.4], [0.0, 0.5],
                  [0.5, 1.0], [2.0], [0.5, 2.0], [-1j, 1j], [0.5, -1j, 1j], [0.3, 0.3]):
        coeffs = np.poly(np.array(roots))[::-1].real.tolist()
        probes.probe("bezier._real_roots_in_01", _roots, tol=1e-10, coeffs=coeffs)


def test_real_roots_in_01_no_roots():
    for coeffs in ([5], [0, 0, 0], []):
        probes.probe("bezier._real_roots_in_01", _roots, coeffs=coeffs)


def test_split_bezier_with_large_values():
    arrow_path = [[96950809781500.0, 804.7503795623779],
                  [96950809781500.0, 859.6242585800646],
                  [96950809781500.0, 914.4981375977513]]
    probes.probe("bezier.split_bezier_intersecting_with_closedpath", _split,
                 bezier=arrow_path, circle=[96950809781500.0, 804.7503795623779, 0.06],
                 tolerance=0.01)
