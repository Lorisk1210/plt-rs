"""
The differences no task owned (task TAIL3), where pltrs did otherwise
than matplotlib 3.11: artists drawn by a program's call of their `draw`
though no figure lists them, ticks read before a first draw after a
measure, the text font a default `ScalarFormatter` checks, font lookups
replayed with their stretch, bars of complex numbers drawn, what a legend
handle takes over from its artist, and the clip box given to a plotting
method. Each expected value is recorded from matplotlib.
"""

import logging
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.patheffects as pe
import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
from pltrs import font_manager, lines, table, ticker
from pltrs.axes import Axes
from pltrs.figure import Figure
from pltrs.legend import Legend
from pltrs.markers import MarkerStyle
from pltrs.transforms import Bbox

X = np.linspace(0, 1, 9)


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def red_pixels(fig):
    img = np.asarray(fig.canvas.buffer_rgba())
    return int(((img[..., 0] > 200) & (img[..., 1] < 60) & (img[..., 2] < 60)).sum())


def warned(work, text):
    """How many warnings saying *text* *work* issues."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        work()
    return sum(text in str(w.message) for w in caught)


# -- artists no figure lists, drawn by their draw -------------------------------

def test_an_axes_never_added_draws_when_another_artist_draws_it():
    class Container(Axes):
        def draw(self, renderer):
            super().draw(renderer)
            self.external.draw(renderer)

    fig = plt.figure(figsize=(3, 3))
    host = fig.add_subplot(axes_class=Container)
    host.external = Axes(fig, (0.3, 0.3, 0.4, 0.4))
    host.external.plot([0, 1], [0, 1], color="red", lw=4)
    fig.canvas.draw()
    assert red_pixels(fig) == 788


@pytest.mark.parametrize("how", ["after a draw", "draw_artist"])
def test_artists_no_figure_lists_draw_where_the_program_draws_them(how):
    fig = plt.figure(figsize=(3, 3), dpi=50)
    host = fig.add_subplot()
    host.set_axis_off()
    line = lines.Line2D([0.3, 0.7], [0.3, 0.7], color="red", lw=4,
                        transform=fig.transFigure)
    legend = Legend(host, [lines.Line2D([], [], color="red", lw=10)], ["red"])
    tab = table.Table(host, bbox=[0.2, 0.2, 0.6, 0.6])
    tab.add_cell(0, 0, 1, 1, facecolor="red", text="")
    counts = []
    for artist in (line, legend, tab):
        fig.canvas.draw()
        if how == "after a draw":
            artist.draw(fig.canvas.get_renderer())
        else:
            fig.draw_artist(artist)
        counts.append(red_pixels(fig))
    assert counts == [184, 147, 4692]
    assert [round(float(v), 1) for v in tab.get_window_extent().bounds] == [
        42.0, 39.6, 69.8, 69.3]


def test_an_axes_removed_fails_to_draw_for_want_of_a_figure():
    fig = plt.figure(figsize=(3, 3), dpi=50)
    fig.canvas.draw()
    ax = fig.add_axes((0.3, 0.3, 0.4, 0.4))
    ax.remove()
    with pytest.raises(AttributeError, match="no attribute 'canvas'"):
        ax.draw(fig.canvas.get_renderer())


# -- ticks before a first draw ---------------------------------------------------

@pytest.mark.parametrize("measure", ["spine", "axis"])
def test_measures_before_a_draw_tick_where_the_axes_stands(measure):
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.set_aspect("equal")
    ax.plot([0, 1], [0, 1])
    if measure == "spine":
        box = ax.spines["bottom"].get_window_extent()
        assert [round(float(v), 1) for v in box.bounds] == [50.0, 28.1, 310.0, 4.9]
    else:
        ax.xaxis.get_tightbbox()
    assert [t.get_text() for t in ax.get_xticklabels()] == [
        '−0.2', '0.0', '0.2', '0.4', '0.6', '0.8', '1.0', '1.2']


# -- the text font a default ScalarFormatter checks ------------------------------

def test_every_default_scalar_formatter_warns_of_cmr10_without_mathtext():
    pltrs.rcParams["font.family"] = "cmr10"
    fig = plt.figure()
    cmr10 = "cmr10 font should ideally be used with mathtext"
    ax = []
    assert warned(lambda: ax.append(fig.add_subplot()), cmr10) == 2
    assert warned(lambda: ax[0].cla(), cmr10) == 2
    assert warned(lambda: ax[0].set_xscale("log"), cmr10) == 0
    assert warned(lambda: ax[0].set_xscale("linear"), cmr10) == 1
    assert warned(lambda: ax[0].twinx(), cmr10) == 2
    assert warned(lambda: Axes(fig, (0, 0, 1, 1)), cmr10) == 2
    assert warned(lambda: ticker.ScalarFormatter(), cmr10) == 1
    assert warned(lambda: ticker.ScalarFormatter(useMathText=False), cmr10) == 0
    pltrs.rcParams["axes.formatter.use_mathtext"] = True
    assert warned(lambda: fig.add_subplot(), cmr10) == 0


def test_font_lookups_of_a_draw_are_cached_as_the_text_asks_them():
    logged = []

    class Handler(logging.Handler):
        def emit(self, record):
            logged.append(record.getMessage())

    log = font_manager._log
    handler = Handler()
    log.addHandler(handler)
    try:
        pltrs.rcParams.update({"font.family": "monospace", "font.weight": 300})
        fig, ax = plt.subplots()
        text = ax.text(0.1, 0.5, "monospace")
        fig.canvas.draw()
        logged.clear()
        text.get_fontname()
    finally:
        log.removeHandler(handler)
    # The text's own lookup (its stretch as given, "normal"), once.
    assert logged == [
        "findfont: Failed to find font weight 300 for DejaVu Sans Mono, now using 400."]


# -- bars of complex numbers --------------------------------------------------------

def test_bars_of_complex_numbers_warn_at_every_draw():
    fig, ax = plt.subplots()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ax.bar([0, 1, 2], np.array([1 + 2j, 3 + 0j, 2 - 1j]))
    cast = "Casting complex values to real discards the imaginary part"
    assert [warned(fig.canvas.draw, cast) for _ in range(2)] == [3, 3]
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ax.barh([0, 1], np.array([1 + 1j, 2 + 0j]))
    assert warned(fig.canvas.draw, cast) == 5


# -- what a legend handle takes over ----------------------------------------------

def test_a_legend_handle_keeps_its_lines_marker_style():
    style = MarkerStyle("1", capstyle="round", joinstyle="round",
                        transform=mtransforms.Affine2D().rotate_deg(30))
    line = lines.Line2D([], [], marker=style, markersize=40, linestyle="none",
                        label="x")
    fig = plt.figure(figsize=(3, 2), dpi=50)
    fig.legend(handles=[line], loc="center")
    fig.canvas.draw()
    marker = fig.legends[0].legend_handles[0]._marker
    assert isinstance(marker, MarkerStyle)
    assert (marker.get_capstyle(), marker.get_joinstyle()) == ("round", "round")
    assert np.round(marker.get_user_transform().get_matrix(), 3).tolist() == [
        [0.866, -0.5, 0.0], [0.5, 0.866, 0.0], [0.0, 0.0, 1.0]]


def test_a_marker_style_given_brings_its_fill_style():
    style = MarkerStyle("o", capstyle="round",
                        transform=mtransforms.Affine2D().rotate_deg(30))
    line = lines.Line2D([], [], marker=style, fillstyle="left")
    assert line.get_fillstyle() == "full"
    line.set_fillstyle("top")
    assert (line.get_fillstyle(), line._marker.get_user_transform()) == ("top", None)
    other = lines.Line2D([], [], marker="o", fillstyle="left")
    other.set_marker(style)
    assert other.get_fillstyle() == "full"


def test_legend_handles_take_what_matplotlibs_handlers_copy():
    fig, ax = plt.subplots()
    plotted, = ax.plot([0, 1], path_effects=[pe.Normal()], label="line")
    scattered = ax.scatter([0, 1, 2], [0, 1, 2], alpha=np.array([0.2, 0.5, 1.0]),
                           label="scatter")
    filled = ax.fill_between([0, 1, 2], [0, 1, 0], hatch="\\\\", edgecolor="b",
                             label="fill")
    filled.set_hatchcolor("g")
    legend = ax.legend()
    fig.canvas.draw()
    line, points, rectangle = legend.legend_handles
    assert len(plotted.get_path_effects()) == len(line.get_path_effects()) == 1
    assert scattered is not points
    assert np.round(points.get_alpha(), 3).tolist() == [0.2, 0.5, 1.0]
    assert tuple(map(float, rectangle.get_hatchcolor())) == (0.0, 0.5, 0.0, 1.0)


# -- the clip box given to a plotting method --------------------------------------

def clip_box_bounds(make, projection=None):
    fig = plt.figure(figsize=(6.4, 4.8))
    ax = fig.add_subplot(projection=projection)
    artist = make(ax, Bbox.from_bounds(10, 10, 100, 100))
    fig.canvas.draw()
    box = artist.get_clip_box()
    return [round(float(v), 1) for v in box.bounds], artist.get_clip_path() is not None


AXES_BOX = [80.0, 52.8, 496.0, 369.6]


@pytest.mark.parametrize("make, expected", [
    (lambda ax, box: ax.plot(X, X, clip_box=box)[0], AXES_BOX),
    (lambda ax, box: ax.scatter(X, X, clip_box=box), AXES_BOX),
    (lambda ax, box: ax.bar(X, X, clip_box=box)[0], AXES_BOX),
    (lambda ax, box: ax.fill_between(X, X, clip_box=box), AXES_BOX),
    (lambda ax, box: ax.axhspan(0.2, 0.8, clip_box=box), AXES_BOX),
    (lambda ax, box: ax.text(0.1, 0.4, "text", clip_box=box), AXES_BOX),
    (lambda ax, box: ax.contour(np.outer(X, X), clip_box=box), AXES_BOX),
    # The square box the image's aspect gives the Axes.
    (lambda ax, box: ax.imshow(np.ones((4, 4)), clip_box=box), [143.2, 52.8, 369.6, 369.6]),
], ids=["plot", "scatter", "bar", "fill_between", "axhspan", "text", "contour", "imshow"])
def test_the_axes_adding_an_artist_clips_it_to_its_patch(make, expected):
    assert clip_box_bounds(make) == (expected, False)


@pytest.mark.parametrize("make, projection", [
    (lambda ax, box: ax.hist(X, clip_box=box)[2][0], None),
    (lambda ax, box: ax.eventplot([X], clip_box=box)[0], None),
    (lambda ax, box: ax.errorbar(X, X, yerr=0.1, clip_box=box)[2][0], None),
    (lambda ax, box: ax.plot(X, X, clip_box=box)[0], "polar"),
], ids=["hist", "eventplot", "errorbar bars", "polar plot"])
def test_a_clip_box_given_after_adding_or_to_a_round_axes_stays(make, projection):
    bounds, path = clip_box_bounds(make, projection)
    assert bounds == [10.0, 10.0, 100.0, 100.0]
    assert path == (projection == "polar")
