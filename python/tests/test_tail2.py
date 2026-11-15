"""
The small leftovers several tasks listed (task TAIL2) where pltrs did
otherwise than matplotlib 3.11: what a pick asks of the tickers of the
Axes it reaches, the tight box of a figure with 3-D Axes before its first
draw, a 3-D axis carried by other edges of its box, empty axis labels
placed by a draw, an Axes drawn alone, limits given to an Axes sharing an
axis, the fonts mathtext opens, and the minor ticks of an axis whose
locator a twin shares (task ECO9). Each expected value is recorded from
matplotlib.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import backend_bases, ticker
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure
from pltrs.mpl_toolkits import axisartist

X = np.linspace(0, 2 * np.pi, 7)
VALUES = [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5]


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def small_figure():
    """A 3 × 2 inch figure at 50 dpi on an Agg canvas, and its renderer."""
    fig = Figure(figsize=(3, 2), dpi=50)
    canvas = FigureCanvasAgg(fig)
    return fig, canvas.get_renderer()


@pytest.mark.parametrize("minor, expected", [
    (False, ["major", "major"]),
    (True, ["major", "minor", "major", "major"]),
])
def test_a_pick_asks_the_locators_of_the_axes_it_reaches_and_no_formatter(minor, expected):
    calls = []

    class Major(ticker.MaxNLocator):
        def __call__(self):
            calls.append("major")
            return super().__call__()

    class Minor(ticker.AutoMinorLocator):
        def __call__(self):
            calls.append("minor")
            return super().__call__()

    fig, ax = plt.subplots()
    twin = ax.twiny()
    twin.xaxis.set_major_locator(Major())
    if minor:
        twin.xaxis.set_minor_locator(Minor())
    twin.xaxis.set_major_formatter(
        ticker.FuncFormatter(lambda v, p: calls.append("format") or str(v)))
    fig.canvas.draw()
    calls.clear()
    x, y = ax.transAxes.transform((0.5, 0.5))
    backend_bases.MouseEvent("scroll_event", fig.canvas, x, y, "up", step=0)._process()
    assert calls == expected


def test_a_figure_with_3d_axes_measures_their_square_box_before_any_draw():
    fig = plt.figure()
    fig.add_subplot(projection="3d")
    assert np.round(fig.get_tightbbox().bounds, 4).tolist() == [
        1.432, 0.4051, 3.9009, 3.8189]


def juggle(axis, triple):
    """Have the 3-D *axis* take its line from the box's near corner in the
    order *triple* (matplotlib's ``_axinfo["juggled"]``)."""
    axinfo = getattr(axis, "_axinfo", None)
    if axinfo is not None:
        axinfo["juggled"] = triple
    else:
        from pltrs._native import mplot3d
        mplot3d.set_juggled(axis.axes._native, axis.axis_name, triple)


@pytest.mark.parametrize("name, triple, position, line, labels", [
    ("x", (2, 0, 1), "default", [[-0.021, 0.082], [0.076, 0.05]],
     [[-0.5, 0.0], [-0.007, 0.09]]),
    ("x", (2, 0, 1), "lower", [[-0.081, 0.023], [-0.049, -0.083]],
     [[-0.5, 0.0], [-0.083, -0.069]]),
    ("y", (0, 2, 1), "upper", [[0.078, 0.082], [-0.027, 0.05]],
     [[500000.0, 0.0], [0.041, -0.094]]),
    ("z", (-3, 2, 0), "both", [[0.078, 0.082], [-0.027, 0.05]],
     [[-0.0, 0.0], [0.098, -0.022]]),
])
def test_a_juggled_3d_axis_is_drawn_along_other_edges(name, triple, position, line, labels):
    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="3d")
    ax.plot([0, 1, 2], [1e6, 3e6, 2e6], [0, 1e-5, 0])
    axis = getattr(ax, name + "axis")
    axis.set_ticks_position(position)
    juggle(axis, triple)
    fig.canvas.draw()
    assert np.round(axis.line.get_data(), 3).tolist() == line
    assert [np.round(t.label1.get_position(), 3).tolist()
            for t in axis.get_major_ticks()[:2]] == labels


def test_empty_axis_labels_are_placed_by_a_draw():
    fig, ax = plt.subplots()
    ax.set_xlabel("")
    ax.set_ylabel("")
    fig.canvas.draw()
    assert ax.xaxis.label.get_position() == (0.5, 23.633333333333333)
    assert ax.yaxis.label.get_position() == (42.722222222222214, 0.5)


def test_an_axes_drawn_alone_draws_its_ticks():
    fig, renderer = small_figure()
    ax = fig.add_subplot(121)
    ax.draw(renderer)
    assert int((np.asarray(renderer.buffer_rgba())[..., 3] > 0).sum()) == 4636


def test_an_axes_drawn_alone_keeps_the_view_its_draw_autoscaled():
    fig, ax = plt.subplots()
    ax.plot(X, np.sin(X))
    ax.draw(fig.canvas.get_renderer())
    ax.set_autoscale_on(False)
    assert ax.get_xlim() == (-0.3141592653589793, 6.5973445725385655)
    assert ax.get_ylim() == (-0.9526279441628829, 0.9526279441628827)


def test_an_axis_artist_label_clears_the_tick_labels_of_a_lone_draw():
    fig, renderer = small_figure()
    ax = fig.add_subplot(121, axes_class=axisartist.Axes)
    ax.draw(renderer)
    ax.set_ylabel("yl")
    assert round(float(ax.get_tightbbox(renderer).x0), 2) == -7.25


def test_limits_given_to_a_shared_axis_autoscale_the_twin_first():
    fig, ax = plt.subplots()
    ax.plot(X, np.sin(X))
    twin = ax.twiny()
    twin.plot(VALUES, X)
    ax.set_ylim(-2, 8)
    twin.set_autoscale_on(False)
    assert twin.get_xlim() == (0.6, 9.4)


def test_mathtext_opens_its_default_font_before_parsing_and_its_glyph_fonts_while():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        pltrs.rcParams["text.kerning_factor"] = 11
    fig = Figure()
    FigureCanvasAgg(fig)
    fig.text(0.5, 0.5, r"$x^2 + \alpha$")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fig.canvas.draw()
    assert all("_kerning_factor" in str(w.message) for w in caught)
    # DejaVu Sans as the fontset is made, pointing at the program; DejaVu
    # Sans Display and Oblique while the formula is parsed, pointing
    # outside it.
    assert [w.filename == __file__ for w in caught] == [
        True, True, False, False, False, False]


def test_a_minor_locator_shared_with_a_twin_places_the_parents_ticks():
    # ultraplot's `alty`: the twin joins the parent's sharing group and
    # takes its minor locator (now set to the twin's axis) and scale, so
    # ticking the parent reads the majors of the twin's axis.
    fig, ax = plt.subplots()
    ax.minorticks_on()
    twin = ax.inset_axes([0, 0, 1, 1])
    twin._shared_axes["x"].join(twin, ax)
    twin.xaxis.set_minor_locator(ax.xaxis.get_minor_locator())
    twin.xaxis._scale = ax.xaxis._scale
    fig.canvas.draw()
    locs = [float(tick.get_loc()) for tick in ax.xaxis.get_minor_ticks()[:4]]
    assert locs == pytest.approx([0.05, 0.1, 0.15, 0.25])
