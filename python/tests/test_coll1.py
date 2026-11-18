"""
What the expert-written family (``spec/diff/written2``, task COLL1)
showed pltrs doing otherwise than matplotlib 3.11 in collections,
contours, tables, histograms and the vector-field helpers, reduced to
matplotlib's API: per-item alphas that do not fit a collection's colours
(kept colours, refusals, which draws raise), the data limits of
collections with offsets, contour label rotations, the algorithm and
corner masking a contour set keeps, hatches assigned to a filled set,
table cells sized by their table and its hit test, histograms of masked
arrays, a quiver key's label colour and streamplot's message. Every
expected value was recorded from matplotlib.
"""

import io
import warnings

import numpy as np
import pytest
from PIL import Image

import pltrs
import pltrs.pyplot as plt
from pltrs import transforms as mtransforms
from pltrs.backend_bases import MouseEvent
from pltrs.collections import (CircleCollection, EllipseCollection, LineCollection,
                               PatchCollection, PolyCollection)
from pltrs.patches import Circle


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _circles(ax, **kwargs):
    pc = PatchCollection([Circle((k, 0), 0.4) for k in range(6)], **kwargs)
    ax.add_collection(pc)
    ax.set_xlim(-1, 6)
    ax.set_ylim(-1, 1)
    return pc


def _pixels(fig):
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png")
    return np.asarray(Image.open(buffer))


def test_alphas_that_do_not_fit_keep_the_colours_and_draw_them():
    fig, ax = plt.subplots(figsize=(5, 2))
    pc = _circles(ax, facecolors=["r", "g"], edgecolors=["k", "b", "y"])
    fig.canvas.draw()
    with pytest.raises(ValueError, match=r"from shape \(6,\) into shape \(2,\)"):
        pc.set_alpha([0.1, 0.3, 0.5, 0.7, 0.9, 1.0])
    # The draws after the first do not resolve the colours again.
    fig.canvas.draw()
    assert pc.get_facecolor()[:, 3].tolist() == [1.0, 1.0]
    assert pc.get_edgecolor()[:, 3].tolist() == [1.0, 1.0, 1.0]
    pc.set_facecolor(["r", "g", "b", "c", "m", "y"])
    pc.set_edgecolor(["k"] * 6)
    pc.set_alpha([0.1, 0.3, 0.5, 0.7, 0.9, 1.0])
    fig.canvas.draw()
    assert np.round(pc.get_facecolor()[:, 3], 2).tolist() == [0.1, 0.3, 0.5, 0.7, 0.9, 1.0]


def test_a_first_draw_raises_for_alphas_that_do_not_fit():
    fig, ax = plt.subplots()
    pc = _circles(ax, facecolors=["r", "g"])
    with pytest.raises(ValueError):
        pc.set_alpha([0.5, 0.25, 1.0])
    with pytest.raises(ValueError, match=r"from shape \(3,\) into shape \(2,\)"):
        fig.canvas.draw()


def test_colours_that_do_not_fit_the_alphas_are_refused_and_kept():
    fig, ax = plt.subplots()
    pc = _circles(ax)
    pc.set_alpha([0.1, 0.3, 0.5, 0.7, 0.9, 1.0])
    fig.canvas.draw()
    with pytest.raises(ValueError, match=r"from shape \(6,\) into shape \(2,\)"):
        pc.set_facecolor(["r", "g"])
    assert np.round(pc.get_facecolor()[:, 3], 1).tolist() == [0.1, 0.3, 0.5, 0.7, 0.9, 1.0]
    fig.canvas.draw()  # draws the colours it had


def test_mapped_collections_raise_at_each_draw():
    fig, ax = plt.subplots()
    sc = ax.scatter(range(6), [0] * 6, c=range(6))
    fig.canvas.draw()
    sc.set_alpha([0.5, 0.25])
    with pytest.raises(ValueError, match=r"Data array shape, \(6,\) is incompatible"):
        fig.canvas.draw()


def test_setting_an_alpha_decides_the_default_edges_anew():
    fig, ax = plt.subplots()
    pc = _circles(ax)
    fig.canvas.draw()
    pc.set_facecolor("none")
    assert pc.get_edgecolor().tolist() == []
    pc.set_alpha([0.5])
    assert pc.get_edgecolor().tolist() == [[0.0, 0.0, 0.0, 0.5]]
    pc.set_alpha(0.5)
    assert np.asarray(pc.get_alpha()).tolist() == [0.5]


