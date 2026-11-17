"""
What task TAIL7 found pltrs doing otherwise than matplotlib 3.11 in the
rows of its table: the renderer a text keeps from a figure's draw, the
label's measure of a spine of a frameless Axes, and the setters a contour
set's constructor calls. Each expected value is recorded from matplotlib.
"""

import collections
import io
import logging

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager
from pltrs.backends.backend_svg import FigureCanvasSVG
from pltrs.contour import QuadContourSet
from pltrs.spines import Spine


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Logged(logging.Handler):
    def __init__(self):
        super().__init__()
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage()[:40])


# -- the renderer a text keeps from a draw --------------------------------------------

def _lookups_of_two_measures(fig, text):
    logged = _Logged()
    font_manager._log.addHandler(logged)
    try:
        text.get_window_extent()
        text.get_window_extent()
    finally:
        font_manager._log.removeHandler(logged)
    return len(logged.messages)


@pytest.mark.parametrize("draw, expected", [
    (lambda fig: fig.draw_without_rendering(), 2),
    (lambda fig: fig.savefig(io.BytesIO(), format="pdf"), 3),
])
def test_a_text_measured_after_a_vector_draw_keeps_that_draws_renderer(draw, expected):
    fig, ax = plt.subplots()
    FigureCanvasSVG(fig)
    title = ax.set_title("title")
    title.set_fontfamily("a family no font has")
    draw(fig)
    assert _lookups_of_two_measures(fig, title) == expected


def test_a_text_keeps_its_renderer_when_the_canvas_changes():
    fig, ax = plt.subplots()
    title = ax.set_title("title")
    title.set_fontfamily("a family no font has")
    fig.canvas.draw()
    FigureCanvasSVG(fig)
    assert _lookups_of_two_measures(fig, title) == 2


# -- a spine of the program's in a frameless Axes --------------------------------------

class _CountedSpine(Spine):
    calls = collections.Counter()

    def _adjust_location(self):
        self.calls["_adjust_location"] += 1
        return super()._adjust_location()


def _frameless(spine_class):
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1], [0, 1])
    for name, axis in (("bottom", ax.xaxis), ("left", ax.yaxis)):
        spine = spine_class.linear_spine(ax, name)
        ax.spines[name] = spine
        spine.register_axis(axis)
    ax.set_frame_on(False)
    return fig


@pytest.mark.parametrize("work, expected", [
    (lambda fig: fig.canvas.draw(), 2),
    (lambda fig: fig.savefig(io.BytesIO(), format="png", bbox_inches="tight"), 6),
])
def test_the_axis_label_measures_the_spines_of_a_frameless_axes(work, expected):
    fig = _frameless(_CountedSpine)
    _CountedSpine.calls.clear()
    work(fig)
    assert _CountedSpine.calls["_adjust_location"] == expected


# -- the setters of a contour set's constructor ----------------------------------------

def _recording(base):
    calls = []

    def recorder(name):
        def method(self, *args, **kwargs):
            calls.append((name, repr(args)[:40]))
            return getattr(super(type(self), self), name)(*args, **kwargs)
        return method
    names = ["set_alpha", "set_cmap", "set_zorder", "set_linewidth", "set_label",
             "set_clip_path", "set_figure", "set_norm"]
    return type("Recorded", (base,), {name: recorder(name) for name in names}), calls


def test_a_contour_set_of_the_programs_sees_its_constructors_setters():
    x, y = np.meshgrid(np.linspace(0, 1, 6), np.linspace(0, 1, 5))
    cls, calls = _recording(QuadContourSet)
    fig, ax = plt.subplots()
    cls(ax, x, y, np.sin(3 * x) + y, colors=["r", "g"])
    # The first call of each setter with its arguments, in order (the
    # norm's callbacks call some again).
    assert [name for name, _ in dict.fromkeys(calls)] == [
        "set_linewidth", "set_zorder", "set_alpha", "set_cmap", "set_norm",
        "set_linewidth", "set_label", "set_zorder", "set_figure", "set_clip_path"]
