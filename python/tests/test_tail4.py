"""
What task TAIL4 found pltrs doing otherwise than matplotlib 3.11: the
font lookups of texts drawn through offset boxes, legends and tables, an
`AnnotationBbox` only given its figure and drawn by the program, the
default `ScalarFormatter`s of polar and 3-D Axes, the warnings of bars of
complex numbers as they are made, and what legend handles read back of
their entries' styles. Each expected value is recorded from matplotlib.
"""

import io
import logging
import pickle
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager, markers
from pltrs.offsetbox import (AnchoredOffsetbox, AnchoredText, AnnotationBbox, HPacker,
                             OffsetImage, TextArea, VPacker)

#: A family no font has: each lookup of it logs.
NOPE = "xkcdnope"

RED = np.array([1.0, 0, 0])


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


def lookups(work):
    """How many times *work* looks up the font of family `NOPE`."""
    logged = []

    class Handler(logging.Handler):
        def emit(self, record):
            if NOPE in record.getMessage():
                logged.append(record)

    handler = Handler()
    log = font_manager._log
    log.addHandler(handler)
    try:
        work()
    finally:
        log.removeHandler(handler)
    return len(logged)


# -- font lookups of texts drawn through boxes ------------------------------------

def _text_area(ax):
    ax.add_artist(AnchoredOffsetbox("center", child=TextArea("t", textprops={"family": NOPE})))


def _packed(ax):
    areas = [TextArea("a", textprops={"family": NOPE}), TextArea("b")]
    ax.add_artist(AnchoredOffsetbox("center", child=HPacker(children=areas, pad=0, sep=4)))


def _anchored_text(ax):
    ax.add_artist(AnchoredText("t", loc="center", prop={"family": NOPE}))


def _annotation_box(ax):
    ax.add_artist(AnnotationBbox(TextArea("t", textprops={"family": NOPE}), (0.5, 0.5)))


def _empty_area(ax):
    areas = [TextArea("", textprops={"family": NOPE}), TextArea("b")]
    ax.add_artist(AnnotationBbox(VPacker(children=[HPacker(children=areas)]), (0.5, 0.5)))


def _legend_text(ax):
    ax.plot([0, 1], label="l")
    ax.legend(prop={"family": NOPE})


def _legend_title(ax):
    ax.plot([0, 1], label="l")
    ax.legend(title="T", title_fontproperties={"family": NOPE})


def _table_cell(ax):
    tab = ax.table(cellText=[["c"]], loc="center")
    tab[0, 0].get_text().set_fontfamily(NOPE)


def _draw(fig):
    fig.canvas.draw()


def _tight_save(fig):
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")


def _svg_save(fig):
    fig.savefig(io.BytesIO(), format="svg")


#: The lookups of a draw, a tight save and an SVG save after a first draw,
#: under no layout engine, then the constrained one and the tight one.
LOOKUPS = [
    (_text_area, [4, 10, 6, 6, 12, 15, 6, 12, 15]),
    (_packed, [5, 12, 7, 7, 14, 17, 7, 14, 17]),
    (_anchored_text, [4, 10, 6, 6, 12, 15, 6, 12, 15]),
    (_annotation_box, [4, 12, 6, 10, 18, 19, 10, 18, 19]),
    (_empty_area, [4, 12, 5, 10, 18, 17, 10, 18, 17]),
    (_legend_text, [7, 16, 9, 9, 18, 21, 9, 18, 21]),
    (_legend_title, [4, 10, 6, 6, 12, 15, 6, 12, 15]),
    (_table_cell, [3, 7, 4, 5, 9, 10, 5, 9, 10]),
]


@pytest.mark.parametrize("make, expected", LOOKUPS, ids=[m.__name__[1:] for m, _ in LOOKUPS])
def test_texts_of_boxes_look_their_font_up_as_matplotlibs_layouts_do(make, expected):
    counts = []
    for layout in [None, "constrained", "tight"]:
        for action in [_draw, _tight_save, _svg_save]:
            fig, ax = plt.subplots(figsize=(3, 2), dpi=50, layout=layout)
            make(ax)
            fig.canvas.draw()
            counts.append(lookups(lambda: action(fig)))
            plt.close(fig)
    assert counts == expected


def test_the_first_measure_of_a_box_measures_its_texts_uncached():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    _packed(ax)
    # A layout, then the uncached measures of the text and of "lp".
    assert lookups(lambda: fig.get_tightbbox()) == 3
    assert [lookups(lambda: _draw(fig)) for _ in range(2)] == [5, 5]


# -- an annotation box only given its figure --------------------------------------

def _figure_box(fig, **kwargs):
    box = AnnotationBbox(OffsetImage(np.tile(RED, (30, 30, 1)), zoom=2), (0.5, 0.5),
                         xycoords="figure fraction", **kwargs)
    box.set_figure(fig)
    return box


