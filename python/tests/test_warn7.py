"""
The warnings of matplotlib's own arithmetic that the differential tester's
programs (seventh round) found pltrs giving otherwise than matplotlib
3.11: numpy's runtime warnings where matplotlib computes the matrices of
its transform graph from view limits without width, infinite rectangles,
infinite tick pads and polar settings that are no numbers, as often as
matplotlib computes each node, and the clip paths its renderers refuse.
Each expected count is recorded from matplotlib.
"""

import collections
import io
import logging
import warnings

import numpy as np
import pytest

import build_facts
import pltrs
import pltrs.patches as mpatches
import pltrs.pyplot as plt

INF = float("inf")
NAN = float("nan")

DOT = "invalid value encountered in dot"
ADD = "invalid value encountered in scalar add"
SUBTRACT = "invalid value encountered in scalar subtract"
MULTIPLY = "invalid value encountered in scalar multiply"
DIVIDE = "invalid value encountered in scalar divide"
BY_ZERO = "divide by zero encountered in scalar divide"
REMAINDER = "invalid value encountered in scalar remainder"
NO_CLIP = "Ill-defined clip_path detected. Returning None."


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


def here(counts):
    """*counts*, of Linux' numpy, as this system's numpy counts: without
    the warnings of matrix products where it warns of none
    (`build_facts.PRODUCTS_WARN`)."""
    return {message: counts[message] for message in build_facts.as_numpy_warns(counts)}


def plotted():
    """A figure with a line."""
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1, 2], [1, 3, 2])
    return fig, ax


# -- rectangles whose own transform is no number ----------------------------------------

def test_spans_warn_where_they_are_added_as_matplotlibs_search_for_the_data_transform():
    _, ax = plotted()
    assert warned(lambda: ax.axvspan(INF, 0.4)) == here({DOT: 4, ADD: 1})
    assert warned(lambda: ax.axvspan(0.2, INF)) == here({DOT: 6})
    assert warned(lambda: ax.axhspan(0.2, 0.4, xmin=INF)) == here({DOT: 1, ADD: 1})


def test_spans_warn_at_every_draw_and_relim():
    fig, ax = plotted()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ax.axvspan(INF, 0.4)
        ax.axvspan(0.2, INF)
        ax.axhspan(0.2, 0.4, xmin=INF)
    assert warned(fig.canvas.draw) == here({DOT: 4, ADD: 2})
    assert warned(fig.canvas.draw) == here({DOT: 4, ADD: 2}), "nothing of them is kept"
    assert warned(ax.relim) == here({DOT: 14, ADD: 2})
    assert warned(lambda: fig.savefig(io.BytesIO(), format="svg")) == here({DOT: 4, ADD: 2})


def test_bars_of_infinite_height_warn_added_and_drawn():
    fig, ax = plotted()
    assert warned(lambda: ax.bar([1, 2], [INF, 3])) == here({DOT: 1})
    assert warned(fig.canvas.draw) == here({DOT: 1})
    fig, ax = plotted()
    assert warned(lambda: ax.hist([3.0, 1.0, 4.0], bottom=INF)) == here({
        DOT: 10, ADD: 10, "invalid value encountered in subtract": 10})
    assert warned(fig.canvas.draw) == here({DOT: 10, ADD: 10})


@pytest.mark.parametrize("make, added, drawn", [
    (lambda: mpatches.Rectangle((INF, 0), 1, 2),
     {DOT: 1, ADD: 1, SUBTRACT: 1}, {DOT: 1, ADD: 1, SUBTRACT: 1}),
    (lambda: mpatches.Rectangle((0.5, INF), -0.3, 0.2, angle=45, rotation_point="center"),
     {DOT: 1, SUBTRACT: 2}, {DOT: 1, SUBTRACT: 2}),
    (lambda: mpatches.Ellipse((0, 0), INF, 2), {MULTIPLY: 2}, {DOT: 1, MULTIPLY: 2}),
    (lambda: mpatches.Circle((0, 0), INF), {MULTIPLY: 4}, {MULTIPLY: 4}),
], ids=["rectangle", "rectangle-about-its-centre", "ellipse", "circle"])
def test_patches_made_by_the_program_warn_once_per_computation(make, added, drawn):
    fig, ax = plotted()
    patch = make()
    assert warned(lambda: ax.add_patch(patch)) == here(added)
    assert warned(fig.canvas.draw) == here(drawn)
    assert warned(fig.canvas.draw) == here(drawn)
    patch.set_visible(False)
    assert warned(fig.canvas.draw) == {}, "a hidden patch computes nothing"


