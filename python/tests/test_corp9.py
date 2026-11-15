"""
What the ninth round of corpora (``cargo xtask diff --corpus``) showed
pltrs doing otherwise than matplotlib 3.11, reduced to matplotlib's API:
the keyword of former versions an `.Axes3D` takes away (pyroomacoustics),
aliases of a mesh's properties set in the order given (seaborn's heatmap),
the texts of text areas measured at the dpi of the figure their box is
given (matplotlib-scalebar), a spine padded by the tick marks drawn on
it, each tick's own (seaborn's pair grids hide them tick by tick), and the
edges of 3-D polygons faded by an alpha set before their colour
(pyroomacoustics' rooms). Every expected value was recorded from
matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.offsetbox import AnchoredOffsetbox, DrawingArea, TextArea, VPacker
from pltrs.mpl_toolkits.mplot3d import Axes3D, art3d


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _bounds(bbox):
    return np.round(bbox.bounds, 2).tolist()


def test_axes3d_takes_auto_add_to_figure_away():
    fig = plt.figure()
    ax = Axes3D(fig, auto_add_to_figure=False)
    assert ax not in fig.axes
    with pytest.raises(AttributeError, match="auto_add_to_figure is no longer supported"):
        Axes3D(fig, auto_add_to_figure=True)


def test_mesh_aliases_set_in_the_order_given():
    fig, ax = plt.subplots()
    data = np.arange(4.).reshape(2, 2)
    mesh = ax.pcolormesh(data, linewidths=0, lw=0.5, edgecolor="w")
    assert mesh.get_linewidth().tolist() == [0.5]
    assert mesh.get_edgecolor().tolist() == [[1.0, 1.0, 1.0, 1.0]]
    assert ax.pcolormesh(data, lw=0.5, linewidths=2).get_linewidth().tolist() == [0.5]
    # pcolor takes linewidth over linewidths.
    assert ax.pcolor(data, linewidths=3, linewidth=1).get_linewidth().tolist() == [1]


def test_text_area_measures_at_the_dpi_of_its_boxes_figure():
    fig, ax = plt.subplots(figsize=(4, 4), dpi=60)
    renderer = fig.canvas.get_renderer()
    area = TextArea("5 cm")
    box = AnchoredOffsetbox(loc="upper right", child=VPacker(
        children=[DrawingArea(10, 10), area], pad=0, sep=5))
    box.set_figure(fig)
    assert _bounds(area.get_bbox(renderer)) == [0.0, -2.0, 20.0, 9.0]
    assert _bounds(box.get_bbox(renderer)) == [-3.33, -16.5, 26.67, 28.17]


def test_spine_is_padded_by_the_marks_its_ticks_show():
    pltrs.rcParams["ytick.left"] = False
    fig, ax = plt.subplots()
    renderer = fig.canvas.get_renderer()
    # twinx shows the left marks again (matplotlib issue 15188).
    ax.twinx()
    assert _bounds(ax.spines.left.get_window_extent(renderer)) == [75.14, 52.8, 4.86, 369.6]
    for tick in ax.yaxis.majorTicks:
        tick.tick1line.set_visible(False)
    assert _bounds(ax.spines.left.get_window_extent(renderer)) == [80.0, 52.8, 0.0, 369.6]
    pltrs.rcdefaults()
    fig, ax = plt.subplots()
    ax.xaxis.get_major_ticks()[0].tick1line.set_visible(False)
    # The others still show theirs.
    assert _bounds(ax.spines.bottom.get_window_extent(renderer)) == [80.0, 47.94, 496.0, 4.86]


def test_poly3d_edges_take_the_alpha_set_before_them():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    faces = [[[0, 0, 0], [1, 0, 0], [1, 1, 1]]]

    def edges(make, then=None):
        polys = make()
        ax.add_collection3d(polys)
        if then is not None:
            then(polys)
        fig.canvas.draw()
        return polys.get_edgecolor().tolist()

    # pyroomacoustics' walls: alpha in the constructor, colours after.
    walls = art3d.Poly3DCollection(faces, alpha=0.5)
    walls.set_color("#336699")
    walls.set_edgecolor("k")
    assert edges(lambda: walls) == [[0.0, 0.0, 0.0, 0.5]]
    assert edges(lambda: art3d.Poly3DCollection(faces, alpha=0.5, color="r")) == [
        [1.0, 0.0, 0.0, 0.5]]
    # An alpha set after the edge colour leaves the edges alone.
    late = art3d.Poly3DCollection(faces)
    late.set_edgecolor("k")
    late.set_alpha(0.3)
    assert edges(lambda: late) == [[0.0, 0.0, 0.0, 1.0]]
    assert edges(lambda: art3d.Poly3DCollection(faces, alpha=0.5),
                 lambda polys: polys.set_edgecolor("b")) == [[0.0, 0.0, 1.0, 0.5]]
