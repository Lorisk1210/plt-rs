"""
How an Axes takes part in interactive navigation (the navigation methods
of matplotlib's ``axes._base``): whether the toolbar acts on it, panning
and zooming it, and the coordinates a status bar shows.

pltrs (Rust) pans and zooms (``_native.navigation``); a pan in progress
keeps the native state of the axes as ``_pan_start``, as matplotlib keeps
its frozen limits and transforms there.
"""

from numbers import Real
from operator import attrgetter

import numpy as np

from .. import _api, _handles, _values
from .._native import navigation as _navigation
from .. import _lazy_imports

_foreign_children = _lazy_imports.module(globals(), "._foreign_children")


def _button_number(button):
    """
    The number of the mouse button *button* as matplotlib's pan methods
    tell buttons apart: 1 (left) and 3 (right) by equality (``1.0`` and
    ``MouseButton.LEFT`` are 1), and 2 for any other value, which neither
    pans nor zooms.
    """
    return 1 if button == 1 else 3 if button == 3 else 2


def _display_point(x, y):
    """The display point ``(x, y)`` as floats, which numpy makes of the
    values matplotlib's transforms are given (None is NaN, ``'60'`` is
    60)."""
    return tuple(np.asarray((x, y), dtype=float).tolist())


class _PanStart:
    """
    A pan in progress (matplotlib's ``_pan_start``): the native state of
    the axes when the button went down (None for an axes not laid out),
    and the coordinates the pan started at as they were given, which
    matplotlib keeps unchecked and subtracts from the pointer's as the
    pointer moves.
    """

    def __init__(self, native, x, y):
        self.native = native
        self.x = x
        self.y = y

    def moved_to(self, x, y):
        """Fail as matplotlib's rectilinear pan fails to measure the
        pointer's motion to ``(x, y)`` (``x - p.x``)."""
        x - self.x  # noqa: B018
        y - self.y  # noqa: B018


class _TwinnedAxes:
    """
    The groups of twinned axes (matplotlib's ``_twinned_axes``, a
    `.cbook.Grouper`), as pltrs (Rust) keeps them.
    """

    def get_siblings(self, a):
        """Return all of the items joined with *a*, including itself."""
        return _handles.wrap_all(_navigation.twins(a._native), type(a),
                                 a._figure)

    def joined(self, a, b):
        """Return whether *a* and *b* are members of the same set."""
        return a.figure is b.figure and b in self.get_siblings(a)

    def join(self, a, *args):
        """
        Join given arguments into the same set.  Accepts one or more arguments.
        """
        for b in args:
            _navigation.join_twins(a._native, b._native)


