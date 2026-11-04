"""
Robustness: what the fuzzer (``python/tests/fuzz``) and the interactive
tests found, each with the values real matplotlib 3.11 gives.

* Code pltrs runs while it lays a figure out or draws it (a formatter, a
  path effect, a norm) may change the figure: the changes are made once the
  draw is done, and nothing reaches Python as a Rust panic.
* ``savefig`` sends its ``draw_event`` once per save, before the file is
  written, as matplotlib's ``Figure.draw`` does inside ``print_<format>``.
* The X display checks are steady in a fresh process.
* Pickles: figures with a layout engine, artists' sticky edges.
* Polar arcs at astronomical angles.
"""

import io
import os
import pickle
import subprocess
import sys

import numpy as np
import pytest

import pltrs
from pltrs import colors, patheffects, ticker, transforms
import pltrs.pyplot as plt

pltrs.use("agg")


@pytest.fixture(autouse=True)
def _close_figures():
    yield
    plt.close("all")


# Changes made while the figure is drawn.

def test_a_formatter_changing_its_figure_has_the_change_made():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1, 2], [3, 1, 2])
    ax.xaxis.set_major_formatter(
        ticker.FuncFormatter(lambda v, p: (line.set_color("red"), str(v))[1]))
    fig.canvas.draw()
    assert line.get_color() == "red"


def test_a_path_effect_reads_and_changes_its_figure():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1, 2], [3, 1, 2])
    text = ax.text(0.5, 0.5, "x")
    seen = []

    class Changing(patheffects.AbstractPathEffect):
        def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
            text.set_text("T")
            seen.append(ax.get_xlim())
            renderer.draw_path(gc, tpath, affine, rgbFace)

    line.set_path_effects([Changing()])
    fig.canvas.draw()
    assert text.get_text() == "T"
    assert seen[0] == ax.get_xlim() == pytest.approx((-0.1, 2.1))


def test_a_norm_changing_its_figure_has_the_change_made():
    fig, ax = plt.subplots()

    class Hiding(colors.Normalize):
        def __call__(self, value, clip=None):
            ax.patch.set_visible(False)
            return super().__call__(value, clip)

    ax.imshow([[0, 1], [2, 3]], norm=Hiding())
    fig.canvas.draw()
    assert not ax.patch.get_visible()


def test_a_transform_rescaling_its_axes_ends_as_in_matplotlib():
    # Found by fuzzing: each `relim` ran the transform, which queued
    # `relim` again, without end.
    fig, ax = plt.subplots()
    calls = []

    class Rescaling(transforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            calls.append(1)
            ax.relim()
            return values

    ax.plot([1, 2], [1, 2], transform=Rescaling() + ax.transData)
    fig.canvas.draw()
    assert calls


def test_what_cannot_run_while_the_figure_is_laid_out_raises_no_panic():
    # matplotlib sets the title. pltrs cannot read a figure it is laying
    # out, which setting the title does first: it raises RuntimeError,
    # never a Rust panic.
    fig, ax = plt.subplots()
    ax.plot([0, 1])
    ax.xaxis.set_major_formatter(
        ticker.FuncFormatter(lambda v, p: (ax.set_title("t"), str(v))[1]))
    with pytest.raises(RuntimeError, match="being laid out") as raised:
        fig.canvas.draw()
    assert "panic" not in str(raised.value)


def test_redrawing_from_a_draw_event_recurses_as_in_matplotlib():
    fig, ax = plt.subplots()
    fig.canvas.mpl_connect("draw_event", lambda event: fig.canvas.draw())
    with pytest.raises(RecursionError):
        fig.canvas.draw()


def test_limit_callbacks_changing_the_limits_again_end():
    # Found by fuzzing: a scroll event from a limit callback laid the
    # figure out again (to pick), which an absurd aspect moved the limits
    # on, without end. matplotlib draws the image.
    from pltrs.backend_bases import MouseEvent
    fig, ax = plt.subplots()
    ax.callbacks.connect("xlim_changed", lambda axes: MouseEvent(
        "scroll_event", fig.canvas, 200, 0, "up", step=0)._process())
    ax.set_xscale("log")
    ax.twiny()
    with plt.rc_context({"image.aspect": 1e300}):
        ax.imshow([[0, 1], [2, 3]])


def test_titles_of_cleared_axes_report_malformed_mathtext():
    # Found by fuzzing: a title reset by `cla` panicked when its draw
    # failed. matplotlib raises the mathtext parser's ValueError.
    fig, ax = plt.subplots()
    ax.cla()
    ax.set_title("$$")
    with pytest.raises(ValueError, match="Expected end of text"):
        fig.canvas.draw()


def test_collections_without_offsets_pick():
    # Found by fuzzing: picking a collection whose offsets were emptied
    # divided by zero. matplotlib picks nothing here.
    from pltrs.backend_bases import MouseEvent
    fig, ax = plt.subplots()
    events = ax.eventplot([1, 2, 3])[0]
    events.set_picker(True)
    events.set_offset_transform(None)
    picked = []
    fig.canvas.mpl_connect("pick_event", lambda event: picked.append(event.ind))
    fig.canvas.draw()
    MouseEvent("button_press_event", fig.canvas, 100, 100, 1)._process()
    assert picked == []


def test_axes_refuse_to_make_absurd_numbers_of_ticks():
    # Found by fuzzing: matplotlib makes ticks until memory runs out.
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="ticks"):
        ax.xaxis.get_minor_ticks(numticks=10**15)
    assert len(ax.xaxis.get_major_ticks(numticks=3)) == 3


