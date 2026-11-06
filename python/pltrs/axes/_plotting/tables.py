"""
The tables of an Axes: `~.Axes.table`, matplotlib's `.table.table` as a
method (``ax.tables`` lists them, see `.._children`).
"""

from ... import table as mtable
from ._helpers import _make_axes_method


class TablesMixin:
    """`~.Axes.table`."""

    table = _make_axes_method(mtable.table)
