"""
The tables of an Axes: `~.Axes.table`, matplotlib's `.table.table` as a
method, and ``ax.tables``.
"""

from ... import table as mtable
from ..._native import table as _table
from ._helpers import wrap_all


class TablesMixin:
    """`~.Axes.table` and ``tables``."""

    table = mtable.table

    @property
    def tables(self):
        """The tables of the Axes, in the order they were added."""
        return wrap_all(self, _table.axes_tables(self._native), mtable.Table)