def test_an_unfilled_collection_gets_default_edges_at_its_first_draw():
    fig, ax = plt.subplots()
    pc = _circles(ax, facecolor="none")
    fig.canvas.draw()
    assert pc.get_edgecolor().tolist() == [[0.0, 0.0, 0.0, 1.0]]


def test_no_line_widths_stroke_with_one_point():
    fig, ax = plt.subplots(figsize=(3, 1), dpi=40)
    lines = LineCollection([[(0, 0), (0.5, 0.5)], [(1, 1), (1.5, 0.2)]])
    ax.add_collection(lines)
    ax.set_xlim(-1, 7)
    ax.set_ylim(-1, 1)
    lines.set_linewidth([])
    before = _pixels(fig)
    lines.set_linewidth(1)
    assert (before == _pixels(fig)).all()


def test_offsets_in_data_are_added_to_data_shapes_in_pixels():
    fig, ax = plt.subplots(figsize=(4, 4), dpi=72)
    ax.scatter([1, 3], [1, 9])
    lines = LineCollection([[(0, 0), (0.5, 0.5)]], offsets=[(20, 0)],
                           offset_transform=ax.transData)
    ax.add_collection(lines)
    ax.autoscale_view()
    fig.canvas.draw()
    assert ax.get_xlim() == pytest.approx((-97.664, 2072.946), abs=1e-3)
    assert ax.dataLim.bounds == pytest.approx((1.0, 1.0, 1973.282, 16.06), abs=1e-3)


def test_blended_offsets_do_not_count():
    fig, ax = plt.subplots()
    ax.plot([0.5, 3], [0.5, 3])
    blended = mtransforms.blended_transform_factory(ax.transData, ax.transAxes)
    ax.add_collection(PolyCollection([[(0, 0), (1, 0), (0, 1)]], offsets=[(2, 3), (4, -1)],
                                     offset_transform=blended))
    assert ax.dataLim.bounds == pytest.approx((0.5, 0.5, 2.5, 2.5))


def test_collections_drawn_in_display_units_replace_a_given_transform():
    fig, ax = plt.subplots()
    scaled = mtransforms.Affine2D().scale(2) + ax.transData
    circles = CircleCollection([100, 400], offsets=[(2, 3), (4, -1)], transform=scaled)
    assert isinstance(circles.get_transform(), mtransforms.IdentityTransform)


def test_ellipses_in_data_units_take_the_data_scale_when_drawn():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ellipses = EllipseCollection([1, 2], [0.5, 1], [0, 30], units="xy",
                                 offsets=[(2, 3), (4, -1)])
    ax.add_collection(ellipses)
    assert isinstance(ellipses.get_transform(), mtransforms.IdentityTransform)
    fig.canvas.draw()
    assert np.diag(ellipses.get_transform().get_matrix()).tolist() == pytest.approx(
        [1409.090909090909, 1050.0, 1.0])
    assert ellipses.get_window_extent().bounds == pytest.approx((2.0, -1.0, 2.0, 4.0))


def test_contour_label_rotations_read_back_as_set_rotation_keeps_them():
    x = np.linspace(-3, 3, 80)
    X, Y = np.meshgrid(x, x)
    fig, axs = plt.subplots(2, 2, figsize=(6, 6), dpi=60)
    cs = axs[0, 0].contour(X, Y, np.hypot(X, Y), levels=[0.5, 1.5], colors="r")
    labels = cs.clabel(manual=[(0.5, 0), (0, 1.5)], fmt="%.2f")
    # (The second label's angle is 359.99999999999983 with glibc and 0.0
    # with Apple's libm, under matplotlib as well: one direction.)
    off = [(text.get_rotation() - angle + 180) % 360 - 180
           for text, angle in zip(labels, [89.99999999999994, 359.99999999999983])]
    assert off == pytest.approx([0, 0], abs=1e-9)


def test_contour_sets_keep_their_algorithm_and_corner_masking():
    z = np.arange(12.).reshape(3, 4)
    fig, ax = plt.subplots()
    with plt.rc_context({"contour.algorithm": "serial", "contour.corner_mask": False}):
        cs = ax.contour(z)
    assert (cs._algorithm, cs._corner_mask) == ("serial", False)
    cs = ax.contourf(z, algorithm="mpl2005")
    assert (cs._algorithm, cs._corner_mask) == ("mpl2005", False)
    cs = ax.contour(z)
    assert (cs._algorithm, cs._corner_mask) == ("mpl2014", True)
    assert (ax.contour(cs)._algorithm, ax.contour(cs)._corner_mask) == ("mpl2014", True)


