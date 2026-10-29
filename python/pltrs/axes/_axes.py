"""
The `Axes` class: plotting methods on a pltrs (Rust) axes (matplotlib's
``axes._axes`` and ``axes._base``).

Each plotting method turns matplotlib's signature (positional forms,
aliases, defaults) into the explicit arguments of a native function of
``_native.axes``, and wraps the artists it returns (see `.._handles`).
"""

import numpy as np

from .. import _handles
from .._native import axes as _axes
from ..artist import Artist
from ..axis import XAxis, YAxis
from ..legend import Legend
from ..lines import Line2D
from ..text import Text
from ._imaging import ImagingMixin
from ._plotting import PlottingMixin
from .._text_methods import AxesText


class Axes(PlottingMixin, ImagingMixin, AxesText, Artist):
    """
    An Axes object encapsulates all the elements of an individual (sub-)plot
    in a figure.

    See matplotlib's documentation of `matplotlib.axes.Axes`.
    """

    #: The name of the projection, under which `.projections` registers
    #: the class.
    name = "rectilinear"

    @classmethod
    def _from_native(cls, native, figure):
        ax = super()._from_native(native, figure)
        ax.xaxis = XAxis(ax)
        ax.yaxis = YAxis(ax)
        ax.child_axes = []
        return ax

    def set_title(self, label, fontdict=None, loc=None, pad=None, *, y=None,
                  **kwargs):
        """Set a title for the Axes."""
        if loc not in (None, "center") or pad is not None or y is not None:
            raise NotImplementedError("pltrs: set_title(loc=..., pad=..., y=...)")
        title = _handles.wrap(_axes.set_title(self._native, str(label)),
                              Text, self.figure)
        title._internal_update({**(fontdict or {}), **kwargs})
        return title

    def get_title(self, loc="center"):
        """Get an Axes title."""
        return _axes.get_title(self._native, loc)

    def set_xlabel(self, xlabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the x-axis."""
        _axes.set_xlabel(self._native, str(xlabel))

    def get_xlabel(self):
        """Get the xlabel text string."""
        return _axes.get_xlabel(self._native)

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the y-axis."""
        _axes.set_ylabel(self._native, str(ylabel))

    def get_ylabel(self):
        """Get the ylabel text string."""
        return _axes.get_ylabel(self._native)

    def _display_to_data(self, xy):
        """The data coordinates of the display point *xy*
        (``self.transData.inverted().transform(xy)``)."""
        x, y = xy
        return _axes.display_to_data(self._native, x, y)

    def get_xlim(self):
        """Return the x-axis view limits."""
        return _axes.get_xlim(self._native)

    def set_xlim(self, left=None, right=None):
        """Set the x-axis view limits."""
        if right is None and np.iterable(left):
            left, right = left
        current = self.get_xlim()
        left = current[0] if left is None else left
        right = current[1] if right is None else right
        _axes.set_xlim(self._native, float(left), float(right))
        return left, right

    def get_ylim(self):
        """Return the y-axis view limits."""
        return _axes.get_ylim(self._native)

    def set_ylim(self, bottom=None, top=None):
        """Set the y-axis view limits."""
        if top is None and np.iterable(bottom):
            bottom, top = bottom
        current = self.get_ylim()
        bottom = current[0] if bottom is None else bottom
        top = current[1] if top is None else top
        _axes.set_ylim(self._native, float(bottom), float(top))
        return bottom, top

    @property
    def lines(self):
        """The lines of the Axes."""
        return _handles.wrap_all(_axes.get_lines(self._native), Line2D, self.figure)

    def legend(self, *args, **kwargs):
        """Place a legend on the Axes."""
        if args or kwargs:
            raise NotImplementedError("pltrs: legend() with arguments")
        return _handles.wrap(_axes.legend(self._native), Legend, self.figure)

    def get_legend(self):
        """Return the `.Legend` instance, or None if no legend is defined."""
        return _handles.wrap(_axes.get_legend(self._native), Legend, self.figure)
