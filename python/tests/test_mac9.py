"""
What task MAC9 found running pltrs on macOS beside matplotlib 3.11.2:
patches of the program's classes giving ``get_transform`` are drawn as
matplotlib draws them in the figure the program made (GEN10's
`test_gen10.py` draws an unpickled one): an arrow keeps its open head, and
spines an Axes class makes for every side draw (they raised TypeError);
and an image's window extent reads the extent it holds, as matplotlib's
reads its `_extent`, not the `get_extent` a class of the program's gives.
The drawings are compared with those of the base classes, which
matplotlib draws identically.
"""

import numpy as np

from pltrs import axes as maxes, patches, spines
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class _Arrow(patches.FancyArrowPatch):
    """A FancyArrowPatch whose class computes its transform."""

    def get_transform(self):
        return super().get_transform()


class _Spine(spines.Spine):
    """A spine whose class computes its transform."""

    def get_transform(self):
        return super().get_transform()


class _Host(maxes.Axes):
    """An Axes whose spines are `_Spine`."""

    def _gen_axes_spines(self, locations=None, offset=0.0, units="inches"):
        return {side: _Spine.linear_spine(self, side)
                for side in ("left", "right", "bottom", "top")}


def _pixels(build):
    """The RGBA pixels of a small figure *build* fills, drawn twice."""
    fig = Figure(figsize=(3, 2), dpi=50)
    FigureCanvasAgg(fig)
    build(fig)
    fig.canvas.draw()
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def _arrow(cls):
    def build(fig):
        ax = fig.subplots()
        ax.add_patch(cls((0.1, 0.1), (0.8, 0.8), arrowstyle="->",
                         mutation_scale=20))
    return build


def test_an_arrow_computing_its_transform_draws_its_open_head():
    assert np.array_equal(_pixels(_arrow(_Arrow)),
                          _pixels(_arrow(patches.FancyArrowPatch)))


def test_spines_computing_their_transform_draw():
    def build(axes_class):
        return lambda fig: fig.add_subplot(axes_class=axes_class).plot([0, 1], [1, 0])
    assert np.array_equal(_pixels(build(_Host)), _pixels(build(maxes.Axes)))


class _Image:
    """Builds an image whose class answers ``get_extent`` and counts it."""

    calls = 0

    @classmethod
    def make(cls, ax):
        from pltrs import image

        class Counted(image.AxesImage):
            def get_extent(self):
                cls.calls += 1
                return super().get_extent()

        art = Counted(ax)
        art.set_data(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
        art.set_extent(art.get_extent())
        ax.add_image(art)
        return art


def test_an_images_window_extent_reads_its_extent_not_the_classes_get_extent():
    fig = Figure(figsize=(3, 2))
    FigureCanvasAgg(fig)
    art = _Image.make(fig.subplots())
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    _Image.calls = 0
    art.get_window_extent(renderer)
    art.get_tightbbox(renderer)
    assert _Image.calls == 0
