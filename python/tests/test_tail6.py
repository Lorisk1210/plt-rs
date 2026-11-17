"""
What task TAIL6 found pltrs doing otherwise than matplotlib 3.11: artists
a program draws without the figure or Axes their draw reads, an image
drawn without a figure, an annotation box given an Axes and no figure,
the default formatters of a polar Axes made sharing an axis, the font
lookups of measuring the texts an axis owns, a geographic Axes made
sharing an axis, and an anchored box placed by the program's
``get_offset`` in an Axes whose aspect shrinks them. Each expected value
is recorded from matplotlib.
"""

import io
import logging
import pickle
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import (collections as mcollections, font_manager, path as mpath,
                   text as mtext)
from pltrs.image import AxesImage
from pltrs.offsetbox import AnchoredOffsetbox, AnnotationBbox, OffsetImage, TextArea
from pltrs.projections.polar import PolarAxes

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


def _drawn_figure():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    fig.canvas.draw()
    return fig, ax, fig.canvas.get_renderer()


# -- artists drawn by the program without a figure --------------------------------

FIGURELESS = [
    ("text", lambda tf: mtext.Text(0.3, 0.5, "x", transform=tf), "dpi"),
    ("annotation in figure fraction",
     lambda tf: mtext.Annotation("x", (0.5, 0.5), xycoords="figure fraction"), "figbbox"),
    ("annotation in data", lambda tf: mtext.Annotation("x", (0.5, 0.5)), "transData"),
    ("poly collection",
     lambda tf: mcollections.PolyCollection([[(0.3, 0.3), (0.7, 0.3), (0.7, 0.7)]],
                                            transform=tf), "dpi"),
    ("hidden poly collection",
     lambda tf: mcollections.PolyCollection([[(0.3, 0.3), (0.7, 0.3), (0.7, 0.7)]],
                                            transform=tf, visible=False), "dpi"),
    ("path collection",
     lambda tf: mcollections.PathCollection([mpath.Path.unit_circle()], sizes=[20],
                                            offsets=[(0.5, 0.5)], offset_transform=tf), "dpi"),
    ("ellipses in points",
     lambda tf: mcollections.EllipseCollection([10], [5], [0], units="points",
                                               offsets=[(0.5, 0.5)], offset_transform=tf),
     "dpi"),
    ("ellipses in data",
     lambda tf: mcollections.EllipseCollection([0.1], [0.05], [0], units="xy",
                                               offsets=[(0.5, 0.5)], offset_transform=tf),
     "transData"),
]


@pytest.mark.parametrize("make, missing", [(m, a) for _, m, a in FIGURELESS],
                         ids=[name for name, _, _ in FIGURELESS])
def test_an_artist_drawn_without_what_its_draw_reads_fails_as_matplotlibs(make, missing):
    fig, ax, renderer = _drawn_figure()
    art = make(fig.transFigure)
    with pytest.raises(AttributeError, match=f"no attribute '{missing}'"):
        art.draw(renderer)


@pytest.mark.parametrize("make", [
    lambda tf: mtext.Text(0.3, 0.5, "x", transform=tf, visible=False),
    lambda tf: mtext.Text(0.3, 0.5, "", transform=tf),
    lambda tf: mcollections.LineCollection([[(0.3, 0.3), (0.7, 0.7)]], transform=tf),
], ids=["hidden text", "empty text", "line collection"])
def test_an_artist_whose_draw_reads_no_figure_draws_without_one(make):
    fig, ax, renderer = _drawn_figure()
    make(fig.transFigure).draw(renderer)


def test_artists_given_their_figure_draw():
    fig, ax, renderer = _drawn_figure()
    for _, make, missing in FIGURELESS:
        if missing == "transData":
            continue
        art = make(fig.transFigure)
        art.set_figure(fig)
        art.draw(renderer)


# -- an image drawn without a figure -----------------------------------------------

@pytest.mark.parametrize("extent, expected", [
    ((0.2, 0.8, 0.2, 0.8), 0), ((10, 40, 10, 40), 504),
    ((60, 90, 60, 90), 900), ((120, 145, 120, 145), 180),
])
def test_an_image_drawn_without_a_figure_is_made_within_its_axes_box(extent, expected):
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.set_axis_off()
    image = AxesImage(ax, extent=extent)
    image.set_data(np.tile(RED, (4, 4, 1)))
    fig.canvas.draw()
    image.draw(fig.canvas.get_renderer())
    assert red_pixels(fig) == expected
    assert image.get_clip_box() is None


def test_an_unclipped_image_drawn_without_a_figure_fails_on_the_figures_box():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    image = AxesImage(ax, extent=(60, 90, 60, 90))
    image.set_data(np.tile(RED, (4, 4, 1)))
    image.set_clip_on(False)
    fig.canvas.draw()
    with pytest.raises(AttributeError, match="no attribute 'bbox'"):
        image.draw(fig.canvas.get_renderer())