def test_hatches_assigned_to_a_filled_set_are_drawn():
    x = np.linspace(-2, 2, 40)
    X, Y = np.meshgrid(x, x)

    def picture(assign, hatch=None):
        fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
        cf = ax.contourf(X, Y, X * Y, levels=3, hatches=["/", "\\", "x"])
        if assign is not None:
            cf.hatches = assign
        if hatch is not None:
            cf.set_hatch(hatch)
        return _pixels(fig)

    fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
    ax.contourf(X, Y, X * Y, levels=3, hatches=["o", "", "+"])
    assert (picture(["o", "", "+"]) == _pixels(fig)).all()
    assert (picture(None, "*") == picture(None)).all()


def test_tables_size_their_cells_where_they_fit_them():
    fig, ax = plt.subplots(figsize=(4, 2), dpi=72)
    table = ax.table(cellText=[["1", "a much longer text"]], loc="center")
    table.auto_set_column_width([0, 1])
    fig.canvas.draw()
    widths = [table[0, c].get_width() for c in range(2)]
    assert [type(w) for w in widths] == [np.float64, np.float64]
    assert widths == pytest.approx([0.0323, 0.5045], abs=1e-4)
    box = table.get_window_extent(fig.canvas.get_renderer())
    inside = MouseEvent("motion_notify_event", fig.canvas,
                        box.x0 + box.width / 2, box.y0 + box.height / 2)
    outside = MouseEvent("motion_notify_event", fig.canvas, 10, 10)
    assert table.contains(inside)[0] and not table.contains(outside)[0]


def test_stretched_tables_size_their_cells():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    table = ax.table(cellText=[["1", "two"], ["3", "4"]], bbox=[0.1, 0.1, 0.5, 0.3])
    table.scale(1.5, 2)
    fig.canvas.draw()
    cell = table[0, 0]
    assert (type(cell.get_width()), type(cell.get_height())) == (np.float64, np.float64)
    assert (cell.get_width(), cell.get_height()) == pytest.approx((0.25, 0.15))


def test_histograms_of_masked_arrays_bin_their_data():
    fig, ax = plt.subplots()
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        counts, edges, _ = ax.hist(np.ma.masked_array([1, 3, 2, 4], [0, 1, 0, 0]), bins=3)
    assert counts.tolist() == [1.0, 1.0, 2.0]
    assert edges.tolist() == [1.0, 2.0, 3.0, 4.0]
    assert caught == []


def test_bars_of_masked_arrays_warn_once():
    fig, ax = plt.subplots()
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        bars = ax.bar(range(4), np.ma.masked_array([1, 3, 2, 4], [0, 1, 0, 0]))
        ax.errorbar(range(2), np.ma.masked_array([1.0, 2.0], [1, 0]),
                    yerr=np.ma.masked_array([0.1, 0.2], [1, 0]))
    assert [str(w.message) for w in caught] == ["Warning: converting a masked element to nan."]
    assert np.isnan(bars[1].get_height())


def test_stairs_draw_the_data_of_masked_arrays():
    fig, ax = plt.subplots()
    ax.stairs(np.ma.masked_array([1.0, 2.0], [1, 1]))
    fig.canvas.draw()
    assert ax.get_xlim() == pytest.approx((-0.1, 2.1))
    assert ax.get_ylim() == pytest.approx((0.0, 2.1))


def test_a_quiver_keys_label_colour_reads_back_as_given():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1], [0, 1], [1, 1], [1, 0])
    key = ax.quiverkey(q, 0.2, 1.05, 1, "1 u", labelcolor="C0")
    assert key.text.get_color() == "C0"


def test_streamplot_writes_start_points_as_python_floats():
    fig, ax = plt.subplots()
    x = np.linspace(0, 1, 5)
    with pytest.raises(ValueError, match=r"^Starting point \(10\.0, 10\.0\) outside of data"):
        ax.streamplot(x, x, np.ones((5, 5)), np.ones((5, 5)), start_points=[(10, 10)])