@pytest.mark.parametrize("how", ["after a draw", "draw_artist", "from a draw"])
def test_an_annotation_box_only_given_its_figure_draws_where_the_program_draws_it(how):
    fig = plt.figure(figsize=(3, 3), dpi=50)
    box = _figure_box(fig, frameon=False)
    if how == "from a draw":
        fig.canvas.mpl_connect("draw_event", lambda event: box.draw(event.renderer))
    fig.canvas.draw()
    if how == "after a draw":
        box.draw(fig.canvas.get_renderer())
    elif how == "draw_artist":
        fig.draw_artist(box)
    assert red_pixels(fig) == 1764
    extent = box.get_window_extent(fig.canvas.get_renderer())
    assert np.allclose(extent.bounds, [51.3889, 51.3889, 47.2222, 47.2222], atol=1e-3)
    assert fig.artists == [] and box.figure is fig


def test_an_annotation_box_only_given_its_figure_measures_and_draws_its_arrow():
    fig = plt.figure(figsize=(3, 3), dpi=50)
    box = AnnotationBbox(OffsetImage(np.tile(RED, (30, 30, 1)), zoom=2), (0.5, 0.5),
                         xybox=(0.2, 0.2), xycoords="figure fraction",
                         arrowprops={"color": "r"})
    box.set_figure(fig)
    fig.canvas.draw()
    box.draw(fig.canvas.get_renderer())
    assert red_pixels(fig) == 1787
    extent = box.get_window_extent(fig.canvas.get_renderer())
    assert np.allclose(extent.bounds, [6.3889, 6.3889, 67.6270, 67.6270], atol=1e-3)


def test_an_annotation_box_given_a_figure_and_no_axes_fails_on_data_coordinates():
    fig = plt.figure()
    box = AnnotationBbox(TextArea("t"), (0.5, 0.5))
    box.set_figure(fig)
    fig.canvas.draw()
    with pytest.raises(AttributeError, match="transData"):
        box.draw(fig.canvas.get_renderer())


def test_an_annotation_box_only_given_its_figure_joins_it_when_added():
    fig = plt.figure(figsize=(3, 3), dpi=50)
    box = _figure_box(fig, frameon=False)
    assert fig.add_artist(box) is box
    fig.canvas.draw()
    assert red_pixels(fig) == 1764
    assert fig.artists == [box]


# -- the default scalar formatters of polar and 3-D Axes --------------------------

CMR10 = "cmr10 font should ideally be used with mathtext"


def test_polar_axes_make_the_formatters_of_matplotlibs_two_clears():
    pltrs.rcParams["font.family"] = "cmr10"
    fig = plt.figure()
    made = []
    assert warned(lambda: made.append(fig.add_subplot(projection="polar")), CMR10) == 8
    polar = made[0]
    assert warned(polar.cla, CMR10) == 4
    assert warned(lambda: polar.set_rscale("linear"), CMR10) == 1
    assert warned(polar.xaxis.clear, CMR10) == 1
    assert warned(lambda: plt.subplots(subplot_kw={"projection": "polar"}), CMR10) == 8


def test_a_polar_axes_sharing_an_axis_keeps_its_partners_scale_when_cleared():
    pltrs.rcParams["font.family"] = "cmr10"
    fig = plt.figure()
    first = fig.add_subplot(1, 2, 1, projection="polar")
    second = fig.add_subplot(1, 2, 2, projection="polar", sharex=first)
    assert warned(second.cla, CMR10) == 4
    assert warned(first.cla, CMR10) == 5


def test_3d_axes_make_a_formatter_per_axis_made_and_cleared():
    pltrs.rcParams["font.family"] = "cmr10"
    fig = plt.figure()
    made = []
    assert warned(lambda: made.append(fig.add_subplot(projection="3d")), CMR10) == 6
    ax = made[0]
    assert warned(ax.cla, CMR10) == 3
    assert warned(lambda: ax.set_zscale("linear"), CMR10) == 1
    assert warned(ax.zaxis.clear, CMR10) == 1
    assert warned(lambda: fig.add_subplot(projection="3d", sharez=ax), CMR10) == 7
    assert warned(lambda: fig.add_subplot(projection="3d", sharez=ax), CMR10) == 8


# -- bars of complex numbers, as they are made --------------------------------------

CAST = "Casting complex values to real discards the imaginary part"


def _cast_warnings(work):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        work()
    return [w.filename.rsplit("/", 1)[-1] for w in caught if CAST in str(w.message)]


@pytest.mark.parametrize("make, expected", [
    (lambda ax: ax.bar(np.array([0 + 1j, 1 + 0j]), [1, 2]), (3, 4)),
    (lambda ax: ax.bar([0, 1, 2], np.array([1 + 2j, 3 + 0j, 2 - 1j])), (1, 3)),
    (lambda ax: ax.barh(np.array([0 + 1j, 1 + 0j]), [1, 2]), (3, 4)),
], ids=["complex positions", "complex heights", "complex positions, horizontal"])
def test_bars_of_complex_numbers_warn_per_extent_as_they_are_made(make, expected):
    fig, ax = plt.subplots()
    files = _cast_warnings(lambda: make(ax))
    # The units' first finite value, then each bar's box for the limits.
    assert (files.count("cbook.py"), files.count("transforms.py")) == expected
    assert len(files) == sum(expected)


