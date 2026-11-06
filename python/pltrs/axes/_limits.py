"""
The view bounds of an Axes and how it autoscales (matplotlib's
``get_xbound``/``set_xbound``, ``get_autoscalex_on``, ``autoscale``,
``margins``, ``relim``, ``update_datalim``, ``use_sticky_edges``): pltrs
(Rust) keeps them
(``_native.axes_base``).
"""

import numpy as np

from .. import _api, _program_classes, _values
from .._native import axes_base as _axes_base
from ._ticks import _axis_method_wrapper


def _check_margin(m):
    """matplotlib's check of a margin."""
    if m <= -0.5:
        raise ValueError("margin must be greater than -0.5")
    return float(m)


def _limit_args(name, low, high, low_alias, high_alias):
    """matplotlib's normalization of the arguments of ``set_xlim`` and
    ``set_ylim``: a ``(low, high)`` pair, and the ``xmin``/``xmax`` aliases."""
    if high is None and np.iterable(low):
        low, high = low
    for value, alias, alias_name, arg_name in (
            (low, low_alias, f"{name}min", "left" if name == "x" else "bottom"),
            (high, high_alias, f"{name}max", "right" if name == "x" else "top")):
        if alias is not None and value is not None:
            raise TypeError(f"Cannot pass both '{arg_name}' and '{alias_name}'")
    return (low if low_alias is None else low_alias,
            high if high_alias is None else high_alias)



#: The start of pltrs (Python)'s refusal to read a figure pltrs (Rust)
#: is laying out, for the program's code run meanwhile (a transform).
_FIGURE_IN_USE = "this figure is being laid out or changed by pltrs"


def _counted_images(axes):
    """
    The `.AxesImage` objects of *axes*, which matplotlib's `relim` reads
    the extents of; none while pltrs (Rust) lays the figure out, when the
    program's code it runs (a transform calling `relim`) cannot read the
    figure, and only the native recomputation runs.
    """
    from ..image import AxesImage
    try:
        images = list(axes.images)
    except RuntimeError as error:
        if str(error).startswith(_FIGURE_IN_USE):
            return []
        raise
    return [image for image in images if isinstance(image, AxesImage)]

