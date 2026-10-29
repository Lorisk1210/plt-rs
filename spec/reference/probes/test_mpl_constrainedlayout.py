"""Constrained layout, ported from matplotlib's own
``tests/test_constrainedlayout.py``: what figure titles and labels do to
the layout, aligned labels, the layout engine a figure is given and a
reused constrained figure.

Area ``mpl_constrainedlayout``. Every op runs in the test style (see
``probes/_mpl_tests.py``): its ``style`` and ``rc`` arguments.

Ops
---
``Figure.suptitle``  {calls} -> the extents of the axes of
    ``plt.subplots(layout="constrained")`` after a draw, then after each
    call ``[text, y]`` (``y`` null: placed by the layout) and a draw.
``Text.get_position``  {calls} -> the height of the title of
    ``test_compressed_suptitle``'s compressed figure after each call
    ``suptitle("Title", **kwargs)`` (``y``, ``in_layout``) and a draw.
``Figure.supxlabel``  {ylabel, x?, y?} -> the tight box (pixels) of the
    axes of ``plt.subplots(layout="constrained")`` after a draw, then
    after ``supxlabel("Boo", x=x)``, ``supylabel(ylabel, y=y)`` and a draw.
``Figure.align_ylabels``  {} -> the left edges (pixels) of the y labels of
    ``test_align_labels``'s three axes (``ax1``, ``ax2``, ``ax3``) after
    ``align_ylabels`` and a draw.
``ConstrainedLayoutEngine.set``  {mosaic, pads} -> the bounds of the axes
    ``"A"`` and ``"B"`` of ``plt.subplot_mosaic(mosaic,
    layout="constrained")`` after ``get_layout_engine().set(**pads)`` and
    a draw.
``Figure.get_constrained_layout``  {constrained_layout} -> whether
    ``plt.subplots(constrained_layout=...)`` is laid out by the
    constrained engine (a dict sets the engine's parameters).
``Figure.set_constrained_layout``  {calls} -> the same after each
    ``set_constrained_layout(value)`` on ``plt.subplots()``.
``pyplot.figure``  {} -> ``test_constrained_layout23``: after making the
    constrained figure ``"123"`` twice, cleared the second time, each
    with a grid nested in a grid and a suptitle, and a draw: the figure
    numbers and labels, the suptitle and the number of figure texts.

Cases
-----
Named after the upstream tests. Adaptations: the tests run with text
drawn as rectangles (``text_placeholders``); these record the positions
matplotlib gives real text, so the tests' literals do not apply; the
probes record the values the tests compare rather than the comparisons;
``test_constrained_layout23`` draws the reused figure and records what
pyplot holds.

Covered elsewhere: the image tests (figure group ``mpl_figure``); the
tests that draw and then check positions are figure cases of the
``mpl_tests_constrainedlayout`` group (``test_identical_subgridspec``,
``test_constrained_layout18`` to ``20``, ``test_hidden_axes``,
``test_colorbar_align``, ``test_manually_set_position``,
``test_gridspec_addressing``, ``test_discouraged_api``, ``test_kwargs``,
``test_rect``, ``test_compressed1``, ``test_submerged_subfig``). Not
ported: the warning of ``test_constrained_layout7`` (pltrs grids have no
``figure`` argument to forget: every grid an axes is placed on takes part
in the layout), the ``TypeError`` of misspelt keywords
(``test_constrained_layout13``), the pending deprecation warnings
(``test_discouraged_api``, ``test_constrained_toggle``), ``test_bbox``
(a saved region given as a box) and ``test_layout_leak`` (garbage
collection).
"""

import matplotlib.pyplot as plt

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_constrainedlayout")
probe = prober(probes)

#: Positions in figure fractions or pixels, compared to a millionth.
EXTENT_TOL = 1e-6


def _extents(bbox):
    return [float(v) for v in bbox.extents]


def _suptitle(calls):
    fig, ax = plt.subplots(layout="constrained")
    fig.draw_without_rendering()
    out = [_extents(ax.get_position())]
    for text, y in calls:
        fig.suptitle(text, **({} if y is None else {"y": y}))
        fig.draw_without_rendering()
        out.append(_extents(ax.get_position()))
    return out


def test_constrained_layout21():
    probe("Figure.suptitle", _suptitle, calls=[["Suptitle0", None], ["Suptitle1", None]],
          tol=EXTENT_TOL)


def test_constrained_layout22():
    probe("Figure.suptitle", _suptitle, calls=[["Suptitle", 0.5]], tol=EXTENT_TOL)