# -- an annotation box given an Axes -----------------------------------------------

def test_an_annotation_box_given_an_axes_and_no_figure_draws_in_it():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.set_axis_off()
    box = AnnotationBbox(OffsetImage(np.tile(RED, (30, 30, 1)), zoom=2), (0.5, 0.5),
                         frameon=False)
    box.axes = ax
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    box.draw(renderer)
    assert red_pixels(fig) == 1764
    assert np.allclose(box.get_window_extent(renderer).bounds,
                       [53.2639, 50.6389, 47.2222, 47.2222], atol=1e-3)
    assert box.figure is None and box.axes is ax and list(fig.artists) == []
    assert ax.add_artist(box) is box
    assert box.figure is fig and list(ax.artists) == [box]


# -- the default formatters of a polar Axes made sharing -----------------------------

CMR10 = "cmr10 font should ideally be used with mathtext"


def warned(work):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        work()
    return sum(CMR10 in str(w.message) for w in caught)


def test_a_polar_axes_made_sharing_counts_the_groups_its_clears_rescale():
    pltrs.rcParams["font.family"] = "cmr10"
    fig = plt.figure()
    first = fig.add_subplot(3, 3, 1, projection="polar")
    counts = [
        warned(lambda: fig.add_subplot(3, 3, 2, projection="polar", sharex=first)),
        warned(lambda: fig.add_subplot(3, 3, 3, projection="polar", sharey=first)),
        warned(lambda: fig.add_subplot(3, 3, 4, projection="polar",
                                       sharex=first, sharey=first)),
        warned(lambda: fig.add_axes((0.1, 0.1, 0.2, 0.2), projection="polar", sharex=first)),
        warned(lambda: PolarAxes(fig, (0.5, 0.5, 0.2, 0.2), sharex=first)),
        warned(lambda: plt.subplots(2, 2, sharex=True, sharey=True,
                                    subplot_kw={"projection": "polar"})),
    ]
    assert counts == [7, 7, 8, 9, 10, 32]


# -- the font lookups of measuring the texts an axis owns -----------------------------

MISSING = "a family no font has"


class _Lookups(logging.Handler):
    """Counts the font manager's warnings about the missing family."""

    def __init__(self):
        super().__init__()
        self.count = 0

    def emit(self, record):
        self.count += MISSING in record.getMessage()


AXIS_TEXTS = {
    "x label": lambda ax: ax.set_xlabel("x label"),
    "y label": lambda ax: ax.set_ylabel("y label"),
    "x offset text": lambda ax: ax.xaxis.get_offset_text(),
    "y offset text": lambda ax: ax.yaxis.get_offset_text(),
    "tick label": lambda ax: ax.get_xticklabels()[1],
    "title": lambda ax: ax.set_title("t"),
}
MEASURES = {
    "none": lambda text, renderer: text.get_window_extent(),
    "renderer": lambda text, renderer: text.get_window_extent(renderer),
    "tight box": lambda text, renderer: text.get_tightbbox(renderer),
    "twice": lambda text, renderer: (text.get_window_extent(), text.get_window_extent()),
}


def _lookups(name, measure, drawn):
    fig, ax = plt.subplots()
    ax.plot([0, 1e6], [0, 2e-7])
    ax.set_title("t", loc="left")
    text = AXIS_TEXTS[name](ax)
    text.set_fontfamily(MISSING)
    if drawn:
        fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    lookups = _Lookups()
    font_manager._log.addHandler(lookups)
    try:
        MEASURES[measure](text, renderer)
    finally:
        font_manager._log.removeHandler(lookups)
    return lookups.count


@pytest.mark.parametrize("name", AXIS_TEXTS)
def test_measuring_a_text_an_axis_owns_looks_its_font_up_as_matplotlibs(name):
    # Laid out and its line measured with a fresh renderer, its line found
    # in the renderer's cache the second time and after a draw; an offset
    # text has no text before the first draw.
    empty = name.endswith("offset text")
    before = [0, 0, 0, 0] if empty else [2, 2, 2, 3]
    counts = [_lookups(name, measure, drawn)
              for drawn in (False, True) for measure in MEASURES]
    assert counts == before + [1, 1, 1, 2]


def test_measuring_the_offset_text_asks_the_renderers_cache_for_it_alone():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 2e-7])
    fig.canvas.draw()
    metrics = mtext._get_text_metrics_function(fig.canvas.get_renderer())
    before = metrics.cache_info()
    ax.yaxis.get_offset_text().get_window_extent()
    after = metrics.cache_info()
    assert (after.hits - before.hits, after.misses - before.misses) == (1, 0)


