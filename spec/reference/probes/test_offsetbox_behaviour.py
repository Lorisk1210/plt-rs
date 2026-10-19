"""Behaviour of offset boxes, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_offsetbox.py``.

Area ``offsetbox_behaviour``.

Ops
---
``offsetbox.packed_offsets``  {widths, total, sep, mode} -> [total, offsets]
    ``_get_packed_offsets(widths, total, sep, mode)``: the total length and
    the offset of each box packed ``"fixed"``, ``"expand"`` or ``"equal"``;
    ``equal`` without a total needs a separation (``sep`` null, a
    ``ValueError``). Mirrors ``test_get_packed_offsets`` (over its whole
    parameter grid), ``test_get_packed_offsets_fixed``,
    ``test_get_packed_offsets_expand``, ``test_get_packed_offsets_equal``
    and ``test_get_packed_offsets_equal_total_none_sep_none``.
``offsetbox.aligned_offsets``  {yspans, height, align} -> [[y0, y1], offsets]
    ``_get_aligned_offsets(yspans, height, align)``: the span the aligned
    boxes cover and the offset of each, the alignment used by
    ``test_packers``.
``offsetbox.extents``  {scene} -> [bbox, ...]
    Window extents in display pixels after a draw on ``Figure(figsize=(4,
    3))``:

    ``borderpad``  ``AnchoredText`` at ``"lower left"`` with ``borderpad=5``,
        ``(5, 5)`` and ``(10, 4)``
        (``test_anchored_offsetbox_tuple_and_float_borderpad``).
    ``annotation_boxes``  over ``ax.axis([0, 1, 0, 1])``: the annotation
        ``"Annotation"`` at ``(.9, .9)`` with its text at ``(1.1, 1.1)`` and
        an arrow, an ``AnnotationBbox`` of a clipped 20 x 20
        ``DrawingArea`` at ``xybox=(-0.2, 0.5)`` (axes fraction,
        ``box_alignment=(0, .5)``) with an arrow, and one of a 10 x 10
        ``OffsetImage`` (zoom 3, values ``(3 r + 7 c) % 10 / 10``) 75
        points above ``(0.5, -.3)`` axes fraction, padded 0.3, with an
        arrow; then ``fig.get_tightbbox()`` in inches
        (``test_annotationbbox_extents``, under the reference rcParams
        rather than ``rcParamsDefault``, with deterministic image data).

Left out: ``test_offsetbox_clip_children`` (figure staleness),
``test_offsetbox_loc_codes``, ``test_expand_with_tight_layout``,
``test_paddedbox_default_values``, ``test_remove_draggable`` (smoke
tests), ``test_picking`` (the ``pick`` area picks offset boxes' kin),
``test_zorder``, ``test_arrowprops_copied``,
``test_annotationbbox_properties``, ``test_textarea_properties`` (Python
attributes), ``test_draggable_in_subfigure`` (a private dragging flag),
and ``test_packers`` beyond its alignment (child offsets of a packer are
not observable from outside plt-rs).
"""

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.offsetbox import (AnchoredText, AnnotationBbox, DrawingArea, OffsetImage,
                                  _get_aligned_offsets, _get_packed_offsets)
import matplotlib.patches as mpatches

from spec_support import ProbeSet

probes = ProbeSet("offsetbox_behaviour")


def _packed(widths, total, sep, mode):
    total, offsets = _get_packed_offsets(widths, total, sep, mode=mode)
    return [float(total), [float(o) for o in offsets]]


def _aligned(yspans, height, align):
    (y0, y1), offsets = _get_aligned_offsets(yspans, height, align)
    return [[float(y0), float(y1)], [float(o) for o in offsets]]


def _extents(scene):
    fig = Figure(figsize=(4, 3), dpi=100)
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    if scene == "borderpad":
        artists = [ax.add_artist(AnchoredText(text, loc="lower left", borderpad=pad))
                   for text, pad in [("float", 5), ("tuple", (5, 5)), ("tuple_asym", (10, 4))]]
        fig.canvas.draw()
        return [a.get_window_extent().get_points().tolist() for a in artists]
    ax.axis([0, 1, 0, 1])
    an1 = ax.annotate("Annotation", xy=(.9, .9), xytext=(1.1, 1.1),
                      arrowprops=dict(arrowstyle="->"), clip_on=False, va="baseline",
                      ha="left")
    da = DrawingArea(20, 20, 0, 0, clip=True)
    da.add_artist(mpatches.Circle((-10, 30), 32))
    ab3 = AnnotationBbox(da, [.5, .5], xybox=(-0.2, 0.5), xycoords="data",
                         boxcoords="axes fraction", box_alignment=(0., .5),
                         arrowprops=dict(arrowstyle="->"))
    ax.add_artist(ab3)
    rows, columns = np.mgrid[0:10, 0:10]
    im = OffsetImage((3 * rows + 7 * columns) % 10 / 10, zoom=3)
    im.image.axes = ax
    ab6 = AnnotationBbox(im, (0.5, -.3), xybox=(0, 75), xycoords="axes fraction",
                         boxcoords="offset points", pad=0.3, arrowprops=dict(arrowstyle="->"))
    ax.add_artist(ab6)
    fig.canvas.draw()
    extents = [a.get_window_extent().get_points().tolist() for a in [an1, ab3, ab6]]
    return extents + [fig.get_tightbbox(fig.canvas.get_renderer()).get_points().tolist()]


WIDTHS = [[150], [150, 150, 150], [0.1], [0.1, 0.1]]
TOTALS = [250, 100, 0, -1, None]
SEPS = [250, 1, 0, -1]
MODES = ["expand", "fixed", "equal"]
EXPLICIT = [
    ([3, 1, 2], None, 1, "fixed"), ([3, 1, 2], 10, 1, "fixed"), ([3, 1, 2], 5, 1, "fixed"),
    ([.1, .1, .1], None, None, "expand"), ([3, 1, 2], 10, 1, "expand"),
    ([3, 1, 2], 5, 1, "expand"), ([3, 2, 1], 6, None, "equal"),
    ([3, 2, 1, .5], 2, None, "equal"), ([.5, 1, .2], None, 1, "equal"),
    ([1, 1, 1], None, None, "equal"),
]


def test_packed_offsets():
    for widths in WIDTHS:
        for total in TOTALS:
            for sep in SEPS:
                for mode in MODES:
                    probes.probe("offsetbox.packed_offsets", _packed, widths=widths, total=total,
                                 sep=sep, mode=mode)
    for widths, total, sep, mode in EXPLICIT:
        probes.probe("offsetbox.packed_offsets", _packed, widths=widths, total=total, sep=sep,
                     mode=mode)


SPANS = [[[0, 30], [0, 60]], [[-5, 10], [2, 4], [-1, 0.5]], [[0, 2]]]


def test_aligned_offsets():
    for yspans in SPANS:
        for height in [None, 100]:
            for align in ["baseline", "left", "bottom", "right", "top", "center"]:
                probes.probe("offsetbox.aligned_offsets", _aligned, yspans=yspans,
                             height=height, align=align)


def test_extents():
    for scene in ["borderpad", "annotation_boxes"]:
        probes.probe("offsetbox.extents", _extents, scene=scene)