def _compressed_suptitle(calls):
    fig, (ax0, ax1) = plt.subplots(
        nrows=2, figsize=(4, 10), layout="compressed",
        gridspec_kw={"height_ratios": (1 / 4, 3 / 4), "hspace": 0})
    ax0.axis("equal")
    ax0.set_box_aspect(1 / 3)
    ax1.axis("equal")
    ax1.set_box_aspect(1)
    out = []
    for kwargs in calls:
        title = fig.suptitle("Title", **kwargs)
        fig.draw_without_rendering()
        out.append(float(title.get_position()[1]))
    return out


def test_compressed_suptitle():
    probe("Text.get_position", _compressed_suptitle,
          calls=[{}, {"y": 0.98}, {"in_layout": False}], tol=EXTENT_TOL)


def _suplabels(ylabel, x=None, y=None):
    fig, ax = plt.subplots(layout="constrained")
    fig.draw_without_rendering()
    out = [_extents(ax.get_tightbbox(fig.canvas.get_renderer()))]
    fig.supxlabel("Boo", **({} if x is None else {"x": x}))
    fig.supylabel(ylabel, **({} if y is None else {"y": y}))
    fig.draw_without_rendering()
    return out + [_extents(ax.get_tightbbox(fig.canvas.get_renderer()))]


def test_suplabels():
    probe("Figure.supxlabel", _suplabels, ylabel="Booy", tol=EXTENT_TOL)
    probe("Figure.supxlabel", _suplabels, ylabel="Boo", x=0.5, y=0.5, tol=EXTENT_TOL)


def _align_labels():
    fig, (ax3, ax1, ax2) = plt.subplots(3, 1, layout="constrained", figsize=(6.4, 8),
                                        gridspec_kw={"height_ratios": (1, 1, 0.7)})
    ax1.set_ylim(0, 1)
    ax1.set_ylabel("Label")
    ax2.set_ylim(-1.5, 1.5)
    ax2.set_ylabel("Label")
    ax3.set_ylim(0, 1)
    ax3.set_ylabel("Label")
    fig.align_ylabels(axs=(ax3, ax1, ax2))
    fig.draw_without_rendering()
    return [float(ax.yaxis.label.get_window_extent().x0) for ax in (ax1, ax2, ax3)]


def test_align_labels():
    probe("Figure.align_ylabels", _align_labels, tol=EXTENT_TOL)


def _submerged_gap(mosaic, pads):
    fig, ax_dict = plt.subplot_mosaic(mosaic, layout="constrained")
    fig.get_layout_engine().set(**pads)
    fig.draw_without_rendering()
    return {label: [float(v) for v in ax_dict[label].get_position().bounds]
            for label in "AB"}


def test_submerged_height_gap():
    for mosaic in ["AC;BC", "ACDE;BCDE"]:
        probe("ConstrainedLayoutEngine.set", _submerged_gap, mosaic=mosaic, pads={"h_pad": 0.2},
              tol=EXTENT_TOL)


def test_submerged_width_gap():
    for mosaic in ["AB;CC", "AB;CC;DD"]:
        probe("ConstrainedLayoutEngine.set", _submerged_gap, mosaic=mosaic, pads={"w_pad": 0.2},
              tol=EXTENT_TOL)


def _get_constrained_layout(constrained_layout):
    fig, ax = plt.subplots(constrained_layout=constrained_layout)
    return fig.get_constrained_layout()


def test_set_constrained_layout():
    for arg in [True, False, {}, {"rect": None}]:
        probe("Figure.get_constrained_layout", _get_constrained_layout, constrained_layout=arg)


def _set_constrained_layout(calls):
    fig, ax = plt.subplots()
    out = []
    for value in calls:
        fig.set_constrained_layout(value)
        out.append(fig.get_constrained_layout())
    return out


def test_constrained_toggle():
    probe("Figure.set_constrained_layout", _set_constrained_layout, calls=[True, False, True])


def _reused_figure():
    for i in range(2):
        fig = plt.figure(layout="constrained", clear=True, num="123")
        gs = fig.add_gridspec(1, 2)
        gs[0].subgridspec(2, 2)
        fig.suptitle(f"Suptitle{i}")
    fig.draw_without_rendering()
    return {"fignums": plt.get_fignums(), "figlabels": plt.get_figlabels(),
            "suptitle": fig.get_suptitle(), "texts": len(fig.texts)}


def test_constrained_layout23():
    probe("pyplot.figure", _reused_figure)