def test_the_offset_text_keeps_its_font_and_reads_what_the_draw_gave_it():
    fig, ax = plt.subplots()
    ax.plot([0, 1e6], [0, 1])
    offset = ax.xaxis.get_offset_text()
    offset.set_fontfamily("serif")
    offset.set_fontweight("bold")
    assert offset.get_text() == ""
    fig.canvas.draw()
    assert offset.get_text() == "1e6"
    assert offset.get_fontfamily() == ["serif"]
    assert offset.get_fontweight() == "bold"


# -- a geographic Axes made sharing an axis -------------------------------------------

GEOGRAPHIC_LIMITS = "Changing axes limits of a geographic projection is not supported"


@pytest.mark.parametrize("make", [
    lambda fig, first: fig.add_subplot(2, 2, 2, projection="aitoff", sharex=first),
    lambda fig, first: fig.add_subplot(2, 2, 2, projection="mollweide", sharey=first),
    lambda fig, first: fig.add_axes((0.1, 0.1, 0.2, 0.2), projection="hammer", sharex=first),
    lambda fig, first: fig.add_subplot(2, 2, 2, projection="lambert").sharex(first),
], ids=["add_subplot sharex", "add_subplot sharey", "add_axes", "sharex"])
def test_a_geographic_axes_made_sharing_refuses_as_its_limits_do(make):
    fig = plt.figure()
    first = fig.add_subplot(2, 2, 1, projection="aitoff")
    with pytest.raises(TypeError, match=GEOGRAPHIC_LIMITS):
        make(fig, first)


def test_a_rectilinear_axes_shares_a_geographic_ones_axis():
    fig = plt.figure()
    first = fig.add_subplot(2, 2, 1, projection="aitoff")
    second = fig.add_subplot(2, 2, 2, sharex=first)
    assert second.get_shared_x_axes().joined(first, second)


# -- an anchored box placed by the program's get_offset ------------------------------

class _ProgramBox(AnchoredOffsetbox):
    def get_offset(self, bbox, renderer):
        return super().get_offset(bbox, renderer)


@pytest.mark.parametrize("loc, extent", [
    ("upper left", [83.7, 143.6, 36.7, 25.4]),
    ("lower right", [187.1, 28.9, 36.7, 25.4]),
])
def test_a_box_placed_by_the_programs_get_offset_follows_the_axes_aspect(loc, extent):
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1], [0, 1])
    ax.imshow(np.arange(12.).reshape(3, 4), extent=(0, 1, 0, 1))
    box = ax.add_artist(_ProgramBox(loc, child=TextArea("box")))
    fig.canvas.draw()
    assert np.round(box.get_window_extent().bounds, 1).tolist() == extent
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png", bbox_inches="tight")
    buffer.seek(0)
    assert plt.imread(buffer).shape[:2] == (202, 216)


class _MeasuredAxis(pltrs.axis.XAxis):
    def _get_ticklabel_bboxes(self, ticks, renderer):
        return super()._get_ticklabel_bboxes(ticks, renderer)


class _MeasuredAxes(pltrs.axes.Axes):
    def _init_axis(self):
        self.xaxis = _MeasuredAxis(self, clear=False)
        self.spines.bottom.register_axis(self.xaxis)
        self.spines.top.register_axis(self.xaxis)
        self.yaxis = pltrs.axis.YAxis(self, clear=False)
        self.spines.left.register_axis(self.yaxis)
        self.spines.right.register_axis(self.yaxis)


def test_a_text_measured_with_a_renderer_pickles_without_it():
    # matplotlib's text pickles no renderer: tick labels a tight SVG print
    # measured with its mixed-mode renderer read back.
    fig = plt.figure(figsize=(3, 2))
    ax = fig.add_subplot(axes_class=_MeasuredAxes)
    ax.plot([0, 1], [0, 1])
    fig.savefig(io.BytesIO(), format="svg", bbox_inches="tight")
    copy = pickle.loads(pickle.dumps(fig))
    assert len(copy.axes[0].lines) == 1


def test_the_offset_text_measures_after_a_draw_through_a_renderer_given_to_the_figure():
    # mplhep moves its label by the width of the offset text, which it
    # measures after ``fig.draw(renderer)``: the text is as wide there as
    # after the canvas' own draw.
    def extent(draw):
        fig, ax = plt.subplots()
        ax.plot([0, 1], [5e6, 6e6])
        renderer = fig.canvas.get_renderer()
        draw(fig, renderer)
        bounds = ax.yaxis.offsetText.get_window_extent(renderer).bounds
        plt.close(fig)
        return bounds

    through_canvas = extent(lambda fig, renderer: fig.canvas.draw())
    through_figure = extent(lambda fig, renderer: fig.draw(renderer))
    assert through_figure[2] > 0
    assert through_figure == pytest.approx(through_canvas)
