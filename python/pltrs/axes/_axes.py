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
from ._base import _AxesBase
from ..axis import XAxis, YAxis
from ..legend import Legend
from ..lines import Line2D
from ..text import Text
from ._imaging import ImagingMixin
from ._insets import InsetsMixin
from ._plotting import PlottingMixin
from .._text_methods import AxesText


class Axes(PlottingMixin, ImagingMixin, InsetsMixin, AxesText, _AxesBase):
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
        ax._init_axes_state()
        return ax

    def _init_axes_state(self):
        super()._init_axes_state()
        self.xaxis = XAxis(self)
        self.yaxis = YAxis(self)

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
        """Set the label for the x-axis (see `.Axis.set_label_text`)."""
        return self.xaxis._set_axes_label(xlabel, fontdict, labelpad, loc,
                                          kwargs)

    def get_xlabel(self):
        """Get the xlabel text string."""
        return _axes.get_xlabel(self._native)

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the y-axis (see `.Axis.set_label_text`)."""
        return self.yaxis._set_axes_label(ylabel, fontdict, labelpad, loc,
                                          kwargs)

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
        left, right = self.xaxis._checked_limits(left, right)
        _axes.set_xlim(self._native, left, right)
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
        bottom, top = self.yaxis._checked_limits(bottom, top)
        _axes.set_ylim(self._native, bottom, top)
        return bottom, top

    @property
    def lines(self):
        """The lines of the Axes."""
        return _handles.wrap_all(_axes.get_lines(self._native), Line2D, self.figure)

    def get_legend_handles_labels(self, legend_handler_map=None):
        """
        Return handles and labels for legend

        ``ax.legend()`` is equivalent to ::

          h, l = ax.get_legend_handles_labels()
          ax.legend(h, l)
        """
        from .. import legend as mlegend
        return mlegend._get_legend_handles_labels([self], legend_handler_map)

    def legend(self, *args, **kwargs):
        """
        Place a legend on the Axes.

        Call signatures::

            legend()
            legend(handles, labels)
            legend(handles=handles)
            legend(labels)

        See matplotlib's documentation of `matplotlib.axes.Axes.legend` for
        the keyword arguments, which are those of `.Legend`.

        Returns
        -------
        `~matplotlib.legend.Legend`
        """
        from .. import legend as mlegend
        handles, labels, kwargs = mlegend._parse_legend_args([self], *args, **kwargs)
        return mlegend.Legend(self, handles, labels, **kwargs)

    def get_legend(self):
        """Return the `.Legend` instance, or None if no legend is defined."""
        from .._native import legend as _legend
        return _handles.wrap(_legend.axes_legend(self._native), Legend, self.figure)

    @property
    def legend_(self):
        """The `.Legend` of the Axes, or None."""
        return self.get_legend()