class LimitsMixin:
    """The limit, bound and autoscaling methods of `~.axes.Axes`."""

    def set_xlim(self, left=None, right=None, *, emit=True, auto=False,
                 xmin=None, xmax=None):
        """
        Set the x-axis view limits.

        Parameters
        ----------
        left : float, optional
            The left xlim in data coordinates. Passing *None* leaves the
            limit unchanged.

            The left and right xlims may also be passed as the tuple
            (*left*, *right*) as the first positional argument (or as
            the *left* keyword argument).
        right : float, optional
            The right xlim in data coordinates. Passing *None* leaves the
            limit unchanged.
        emit : bool, default: True
            Whether to notify observers of limit change.
        auto : bool or None, default: False
            Whether to turn on autoscaling of the x-axis. True turns on,
            False turns off, None leaves unchanged.
        xmin, xmax : float, optional
            They are equivalent to left and right respectively, and it is an
            error to pass both *xmin* and *left* or *xmax* and *right*.

        Returns
        -------
        left, right : (float, float)
            The new x-axis limits in data coordinates.

        ACCEPTS: (left: float, right: float)
        """
        left, right = _limit_args("x", left, right, xmin, xmax)
        return self.xaxis._set_lim(left, right, emit=emit, auto=auto)

    def set_ylim(self, bottom=None, top=None, *, emit=True, auto=False,
                 ymin=None, ymax=None):
        """
        Set the y-axis view limits.

        Parameters
        ----------
        bottom : float, optional
            The bottom ylim in data coordinates. Passing *None* leaves the
            limit unchanged.

            The bottom and top ylims may also be passed as the tuple
            (*bottom*, *top*) as the first positional argument (or as
            the *bottom* keyword argument).
        top : float, optional
            The top ylim in data coordinates. Passing *None* leaves the
            limit unchanged.
        emit : bool, default: True
            Whether to notify observers of limit change.
        auto : bool or None, default: False
            Whether to turn on autoscaling of the y-axis. *True* turns on,
            *False* turns off, *None* leaves unchanged.
        ymin, ymax : float, optional
            They are equivalent to bottom and top respectively, and it is an
            error to pass both *ymin* and *bottom* or *ymax* and *top*.

        Returns
        -------
        bottom, top : (float, float)
            The new y-axis limits in data coordinates.

        ACCEPTS: (bottom: float, top: float)
        """
        bottom, top = _limit_args("y", bottom, top, ymin, ymax)
        return self.yaxis._set_lim(bottom, top, emit=emit, auto=auto)

    def _set_bound(self, name, lower, upper):
        if upper is None and np.iterable(lower):
            lower, upper = lower
        axis = getattr(self, f"{name}axis")
        if _program_classes.takes_from_program(type(axis), "get_view_interval"):
            # matplotlib's code, which asks the program's class of the axis
            # whether it is inverted.
            old_lower, old_upper = getattr(self, f"get_{name}bound")()
            bounds = (old_lower if lower is None else lower,
                      old_upper if upper is None else upper)
            getattr(self, f"set_{name}lim")(
                sorted(bounds, reverse=bool(axis.get_inverted())), auto=None)
            return
        _axes_base.set_bound(self._native, name,
                             None if lower is None else float(lower),
                             None if upper is None else float(upper))

    def get_xbound(self):
        """
        Return the lower and upper x-axis bounds, in increasing order.
        """
        return _values.float64s(_axes_base.get_bound(self._native, "x"))

    def set_xbound(self, lower=None, upper=None):
        """
        Set the lower and upper numerical bounds of the x-axis.

        This method will honor axis inversion regardless of parameter order.
        It will not change the autoscaling setting (`.get_autoscalex_on()`).

        Parameters
        ----------
        lower, upper : float or None
            The lower and upper bounds. If *None*, the respective axis bound
            is not modified.

        ACCEPTS: (lower: float, upper: float)
        """
        self._set_bound("x", lower, upper)

    def get_ybound(self):
        """
        Return the lower and upper y-axis bounds, in increasing order.
        """
        return _values.float64s(_axes_base.get_bound(self._native, "y"))

    def set_ybound(self, lower=None, upper=None):
        """
        Set the lower and upper numerical bounds of the y-axis.

        This method will honor axis inversion regardless of parameter order.
        It will not change the autoscaling setting (`.get_autoscaley_on()`).

        Parameters
        ----------
        lower, upper : float or None
            The lower and upper bounds. If *None*, the respective axis bound
            is not modified.

        ACCEPTS: (lower: float, upper: float)
        """
        self._set_bound("y", lower, upper)

    def get_autoscale_on(self):
        """Return True if each axis is autoscaled, False otherwise."""
        return _axes_base.get_autoscale_on(self._native, None)

    def set_autoscale_on(self, b):
        """
        Set whether autoscaling is applied to each axis on the next draw or
        call to `.Axes.autoscale_view`.

        Parameters
        ----------
        b : bool
        """
        _axes_base.set_autoscale_on(self._native, None, bool(b))

    def get_autoscalex_on(self):
        """Return whether the xaxis is autoscaled."""
        return _axes_base.get_autoscale_on(self._native, "x")

    def set_autoscalex_on(self, b):
        """
        Set whether the xaxis is autoscaled when drawing or by
        `.Axes.autoscale_view`.

        Parameters
        ----------
        b : bool
        """
        _axes_base.set_autoscale_on(self._native, "x", bool(b))

    def get_autoscaley_on(self):
        """Return whether the yaxis is autoscaled."""
        return _axes_base.get_autoscale_on(self._native, "y")

    def set_autoscaley_on(self, b):
        """
        Set whether the yaxis is autoscaled when drawing or by
        `.Axes.autoscale_view`.

        Parameters
        ----------
        b : bool
        """
        _axes_base.set_autoscale_on(self._native, "y", bool(b))

    def autoscale(self, enable=True, axis='both', tight=None):
        """
        Autoscale the axis view to the data (toggle).

        Convenience method for simple axis view autoscaling.
        It turns autoscaling on or off, and then,
        if autoscaling for either axis is on, it performs
        the autoscaling on the specified axis or Axes.

        Parameters
        ----------
        enable : bool or None, default: True
            True turns autoscaling on, False turns it off.
            None leaves the autoscaling state unchanged.
        axis : {'both', 'x', 'y'}, default: 'both'
            The axis on which to operate.
        tight : bool or None, default: None
            If True, first set the margins to zero.  Then, this argument is
            forwarded to `~.axes.Axes.autoscale_view` (regardless of
            its value); see the description of its behavior there.
        """
        _api.check_in_list(['both', 'x', 'y'], axis=axis)
        _axes_base.autoscale(self._native,
                             None if enable is None else bool(enable),
                             None if axis == 'both' else axis,
                             None if tight is None else bool(tight))
        if tight:
            # The margins zeroed, which matplotlib zeroes with the int 0.
            if enable is None:
                scalex = scaley = True
            else:
                scalex = axis in ('x', 'both') and self.get_autoscalex_on()
                scaley = axis in ('y', 'both') and self.get_autoscaley_on()
            for name, scaled in (("xmargin", scalex), ("ymargin", scaley)):
                if scaled:
                    _values.remember(self, name, 0)

    def autoscale_view(self, tight=None, scalex=True, scaley=True):
        """
        Autoscale the view limits using the data limits.

        Parameters
        ----------
        tight : bool or None
            If *True*, only expand the axis limits using the margins.  If
            *False*, also expand the axis limits to the nearest "round"
            values given by the locator. If None, keep the previous setting.
        scalex : bool, default: True
            Whether to autoscale the x-axis.
        scaley : bool, default: True
            Whether to autoscale the y-axis.
        """
        _axes_base.autoscale_view(self._native,
                                  None if tight is None else bool(tight),
                                  bool(scalex), bool(scaley))

    def get_xmargin(self):
        """Retrieve autoscaling margin of the x-axis."""
        # As given, as matplotlib keeps it.
        return _values.given(self, "xmargin", _axes_base.get_margin(self._native, "x"))

    def get_ymargin(self):
        """Retrieve autoscaling margin of the y-axis."""
        return _values.given(self, "ymargin", _axes_base.get_margin(self._native, "y"))

    @property
    def _xmargin(self):
        return self.get_xmargin()

    @property
    def _ymargin(self):
        return self.get_ymargin()

    def set_xmargin(self, m):
        """
        Set padding of X data limits prior to autoscaling.

        Parameters
        ----------
        m : float greater than -0.5
        """
        _axes_base.set_margins(self._native, _check_margin(m), None, None)
        _values.remember(self, "xmargin", m)

    def set_ymargin(self, m):
        """
        Set padding of Y data limits prior to autoscaling.

        Parameters
        ----------
        m : float greater than -0.5
        """
        _axes_base.set_margins(self._native, None, _check_margin(m), None)
        _values.remember(self, "ymargin", m)

    def margins(self, *margins, x=None, y=None, tight=True):
        """
        Set or retrieve margins around the data for autoscaling axis limits.

        Call signatures::

            margins()                # get existing margins
            margins(margin)          # set both margins to the same value
            margins(xmargin, ymargin)
            margins(x=xmargin, y=ymargin)

        Parameters
        ----------
        *margins : float, optional
            If a single positional argument is provided, it specifies
            both margins of the x-axis and y-axis limits. If two
            positional arguments are provided, they will be interpreted
            as *xmargin*, *ymargin*.
        x, y : float, optional
            Specific margin values for the x-axis and y-axis,
            respectively.
        tight : bool or None, default: True
            The *tight* parameter is passed to `~.axes.Axes.autoscale_view`.

        Returns
        -------
        xmargin, ymargin : float
        """
        if margins and (x is not None or y is not None):
            raise TypeError('Cannot pass both positional and keyword '
                            'arguments for x and/or y.')
        elif len(margins) == 1:
            x = y = margins[0]
        elif len(margins) == 2:
            x, y = margins
        elif margins:
            raise TypeError('Must pass a single positional argument for all '
                            'margins, or one for each margin (x, y).')

        if x is None and y is None:
            if tight is not True:
                _api.warn_external(f'ignoring tight={tight!r} in get mode')
            return self.get_xmargin(), self.get_ymargin()
        _axes_base.set_margins(self._native,
                               None if x is None else _check_margin(x),
                               None if y is None else _check_margin(y),
                               None if tight is None else bool(tight))
        for name, m in (("xmargin", x), ("ymargin", y)):
            if m is not None:
                _values.remember(self, name, m)

    def relim(self, visible_only=False):
        """
        Recompute the data limits based on current artists.

        Parameters
        ----------
        visible_only : bool, default: False
            Whether to exclude invisible artists.
        """
        for image in _counted_images(self):
            if not visible_only or image.get_visible():
                # matplotlib reads the extent of each image it counts,
                # which one without data refuses.
                image.get_extent()
        _axes_base.relim(self._native, bool(visible_only))

    def has_data(self):
        """
        Return whether any artists have been added to the Axes.

        This should not be used to determine whether the *dataLim*
        need to be updated, and may not actually be useful for
        anything.
        """
        return _axes_base.has_data(self._native)

    def update_datalim(self, xys, updatex=True, updatey=True):
        """
        Extend the `~.Axes.dataLim` Bbox to include the given points.

        If no data is set currently, the Bbox will ignore its limits and set
        the bound to be the bounds of the xydata (*xys*). Otherwise, it will
        compute the bounds of the union of its current data and the data in
        *xys*.

        Parameters
        ----------
        xys : 2D array-like
            The points to include in the data limits Bbox. This can be either
            a list of (x, y) tuples or a (N, 2) array.

        updatex, updatey : bool, default: True
            Whether to update the x/y limits.
        """
        xys = np.asarray(xys, dtype=float)
        if not np.any(np.isfinite(xys)):
            return
        _axes_base.update_datalim(
            self._native, xys.reshape(-1, 2), bool(updatex), bool(updatey))

    set_xinverted = _axis_method_wrapper("xaxis", "set_inverted")
    get_xinverted = _axis_method_wrapper("xaxis", "get_inverted")
    set_yinverted = _axis_method_wrapper("yaxis", "set_inverted")
    get_yinverted = _axis_method_wrapper("yaxis", "get_inverted")

    @property
    def use_sticky_edges(self):
        """
        When autoscaling, whether to obey all `.Artist.sticky_edges`.
        """
        return _axes_base.get_use_sticky_edges(self._native)

    @use_sticky_edges.setter
    def use_sticky_edges(self, b):
        _axes_base.set_use_sticky_edges(self._native, bool(b))

    @property
    def ignore_existing_data_limits(self):
        """
        Whether the next data added replaces the data limits rather than
        extending them: True once the Axes is cleared or its limits are
        recomputed (`relim`), until data extends them.
        """
        return _axes_base.data_limits_empty(self._native)
