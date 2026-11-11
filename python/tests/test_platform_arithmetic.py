"""
What matplotlib's compiled extensions compute depends on the build of them
a platform has: the wheel for arm64 macOS fuses products into the sums they
feed (`python/DESIGN.md`, "Apple silicon"), the x86-64 wheels round the
products first. pltrs follows the wheel of the platform it runs on. The
values here are matplotlib 3.11.2's and contourpy 1.4.0's, recorded on
Linux x86-64 and on arm64 macOS.
"""

import platform
import sys

import numpy as np
import pytest

import pltrs.tri as mtri
from pltrs.figure import Figure

#: Whether matplotlib's wheel for this platform fuses.
FUSED = sys.platform == "darwin" and platform.machine() == "arm64"


def test_cocircular_points_are_triangulated_as_on_this_platform():
    x, y = (a.ravel() for a in np.meshgrid(np.linspace(0, 1, 2), np.linspace(0, 1, 5)))
    triangles = mtri.Triangulation(x, y).triangles.tolist()
    if FUSED:
        assert triangles == [[8, 6, 9], [9, 6, 7], [7, 6, 5], [5, 6, 4],
                             [3, 2, 0], [0, 1, 3], [4, 2, 3], [3, 5, 4]]
    else:
        assert triangles == [[8, 6, 9], [9, 6, 7], [3, 2, 0], [0, 1, 3],
                             [7, 6, 5], [5, 6, 4], [4, 2, 5], [2, 3, 5]]


def test_plane_coefficients_of_collinear_points_take_this_platforms_branch():
    tri = mtri.Triangulation([0.0, 0.3, 0.6, 0.1], [0.0, 0.1, 0.2, 0.7],
                             [[0, 1, 2], [0, 2, 3]])
    planes = tri.calculate_plane_coefficients(np.array([0.3, 0.7, 0.1, 0.9]))
    # A fused cross product of dependent sides keeps a residue: the plane
    # of the degenerate triangle is steep there and flat elsewhere.
    flat = [2.0 ** 54 * 5 / 3, -(2.0 ** 56) * 1.25, 0.3] if FUSED else [0.0, 0.0, 0.3]
    assert planes[0].tolist() == flat
    assert planes[1].tolist() == [-0.65, 0.9500000000000002, 0.3]


@pytest.mark.parametrize(("algorithm", "fused", "rounded"), [
    ("mpl2014", [0.09999999999999999, 0.44146341463414634],
     [0.09999999999999999, 0.4414634146341463]),
    ("serial", [0.09999999999999999, 0.44146341463414634],
     [0.09999999999999999, 0.4414634146341463]),
    ("mpl2005", [0.28913043478260875, 0.3], [0.2891304347826088, 0.3]),
])
def test_contour_crossings_round_as_on_this_platform(algorithm, fused, rounded):
    z = np.array([[0.11, 0.57, 0.23], [0.93, 0.31, 0.77]])
    contours = Figure().subplots().contour([0.1, 0.4, 0.9], [0.3, 0.7], z, [0.4],
                                           algorithm=algorithm)
    assert contours.get_paths()[0].vertices[0].tolist() == (fused if FUSED else rounded)


@pytest.mark.parametrize("use", [
    lambda tri: tri.edges,
    lambda tri: tri.neighbors,
    lambda tri: tri.get_trifinder(),
    lambda tri: tri.calculate_plane_coefficients(np.zeros(4)),
    lambda tri: tri.get_cpp_triangulation(),
    lambda tri: Figure().subplots().tricontour(tri, np.arange(4.0)),
], ids=["edges", "neighbors", "trifinder", "planes", "cpp", "tricontour"])
def test_given_triangles_are_turned_in_place_once_the_extension_has_them(use):
    given = [[0, 2, 1], [1, 2, 3]]
    tri = mtri.Triangulation([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0], given)
    assert tri.triangles.tolist() == given
    assert tri.get_masked_triangles().tolist() == given
    use(tri)
    assert tri.triangles.tolist() == [[0, 1, 2], [1, 3, 2]]
    assert tri.get_masked_triangles().tolist() == [[0, 1, 2], [1, 3, 2]]
    assert tri.neighbors.tolist() == [[-1, 1, -1], [-1, -1, 0]]


def test_a_mask_keeps_the_triangles_turned():
    tri = mtri.Triangulation([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0],
                             [[0, 2, 1], [1, 2, 3]])
    tri.set_mask([True, False])
    assert tri.get_masked_triangles().tolist() == [[1, 2, 3]]
    assert tri.edges.tolist() == [[2, 1], [3, 1], [3, 2]]
    assert tri.triangles.tolist() == [[0, 1, 2], [1, 3, 2]]
    assert tri.get_masked_triangles().tolist() == [[1, 3, 2]]


def test_delaunay_triangles_are_left_as_computed():
    x, y = np.array([0.0, 1.0, 0.0, 1.0, 0.4]), np.array([0.0, 0.0, 1.0, 1.0, 0.3])
    tri = mtri.Triangulation(x, y)
    before = tri.triangles.copy()
    tri.get_trifinder()
    tri.calculate_plane_coefficients(x)
    assert tri.triangles.tolist() == before.tolist()


def test_logit_ticks_of_single_precision_limits_start_at_numpys_decade():
    from pltrs.ticker import LogitLocator
    # matplotlib takes the logarithm in the limit's precision, and numpy's
    # single-precision log10 of 0.01 is -2 or just below it, by platform
    # (the C library's, or Intel's SVML where numpy runs with AVX-512).
    low = np.float32(0.01)
    decade = int(np.floor(np.log10(low)))
    assert decade in (-2, -3)
    ticks = LogitLocator().tick_values(low, 0.99)
    assert ticks[0] == 10.0 ** decade
    assert len(LogitLocator(minor=True).tick_values(low, 0.99)) == (22 if decade == -2 else 30)
    # Limits stay of their precision through nonsingular.
    low, high = LogitLocator().nonsingular(np.float32(0.001), np.float32(0.9))
    assert (type(low), type(high)) == (np.float32, np.float32)
    assert LogitLocator().tick_values(np.float32(0.001), np.float32(0.9))[0] == (
        10.0 ** int(np.floor(np.log10(np.float32(0.001)))))
