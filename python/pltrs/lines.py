"""
2D lines with markers (matplotlib's ``lines``).
"""

import numpy as np

from . import _api
from ._native import lines as _lines
from .artist import Artist


@_api.define_aliases({
    "color": ["c"],
    "linestyle": ["ls"],
    "linewidth": ["lw"],
    "markersize": ["ms"],
})
class Line2D(Artist):
    """
    A line - the line can have both a solid linestyle connecting all
    the vertices, and a marker at each vertex.
    """

    def get_xdata(self, orig=True):
        """Return the xdata."""
        return _lines.get_xdata(self._native)

    def get_ydata(self, orig=True):
        """Return the ydata."""
        return _lines.get_ydata(self._native)

    def get_data(self, orig=True):
        """Return the line data as an ``(xdata, ydata)`` pair."""
        return self.get_xdata(orig=orig), self.get_ydata(orig=orig)

    def set_data(self, *args):
        """Set the x and y data."""
        if len(args) == 1:
            (x, y), = args
        else:
            x, y = args
        _lines.set_data(self._native, np.asarray(x, dtype=float),
                        np.asarray(y, dtype=float))

    def set_xdata(self, x):
        """Set the data array for x."""
        self.set_data(x, self.get_ydata())

    def set_ydata(self, y):
        """Set the data array for y."""
        self.set_data(self.get_xdata(), y)

    def get_color(self):
        """Return the line color."""
        return _lines.get_color(self._native)

    def set_color(self, color):
        """Set the color of the line."""
        _lines.set_color(self._native, color)

    def get_linewidth(self):
        """Return the linewidth in points."""
        return _lines.get_linewidth(self._native)

    def set_linewidth(self, w):
        """Set the line width in points."""
        _lines.set_linewidth(self._native, float(w))

    def get_linestyle(self):
        """Return the linestyle."""
        return _lines.get_linestyle(self._native)

    def set_linestyle(self, ls):
        """Set the linestyle of the line."""
        _lines.set_linestyle(self._native, ls)

    def get_markersize(self):
        """Return the marker size in points."""
        return _lines.get_markersize(self._native)

    def set_markersize(self, sz):
        """Set the marker size in points."""
        _lines.set_markersize(self._native, float(sz))
