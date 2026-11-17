"""
An Axes made for a figure but never added to it, drawn by another
artist's draw (ultraplot's ternary container draws mpltern's
`TernaryAxes` so), by `Figure.draw_artist` or in a tight save: matplotlib's
`Axes.draw` applies its aspect where its locator places it, in the figure
the Axes was made for, as it does for any Axes. Each kind of Axes is
tried: a plain one, a polar one, one of a projection of the program's
(a data ratio of its own, as mpltern's) and a 3-D one. The draw ends when
the program's call returns: what the program does after it (the Axes'
`get_position`, which applies the aspect anew) leaves the Axes stale, and
ultraplot's container draws the Axes again on its next draw only then.
The expected values were recorded from matplotlib 3.11.2 running the same
code.
"""

import io

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs.artist import Artist
from pltrs.axes import Axes
from pltrs.projections.polar import PolarAxes
from pltrs.mpl_toolkits.mplot3d import Axes3D
from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class _Triangle(Axes):
    """A projection of the program's whose box keeps an equilateral
    triangle's proportions (mpltern's `TernaryAxes` by its data ratio)."""

    name = "eco11-triangle"

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.set_aspect("equal", adjustable="box")

    def get_data_ratio(self):
        return np.sqrt(3) / 2


def _plain(fig):
    ax = Axes(fig, [0.1, 0.1, 0.8, 0.8])
    ax.set_aspect("equal")
    ax.set_xlim(0, 2)
    return ax


def _polar(fig):
    return PolarAxes(fig, [0.1, 0.1, 0.8, 0.5])


def _triangle(fig):
    return _Triangle(fig, [0.1, 0.1, 0.8, 0.8])


def _three_d(fig):
    ax = Axes3D(fig, [0.1, 0.1, 0.8, 0.5])
    ax.set_box_aspect((1, 1, 1))
    ax.xaxis.set_pane_color("red")
    return ax


#: Each kind's position once drawn, and the red pixels its draw leaves.
#: matplotlib's `Axes3D.draw` paints its background before it squares
#: the box, so its first draw of an Axes the figure does not list paints
#: the unsquared box: the 3-D counts are those of its draws after that
#: (and of its tight save, whose measuring draw comes first).
_EXPECTED = {
    "plain": ((0.1, 0.3, 0.8, 0.4), 7021),
    "polar": ((0.25, 0.1, 0.5, 0.5), 3435),
    "triangle": ((0.1, 0.1536, 0.8, 0.6928), 12257),
    "three_d": ((0.25, 0.1, 0.5, 0.5), 2858),
}
_KINDS = {"plain": _plain, "polar": _polar, "triangle": _triangle, "three_d": _three_d}


class _Drawer(Artist):
    """Draws the Axes it holds from its own draw (ultraplot's container)."""

    def __init__(self, inner):
        super().__init__()
        self.inner = inner

    def draw(self, renderer):
        self.inner.draw(renderer)


def _red(pixels):
    pixels = np.asarray(pixels)
    return int(((pixels[..., 0] > 200) & (pixels[..., 1] < 50) & (pixels[..., 2] < 50)).sum())


def _made(kind):
    fig = plt.figure(figsize=(3, 3), dpi=50)
    inner = _KINDS[kind](fig)
    inner.set_facecolor("red")
    return fig, inner


def _position(ax):
    return tuple(round(float(v), 4) for v in ax.get_position().bounds)


@pytest.mark.parametrize("kind", sorted(_KINDS))
def test_an_axes_drawn_by_another_artists_draw_applies_its_aspect(kind):
    fig, inner = _made(kind)
    assert inner not in fig.axes
    fig.add_artist(_Drawer(inner))
    fig.canvas.draw()
    if kind == "three_d":
        fig.canvas.draw()
    position, red = _EXPECTED[kind]
    assert _position(inner) == position
    assert _red(fig.canvas.buffer_rgba()) == red


