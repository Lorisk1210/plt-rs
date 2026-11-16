"""
An affine transform of the program's computing its own matrix, asked as
matplotlib's draws and measures ask it: by each kind of artist, in the
draws of a tight print, and where a text is measured. The expected counts
are matplotlib 3.11.2's.
"""

import collections
import io

import numpy as np
import pytest

from pltrs import collections as mcollections
from pltrs import patches, transforms
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

#: The calls of each counted method of `Scale`.
CALLS = collections.Counter()


def _counted(name):
    """A method calling the base class' *name*, counted in `CALLS`."""
    def method(self, *args, **kwargs):
        CALLS[name] += 1
        return getattr(super(type(self), self), name)(*args, **kwargs)
    method.__name__ = name
    return method


class _Scale(transforms.Affine2DBase):
    """An affine transform computing its matrix from *factor* when asked."""

    def __init__(self, factor=0.9):
        super().__init__()
        self.factor = factor

    def get_matrix(self):
        return np.array([[self.factor, 0.0, 0.0], [0.0, self.factor, 0.0], [0.0, 0.0, 1.0]])


Scale = type("Scale", (_Scale,), {name: _counted(name)
                                  for name in ("get_affine", "get_matrix", "frozen")})


def _axes(**kwargs):
    fig = Figure(figsize=(3, 2), **kwargs)
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    ax.set(xlim=(0, 1), ylim=(0, 1))
    return fig, ax


def _taken():
    counts = dict(CALLS)
    CALLS.clear()
    return counts


def _add(kind, ax, transform):
    """The artist of *kind* in *ax*, given *transform*."""
    if kind == "line":
        return ax.plot([0.1, 0.5, 0.9], [0.2, 0.8, 0.4], transform=transform)[0]
    if kind == "patch":
        return ax.add_patch(patches.Rectangle((0.2, 0.2), 0.4, 0.3, transform=transform))
    if kind == "collection":
        return ax.add_collection(mcollections.LineCollection(
            [[(0.1, 0.1), (0.9, 0.5)], [(0.2, 0.7), (0.6, 0.3)]], transform=transform))
    if kind == "scatter":
        return ax.scatter([0.2, 0.5, 0.8], [0.3, 0.6, 0.4], transform=transform)
    if kind == "image":
        return ax.imshow(np.arange(12.0).reshape(3, 4), extent=(0.1, 0.9, 0.1, 0.9),
                         transform=transform)
    return ax.text(0.3, 0.5, "probe", transform=transform)


@pytest.mark.parametrize("kind, asked", [
    ("line", {"get_affine": 1, "get_matrix": 1}),
    ("patch", {"get_affine": 1, "get_matrix": 1}),
    ("collection", {"frozen": 2, "get_matrix": 2}),
    ("scatter", {"get_affine": 1, "get_matrix": 1}),
    ("image", {"get_affine": 6, "get_matrix": 6}),
    ("text", {"get_affine": 1, "get_matrix": 1}),
])
def test_each_draw_asks_as_matplotlibs_draw_of_the_artist(kind, asked):
    fig, ax = _axes()
    _add(kind, ax, Scale() + ax.transData)
    fig.canvas.draw()
    _taken()
    fig.canvas.draw()
    assert _taken() == asked


@pytest.mark.parametrize("kind, asked", [
    ("line", {"get_affine": 2, "get_matrix": 2}),
    ("collection", {"frozen": 4, "get_matrix": 4}),
    # The draw laying the print out renders nothing: no renderer reads the
    # offsets' matrix there.
    ("scatter", {"get_affine": 1, "get_matrix": 1}),
    ("image", {"get_affine": 12, "get_matrix": 12}),
    # Both draws, and the text measured for the tight box.
    ("text", {"get_affine": 3, "get_matrix": 4}),
])
def test_a_tight_print_asks_in_both_its_draws(kind, asked):
    fig, ax = _axes()
    _add(kind, ax, Scale() + ax.transData)
    fig.canvas.draw()
    _taken()
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert _taken() == asked


def test_a_text_measured_asks_its_transform_as_matplotlibs_measures():
    fig, ax = _axes()
    text = _add("text", ax, Scale() + ax.transData)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    _taken()
    text.get_window_extent(renderer)
    assert _taken() == {"get_affine": 1, "get_matrix": 1}
    # The tight box compares the transform with the data transform first.
    text.get_tightbbox(renderer)
    assert _taken() == {"get_affine": 1, "get_matrix": 2}
    fig, ax = _axes(layout="constrained")
    _add("text", ax, Scale() + ax.transData)
    _taken()
    fig.canvas.draw()
    assert _taken() == {"get_affine": 3, "get_matrix": 5}


def test_a_scatter_keeps_the_offset_transform_it_was_given():
    fig, ax = _axes()
    given = _Scale(1.0) + ax.transData
    scatter = _add("scatter", ax, given)
    assert scatter.get_offset_transform() is given
    fig.canvas.draw()
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    # Drawn where the transform computes the offsets now, as matplotlib's.
    given._a.factor = 0.5
    scatter.stale = True
    fig.canvas.draw()
    assert not np.array_equal(before, np.asarray(fig.canvas.buffer_rgba()))
