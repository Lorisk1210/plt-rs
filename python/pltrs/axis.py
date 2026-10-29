"""
The x and y axes of an Axes (matplotlib's ``axis``): ticks, their labels
and formatters.
"""

from ._native import ticker as _ticker


class Axis:
    """
    Base class for `.XAxis` and `.YAxis`.

    An Axis belongs to an `~.axes.Axes` (``axis.axes``); plt-rs reaches it
    through the axes and the axis name.
    """

    axis_name = None

    def __init__(self, axes):
        self.axes = axes

    def set_major_formatter(self, formatter):
        """
        Set the formatter of the major ticker: a `.ticker.Formatter`, or a
        function ``f(x, pos)`` (made a `.ticker.FuncFormatter`).

        plt-rs calls the formatter (any callable taking ``(x, pos)``) each
        time it labels the ticks.
        """
        self._set_formatter("major", formatter)

    def set_minor_formatter(self, formatter):
        """Set the formatter of the minor ticker (see `set_major_formatter`)."""
        self._set_formatter("minor", formatter)

    def _set_formatter(self, level, formatter):
        from .ticker import Formatter, FuncFormatter
        if not isinstance(formatter, Formatter) and callable(formatter):
            formatter = FuncFormatter(formatter)
        if not isinstance(formatter, Formatter):
            raise TypeError("formatter argument should be instance of "
                            "matplotlib.ticker.Formatter")
        formatter.axis = self
        _ticker.set_formatter(self.axes._native, self.axis_name, level, formatter)


class XAxis(Axis):
    """The x-axis of an Axes."""

    axis_name = "x"


class YAxis(Axis):
    """The y-axis of an Axes."""

    axis_name = "y"
