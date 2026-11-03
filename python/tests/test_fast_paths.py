"""
Smoke tests of the shortcuts pltrs (Python) takes for speed and memory:
the composed affine and inverse a composite transform keeps until a child
changes, the rcParams values read once per generation of the parameters,
the marker test of `Line2D.get_window_extent`, the copies `set_data`,
`pcolormesh` and `tricontourf` no longer make, Pillow and the 3D toolkit
imported on first use. The expected values were recorded from matplotlib
3.11.2 running the same code.
"""

import subprocess
import sys

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import projections, tri

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


@pytest.fixture
def axes():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_xlim(0, 10)
    ax.set_ylim(-1, 1)
    return fig, ax


def test_composite_transforms_follow_their_children(axes):
    fig, ax = axes
    t = ax.transData
    assert t.transform((5, 0.5)).tolist() == [205.0, 206.25]
    first = t.get_affine().get_matrix().copy()
    assert t.get_affine() is t.get_affine()
    ax.set_xlim(0, 20)
    assert t.transform((5, 0.5)).tolist() == [127.5, 206.25]
    assert not np.allclose(first, t.get_affine().get_matrix())
    np.testing.assert_allclose(
        t.inverted().transform(t.transform((3, -0.25))), [3.0, -0.25])
    ax.set_ylim(0, 4)
    np.testing.assert_allclose(t.inverted().transform((100, 100)),
                               [3.225806451612903, 1.1601731601731604])


def test_line_extents_and_data(axes):
    fig, ax = axes
    ax.set_xlim(0, 20)
    ax.set_ylim(0, 4)
    renderer = fig.canvas.get_renderer()
    line, = ax.plot([1, 2, 3], [0, 1, 4], "o", markersize=10)
    np.testing.assert_allclose(
        line.get_window_extent(renderer).extents,
        [58.55555555555556, 26.05555555555555, 103.44444444444444,
         270.94444444444446])
    line.set_marker("None")
    np.testing.assert_allclose(line.get_window_extent(renderer).extents,
                               [65.5, 33.0, 96.5, 264.0])
    x = np.array([0.0, 5.0])
    line.set_data(x, np.array([1, 3]))
    x[0] = 9.0  # The line holds a copy.
    assert line.get_xydata().tolist() == [[0.0, 1.0], [5.0, 3.0]]


def test_rcparams_reads_follow_changes():
    with pltrs.rc_context({"lines.linewidth": 3.5}):
        assert pltrs.rcParams["lines.linewidth"] == 3.5
    assert pltrs.rcParams["lines.linewidth"] == 1.5
    pltrs.rcParams["path.simplify_threshold"] = 0.5
    assert pltrs.rcParams["path.simplify_threshold"] == 0.5
    pltrs.rcdefaults()
    assert pltrs.rcParams["path.simplify_threshold"] == 0.111111111111


def test_the_3d_projection_is_registered_before_its_import():
    names = projections.get_projection_names()
    # (Other tests may register projections of their own.)
    assert names == sorted(names)
    assert {"3d", "aitoff", "hammer", "lambert", "mollweide", "polar",
            "rectilinear"} <= set(names)
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    assert type(ax).__name__ == "Axes3D"
    assert fig.axes == [ax]


def test_pyplot_imports_neither_pillow_nor_the_3d_toolkit():
    code = ("import sys, pltrs.pyplot; "
            "print(any(m.startswith(('PIL', 'pltrs.mpl_toolkits')) for m in sys.modules))")
    out = subprocess.run([sys.executable, "-c", code], capture_output=True,
                         text=True, check=True)
    assert out.stdout.strip() == "False"


def test_pillow_images_still_show(axes):
    PIL = pytest.importorskip("PIL.Image")
    fig, ax = axes
    image = ax.imshow(PIL.new("RGB", (3, 2), (10, 20, 30)))
    assert image.get_array().shape == (2, 3, 3)
    assert plt.get_cmap("viridis")._repr_png_()[:4] == b"\x89PNG"


def test_triangulations_and_tricontourf(axes):
    x = np.array([0.0, 1, 0, 1, 0.5, 3])
    y = np.array([0.0, 0, 1, 1, 0.5, 3])
    tr = tri.Triangulation(x, y)
    assert tr.triangles.tolist() == [
        [3, 1, 5], [5, 2, 3], [4, 2, 0], [0, 1, 4], [4, 3, 2], [1, 3, 4]]
    assert tr.neighbors.tolist() == [
        [5, -1, 1], [-1, 4, 0], [4, -1, 3], [-1, 5, 2], [5, 1, 2], [0, 4, 3]]
    tr.set_mask([True] + [False] * 5)
    assert tr.neighbors.tolist() == [
        [-1, -1, -1], [-1, 4, -1], [4, -1, 3], [-1, 5, 2], [5, 1, 2], [-1, 4, 3]]
    fig, ax = axes
    cs = ax.tricontourf(x, y, x + y, levels=3)
    assert cs.levels.tolist() == [0.0, 1.5, 3.0, 4.5, 6.0]
    with pytest.raises(ValueError, match="non-finite"), \
            pytest.warns(RuntimeWarning, match="invalid value encountered in cast"):
        ax.tricontourf(x, y, np.r_[x[:-1], np.nan])


def test_pcolormesh_masks_invalid_values(axes):
    fig, ax = axes
    c = np.arange(12.0).reshape(3, 4)
    c[1, 2] = np.inf
    qm = ax.pcolormesh(c)
    array = qm.get_array()
    assert np.ma.filled(array, -1).ravel().tolist() == [
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, -1.0, 7.0, 8.0, 9.0, 10.0, 11.0]
    assert np.ma.getmaskarray(array).ravel().tolist() == [False] * 6 + [True] + [False] * 5
    assert list(qm.get_clim()) == [0.0, 11.0]
    assert np.isinf(c[1, 2])  # The values given are left alone.
