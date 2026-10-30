"""
The axes of a 3-D axes (matplotlib's ``mpl_toolkits.mplot3d.axis3d``):
`XAxis`, `YAxis` and `ZAxis`, each reaching pltrs (Rust)'s axis through its
`.Axes3D` and its name.
"""

import numpy as np

from pltrs import _api, colors as mcolors, ticker as mticker
from pltrs._native import mplot3d as _mplot3d


class _AxisLine:
    """
    The bolded line of a 3-D axis (matplotlib's ``axis.line``): pltrs
    (Rust) draws it along the edge the tick position names; this reads its
    projected end points and shows or hides it.
    """

    def __init__(self, axis):
        self._axis = axis

    def _args(self):
        return self._axis.axes._native, self._axis.axis_name

    def get_data(self, orig=True):
        """Return the x and y data of the line's end points, in projected
        coordinates."""
        data = _mplot3d.axis_line_data(*self._args())
        if data is None:
            return np.array([]), np.array([])
        return np.array(data[0]), np.array(data[1])

    def get_xdata(self, orig=True):
        """Return the x data of the line's end points."""
        return self.get_data()[0]

    def get_ydata(self, orig=True):
        """Return the y data of the line's end points."""
        return self.get_data()[1]

    def get_visible(self):
        """Return whether the line is drawn."""
        return _mplot3d.get_line_visible(*self._args())

    def set_visible(self, b):
        """Set whether the line is drawn."""
        _mplot3d.set_line_visible(*self._args(), bool(b))


class Axis:
    """An Axis class for the 3D plots."""

    axis_name = None

    def __init__(self, axes):
        self.axes = axes
        self.line = _AxisLine(self)
        #: The scale object the axis was last given (`get_transform`).
        self._scale = None

    def get_transform(self):
        """Return the transform of the axis' scale."""
        from pltrs import scale as mscale
        scale = self._scale if self._scale is not None else mscale.LinearScale(self)
        return scale.get_transform()

    def _get_tickdir(self, position):
        """
        Get the direction of the tick.

        Parameters
        ----------
        position : str, optional : {'upper', 'lower', 'default'}
            The position of the axis.

        Returns
        -------
        tickdir : int
            Index which indicates which coordinate the tick line will
            align with.
        """
        _api.check_in_list(('upper', 'lower', 'default'), position=position)
        return _mplot3d.tick_direction(self.axes._native, self.axis_name, position)

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

    def set_ticklabels(self, labels, *, minor=False, fontdict=None, **kwargs):
        """
        Set this Axis' tick labels with list of string labels.

        pltrs: the tick labels of a 3-D axis are drawn by pltrs (Rust)
        rather than kept as `.Text` artists, so nothing is returned.
        """
        if fontdict is not None or kwargs:
            raise NotImplementedError(
                "pltrs: set_ticklabels(<text properties>) on 3-D axes")
        _mplot3d.set_ticklabels(self.axes._native, self.axis_name,
                                [str(label) for label in labels], bool(minor))

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
