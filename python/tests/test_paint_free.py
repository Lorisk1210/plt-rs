"""
The program's code a draw runs while it paints (a path effect, a renderer
of the program's, a replaced ``Text.draw``) finds the figure free: it
changes it at once, reads its changes back, may make changes returning
what they changed, and the rest of the draw sees them. The expected values
are real matplotlib 3.11's for the same programs.
"""

import numpy as np
import pytest

import pltrs
import pltrs.patheffects as pe
import pltrs.pyplot as plt
from pltrs import text as mtext
from pltrs.backend_bases import RendererBase


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _two_lines():
    """A figure of two lines, the second drawn after the first."""
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    line, = ax.plot([0, 1], [1, 0], zorder=1)
    other, = ax.plot([0, 1], [0, 1], zorder=2)
    return fig, ax, line, other


def test_a_path_effect_changes_the_figure_at_once():
    class Changing(pe.AbstractPathEffect):
        def __init__(self, line, other, ax, seen):
            super().__init__()
            self.line, self.other, self.ax, self.seen = line, other, ax, seen

        def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
            self.line.set_color("red")
            self.seen.append(self.line.get_color())
            title = self.ax.set_title("made while drawing")
            self.seen.append((type(title).__name__, title.get_text(),
                              self.ax.get_title()))
            self.other.set_linewidth(4)
            self.seen.append(self.other.get_linewidth())
            self.seen.append(tuple(map(float, self.ax.get_xlim())))
            renderer.draw_path(gc, tpath, affine, rgbFace)

    fig, ax, line, other = _two_lines()
    seen = []
    line.set_path_effects([Changing(line, other, ax, seen)])
    fig.canvas.draw()
    assert seen == [
        "red", ("Text", "made while drawing", "made while drawing"), 4.0,
        (-0.05, 1.05)]
    assert (fig.stale, line.stale, other.stale, ax.stale) == (
        False, False, False, False)
    drawn = np.asarray(fig.canvas.buffer_rgba()).copy()
    # The line was drawn as it was when its path effect ran; the other
    # line and the title, drawn after, with the changes.
    made, made_ax, _, made_other = _two_lines()
    made_other.set_linewidth(4)
    made_ax.set_title("made while drawing")
    made.canvas.draw()
    assert np.array_equal(drawn, np.asarray(made.canvas.buffer_rgba()))


def test_a_renderer_of_the_program_changes_what_it_draws():
    class Recording(RendererBase):
        def __init__(self, line, seen):
            super().__init__()
            self.line, self.seen = line, seen

        def draw_path(self, gc, path, transform, rgbFace=None):
            self.line.set_linewidth(5)
            self.seen.append(self.line.get_linewidth())
            self.seen.append(self.line.axes.set_xlabel("drawn").get_text())

    fig, ax = plt.subplots()
    line, = ax.plot([0, 1], [0, 1])
    fig.canvas.draw()
    seen = []
    line.draw(Recording(line, seen))
    assert seen == [5.0, "drawn"]
    assert (line.get_linewidth(), ax.get_xlabel(), line.stale) == (
        5.0, "drawn", False)


def _two_texts(later_text):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.text(0.2, 0.5, "first", zorder=3)
    later = ax.text(0.6, 0.5, later_text, zorder=4)
    return fig, ax, later


def test_a_replaced_text_draw_changes_a_text_drawn_later(monkeypatch):
    fig, ax, later = _two_texts("later")
    seen = []
    original = mtext.Text.draw

    def draw(self, renderer):
        if self.get_text() == "first":
            later.set_text("changed")
            seen.append(later.get_text())
            seen.append(type(ax.set_ylabel("y")).__name__)
        return original(self, renderer)

    monkeypatch.setattr(mtext.Text, "draw", draw)
    fig.canvas.draw()
    monkeypatch.undo()
    assert seen == ["changed", "Text"]
    assert (later.stale, ax.get_ylabel()) == (False, "y")
    drawn = np.asarray(fig.canvas.buffer_rgba()).copy()
    # The y axis, with its label, was drawn before the texts.
    made, _, _ = _two_texts("changed")
    made.canvas.draw()
    assert np.array_equal(drawn, np.asarray(made.canvas.buffer_rgba()))


def test_a_replaced_text_draw_calling_the_original_draws_the_text(monkeypatch):
    fig, _, _ = _two_texts("later")
    fig.canvas.draw()
    plain = np.asarray(fig.canvas.buffer_rgba()).copy()
    original = mtext.Text.draw
    calls = []

    def draw(self, renderer):
        calls.append(self.get_text())
        return original(self, renderer)

    monkeypatch.setattr(mtext.Text, "draw", draw)
    fig.canvas.draw()
    monkeypatch.undo()
    assert {"first", "later"} <= set(calls)
    assert np.array_equal(plain, np.asarray(fig.canvas.buffer_rgba()))
