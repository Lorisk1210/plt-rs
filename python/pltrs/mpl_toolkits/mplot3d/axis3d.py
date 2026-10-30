"""
The axes of a 3-D axes (matplotlib's ``mpl_toolkits.mplot3d.axis3d``):
`XAxis`, `YAxis` and `ZAxis`, each reaching pltrs (Rust)'s axis through its
`.Axes3D` and its name.
"""

import numpy as np

from pltrs import _api, colors as mcolors, ticker as mticker
from pltrs._native import mplot3d as _mplot3d


class Axis:
    """An Axis class for the 3D plots."""

    axis_name = None

    def __init__(self, axes):
        self.axes = axes

    def get_view_interval(self):
        """Return the view limits ``(min, max)`` of this axis."""
        return _mplot3d.get_lim(self.axes._native, self.axis_name)

    def get_scale(self):
        """Return this Axis' scale (as a str)."""
        return _mplot3d.get_scale(self.axes._native, self.axis_name)

    def get_inverted(self):
        """Return whether this Axis is oriented in the "inverse" direction."""
        return _mplot3d.get_inverted(self.axes._native, self.axis_name)

    def set_inverted(self, inverted):
        """Set whether this Axis is oriented in the "inverse" direction."""
        _mplot3d.set_inverted(self.axes._native, self.axis_name, bool(inverted))

    def get_label_text(self):
        """Get the text of the label."""
        return _mplot3d.get_label(self.axes._native, self.axis_name)

    def set_label_text(self, label, fontdict=None, **kwargs):
        """Set the text value of the axis label."""
        self.axes._set_axis_label(self.axis_name, label,
                                  {**(fontdict or {}), **kwargs})

    def set_major_formatter(self, formatter):
        """
        Set the formatter of the major ticker: a `.ticker.Formatter`, or a
        function ``f(x, pos)`` (made a `.ticker.FuncFormatter`).
        """
        self._set_formatter("major", formatter)

    def set_minor_formatter(self, formatter):
        """Set the formatter of the minor ticker (see `set_major_formatter`)."""
        self._set_formatter("minor", formatter)

    def _set_formatter(self, level, formatter):
        if not isinstance(formatter, mticker.Formatter) and callable(formatter):
            formatter = mticker.FuncFormatter(formatter)
        if not isinstance(formatter, mticker.Formatter):
            raise TypeError("formatter argument should be instance of "
                            "matplotlib.ticker.Formatter")
        formatter.axis = self
        _mplot3d.set_formatter(self.axes._native, self.axis_name, level,
                               formatter)

    def set_ticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """Set this Axis' tick locations."""
        if labels is not None or kwargs:
            raise NotImplementedError("pltrs: set_ticks(labels=...) on 3-D axes")
        _mplot3d.set_ticks(self.axes._native, self.axis_name,
                           [float(t) for t in ticks], bool(minor))

    def get_ticklocs(self, *, minor=False):
        """Return this Axis' tick locations in data coordinates."""
        return np.array(_mplot3d.get_ticklocs(self.axes._native,
                                              self.axis_name, bool(minor)))

    def get_majorticklocs(self):
        """Return this Axis' major tick locations in data coordinates."""
        return self.get_ticklocs()

    def get_minorticklocs(self):
        """Return this Axis' minor tick locations in data coordinates."""
        return self.get_ticklocs(minor=True)

    def set_ticks_position(self, position):
        """
        Set the ticks position.

        Parameters
        ----------
        position : {'lower', 'upper', 'both', 'default', 'none'}
            The position of the bolded axis lines, ticks, and tick labels.
        """
        _api.check_in_list(['lower', 'upper', 'both', 'default', 'none'],
                           position=position)
        _mplot3d.set_ticks_position(self.axes._native, self.axis_name, position)

    def get_ticks_position(self):
        """Get the ticks position."""
        return _mplot3d.get_ticks_position(self.axes._native, self.axis_name)

    def set_label_position(self, position):
        """
        Set the label position.

        Parameters
        ----------
        position : {'lower', 'upper', 'both', 'default', 'none'}
            The position of the axis label.
        """
        _api.check_in_list(['lower', 'upper', 'both', 'default', 'none'],
                           position=position)
        _mplot3d.set_label_position(self.axes._native, self.axis_name, position)

    def get_label_position(self):
        """Get the label position."""
        return _mplot3d.get_label_position(self.axes._native, self.axis_name)

    def set_pane_color(self, color, alpha=None):
        """
        Set pane color.

        Parameters
        ----------
        color : :mpltype:`color`
            Color for axis pane.
        alpha : float, optional
            Alpha value for axis pane. If None, base it on *color*.
        """
        _mplot3d.set_pane_color(self.axes._native, self.axis_name,
                                mcolors.to_rgba(color, alpha))


class XAxis(Axis):
    """The x axis of a 3-D axes."""
    axis_name = "x"


class YAxis(Axis):
    """The y axis of a 3-D axes."""
    axis_name = "y"


class ZAxis(Axis):
    """The z axis of a 3-D axes."""
    axis_name = "z"