# -- legend handles' styles ----------------------------------------------------------

DASH = (4, (8, 3))


def test_a_bar_handle_reads_back_its_own_solid_line_style():
    fig, ax = plt.subplots()
    ax.bar([0, 1], [1, 2], ls=DASH, lw=2, label="h")
    handle = ax.legend().legend_handles[0]
    assert handle.get_linestyle() == "solid"


def test_a_poly_collection_handle_reads_back_the_collections_dash_pattern():
    fig, ax = plt.subplots()
    ax.fill_between([0, 1, 2], [0, 1, 0], lw=2, ls=DASH, label="h")
    offset, dashes = ax.legend().legend_handles[0].get_linestyle()
    assert (offset, dashes) == (8, [16, 6])
    assert all(type(value) is np.int64 for value in [offset, *dashes])


def test_a_stem_handle_scales_the_dashes_of_the_stems():
    fig, ax = plt.subplots()
    ax.stem([0, 1, 2], [1, 2, 1], linefmt="C1--", label="h")
    offset, dashes = ax.legend().legend_handles[0]._dash_pattern
    assert offset == 0 and np.allclose(dashes, [8.325, 3.6])
    assert all(type(value) is np.float64 for value in [offset, *dashes])


def test_a_lines_marker_is_named_as_matplotlibs_and_pickles():
    fig, ax = plt.subplots(subplot_kw={"projection": "polar"})
    line, = ax.plot([0, 1], [0, 1], marker="o")
    assert type(line._marker).__qualname__ == "MarkerStyle"
    assert isinstance(line._marker, markers.MarkerStyle)
    again = pickle.loads(pickle.dumps(fig))
    assert again.axes[0].lines[0]._marker.get_marker() == "o"


def test_a_line_collection_handle_reads_back_the_dash_pattern_as_given():
    # The classic style matplotlib's suite runs in: dashes are not scaled.
    with pltrs.style.context(["classic", "_classic_test_patch"]):
        fig, ax = plt.subplots()
        lines = pltrs.collections.LineCollection([[(0, .5), (.5, 1)]], linestyles="--", lw=3)
        ax.add_collection(lines)
        handle = ax.legend([lines], ["l"]).legend_handles[0]
        assert handle._dash_pattern == (0.0, (6.0, 6.0))
        assert handle._dash_pattern == lines.get_linestyles()[0]


@pytest.mark.parametrize("boxcoords", ["axes pixels", "axes points", "axes fraction"])
def test_an_annotation_box_placed_in_axes_coordinates_is_picked(boxcoords):
    from pltrs.backend_bases import MouseButton, MouseEvent
    fig, ax = plt.subplots()
    box = AnnotationBbox(TextArea("\N{Black Square}", textprops={"fontsize": 5}),
                         (0.5, 0.5), boxcoords=boxcoords)
    box.set_picker(True)
    ax.add_artist(box)
    picked = []
    fig.canvas.mpl_connect("pick_event", lambda event: picked.append(event.artist))
    x, y = ax.transAxes.transform((0, 0) if boxcoords != "axes fraction" else (0.5, 0.5))
    shift = {"axes pixels": 0.5, "axes points": 0.5 * fig.dpi / 72, "axes fraction": 0}
    fig.canvas.draw()
    MouseEvent("button_press_event", fig.canvas, x + shift[boxcoords],
               y + shift[boxcoords], MouseButton.LEFT)._process()
    assert picked == [box]


@pytest.mark.parametrize("arrow, loc, expected", [
    (False, "best", (59, 140)), (True, "best", (62, 148)),
    (False, "upper left", (57, 135)), (True, "upper left", (58, 138)),
], ids=["best", "best, arrow", "upper left", "upper left, arrow"])
def test_a_best_placed_legend_measures_the_texts_it_avoids(arrow, loc, expected):
    # matplotlib's `_auto_legend_data` measures each text of the Axes (and
    # an annotation's arrow places it first) as the legend is placed.
    pltrs.rcParams["font.family"] = NOPE
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="energy")
    ax.annotate("friday", xy=(1, 1), xytext=(0.5, 0.5),
                **({"arrowprops": {"arrowstyle": "->"}} if arrow else {}))
    ax.legend(loc=loc)
    fig.canvas.draw()
    assert (lookups(lambda: _draw(fig)), lookups(lambda: _tight_save(fig))) == expected


def test_a_box_and_a_legend_measured_without_a_renderer_measure_through_the_figures():
    # matplotlib asks the figure for its renderer, whose text metrics cache
    # holds the lines the draw measured.
    fig, ax = plt.subplots()
    box = ax.add_artist(AnchoredText("box", loc="center", prop={"family": NOPE}))
    ax.plot([0, 1], label="l")
    legend = ax.legend(prop={"family": NOPE})
    fig.canvas.draw()
    assert [lookups(box.get_window_extent) for _ in range(2)] == [1, 1]
    assert [lookups(legend.get_window_extent) for _ in range(2)] == [1, 1]
