"""
How an Axes takes part in interactive navigation (the navigation methods
of matplotlib's ``axes._base``): whether the toolbar acts on it, panning
and zooming it, and the coordinates a status bar shows.

pltrs (Rust) pans and zooms (``_native.navigation``); a pan in progress
is the native state the axes keeps as ``_pan_start``, as matplotlib keeps
its frozen limits and transforms there.
"""

from numbers import Integral

import numpy as np

from .. import _api, _handles
from .._native import navigation as _navigation


def _button_number(button):
    """matplotlib's number of the mouse button *button*, or None."""
    return int(button) if isinstance(button, Integral) else None


class _TwinnedAxes:
    """
    The groups of twinned axes (matplotlib's ``_twinned_axes``, a
    `.cbook.Grouper`), as pltrs (Rust) keeps them.
    """

    def get_siblings(self, a):
        """Return all of the items joined with *a*, including itself."""
        return _handles.wrap_all(_navigation.twins(a._native), type(a),
                                 a.figure)

    def joined(self, a, b):
        """Return whether *a* and *b* are members of the same set."""
        return a.figure is b.figure and b in self.get_siblings(a)


class NavigationMixin:
    """The navigation methods of `~.axes.Axes`."""

    _twinned_axes = _TwinnedAxes()

    def remove(self):
        """
        Remove the Axes from its figure, whose Axes observers learn it, and
        release the mouse if the Axes grabs it.
        """
        figure = self.get_figure(root=False)
        super().remove()
        if figure is not None:
            figure._axes_removed(self)

    def _different_canvas(self, event):
        """Whether *event* happened on another canvas than the Axes'."""
        return (getattr(event, "canvas", None) is not None
                and (fig := self.get_figure(root=True)) is not None
                and event.canvas is not fig.canvas)

    def contains(self, mouseevent):
        """
        Test whether the mouse event occurred in the Axes patch.

        Returns
        -------
        bool, dict
        """
        if self._different_canvas(mouseevent) or mouseevent.x is None:
            return False, {}
        return self.contains_point((mouseevent.x, mouseevent.y)), {}

    def contains_point(self, point):
        """
        Return whether *point* (pair of pixel coordinates) is inside the Axes
        patch.
        """
        x, y = point
        return _navigation.contains_point(self._native, float(x), float(y))

    def in_axes(self, mouseevent):
        """
        Return whether the given event (in display coords) is in the Axes.
        """
        return self.contains(mouseevent)[0]

    def can_zoom(self):
        """
        Return whether this Axes supports the zoom box button functionality.
        """
        return _navigation.can_zoom(self._native)

    def can_pan(self):
        """
        Return whether this Axes supports any pan/zoom button functionality.
        """
        return _navigation.can_pan(self._native)

    def get_navigate(self):
        """Get whether the Axes responds to navigation commands."""
        return _navigation.get_navigate(self._native)

    def set_navigate(self, b):
        """
        Set whether the Axes responds to navigation toolbar commands.

        Parameters
        ----------
        b : bool
        """
        _navigation.set_navigate(self._native, bool(b))

    def get_navigate_mode(self):
        """
        Get the navigation toolbar button status: 'PAN', 'ZOOM', or None.
        """
        canvas = self.get_figure(root=True).canvas
        toolbar = canvas.toolbar
        if toolbar:
            return None if toolbar.mode.name == "NONE" else toolbar.mode.name
        manager = canvas.manager
        if manager and manager.toolmanager:
            mode = manager.toolmanager.active_toggle.get("default")
            return None if mode is None else mode.upper()

    @_api.deprecated("3.11")
    def set_navigate_mode(self, b):
        """
        Set the navigation toolbar button status.

        .. warning::
            This is not a user-API function.
        """

    def set_forward_navigation_events(self, forward):
        """
        Set how pan/zoom events are forwarded to Axes below this one.

        Parameters
        ----------
        forward : bool or "auto"
            Possible values:

            - True: Forward events to other axes with lower or equal zorder.
            - False: Events are only executed on this axes.
            - "auto": Default behaviour (*True* for axes with an invisible
              patch and *False* otherwise)
        """
        _navigation.set_forward_navigation_events(
            self._native, None if forward == "auto" else bool(forward))

    def get_forward_navigation_events(self):
        """Get how pan/zoom events are forwarded to Axes below this one."""
        forward = _navigation.get_forward_navigation_events(self._native)
        return "auto" if forward is None else forward

    def _get_view(self):
        """
        Save information required to reproduce the current view: the view
        limits and the autoscaling state, for `_set_view`.
        """
        return {
            "xlim": self.get_xlim(), "autoscalex_on": self.get_autoscalex_on(),
            "ylim": self.get_ylim(), "autoscaley_on": self.get_autoscaley_on(),
        }

    def _set_view(self, view):
        """Apply a previously saved view (the value of `_get_view`)."""
        self.set(**view)

    def _prepare_view_from_bbox(self, bbox, direction='in',
                                mode=None, twinx=False, twiny=False):
        """
        Helper function to prepare the new bounds from a bbox.

        This helper function returns the new x and y bounds from the zoom
        bbox. This a convenience method to abstract the bbox logic
        out of the base setter.
        """
        if len(bbox) not in (3, 4):
            _api.warn_external(
                "Warning in _set_view_from_bbox: bounding box is not a tuple "
                "of length 3 or 4. Ignoring the view change.")
            return
        return _navigation.zoomed_bounds(
            self._native, [float(v) for v in bbox], direction, mode,
            bool(twinx), bool(twiny))

    def _set_view_from_bbox(self, bbox, direction='in',
                            mode=None, twinx=False, twiny=False):
        """
        Update view from a selection bbox.

        Parameters
        ----------
        bbox : 4-tuple or 3 tuple
            * If bbox is a 4 tuple, it is the selected bounding box limits,
              in *display* coordinates.
            * If bbox is a 3 tuple, it is an (xp, yp, scl) triple, where
              (xp, yp) is the center of zooming and scl the scale factor to
              zoom by.

        direction : str
            The direction to apply the bounding box: `'in'` (the box becomes
            the view) or `'out'` (the view shrinks into the box).

        mode : str or None
            The selection mode, whether to apply the bounding box in only the
            `'x'` direction, `'y'` direction or both (`None`).

        twinx : bool
            Whether this axis is twinned in the *x*-direction.

        twiny : bool
            Whether this axis is twinned in the *y*-direction.
        """
        if len(bbox) not in (3, 4):
            self._prepare_view_from_bbox(bbox)
            return
        _navigation.zoom_view(
            self._native, [float(v) for v in bbox], direction, mode,
            bool(twinx), bool(twiny))

    def start_pan(self, x, y, button):
        """
        Called when a pan operation has started.

        Parameters
        ----------
        x, y : float
            The mouse coordinates in display coords.
        button : `.MouseButton`
            The pressed mouse button.
        """
        self._pan_start = _navigation.start_pan(
            self._native, float(x), float(y), _button_number(button))

    def end_pan(self):
        """
        Called when a pan operation completes (when the mouse button is up.)
        """
        if self._pan_start is not None:
            _navigation.end_pan(self._pan_start, self._native)
        del self._pan_start

    def _get_pan_points(self, button, key, x, y):
        """
        Helper function to return the new points after a pan: the 2x2
        array ``[[x0, y0], [x1, y1]]`` of the new limits, None where a
        limit would not be finite; None when nothing moves.
        """
        if self._pan_start is None:
            return None
        limits = _navigation.pan_limits(
            self._pan_start, self._native, _button_number(button), key,
            float(x), float(y))
        if limits is None:
            return None
        points = np.empty((2, 2), dtype=object)
        points[:, 0], points[:, 1] = limits
        return points

    def drag_pan(self, button, key, x, y):
        """
        Called when the mouse moves during a pan operation.

        Parameters
        ----------
        button : `.MouseButton`
            The pressed mouse button.
        key : str or None
            The pressed key, if any.
        x, y : float
            The mouse coordinates in display coords.
        """
        if self._pan_start is not None:
            _navigation.drag_pan(self._pan_start, self._native,
                                 _button_number(button), key,
                                 float(x), float(y))

    @property
    def fmt_xdata(self):
        """The function formatting x coordinates for the status bar, or
        None for the x-axis' major formatter."""
        return self.__dict__.get("_fmt_xdata")

    @fmt_xdata.setter
    def fmt_xdata(self, func):
        self._fmt_xdata = func
        _navigation.set_fmt_data(self._native, "x", func)

    @property
    def fmt_ydata(self):
        """The function formatting y coordinates for the status bar, or
        None for the y-axis' major formatter."""
        return self.__dict__.get("_fmt_ydata")

    @fmt_ydata.setter
    def fmt_ydata(self, func):
        self._fmt_ydata = func
        _navigation.set_fmt_data(self._native, "y", func)

    def format_xdata(self, x):
        """
        Return *x* formatted as an x-value.

        This function will use the `!fmt_xdata` attribute if it is not None,
        else will fall back on the xaxis major formatter.
        """
        if self.fmt_xdata is not None:
            return self.fmt_xdata(x)
        return _navigation.format_data(self._native, "x", float(x)) or ""

    def format_ydata(self, y):
        """
        Return *y* formatted as a y-value.

        This function will use the `!fmt_ydata` attribute if it is not None,
        else will fall back on the yaxis major formatter.
        """
        if self.fmt_ydata is not None:
            return self.fmt_ydata(y)
        return _navigation.format_data(self._native, "y", float(y)) or ""

    @property
    def _mouseover_set(self):
        """The artists of the Axes whose data a status bar shows when the
        mouse is over them (matplotlib's ordered set of them)."""
        return tuple(a for a in self._children if a.get_mouseover())

    def _cursor_text(self, event):
        """The data of the topmost visible artist with mouseover set under
        the mouse *event*, as a status bar shows it, or None."""
        if event.x is None:
            return None
        return _navigation.cursor_text(self._native, float(event.x),
                                       float(event.y))

    def format_coord(self, x, y):
        """Return a format string formatting the *x*, *y* coordinates."""
        if x is None or y is None:
            return "(x, y) = ({}, {})".format(
                "???" if x is None else self.format_xdata(x),
                "???" if y is None else self.format_ydata(y))
        return _navigation.format_coord(self._native, float(x), float(y))
