"""
The x and y axes of an Axes (matplotlib's ``axis``): ticks, their labels
and formatters.
"""

import numpy as np

from . import _api
from ._native import axis as _axis, ticker as _ticker


class Axis:
    """
    Base class for `.XAxis` and `.YAxis`.

    An Axis belongs to an `~.axes.Axes` (``axis.axes``); pltrs (Rust) reaches it
    through the axes and the axis name.
    """

    axis_name = None

    def __init__(self, axes):
        self.axes = axes

    def get_ticklocs(self, *, minor=False):
        """Return this Axis' tick locations in data coordinates."""
        return _axis.get_ticklocs(self.axes._native, self.axis_name, bool(minor))

    def get_majorticklocs(self):
        """Return this Axis' major tick locations in data coordinates."""
        return self.get_ticklocs(minor=False)

    def get_minorticklocs(self):
        """Return this Axis' minor tick locations in data coordinates."""
        return self.get_ticklocs(minor=True)

    def set_ticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """
        Set this Axis' tick locations and optionally tick labels.

        If necessary, the view limits of the Axis are expanded so that all
        given ticks are visible.

        Parameters
        ----------
        ticks : 1D array-like
            Array of tick locations (either floats or in axis units).
        labels : list of str, optional
            Tick labels for each location in *ticks*; must have the same length
            as *ticks*. If set, the labels are used as is, via a
            `.FixedFormatter`.
        minor : bool, default: False
            If ``False``, set only the major ticks; if ``True``, only the minor
            ticks.
        **kwargs
            `.Text` properties for the labels. Using these is only allowed if
            you pass *labels*.
        """
        if labels is None and kwargs:
            first_key = next(iter(kwargs))
            raise ValueError(
                f"Incorrect use of keyword argument {first_key!r}. Keyword arguments "
                "other than 'minor' modify the text labels and can only be used if "
                "'labels' are passed as well.")
        if kwargs:
            raise NotImplementedError(
                f"pltrs: set_ticks({next(iter(kwargs))}=...)")
        _axis.set_ticks(self.axes._native, self.axis_name,
                        np.asarray(ticks, dtype=float).ravel().tolist(),
                        bool(minor))
        if labels is not None:
            self.set_ticklabels(labels, minor=minor)
        return self.get_ticklocs(minor=minor)

    def set_ticklabels(self, labels, *, minor=False, **kwargs):
        r"""
        [*Discouraged*] Set this Axis' tick labels with list of string labels.

        Parameters
        ----------
        labels : sequence of str
            Texts for labeling each tick location in the sequence set by
            `.Axis.set_ticks`.
        minor : bool
            If True, set minor ticks instead of major ticks.
        **kwargs
            Text properties.
        """
        if kwargs:
            raise NotImplementedError(
                f"pltrs: set_ticklabels({next(iter(kwargs))}=...)")
        _axis.set_ticklabels(self.axes._native, self.axis_name,
                             [str(label) for label in labels], bool(minor))

    def get_inverted(self):
        """
        Return whether this Axis is oriented in the "inverse" direction.

        The "normal" direction is increasing to the right for the x-axis and to
        the top for the y-axis; the "inverse" direction is increasing to the
        left for the x-axis and to the bottom for the y-axis.
        """
        return _axis.get_inverted(self.axes._native, self.axis_name)

    def set_inverted(self, inverted):
        """
        Set whether this Axis is oriented in the "inverse" direction.

        The "normal" direction is increasing to the right for the x-axis and to
        the top for the y-axis; the "inverse" direction is increasing to the
        left for the x-axis and to the bottom for the y-axis.
        """
        _axis.set_inverted(self.axes._native, self.axis_name, bool(inverted))

    def get_ticks_position(self):
        """
        Return the ticks position ("top", "bottom", "left", "right",
        "default" or "unknown").
        """
        return _axis.get_ticks_position(self.axes._native, self.axis_name)

    def _set_ticks_position(self, position):
        """Put the ticks where *position* says (natively checked)."""
        _axis.set_ticks_position(self.axes._native, self.axis_name, position)

    def get_label_position(self):
        """
        Return the label position (top, bottom, left or right).
        """
        return _axis.get_label_position(self.axes._native, self.axis_name)

    @property
    def _major_tick_kw(self):
        """The keyword state of the major ticks (``tick1On``, ``tick2On``,
        ``label1On``, ``label2On``, ``gridOn``)."""
        return self._tick_kw(minor=False)

    @property
    def _minor_tick_kw(self):
        """The keyword state of the minor ticks (see `_major_tick_kw`)."""
        return self._tick_kw(minor=True)

    def _tick_kw(self, minor):
        """The keyword state of the major or *minor* ticks."""
        keys = ("tick1On", "tick2On", "label1On", "label2On", "gridOn")
        return dict(zip(keys, _axis.tick_switches(
            self.axes._native, self.axis_name, minor)))

    def get_label_text(self):
        """Get the text of the label."""
        if self.axis_name == "x":
            return self.axes.get_xlabel()
        return self.axes.get_ylabel()

    def set_major_formatter(self, formatter):
        """
        Set the formatter of the major ticker: a `.ticker.Formatter`, or a
        function ``f(x, pos)`` (made a `.ticker.FuncFormatter`).

        pltrs (Rust) calls the formatter (any callable taking ``(x, pos)``) each
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

    def set_label_position(self, position):
        """
        Set the label position (top or bottom)

        Parameters
        ----------
        position : {'top', 'bottom'}
        """
        _api.check_in_list(['top', 'bottom'], position=position)
        _axis.set_label_position(self.axes._native, self.axis_name, position)

    def set_ticks_position(self, position):
        """
        Set the ticks position.

        Parameters
        ----------
        position : {'top', 'bottom', 'both', 'default', 'none'}
            'both' sets the ticks to appear on both positions, but does not
            change the tick labels.  'default' resets the tick positions to
            the default: ticks on both positions, labels at bottom.  'none'
            can be used if you don't want any ticks. 'none' and 'both'
            affect only the ticks, not the labels.
        """
        _api.check_in_list(['top', 'bottom', 'both', 'default', 'none'],
                           position=position)
        self._set_ticks_position(position)

    def tick_top(self):
        """
        Move ticks and ticklabels (if present) to the top of the Axes.
        """
        self.set_ticks_position('top')

    def tick_bottom(self):
        """
        Move ticks and ticklabels (if present) to the bottom of the Axes.
        """
        self.set_ticks_position('bottom')


class YAxis(Axis):
    """The y-axis of an Axes."""

    axis_name = "y"

    def set_label_position(self, position):
        """
        Set the label position (left or right)

        Parameters
        ----------
        position : {'left', 'right'}
        """
        _api.check_in_list(['left', 'right'], position=position)
        _axis.set_label_position(self.axes._native, self.axis_name, position)

    def set_ticks_position(self, position):
        """
        Set the ticks position.

        Parameters
        ----------
        position : {'left', 'right', 'both', 'default', 'none'}
            'both' sets the ticks to appear on both positions, but does not
            change the tick labels.  'default' resets the tick positions to
            the default: ticks on both positions, labels at left.  'none'
            can be used if you don't want any ticks. 'none' and 'both'
            affect only the ticks, not the labels.
        """
        _api.check_in_list(['left', 'right', 'both', 'default', 'none'],
                           position=position)
        self._set_ticks_position(position)

    def tick_right(self):
        """
        Move ticks and ticklabels (if present) to the right of the Axes.
        """
        self.set_ticks_position('right')

    def tick_left(self):
        """
        Move ticks and ticklabels (if present) to the left of the Axes.
        """
        self.set_ticks_position('left')
