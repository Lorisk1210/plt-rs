"""
The warnings the differential tester's programs (eighth round) found
pltrs giving otherwise than matplotlib 3.11: numpy's runtime warnings
where matplotlib evaluates its transform graph while it measures (tight
boxes, layout engines, tight prints), for the artists the replay had no
part for (legends, annotations, images, texts with a box), in figures
that were pickled, and those of matplotlib's own numpy arithmetic and of
its log messages elsewhere. Each expected count is recorded from
matplotlib.
"""

import collections
import io
import logging
import pickle
import warnings

import numpy as np
import pytest

import build_facts
import pltrs
import pltrs.patches as mpatches
import pltrs.pyplot as plt

INF = float("inf")
NAN = float("nan")

#: Counts recorded with Linux' numpy, as this system's numpy counts them
#: (`build_facts.PRODUCTS_WARN`).
here = build_facts.as_numpy_counts

DOT = "invalid value encountered in dot"
MULTIPLY = "invalid value encountered in scalar multiply"
BY_ZERO = "divide by zero encountered in scalar divide"
NO_TRANSPARENCY = ("The PostScript backend does not support transparency; "
                   "partially transparent artists will be rendered opaque.")

#: What the first draw of a figure whose y view has no width computes.
FIRST = {BY_ZERO: 1, MULTIPLY: 1}


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Logged(logging.Handler):
    """The messages logged meanwhile."""

    def __init__(self):
        super().__init__(logging.WARNING)
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage())


def warned(call):
    """How often each warning and logged message (its first line) came
    while *call* ran; what it raised fails the test."""
    logged = _Logged()
    logging.getLogger().addHandler(logged)
    try:
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            call()
    finally:
        logging.getLogger().removeHandler(logged)
    return dict(collections.Counter(
        str(message).split("\n")[0]
        for message in [warning.message for warning in caught] + logged.messages))


