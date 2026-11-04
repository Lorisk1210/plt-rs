"""
Smoke tests of what pltrs (Python) defers or reads directly for speed and
memory: `Line2D.set_data` handing the numbers to pltrs (Rust) at the next
native call, affine transforms reading their points without
``np.asarray`` and returning arrays numpy made, and colour-mapped meshes
normalised a block at a time when drawn. The expected values were
recorded from matplotlib 3.11.2 running the same code.
"""

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import colors as mcolors
from pltrs import transforms as mtransforms

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


@pytest.fixture
def axes():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_xlim(0, 10)
    ax.set_ylim(-1, 1)
    return fig, ax


def test_set_data_is_seen_at_once(axes):
    fig, ax = axes
    line, = ax.plot([1, 2, 3], [0, 0.5, 1])
    fig.canvas.draw()
    assert (line.stale, ax.stale, fig.stale) == (False, False, False)
    line.set_data([4, 5, 6, 7], [1, 2, 3, 4])
    assert (line.stale, ax.stale, fig.stale) == (True, True, True)
    assert line.get_data() == ([4, 5, 6, 7], [1, 2, 3, 4])
    assert line.get_xdata(orig=False).tolist() == [4.0, 5.0, 6.0, 7.0]
    assert line.get_ydata(orig=False).tolist() == [1.0, 2.0, 3.0, 4.0]
    expected = [[4.0, 1.0], [5.0, 2.0], [6.0, 3.0], [7.0, 4.0]]
    assert line.get_xydata().tolist() == expected
    assert line.get_path().vertices.tolist() == expected
    ax.relim()
    assert ax.dataLim.bounds == (4.0, 1.0, 3.0, 3.0)
    line.set_xdata([0, 1, 2, 3])
    assert line.get_xydata().tolist() == [[0.0, 1.0], [1.0, 2.0], [2.0, 3.0],
                                          [3.0, 4.0]]
    line.set_ydata(np.array([3.0, 2, 1, 0]))
    assert line.get_xydata().tolist() == [[0.0, 3.0], [1.0, 2.0], [2.0, 1.0],
                                          [3.0, 0.0]]


def test_many_lines_set_before_a_draw(axes):
    fig, ax = axes
    lines = [ax.plot([0, 1], [0, 1])[0] for _ in range(50)]
    for i, line in enumerate(lines):
        line.set_data([i, i + 1], [0, i])
    assert lines[-1].get_xydata().tolist() == [[49.0, 0.0], [50.0, 49.0]]
    assert lines[7].get_path().vertices.tolist() == [[7.0, 0.0], [8.0, 7.0]]
    fig.canvas.draw()
    np.testing.assert_allclose(lines[-1].get_window_extent().extents,
                               [1569.0, 148.5, 1600.0, 5808.0])


@pytest.mark.parametrize("points, expected, shape", [
    ((0.5, 0.5), [205.0, 148.5], (2,)),
    ([0.25, 1], [127.5, 264.0], (2,)),
    ((1, 0), [360.0, 32.99999999999999], (2,)),
    ([[0, 0], [1, 1]], [[50.0, 32.99999999999999], [360.0, 264.0]], (2, 2)),
    (np.array([0.5, 0.25]), [205.0, 90.75], (2,)),
    (np.array([[0, 1]], dtype=np.int32), [[50.0, 264.0]], (1, 2)),
    (("0.5", "1"), [205.0, 264.0], (2,)),
])
def test_affine_transforms_read_any_points(axes, points, expected, shape):
    fig, ax = axes
    out = ax.transAxes.transform(points)
    assert out.tolist() == expected
    assert (out.dtype, out.shape) == (np.float64, shape)
    assert out.flags.writeable and out.flags.owndata


def test_affine_transforms_of_boxes_and_masked_points(axes):
    fig, ax = axes
    t = ax.transAxes
    assert t.transform(mtransforms.Bbox([[0, 0], [1, 1]])).tolist() == [
        [50.0, 32.99999999999999], [360.0, 264.0]]
    masked = np.ma.masked_array([[0.5, 0.5], [1, 1]], mask=[[0, 1], [0, 0]])
    out = t.transform(masked)
    assert isinstance(out, np.ma.MaskedArray)
    assert out.data.tolist() == [[205.0, 148.5], [360.0, 264.0]]
    assert np.ma.getmaskarray(out).tolist() == [[False, True], [False, False]]
    with pytest.raises(RuntimeError, match="Invalid vertices array."):
        t.transform((1, 2, 3))
    assert ax.transData.transform([(5, 0.5), (0, 0)]).tolist() == [
        [205.0, 206.25], [50.0, 148.5]]


def test_meshes_are_coloured_a_block_at_a_time():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    values = (np.arange(300 * 300).reshape(300, 300) % 7).astype(float)
    values[5, 5] = np.nan
    alpha = np.linspace(0, 1, 300 * 300).reshape(300, 300)
    mesh = ax.pcolormesh(values, alpha=alpha)
    fig.canvas.draw()
    picked = [0, 1, 1505, 65535, 65536, 70001, 89999]
    assert mesh.get_facecolors().shape == (90000, 4)
    np.testing.assert_allclose(mesh.get_facecolors()[picked], [
        [0.267004, 0.004874, 0.329415, 0.0],
        [0.267968, 0.223549, 0.512008, 1.1e-05],
        [0.0, 0.0, 0.0, 0.0],
        [0.267968, 0.223549, 0.512008, 0.728175],
        [0.190631, 0.407061, 0.556089, 0.728186],
        [0.267968, 0.223549, 0.512008, 0.777798],
        [0.267004, 0.004874, 0.329415, 1.0]], atol=1e-6)
    array = mesh.get_array()
    assert array.shape == (300, 300) and np.ma.is_masked(array)
    assert float(np.nansum(array)) == 269997.0
    mesh.set_clim(1, 5)
    fig.canvas.draw()
    np.testing.assert_allclose(mesh.get_facecolors()[[1, 65536]], [
        [0.267004, 0.004874, 0.329415, 1.1e-05],
        [0.229739, 0.322361, 0.545706, 0.728186]], atol=1e-6)
    fig, ax = plt.subplots()
    binned = ax.pcolormesh(values, norm=mcolors.BoundaryNorm([0, 2, 4, 6], 256))
    fig.canvas.draw()
    np.testing.assert_allclose(binned.get_facecolors()[[0, 1505, 65536]], [
        [0.267004, 0.004874, 0.329415, 1.0], [0.0, 0.0, 0.0, 0.0],
        [0.128729, 0.563265, 0.551229, 1.0]], atol=1e-6)
    fig, ax = plt.subplots()
    empty = ax.pcolormesh(np.zeros((0, 0)))
    fig.canvas.draw()
    assert empty.get_facecolors().shape == (0, 4)
