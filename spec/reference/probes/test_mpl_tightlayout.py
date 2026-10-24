"""Tight layout, ported from matplotlib's own ``tests/test_tightlayout.py``:
the layouts tight layout declines, the figure title above the axes
title, and the layout engine a figure is given.

Area ``mpl_tightlayout``. Every op runs in the test style (see
``probes/_mpl_tests.py``): its ``style`` and ``rc`` arguments.

Ops
---
``Figure.tight_layout``  {scene} -> whether ``tight_layout`` declined
    (matplotlib warns and leaves the figure alone), and what it left:
    ``empty``: ``{"warned", "params"}``, the subplot parameters ``[left,
    bottom, right, top, wspace, hspace]`` of a figure without axes after
    ``tight_layout()``; ``badsubplotgrid``: ``{"warned", "positions"}``,
    the extents of ``test_badsubplotgrid``'s two axes on grids of 4 x 5
    and 5 x 5 cells after ``tight_layout()``; ``collapsed``: ``{"warned",
    "widths"}``, for ``test_collapsed``'s axes with a long annotation
    far to its right in a figure laid out by the tight engine: the axes'
    width before, after ``tight_layout()`` and after ``tight_layout(rect=
    [0, 0, 0.8, 0.8])``, and whether each call declined.
``Text.get_window_extent``  {} -> ``[suptitle_y0, title_y1]``: the bottom
    of the figure title and the top of the axes title (pixels) of a
    figure laid out by the tight engine, after a draw.
``Figure.get_tight_layout``  {calls} -> whether the figure is laid out
    by the tight engine after each ``set_tight_layout(value)`` on
    ``plt.subplots()``.

Cases
-----
Named after the upstream tests. Adaptations: the tests run with text
drawn as rectangles (``text_placeholders``); these record the extents
matplotlib gives real text; the probes record the values the tests
compare (warned or not, extents) rather than the comparisons; pyplot's
current figure is the one named.

Covered elsewhere: the image tests (figure group ``mpl_figure``); the
tests that lay out and then check positions are figure cases of the
``mpl_tests_tightlayout`` group (``test_outward_ticks``,
``test_tight_layout_offsetboxes``, whose images show where the boxes
went, ``test_verybig_decorators``, ``test_big_decorators_*``,
``test_badsubplotgrid``, ``test_collapsed``, ``test_suptitle``,
``test_manual_colorbar``, ``test_tight_pads`` and ``test_tight_kwargs``,
which make the same figure). Not ported: the
warning of ``test_manual_colorbar`` (plt-rs leaves axes off grids out
without a warning; its figure is a case), the pending deprecation
warnings (``test_tight_pads``, ``test_tight_toggle``),
``test_non_agg_renderer`` (which renderer class a backend instantiates)
and ``test_clipped_to_axes`` (the private ``_fully_clipped_to_axes``).
"""

import warnings

import matplotlib.pyplot as plt

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_tightlayout")
probe = prober(probes)

#: Positions in figure fractions or pixels, compared to a millionth.
EXTENT_TOL = 1e-6


def _extents(bbox):
    return [float(v) for v in bbox.extents]


def _declined(fn):
    """Whether ``fn()`` warned that the layout was not applied."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fn()
    return any(issubclass(w.category, UserWarning) for w in caught)


def _tight_layout(scene):
    if scene == "empty":
        fig = plt.gcf()
        warned = _declined(fig.tight_layout)
        params = fig.subplotpars
        return {"warned": warned, "params": [params.left, params.bottom, params.right,
                                             params.top, params.wspace, params.hspace]}
    if scene == "badsubplotgrid":
        fig = plt.figure()
        plt.subplot2grid((4, 5), (0, 0))
        plt.subplot2grid((5, 5), (0, 3), colspan=3, rowspan=5)
        warned = _declined(plt.tight_layout)
        return {"warned": warned, "positions": [_extents(ax.get_position()) for ax in fig.axes]}
    fig, ax = plt.subplots(tight_layout=True)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.annotate('BIG LONG STRING', xy=(1.25, 2), xytext=(10.5, 1.75), annotation_clip=False)
    widths = [ax.get_position().width]
    warned = [_declined(plt.tight_layout)]
    widths.append(ax.get_position().width)
    warned.append(_declined(lambda: plt.tight_layout(rect=[0, 0, 0.8, 0.8])))
    widths.append(ax.get_position().width)
    return {"warned": warned, "widths": widths}


def test_empty_layout():
    probe("Figure.tight_layout", _tight_layout, scene="empty")


def test_badsubplotgrid():
    probe("Figure.tight_layout", _tight_layout, scene="badsubplotgrid", tol=EXTENT_TOL)


def test_collapsed():
    probe("Figure.tight_layout", _tight_layout, scene="collapsed", tol=EXTENT_TOL)


def _suptitle():
    fig, ax = plt.subplots(tight_layout=True)
    st = fig.suptitle("foo")
    t = ax.set_title("bar")
    fig.canvas.draw()
    return [float(st.get_window_extent().y0), float(t.get_window_extent().y1)]


def test_suptitle():
    probe("Text.get_window_extent", _suptitle, tol=EXTENT_TOL)


def _set_tight_layout(calls):
    fig, ax = plt.subplots()
    out = []
    for value in calls:
        fig.set_tight_layout(value)
        out.append(fig.get_tight_layout())
    return out


def test_tight_toggle():
    probe("Figure.get_tight_layout", _set_tight_layout, calls=[True, False, True])