@pytest.mark.parametrize("kind", sorted(_KINDS))
def test_an_axes_drawn_by_draw_artist_applies_its_aspect(kind):
    fig, inner = _made(kind)
    fig.canvas.draw()
    fig.draw_artist(inner)
    if kind == "three_d":
        # Over a canvas drawn anew (the first draw's background stays on
        # the canvas, which `draw_artist` does not clear).
        fig.canvas.draw()
        fig.draw_artist(inner)
    position, red = _EXPECTED[kind]
    assert _position(inner) == position
    assert _red(fig.canvas.buffer_rgba()) == red


#: The red pixels of the tight save (a pixel row more or less of the
#: polar disc than on the canvas: the save's crop).
_TIGHT_RED = {"plain": 7021, "polar": 3505, "triangle": 12257, "three_d": 2866}


@pytest.mark.parametrize("kind", sorted(_KINDS))
def test_an_axes_drawn_in_a_tight_save_applies_its_aspect(kind):
    from PIL import Image
    fig, inner = _made(kind)
    fig.add_artist(_Drawer(inner))
    # Texts at the corners hold the tight box at the figure's edges.
    fig.text(0, 0, ".")
    fig.text(0.97, 0.95, ".")
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png", bbox_inches="tight")
    buffer.seek(0)
    assert _position(inner) == _EXPECTED[kind][0]
    assert _red(Image.open(buffer).convert("RGB")) == _TIGHT_RED[kind]


def test_apply_aspect_places_an_axes_the_figure_does_not_list():
    fig = plt.figure(figsize=(3, 3), dpi=50)
    inner = _plain(fig)
    inner.apply_aspect()
    assert _position(inner) == (0.1, 0.3, 0.8, 0.4)


@pytest.mark.parametrize("kind, expected", [
    (_plain, (3081, (111, 189, 31, 69), (0.1, 0.3, 0.8, 0.4))),
    (_polar, (4052, (111, 188, 11, 88), (0.1, 0.1, 0.8, 0.8))),
])
def test_an_axes_made_for_a_subfigure_is_placed_in_it(kind, expected):
    fig = plt.figure(figsize=(4, 2), dpi=50)
    _, right = fig.subfigures(1, 2)
    inner = kind(right)
    if kind is _polar:
        inner.set_position([0.1, 0.1, 0.8, 0.8])
    inner.set_facecolor("red")
    right.add_artist(_Drawer(inner))
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())
    red = (pixels[..., 0] > 200) & (pixels[..., 1] < 50) & (pixels[..., 2] < 50)
    rows, columns = np.nonzero(red)
    count, extent, position = expected
    assert int(red.sum()) == count
    assert (columns.min(), columns.max(), rows.min(), rows.max()) == extent
    assert _position(inner) == position


def test_what_the_program_does_after_an_axes_draw_leaves_it_stale():
    fig = plt.figure(figsize=(3, 3), dpi=50)
    inner = _plain(fig)
    seen = []

    class Drawer(Artist):
        def draw(self, renderer):
            inner.draw(renderer)
            seen.append(inner.stale)
            inner.get_position()
            seen.append(inner.stale)

    fig.add_artist(Drawer())
    fig.canvas.draw()
    assert seen == [False, True]
    assert inner.stale


class _Container(Artist):
    """Draws the Axes it holds only while it is stale (ultraplot's
    external-axes container), reading its position after."""

    def __init__(self, inner):
        super().__init__()
        self.inner = inner

    def draw(self, renderer):
        if self.inner.stale:
            self.inner.draw(renderer)
            self.inner.get_position()


def test_an_axes_a_container_draws_while_stale_is_drawn_again_in_a_save():
    from PIL import Image
    fig, inner = _made("plain")
    fig.add_artist(_Container(inner))
    fig.canvas.draw()
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png")
    buffer.seek(0)
    assert _red(Image.open(buffer).convert("RGB")) == 7021