def quietly(call):
    """What *call* returns, its warnings dropped."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return call()


def plotted():
    """A figure with a line."""
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1, 2], [1, 3, 2])
    return fig, ax


def flat(add=None):
    """A figure with a line (and what *add* adds to its Axes) whose y view
    has no width."""
    fig, ax = plotted()
    if add is not None:
        quietly(lambda: add(ax))
    ax.yaxis.set_view_interval(0, 0, ignore=True)
    return fig, ax


def padded():
    """A drawn figure with a line whose y tick labels are an infinite
    distance from the Axes."""
    fig, ax = plotted()
    fig.canvas.draw()
    ax.tick_params(axis="y", pad=INF)
    return fig, ax


def saved(fig, **kwargs):
    """Save *fig* to nowhere."""
    return lambda: fig.savefig(io.BytesIO(), **kwargs)


# -- measuring evaluates the graph ----------------------------------------------------------

def test_a_tight_box_computes_the_nodes_a_draw_would():
    fig, ax = flat()
    assert warned(ax.get_tightbbox) == here({**FIRST, DOT: 6})
    assert warned(ax.get_tightbbox) == {}, "kept"
    # The line's path is all that is left to the draw.
    assert warned(fig.canvas.draw) == here({DOT: 2})


def test_tight_boxes_measure_the_tick_labels_each_time():
    fig, ax = padded()
    assert warned(ax.get_tightbbox) == here({DOT: 10})
    assert warned(lambda: ax.get_tightbbox(for_layout_only=True)) == here({DOT: 10})
    assert warned(fig.get_tightbbox) == here({DOT: 10})
    assert warned(ax.yaxis.get_tightbbox) == here({DOT: 10})
    assert warned(ax.xaxis.get_tightbbox) == {}
    assert warned(fig.canvas.draw) == here({DOT: 15})


def test_tight_prints_draw_measure_and_draw():
    fig, _ = padded()
    assert warned(saved(fig, format="png", bbox_inches="tight")) == here({DOT: 40})
    assert warned(saved(fig, format="svg", bbox_inches="tight")) == here({DOT: 40})


def test_layout_engines_measure_every_axes_twice():
    fig, _ = padded()
    assert warned(fig.tight_layout) == here({DOT: 10}), "once without a rectangle"
    for engine in ("tight", "constrained"):
        fig, _ = padded()
        fig.set_layout_engine(engine)
        assert warned(fig.canvas.draw) == here({DOT: 35}), engine
        assert warned(fig.canvas.draw) == here({DOT: 35}), engine


def test_titles_are_placed_past_the_y_axis_measured_again():
    fig, ax = padded()
    ax.set_title("title")
    assert warned(fig.canvas.draw) == here({DOT: 25})
    assert warned(ax.get_tightbbox) == here({DOT: 20})


def test_texts_and_legends_asked_for_their_extent():
    fig, ax = flat(lambda ax: ax.legend(["a"]))
    quietly(fig.canvas.draw)
    assert warned(ax.get_legend().get_window_extent) == here({DOT: 2})
    assert warned(ax.get_legend().get_tightbbox) == here({DOT: 2})
    # A legend placed at the best location stays clear of the texts too.
    fig, ax = flat(lambda ax: (ax.text(1, 2, "text"), ax.legend(["a"])))
    quietly(fig.canvas.draw)
    assert warned(ax.texts[0].get_window_extent) == here({DOT: 2})
    assert warned(ax.get_legend().get_window_extent) == here({DOT: 4})


# -- the artists of a wild Axes -------------------------------------------------------------

@pytest.mark.parametrize("add, first, again, measured", [
    (lambda ax: ax.legend(["a"]), 12, 6, 2),
    (lambda ax: (ax.bar([1, 2], [2, 3], label="b"), ax.legend()), 24, 18, 6),
    (lambda ax: ax.legend(["a"], loc="upper left"), 8, 2, 0),
    (lambda ax: ax.annotate("note", (1, 2)), 10, 4, 2),
    (lambda ax: ax.annotate("note", (1, 2), xytext=(0.5, 0.5), textcoords="axes fraction",
                            arrowprops=dict(arrowstyle="->")), 10, 4, 2),
    (lambda ax: ax.text(1, 2, "text"), 10, 4, 2),
    (lambda ax: ax.text(1, 2, "text", bbox=dict(facecolor="w")), 10, 4, 2),
    (lambda ax: ax.fill_between([0, 1, 2], [1, 2, 1]), 12, 6, 0),
    (lambda ax: ax.errorbar([0, 1], [1, 2], yerr=0.1), 14, 8, 0),
    (lambda ax: ax.pcolormesh([[1, 2], [3, 4]]), 10, 4, 0),
    (lambda ax: ax.scatter([0, 1], [1, 2]), 10, 4, 0),
], ids=["legend", "legend-of-bars", "legend-placed", "annotation", "annotation-arrow",
        "text", "text-box", "fill", "errorbar", "mesh", "scatter"])
def test_artists_evaluate_the_data_transform_as_matplotlibs_draws(add, first, again, measured):
    fig, ax = flat(add)
    assert warned(fig.canvas.draw) == here({**FIRST, DOT: first})
    assert warned(fig.canvas.draw) == here({DOT: again})
    assert warned(ax.get_tightbbox).get(DOT, 0) == here({DOT: measured}).get(DOT, 0)
    assert warned(saved(fig, format="pdf")) == here({DOT: first})


def test_images_map_their_extent():
    fig, ax = flat(lambda ax: ax.imshow([[1, 2], [3, 4]], clip_on=False))
    assert warned(fig.canvas.draw) == here({**FIRST, DOT: 5})
    assert warned(fig.canvas.draw) == here({DOT: 2})
    assert warned(ax.get_tightbbox) == here({DOT: 1})


# -- figures that were pickled --------------------------------------------------------------

def test_an_unpickled_figure_keeps_what_the_pickled_one_computed():
    fig, _ = flat()
    assert warned(fig.canvas.draw) == here({**FIRST, DOT: 8})
    loaded = pickle.loads(pickle.dumps(fig))
    # The limits' matrix is kept; what is built on the resolution, which
    # unpickling sets anew, is computed again.
    assert warned(loaded.canvas.draw) == here({DOT: 8})
    assert warned(loaded.canvas.draw) == here({DOT: 2})
    again = pickle.loads(pickle.dumps(loaded))
    assert warned(again.axes[0].get_tightbbox) == here({DOT: 6})


# -- what a draw drew before it failed -----------------------------------------------------

def test_a_failing_print_logs_what_it_drew_before():
    fig, ax = plotted()
    ax.scatter([0, 1], [1, 2], alpha=0.5)
    ax.scatter([0, 1], [1, 2], linewidths="ab")

    def failing():
        with pytest.raises(ValueError, match="could not convert string to float"):
            fig.savefig(io.BytesIO(), format="eps")

    assert warned(failing) == {NO_TRANSPARENCY: 1}
    fig, ax = plotted()
    ax.scatter([0, 1], [1, 2], linewidths="ab")
    ax.scatter([0, 1], [1, 2], alpha=0.5)
    assert warned(failing) == {}, "the transparent one comes after the failure"


def test_a_rasterized_inset_indicator_is_drawn_as_it_is():
    fig, ax = plotted()
    indicator = ax.indicate_inset_zoom(ax.inset_axes([0.6, 0.6, 0.3, 0.3]))
    indicator.set(rasterized=True)
    assert warned(saved(fig, format="ps")) == {NO_TRANSPARENCY: 1}


def test_contour_labels_take_the_alpha_of_their_set_when_it_changes():
    _, ax = plotted()
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    contours = ax.contour(z, levels=3)
    contours.clabel()
    contours.set_alpha(0.5)
    assert [label.get_alpha() for label in contours.labelTexts][:2] == [None, None]
    contours.set_cmap("plasma")
    assert [label.get_alpha() for label in contours.labelTexts][:2] == [0.5, 0.5]


# -- matplotlib's own arithmetic ------------------------------------------------------------

def test_coherence_divides_as_numpy():
    _, ax = plotted()
    x = np.sin(np.arange(256) * 0.3)
    y = np.cos(np.arange(256) * 0.2)
    divide = "invalid value encountered in divide"
    assert warned(lambda: ax.cohere(x, y[:0], NFFT=64)) == {divide: 1}
    # A NaN rate divides the complex spectra by NaN: numpy warns of it
    # where its complex division raises the invalid exception.
    nan_rate = {divide: 3} if build_facts.COMPLEX_DIVISION_OF_NAN_WARNS else {}
    assert warned(lambda: ax.cohere(x, y, NFFT=64, Fs=NAN)) == nan_rate
    assert warned(lambda: ax.cohere(x, y, NFFT=64)) == {}


def test_bars_of_infinite_size_in_3d():
    ax = plt.figure().add_subplot(projection="3d")
    assert warned(lambda: ax.bar3d([0, 1], [0, 1], [0, 0], INF, 0.5, [1, 2])) == {
        "invalid value encountered in multiply": 1,
        "invalid value encountered in subtract": 2}
    assert warned(lambda: ax.bar3d([0, 1], [0, 1], [0, 0], 0.5, 0.5, [1, 2])) == {}


def test_scales_warn_of_data_they_cannot_take():
    no_positive = "Data has no positive values, and therefore cannot be log-scaled."
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [-2, -5, -9])
    # Once setting the scale, once more as the limits waiting are read.
    assert warned(lambda: ax.set_yscale("log")) == {no_positive: 2}
    assert warned(fig.canvas.draw) == {}
    assert warned(lambda: (ax.relim(), ax.autoscale_view())) == {no_positive: 1}
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [2, 5, 9])
    ax.set_yscale("log")
    assert warned(lambda: ax.set_yscale("logit")) == {
        "Data has no values between 0 and 1, and therefore cannot be logit-scaled.": 1}


def test_an_image_takes_its_properties_before_its_data():
    _, ax = plotted()
    rgb = np.linspace(-2, 2, 36).reshape(3, 4, 3)

    def unknown():
        with pytest.raises(AttributeError, match="unexpected keyword argument"):
            ax.imshow(rgb, nonexistent_kw=1)

    assert warned(unknown) == {}, "no data clipped"
    assert len(warned(lambda: ax.imshow(rgb))) == 1


def test_an_axis_artist_label_an_infinite_distance_away():
    import pltrs.mpl_toolkits.axisartist as axisartist
    fig = plt.figure(figsize=(3, 2), dpi=50)
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.set_xlabel("x")
    ax.axis["bottom"].label.set_pad(INF)
    assert warned(fig.canvas.draw) == here({DOT: 1})
    ax.axis["bottom"].label.set_pad(NAN)
    assert warned(fig.canvas.draw) == {}


def test_a_fancy_box_stretched_infinitely():
    fig, ax = plotted()
    box = ax.add_patch(mpatches.FancyBboxPatch((0.2, 0.2), 0.4, 0.3))
    box.set_mutation_aspect(INF)
    assert warned(fig.canvas.draw) == {"invalid value encountered in multiply": 1}


def test_an_extreme_finder_of_a_box_that_is_no_number():
    from pltrs.mpl_toolkits.axisartist.grid_finder import ExtremeFinderSimple
    finder = ExtremeFinderSimple(20, 20)
    assert warned(lambda: finder(lambda x, y: (x, y), INF, 0, 1, 1)) == {
        "invalid value encountered in multiply": 1,
        "invalid value encountered in add": 1}
    assert warned(lambda: finder(lambda x, y: (x, y), 0, 0, INF, 1)) == {
        "invalid value encountered in multiply": 1,
        "invalid value encountered in scalar subtract": 2}
    assert warned(lambda: finder(lambda x, y: (x, y), 0, 0, 1, 1)) == {}


# -- the fonts matplotlib opens and looks up ------------------------------------------------

def test_fonts_opened_with_the_deprecated_kerning_factor_warn_once_each():
    deprecated = ("The _kerning_factor parameter was deprecated in Matplotlib 3.11 "
                  "and will be removed in 3.13.")
    quietly(lambda: pltrs.rcParams.update({"text.kerning_factor": 7}))
    fig = plt.figure(figsize=(3, 2), dpi=50)
    fig.text(0.5, 0.5, "sans")
    # The font and the last resort behind it, opened once (and kept).
    assert warned(fig.canvas.draw) == {deprecated: 2}
    assert warned(fig.canvas.draw) == {}
    fig.text(0.5, 0.2, "serif", family="serif")
    assert warned(fig.canvas.draw) == {deprecated: 2}


@pytest.mark.parametrize("rc, fmt, logged", [("ps.useafm", "ps", 1),
                                             ("pdf.use14corefonts", "pdf", 2)])
def test_afm_fonts_of_another_weight_are_logged_as_they_are_looked_up(rc, fmt, logged):
    pltrs.rcParams[rc] = True
    fig = plt.figure(figsize=(3, 2), dpi=50)
    # (A size no other test looks fonts up at: the font manager keeps
    # what it found.)
    fig.text(0.5, 0.5, "afm", size=11.25 + logged)
    assert warned(saved(fig, format=fmt)) == {
        "findfont: Failed to find font weight normal for Helvetica, now using medium.": logged}
    assert warned(saved(fig, format=fmt)) == {}


# -- nothing for finite input ---------------------------------------------------------------

def test_finite_figures_warn_of_nothing():
    fig, ax = plotted()
    ax.legend(["a"])
    ax.annotate("note", (1, 2), xytext=(0.5, 0.5), textcoords="axes fraction",
                arrowprops=dict(arrowstyle="->"))
    ax.imshow([[1, 2], [3, 4]])
    ax.set_title("title")
    ax.set_yscale("log")

    def use():
        fig.canvas.draw()
        fig.get_tightbbox()
        fig.tight_layout()
        fig.savefig(io.BytesIO(), format="pdf", bbox_inches="tight")
        pickle.loads(pickle.dumps(fig)).canvas.draw()

    assert warned(use) == {}