# The draw event of a save.

FORMATS = ["png", "jpg", "tiff", "webp", "gif", "raw", "rgba", "svg", "svgz", "pdf", "ps",
           "eps", "pgf"]
#: The formats matplotlib writes only once the figure is drawn (it streams
#: SVG and PDF while it draws).
WRITTEN_AFTER = {"png", "jpg", "tiff", "webp", "gif", "raw", "rgba", "ps", "eps", "pgf"}


@pytest.mark.parametrize("fmt", FORMATS)
def test_saving_sends_one_draw_event_before_writing(fmt):
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    ax.plot([0, 1], [1, 0])
    buffer = io.BytesIO()
    written = []
    fig.canvas.mpl_connect("draw_event", lambda event: written.append(buffer.tell()))
    fig.savefig(buffer, format=fmt)
    assert len(written) == 1
    if fmt in WRITTEN_AFTER:
        assert written == [0]


# The X display checks.

@pytest.mark.skipif(not os.environ.get("DISPLAY"), reason="no X display")
def test_the_display_checks_agree_in_fresh_processes():
    check = ("from pltrs import _c_internal_utils as u; "
             "print(u.xdisplay_is_valid(), u.display_is_valid(), u.xdisplay_is_valid())")
    for _ in range(20):
        out = subprocess.run([sys.executable, "-c", check], capture_output=True, text=True,
                             check=True).stdout.split()
        assert out == ["True"] * 3


# Pickles.

def test_figures_with_a_layout_engine_pickle():
    fig, ax = plt.subplots(layout="constrained")
    ax.plot([1, 2])
    restored = pickle.loads(pickle.dumps(fig))
    engine = restored.get_layout_engine()
    assert type(engine).__name__ == "ConstrainedLayoutEngine"
    engine.set(w_pad=0.5)
    restored.savefig(io.BytesIO(), format="png")


def test_sticky_edges_pickle_with_their_values():
    fig, ax = plt.subplots()
    image = ax.imshow([[1, 2], [3, 4]])
    image.sticky_edges.x[:] = [0, 5]
    restored = pickle.loads(pickle.dumps(fig)).axes[0].images[0]
    assert restored.sticky_edges.x == [0, 5]
    assert restored.sticky_edges.y == [1.5, -0.5]
    restored.sticky_edges.x.append(9)
    assert restored.sticky_edges.x == [0, 5, 9]


# Polar arcs.

def test_polar_axes_at_astronomical_angles_save():
    # matplotlib retraces the circle once per turn here, without end.
    fig = plt.figure()
    ax = fig.add_subplot(projection="polar")
    ax.set_xlim(-1, 2**63)
    fig.savefig(io.BytesIO(), format="png")
    assert np.isfinite(ax.get_xlim()).all()
