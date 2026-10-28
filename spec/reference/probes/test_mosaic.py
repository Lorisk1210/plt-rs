"""Named subplot layouts (``Figure.subplot_mosaic``): parsing string and
nested-list mosaics, and the axes they make.

Area ``mosaic``. Every op lays the mosaic out in ``Figure(figsize=(4, 3),
dpi=100)``.

Ops
---
``figure.subplot_mosaic``  {mosaic, width_ratios?, height_ratios?}
    -> ``[[label, [x0, y0, width, height]], ...]``: every axes in creation
    order with its position in figure fractions. ``mosaic`` is a string
    (rows split by ``;`` or newlines, ``.`` for an empty cell) or nested
    lists of labels, ``null`` for an empty cell and lists for nested
    mosaics.
"""

from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mosaic")


def _mosaic(mosaic, width_ratios=None, height_ratios=None):
    fig = Figure(figsize=(4, 3), dpi=100)
    if isinstance(mosaic, list):
        mosaic = _cells(mosaic)
    axes = fig.subplot_mosaic(mosaic, width_ratios=width_ratios, height_ratios=height_ratios,
                              empty_sentinel=None if not isinstance(mosaic, str) else ".")
    return [[str(label), [float(v) for v in ax.get_position().bounds]]
            for label, ax in axes.items()]


def _cells(rows):
    """JSON rows to matplotlib's nested lists (nested mosaics stay lists)."""
    return [[_cells(cell) if isinstance(cell, list) else cell for cell in row] for row in rows]


STRINGS = ["AB;CD", "AAB;CCB", "A.B;CCC", "AB\nCD", "\n    AB\n    CD\n", "A", "AAA;BCD",
           "ABC", "A;B;C", "AB;BA", "AB;C", "", "ABA", ".A;..", "A..;BBB", "aaa;bcc;bcc",
           "AB;CD;EF", "AA;AA", "xyz;xyz", "A B", "AB\n\nCD", "  AB\n   CD", "\tAB\n\tCD",
           "AB;", ";AB"]
NESTED = [
    [["A", "B"], ["C", [["D", "E"], ["F", "G"]]]],
    [["A", [["B"], ["C"]]], ["A", "D"]],
    [["A", None], [None, "B"]],
    [[[["A", "B"]], "C"]],
    [["A", [["A"]]]],
    [["A", "B"], ["C"]],
    [[[["A", [["B", "C"]]]], "D"]],
]


def test_mosaics():
    for mosaic in STRINGS + NESTED:
        probes.probe("figure.subplot_mosaic", _mosaic, mosaic=mosaic)
    for mosaic, widths, heights in [("AB;CD", [1, 2], None), ("AB;CD", None, [3, 1]),
                                    ("AAB;CCB", [1, 1, 2], [1, 2]), ("AB;CD", [1], None),
                                    ("AB;CD", [1, 2], [1, 2, 3])]:
        probes.probe("figure.subplot_mosaic", _mosaic, mosaic=mosaic, width_ratios=widths,
                     height_ratios=heights)


def test_string_parser():
    """matplotlib's ``TestSubplotMosaic.test_string_parser``: the grids its
    strings parse to (``Figure._normalize_grid_string``) show in the axes
    they lay out."""
    for mosaic in ("ABC", "AB;CC", "AB;CC;DE", """
                         ABC
                         """, """
                         AB
                         CC
                         """, """
                         AB
                         CC
                         DE
                         """):
        probes.probe("figure.subplot_mosaic", _mosaic, mosaic=mosaic)
