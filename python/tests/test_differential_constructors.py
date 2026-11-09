"""
What the differential tester (``cargo xtask diff``) found pltrs doing
otherwise than matplotlib 3.11 around artists' constructors and figures:
constructors that add nothing, the legends of a figure, dragging a legend
through its box, `.Axes.set_figure`, what a removed artist keeps, a
category axis after dates, and values measured at another renderer's
resolution, the names and docstrings of inherited setters, and a contour
set's explicit colours after `changed`. Each expected value is
matplotlib's.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, lines, patches, ticker, units
from pltrs.artist import ArtistInspector
from pltrs.backend_bases import MouseEvent
from pltrs.backends.backend_agg import RendererAgg
from pltrs.figure import SubFigure
from pltrs.legend import DraggableLegend, Legend
from pltrs.offsetbox import AnchoredText
from pltrs.quiver import Barbs, Quiver, QuiverKey


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _grid():
    return np.meshgrid([0, 1], [0, 1])


def test_field_constructors_add_nothing():
    fig, ax = plt.subplots()
    X, Y = _grid()
    q = Quiver(ax, X, Y, X + 1, Y + 1)
    b = Barbs(ax, X, Y, X * 10, Y * 10)
    key = QuiverKey(q, 0.5, 0.5, 1, "one")
    for artist in (q, b, key):
        assert artist.figure is None and artist.axes is None
    assert list(ax.collections) == [] and list(ax.artists) == []
    ax.add_collection(q)
    ax.add_artist(key)
    assert q.figure is fig and q.axes is ax and key.axes is ax
    assert list(ax.collections) == [q] and list(ax.artists) == [key]
    fig.canvas.draw()


def test_axes_field_methods_add_what_they_make():
    fig, ax = plt.subplots()
    X, Y = _grid()
    q = ax.quiver(X, Y, X + 1, Y + 1)
    key = ax.quiverkey(q, 0.5, 0.5, 1, "one")
    b = ax.barbs(X, Y, X * 10, Y * 10)
    assert list(ax.collections) == [q, b] and list(ax.artists) == [key]
    assert key.axes is ax


def test_a_figure_draws_the_legends_in_its_list():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1], label="a")
    made = Legend(fig, [line], ["a"])
    assert made.figure is fig
    assert fig.legends == [] and made not in fig.get_children()
    fig.legends.append(made)
    assert fig.legends == [made] and made in fig.get_children()
    legend = fig.legend([line], ["b"])
    assert fig.legends == [made, legend]
    fig.canvas.draw()


def test_a_subfigure_constructor_adds_nothing():
    fig = plt.figure()
    sub = SubFigure(fig, fig.add_gridspec(1, 1)[0])
    assert fig.subfigs == [] and sub not in fig.get_children()


def test_a_legend_is_dragged_through_its_box():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    legend = ax.legend()
    fig.canvas.draw()
    dragged = legend.set_draggable(True)
    assert type(dragged) is DraggableLegend and legend.get_draggable()
    box = legend._legend_box
    assert type(box).__name__ == "VPacker"
    assert box.get_figure() is fig and box.axes is ax
    np.testing.assert_allclose(legend.get_window_extent().bounds,
                               (86.944444, 390.007856, 59.0, 25.4477), atol=1e-6)
    box.set_offset((100.0, 120.0))
    np.testing.assert_allclose(legend.get_window_extent().bounds,
                               (94.444444, 111.107856, 59.0, 25.4477), atol=1e-6)
    dragged.finalize_offset()
    np.testing.assert_allclose(legend._loc, (0.029121864, 0.15775935), atol=1e-9)
    legend.set_loc("upper left")
    assert legend._loc == 2 and callable(box._offset)
    assert legend.set_draggable(False) is None and not legend.get_draggable()


def _drag(fig, legend, dx, dy):
    """Drag *legend* by (*dx*, *dy*) pixels with the mouse."""
    x0, y0, width, height = legend.get_window_extent().bounds
    x, y = x0 + width / 2, y0 + height / 2
    MouseEvent("button_press_event", fig.canvas, x, y, 1)._process()
    MouseEvent("motion_notify_event", fig.canvas, x + dx, y + dy, 1)._process()
    moved = legend.get_window_extent().bounds
    MouseEvent("button_release_event", fig.canvas, x + dx, y + dy, 1)._process()
    return moved


def test_dragging_a_legend_with_the_mouse_moves_its_location():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    legend = ax.legend(loc="lower left")
    legend.set_draggable(True)
    fig.canvas.draw()
    np.testing.assert_allclose(_drag(fig, legend, 50, 30),
                               (136.944444, 89.744444, 59.0, 25.4477), atol=1e-6)
    np.testing.assert_allclose(legend._loc, (0.114807348, 0.099957912), atol=1e-9)
    fig.canvas.draw()
    np.testing.assert_allclose(legend.get_window_extent().bounds,
                               (136.944444, 89.744444, 59.0, 25.4477), atol=1e-6)
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    legend = ax.legend(loc="upper right")
    legend.set_draggable(True, update="bbox")
    fig.canvas.draw()
    _drag(fig, legend, -100, -40)
    # matplotlib maps the point in display pixels through transAxes.
    assert legend._loc == 1
    np.testing.assert_allclose(legend.get_bbox_to_anchor().bounds,
                               (100919987.555556, 47832096.829, 0, 0), atol=1e-3)


def test_set_figure_resets_the_limits_of_an_axes():
    fig, ax = plt.subplots()
    ax.plot([0, 10], [0, 5])
    ax.set_figure(fig)
    assert ax.get_xlim() == ax.get_ylim() == (-0.05500000000000001, 0.05500000000000001)
    assert ax.dataLim.bounds == (np.inf, np.inf, -np.inf, -np.inf)
    fig, ax = plt.subplots()
    ax.plot([0, 10], [0, 5])
    ax.figure = fig  # Artist.set_figure: the limits stay.
    assert ax.get_xlim() == (-0.5, 10.5) and ax.get_ylim() == (-0.25, 5.25)


def test_a_removed_artist_keeps_its_clip_box_and_transform():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    clip, transform = line.get_clip_box(), line.get_transform()
    line.remove()
    assert line.figure is None and line.axes is None
    assert line.get_clip_box() is clip and line.get_transform() is transform


def test_categories_after_dates_fail_to_convert_the_dates():
    days = np.arange("2024-01-01", "2024-01-08", dtype="datetime64[D]")
    fig, ax = plt.subplots()
    ax.plot(days, np.arange(7))
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        with pytest.raises(units.ConversionError, match="datetime64"):
            ax.plot(["a", "b"], [1, 2])
    assert [str(w.message) for w in caught] == [
        "This axis already has a converter set and is updating to a "
        "potentially incompatible converter"]


def test_a_mesh_decides_its_default_edges_when_set():
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(np.arange(6.0).reshape(2, 3), facecolor="none")
    mesh.set_edgecolor(None)
    assert mesh.get_edgecolor().tolist() == [[0.0, 0.0, 0.0, 1.0]]
    mesh.set_facecolor("r")
    assert mesh.get_edgecolor().tolist() == [[0.0, 0.0, 0.0, 1.0]]
    mesh.set_edgecolor(None)
    assert mesh.get_edgecolor().tolist() == []


def test_values_without_an_axis_or_a_log_norm():
    formatter = ticker.ScalarFormatter()
    assert formatter.format_data(1234.5) == "1.2345e3"
    assert formatter.format_data(0.000123) == "1.23e\N{MINUS SIGN}4"
    ax = plt.figure().add_subplot(projection="polar")
    ax.set_rorigin(-1)
    ax.set_rlim(0, 2)
    assert type(ax.get_rsign()) is np.float64 and ax.get_rsign() == 1.0
    fig, ax = plt.subplots()
    Z = np.outer(np.linspace(1, 10, 5), np.linspace(1, 100, 6))
    named = ax.contour(Z, norm="log")  # a name, not a LogNorm
    assert not named.logscale
    assert named.levels.tolist() == [0, 150, 300, 450, 600, 750, 900, 1050]
    logged = ax.contour(Z, norm=colors.LogNorm())
    assert logged.logscale and logged.levels.tolist() == [0.1, 1, 10, 100, 1000, 10000]


def test_pads_are_measured_at_the_renderers_resolution():
    fig, ax = plt.subplots()
    text = AnchoredText("text", loc="upper left")
    ax.add_artist(text)
    fig.canvas.draw()
    np.testing.assert_allclose(text.get_window_extent(RendererAgg(400, 300, 200)).bounds,
                               (93.8889, 361.9523, 77.7378, 46.5588), atol=1e-4)
    np.testing.assert_allclose(text.get_window_extent().bounds,
                               (86.9444, 390.0079, 37.8611, 25.4477), atol=1e-4)


def test_inherited_setters_keep_their_names_and_docstrings():
    assert lines.Line2D.set_visible.__qualname__ == "Artist.set_visible"
    inspector = ArtistInspector(patches.Rectangle((0, 0), 1, 1))
    assert inspector.get_valid_values("alpha") == "float or None"


def test_explicit_contour_colours_index_their_bands_after_changed():
    fig, ax = plt.subplots()
    x = np.linspace(-3, 3, 30)
    Z = np.add.outer(x, x) / 4
    cs = ax.contourf(Z, [-1.5, -1, -0.5, 0, 0.5, 1], colors=("r", "g", "b"),
                     extend="both")
    cs.cmap.set_under("yellow")
    cs.cmap.set_over("cyan")
    cs.changed()
    assert cs.get_array().tolist() == [-1, 0, 1, 2, 3, 4, 5]
    np.testing.assert_array_equal(cs.get_facecolor()[:4],
                                  [[1, 1, 0, 1], [1, 0, 0, 1], [0, 0.5, 0, 1], [0, 0, 1, 1]])