# -- view limits without width ----------------------------------------------------------

def test_a_view_without_width_warns_as_the_nodes_built_on_it_are_computed():
    fig, ax = plotted()
    ax.yaxis.set_view_interval(0, 0, ignore=True)
    first = here({BY_ZERO: 1, DOT: 8, MULTIPLY: 1})
    assert warned(fig.canvas.draw) == first
    # The limits' matrix and the blends are kept; the line's is made anew.
    assert warned(fig.canvas.draw) == here({DOT: 2})
    # Setting the limits, to the same values too, invalidates them.
    ax.yaxis.set_view_interval(0, 0, ignore=True)
    assert warned(fig.canvas.draw) == first
    ax.yaxis.set_view_interval(0, 1, ignore=True)
    assert warned(fig.canvas.draw) == {}


def test_the_data_transform_is_composed_at_every_call():
    fig, ax = plotted()
    ax.yaxis.set_view_interval(0, 0, ignore=True)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig.canvas.draw()
    assert warned(ax.transData.get_affine) == here({DOT: 2})
    assert warned(ax.transData.get_affine) == here({DOT: 2})
    assert warned(lambda: ax.transData.transform((0.5, 0.5))) == here({DOT: 2})


def test_a_view_without_width_on_a_log_axis():
    fig, ax = plotted()
    ax.xaxis.set_view_interval(1, 1, ignore=True)
    ax.set_yscale("log")
    assert warned(fig.canvas.draw) == here({BY_ZERO: 1, DOT: 6})


# -- tick labels an infinite distance away ----------------------------------------------

def test_infinite_tick_pads_warn_each_time_a_label_is_placed():
    fig, ax = plotted()
    fig.canvas.draw()
    ax.xaxis.get_major_ticks()[1].set_pad(INF)
    # Measured for the draw, drawn, measured again to place the axis label.
    assert warned(fig.canvas.draw) == here({DOT: 3})
    ax.tick_params(axis="y", pad=INF)
    assert warned(fig.canvas.draw) == here({DOT: 18})


# -- polar Axes ---------------------------------------------------------------------------

def polar():
    """A drawn figure with a polar Axes."""
    fig = plt.figure(figsize=(3, 2), dpi=50)
    ax = fig.add_subplot(projection="polar")
    ax.plot([0, 1, 2], [1, 2, 3])
    fig.canvas.draw()
    return fig, ax


def test_radial_labels_at_an_infinite_angle():
    fig, ax = polar()
    ax.set_rlabel_position(INF)
    # numpy's remainder of an infinity is the C library's fmod, as its
    # floor division is: no warning where that raises no exception.
    remainders = build_facts.FLOOR_DIVIDING_INFINITY_WARNS
    assert warned(fig.canvas.draw) == {
        "invalid value encountered in cos": 9, "invalid value encountered in sin": 9,
        **({REMAINDER: 8} if remainders else {})}
    assert warned(ax.yaxis.get_ticklabels) == ({REMAINDER: 4} if remainders else {})


def test_a_radial_origin_that_is_no_number():
    fig, ax = polar()
    ax.set_rorigin(NAN)
    assert warned(fig.canvas.draw) == {NO_CLIP: 13, SUBTRACT: 2}
    ax.set_rorigin(INF)
    assert warned(fig.canvas.draw) == {
        NO_CLIP: 13, "invalid value encountered in multiply": 15, DIVIDE: 1, SUBTRACT: 3}


def test_longitude_grid_ends_that_are_no_number():
    ax = plt.figure().add_subplot(projection="aitoff")
    assert warned(lambda: ax.set_longitude_grid_ends(INF)) == {MULTIPLY: 2}
    assert warned(lambda: ax.set_longitude_grid_ends(NAN)) == {}


# -- nothing for finite input -------------------------------------------------------------

def test_finite_figures_warn_of_nothing():
    fig, ax = plotted()
    ax.bar([1, 2], [2, 3])
    ax.axvspan(0.2, 0.4)
    ax.add_patch(mpatches.Ellipse((0, 0), 1, 2))

    def use():
        fig.canvas.draw()
        ax.relim()
        fig.savefig(io.BytesIO(), format="pdf")
        ax.transData.transform(np.zeros((3, 2)))

    assert warned(use) == {}
