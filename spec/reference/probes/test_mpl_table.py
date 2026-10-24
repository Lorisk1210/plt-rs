"""matplotlib's own non-image tests of tables
(``matplotlib/tests/test_table.py``).

Area ``mpl_table``. Every op runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``).

Ops
---
``CustomCell.get_path``  {visible_edges} -> the codes of the segments of
    ``CustomCell((0, 0), visible_edges=..., width=1, height=1).get_path()``:
    a ``MOVETO`` then a ``LINETO`` for each drawn edge and a ``MOVETO`` for
    each hidden one, bottom, right, top, left, the last closing the path
    when all four are drawn (``test_customcell``).
``Table.fontsize``  {fontsize} -> the font size of the cells ``(0, 0)`` and
    ``(1, 1)`` of ``ax.table(cellText=[["a", 1], ["b", 2]], loc="top",
    fontsize=fontsize)`` (``test_table_fontsize``).

``test_non_square``, which only checks that a table of two colours in one
column draws, is a case of ``mpl_tests_table``; the image tests are cases
of ``mpl_misc``.

Left out: ``test_table_cells`` (``Table(ax)`` made empty outside the axes
and Python's item protocol, ``table[row, col] = cell``; plt-rs tables come
from ``Axes::table`` and take cells with ``add_cell``, which the unit tests
check) and ``test_table_dataframe`` (pandas).
"""

import matplotlib.pyplot as plt
from matplotlib.table import CustomCell

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_table")
probe = prober(probes)


def _customcell(visible_edges):
    cell = CustomCell((0, 0), visible_edges=visible_edges, width=1, height=1)
    return [int(code) for _, code in cell.get_path().iter_segments()]


def test_customcell():
    for edges in ("horizontal", "vertical", "open", "closed", "T", "R", "B", "L", "BRL"):
        probe("CustomCell.get_path", _customcell, visible_edges=edges)


def _fontsize(fontsize):
    fig, ax = plt.subplots()
    table = ax.table(cellText=[["a", 1], ["b", 2]], loc="top", fontsize=fontsize)
    return [table[(0, 0)].get_fontsize(), table[(1, 1)].get_fontsize()]


def test_table_fontsize():
    assert probe("Table.fontsize", _fontsize, fontsize=20) == [20, 20]
