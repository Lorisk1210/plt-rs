"""
What matplotlib's tests found once the suite ran with the GUI toolkits,
a display and pandas: a canvas's new device pixel ratio keeps its window's
size, a `.ConnectionPatch` converts the units of its data points, and the
canvases matplotlib draws with Cairo draw in Cairo's graphics state. The
expected values are matplotlib 3.11's.
"""

import datetime
import io

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import collections as mcollections, patches as mpatches
from pltrs import path as mpath
from pltrs._native import backends as _backends
from pltrs.backends._cairo_canvas import _CairoCanvas
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class _Manager:
    """A figure manager recording the sizes its window is given."""

    def __init__(self):
        self.sizes = []

    def resize(self, width, height):
        self.sizes.append((int(width), int(height)))


def test_device_pixel_ratio_keeps_the_window_size():
    """matplotlib's `_set_device_pixel_ratio` sets the resolution with
    ``forward=False``; `set_dpi` and ``dpi =`` resize the window."""
    fig = Figure(figsize=(6.4, 4.8), dpi=100)
    canvas = FigureCanvasAgg(fig)
    canvas.manager = _Manager()
    assert canvas._set_device_pixel_ratio(2)
    assert fig.dpi == 200
    assert canvas.get_width_height() == (640, 480)
    assert canvas.get_width_height(physical=True) == (1280, 960)
    assert not canvas._set_device_pixel_ratio(2)
    assert canvas.manager.sizes == []
    fig.set_dpi(150)
    fig.dpi = 120
    assert canvas.manager.sizes == [(960, 720), (768, 576)]
    fig._set_dpi(60, forward=False)
    assert fig.dpi == 60
    assert canvas.manager.sizes == [(960, 720), (768, 576)]
    assert fig.get_size_inches().tolist() == [6.4, 4.8]


def test_connection_patch_converts_units():
    """Dates and categories at a connection's ends, in its Axes' units."""
    fig, (ax1, ax2) = plt.subplots(nrows=2, figsize=(4, 4), dpi=50)
    when = datetime.datetime(2017, 1, 1, 12)
    ax1.axvline(when)
    ax2.axhline("b")
    ax2.axhline("test")
    arrow = mpatches.ConnectionPatch((when, 0), (0, "test"), coordsA="data",
                                     coordsB="data", axesA=ax1, axesB=ax2)
    fig.add_artist(arrow)
    fig.canvas.draw()
    assert arrow.get_path().vertices.round(6).tolist() == [
        [0.5125, 0.53], [0.31875, 0.487045], [0.125, 0.444091]]
    assert arrow.xy1 == (when, 0)
    assert arrow.xy2 == (0, "test")


class _CairoCanvasAgg(_CairoCanvas, FigureCanvasAgg):
    """The Agg canvas drawing as matplotlib's Cairo renderer does."""


def _cut_stars(canvas_class, joinstyle, collection):
    """The pixels of two cut-out stars with dashed edges, as one collection
    of *joinstyle* or as two patches, drawn by a *canvas_class* (from
    matplotlib's ``test_patch_alpha_coloring``)."""
    star = mpath.Path.unit_regular_star(6)
    circle = mpath.Path.unit_circle()
    verts = np.concatenate([circle.vertices, star.vertices[::-1]])
    codes = np.concatenate([circle.codes, star.codes])
    paths = [mpath.Path(verts, codes), mpath.Path(verts + 1, codes)]
    fig = Figure()
    canvas_class(fig)
    ax = fig.subplots()
    ax.set(xlim=(-1, 2), ylim=(-1, 2))
    style = dict(linewidth=5, facecolor=(1, 0, 0, 0.5),
                 edgecolor=(0, 0, 1, 0.75))
    if collection:
        ax.add_collection(mcollections.PathCollection(
            paths, linestyles="dashdot", joinstyle=joinstyle, **style))
    else:
        for path in paths:
            ax.add_patch(mpatches.PathPatch(path, linestyle="dashdot", **style))
    buffer = io.BytesIO()
    fig.savefig(buffer, format="rgba")
    return np.frombuffer(buffer.getvalue(), np.uint8).reshape(-1, 4)


@pytest.mark.parametrize("canvas_class, joinstyle, differing", [
    # matplotlib: Agg joins a collection without a join with rounds, a
    # patch with miters; Cairo's context joins both with miters.
    (FigureCanvasAgg, None, 355),
    (FigureCanvasAgg, "miter", 0),
    (_CairoCanvasAgg, None, 0),
    (_CairoCanvasAgg, "miter", 0),
])
def test_cairo_canvases_join_as_cairo(canvas_class, joinstyle, differing):
    patches = _cut_stars(canvas_class, joinstyle, collection=False)
    collection = _cut_stars(canvas_class, joinstyle, collection=True)
    assert (patches != collection).any(axis=-1).sum() == differing


def test_cairo_drawing_raises_what_its_call_raises():
    def failing():
        raise ValueError("drawn badly")

    with pytest.raises(ValueError, match="drawn badly"):
        _backends.drawing_like_cairo(failing)
    calls = []
    _backends.drawing_like_cairo(lambda: calls.append(True))
    assert calls == [True]