class NavigationMixin:
    """The navigation methods of `~.axes.Axes`."""

    _twinned_axes = _TwinnedAxes()

    def remove(self):
        """
        Remove the Axes from its figure, whose Axes observers learn it, and
        release the mouse if the Axes grabs it.
        """
        if _foreign_children.relisted(self):
            # Listed among the children of an Axes (its `add_artist`), it
            # is taken out of them, and decoupled from that Axes.
            super().remove()
            self._axes = None
            return
        self._check_removable()
        if "_child_of" in self.__dict__:
            self._leave_host()
        axes = self.axes
        figure = self.get_figure(root=False)
        self._keep_trees()
        super().remove()
        if figure is not None:
            figure._axes_removed(self)
        # As matplotlib's `Artist.remove` marks the Axes of what it removes
        # stale and decouples it from it: an Axes is its own, a child Axes'
        # is its parent.
        if axes is not None:
            axes.stale = True
        self._axes = None

    def _leave_host(self):
        """Keep the trees of this child Axes at the place its host gives it
        now, as it leaves the host (removed, or its host cleared): see
        `.._transform_trees._apply_axes`."""
        self._keep_trees()
        self._left_host = True

    def _keep_trees(self):
        """Build the trees of the Axes from its boxes as they are now (its
        position of the last draw), before it leaves its figure or host:
        it measures with them from then on, as matplotlib's keeps its
        trees."""
        if "_transform_trees" not in self.__dict__:
            self._build_transform_trees()

    def _check_removable(self):
        """
        Fail as matplotlib's removal method of the Axes fails: a child
        Axes' takes it out of the list its parent had of them when it was
        added (`add_child_axes`; clearing the parent makes a new one), any
        other Axes' is its figure's `delaxes`, which refuses an Axes its
        figure no longer has.
        """
        if "_child_of" in self.__dict__:
            if self.__dict__.get("_removed"):
                raise ValueError("list.remove(x): x not in list")
            return
        figure = self.get_figure(root=True)
        if figure is None or self not in figure.axes:
            raise KeyError(self)

    def _different_canvas(self, event):
        """Whether *event* happened on another canvas than the Axes'."""
        return (getattr(event, "canvas", None) is not None
                and (fig := self.get_figure(root=True)) is not None
                and event.canvas is not fig.canvas)

    def contains(self, mouseevent):
        if self._different_canvas(mouseevent) or mouseevent.x is None:
            return False, {}
        x, y = mouseevent.x, mouseevent.y
        if not all(isinstance(value, Real) for value in (x, y)):
            # matplotlib's patch hit-tests what are no numbers with its
            # path (``point_in_path``, which refuses them).
            return self.contains_point((x, y)), {}
        # pltrs (Rust) tests the patch's outline as matplotlib's
        # ``patch.contains`` does, each of its sub-paths on its own.
        return _navigation.contains_point(self._native, float(x), float(y)), {}

    def contains_point(self, point):
        """
        Return whether *point* (pair of pixel coordinates) is inside the Axes
        patch.
        """
        # matplotlib's ``patch.contains_point(point, radius=1.0)``: the
        # patch's path as a whole, one pixel around it included.
        patch = self.patch
        return patch.get_path().contains_point(point, patch.get_transform(), 1.0)

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
        """
        Get whether the Axes responds to navigation commands.
        """
        # matplotlib keeps the object it was given (``1``, ``'False'``).
        return _values.given_flag(self, "navigate", _navigation.get_navigate(self._native))

    def set_navigate(self, b):
        """
        Set whether the Axes responds to navigation toolbar commands.

        Parameters
        ----------
        b : bool

        See Also
        --------
        matplotlib.axes.Axes.set_forward_navigation_events

        """
        _navigation.set_navigate(self._native, bool(b))
        _values.remember(self, "navigate", b)

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

        See Also
        --------
        matplotlib.axes.Axes.set_navigate

        """
        _navigation.set_forward_navigation_events(
            self._native, None if forward == "auto" else bool(forward))
        _values.remember(self, "forward_navigation_events", forward)

    def get_forward_navigation_events(self):
        """Get how pan/zoom events are forwarded to Axes below this one."""
        forward = _navigation.get_forward_navigation_events(self._native)
        if forward is None:
            return "auto"
        # matplotlib keeps the object it was given (``1``, ``''``).
        return _values.given_flag(self, "forward_navigation_events", forward)

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

        Notes
        -----
        This is intended to be overridden by new projection types.
        """
        self._pan_start = _PanStart(
            _navigation.start_pan(self._native, *_display_point(x, y),
                                  _button_number(button)), x, y)

    def end_pan(self):
        """
        Called when a pan operation completes (when the mouse button is up.)

        Notes
        -----
        This is intended to be overridden by new projection types.
        """
        if self._pan_start.native is not None:
            _navigation.end_pan(self._pan_start.native, self._native)
        del self._pan_start

    def _get_pan_points(self, button, key, x, y):
        """
        Helper function to return the new points after a pan: the 2x2
        array ``[[x0, y0], [x1, y1]]`` of the new limits, None where a
        limit would not be finite; None when nothing moves.
        """
        start = self._pan_start
        start.moved_to(x, y)
        if start.native is None:
            return None
        limits = _navigation.pan_limits(
            start.native, self._native, _button_number(button), key,
            *_display_point(x, y))
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

        Notes
        -----
        This is intended to be overridden by new projection types.
        """
        start = self._pan_start
        if self.name != "polar":
            # matplotlib's polar Axes compute with the pointer only in the
            # mode the press chose.
            start.moved_to(x, y)
        if start.native is not None:
            _navigation.drag_pan(start.native, self._native,
                                 _button_number(button), key,
                                 *_display_point(x, y))

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
        return (self.fmt_xdata if self.fmt_xdata is not None
                else self.xaxis.get_major_formatter().format_data_short)(x)

    def format_ydata(self, y):
        """
        Return *y* formatted as a y-value.

        This function will use the `!fmt_ydata` attribute if it is not None,
        else will fall back on the yaxis major formatter.
        """
        return (self.fmt_ydata if self.fmt_ydata is not None
                else self.yaxis.get_major_formatter().format_data_short)(y)

    @property
    def _mouseover_set(self):
        """The artists of the Axes whose data a status bar shows when the
        mouse is over them (matplotlib's ordered set of them)."""
        return tuple(a for a in self._children_now() if a.get_mouseover())

    def _cursor_text(self, event):
        """The data of the topmost visible artist with mouseover set under
        the mouse *event*, as a status bar shows it, or None."""
        if event.x is None:
            return None
        return _navigation.cursor_text(self._native, float(event.x),
                                       float(event.y))

    def format_coord(self, x, y):
        """Return a format string formatting the *x*, *y* coordinates."""
        twins = self._twinned_axes.get_siblings(self)
        if len(twins) == 1:
            return "(x, y) = ({}, {})".format(
                "???" if x is None else self.format_xdata(x),
                "???" if y is None else self.format_ydata(y))
        screen_xy = self.transData.transform((x, y))
        xy_strs = []
        # Retrieve twins in the order of self.figure.axes to sort tied zorders (which is
        # the common case) by the order in which they are added to the figure.
        for ax in sorted(twins, key=attrgetter("zorder")):
            data_x, data_y = ax.transData.inverted().transform(screen_xy)
            xy_strs.append(
                "({}, {})".format(ax.format_xdata(data_x), ax.format_ydata(data_y)))
        return "(x, y) = {}".format(" | ".join(xy_strs))
