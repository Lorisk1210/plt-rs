"""matplotlib's own non-image tests of spines
(``matplotlib/tests/test_spines.py``).

Area ``mpl_spines``. Every op runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``) on ``plt.subplots()``.
Boxes are ``[[x0, y0], [x1, y1]]`` in display pixels.

Ops
---
``Spine.set_bounds``  {} -> ``{bottom, left, xlim, ylim}`` for
    ``test_spine_set_bounds_with_none``: after plotting a sine over
    ``linspace(0, 10, 100)``, ``get_bounds()`` of the bottom spine given
    ``set_bounds(2, None)`` and of the left one given ``set_bounds(None,
    None)``, and the view limits the missing ends come from.
``Axis.label``  {} -> ``{ylabel, left, xlabel, bottom}`` for
    ``test_label_without_ticks``: on an axes moved in by
    ``subplots_adjust(left=0.3, bottom=0.3)``, with no ticks, its left and
    bottom spines 30 points outward and the others hidden, the window
    extents of the axis labels and the extents of the spine lines
    (``spine.get_transform().transform_path(spine.get_path())``): each
    label lies beyond its spine.

The image tests are cases of ``mpl_small``. Left out: ``test_spine_class``
(the ``Spines`` container's Python indexing, slicing and proxying; plt-rs
spines are the fields of a struct) and ``test_arc_spine_inner_no_axis``
(drawing a polar inner spine detached from its axis with
``register_axis(None)``, an object link plt-rs spines do not have).
"""

import matplotlib.pyplot as plt
import numpy as np

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_spines")
probe = prober(probes)


def _set_bounds():
    fig, ax = plt.subplots()
    x = np.linspace(0, 10, 100)
    ax.plot(x, np.sin(x))
    xlim = list(ax.viewLim.intervalx)
    ylim = list(ax.viewLim.intervaly)
    ax.spines["bottom"].set_bounds(2, None)
    ax.spines["left"].set_bounds(None, None)
    return {"bottom": list(ax.spines["bottom"].get_bounds()),
            "left": list(ax.spines["left"].get_bounds()), "xlim": xlim, "ylim": ylim}


def test_spine_set_bounds_with_none():
    res = probe("Spine.set_bounds", _set_bounds)
    assert res["bottom"] == [2, res["xlim"][1]] and res["left"] == res["ylim"]


def _line_extent(spine):
    return spine.get_transform().transform_path(spine.get_path()).get_extents().get_points()


def _label_without_ticks():
    fig, ax = plt.subplots()
    fig.subplots_adjust(left=0.3, bottom=0.3)
    ax.plot(np.arange(10))
    ax.yaxis.set_ticks_position("left")
    ax.spines.left.set_position(("outward", 30))
    ax.spines.right.set_visible(False)
    ax.set_ylabel("y label")
    ax.xaxis.set_ticks_position("bottom")
    ax.spines.bottom.set_position(("outward", 30))
    ax.spines.top.set_visible(False)
    ax.set_xlabel("x label")
    ax.xaxis.set_ticks([])
    ax.yaxis.set_ticks([])
    fig.canvas.draw()
    return {"ylabel": ax.yaxis.label.get_window_extent().get_points(),
            "left": _line_extent(ax.spines.left),
            "xlabel": ax.xaxis.label.get_window_extent().get_points(),
            "bottom": _line_extent(ax.spines.bottom)}


def test_label_without_ticks():
    res = probe("Axis.label", _label_without_ticks)
    assert res["ylabel"][1][0] < res["left"][0][0]
    assert res["xlabel"][1][1] < res["bottom"][0][1]
