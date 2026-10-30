"""
The view bounds of an Axes and how it autoscales (matplotlib's
``get_xbound``/``set_xbound``, ``get_autoscalex_on``, ``autoscale``,
``margins``, ``relim``, ``use_sticky_edges``): pltrs (Rust) keeps them
(``_native.axes_base``).
"""

import numpy as np

from .. import _api
from .._native import axes_base as _axes_base


def _check_margin(m):
    """matplotlib's check of a margin."""
    if m <= -0.5:
        raise ValueError("margin must be greater than -0.5")
    return float(m)


class LimitsMixin:
    """The bound and autoscaling methods of `~.axes.Axes`."""

    def _set_bound(self, name, lower, upper):
        if upper is None and np.iterable(lower):
            lower, upper = lower
        _axes_base.set_bound(self._native, name,
                             None if lower is None else float(lower),
                             None if upper is None else float(upper))

    def get_xbound(self):
        """
        Return the lower and upper x-axis bounds, in increasing order.
        """
        return _axes_base.get_bound(self._native, "x")

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
        """
        self._set_bound("x", lower, upper)

    def get_ybound(self):
        """
        Return the lower and upper y-axis bounds, in increasing order.
        """
        return _axes_base.get_bound(self._native, "y")

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
        if not (scalex and scaley):
            raise NotImplementedError(
                "pltrs: autoscale_view(scalex=False or scaley=False)")
        _axes_base.autoscale_view(self._native,
                                  None if tight is None else bool(tight))

    def get_xmargin(self):
        """Retrieve autoscaling margin of the x-axis."""
        return _axes_base.get_margin(self._native, "x")

    def get_ymargin(self):
        """Retrieve autoscaling margin of the y-axis."""
        return _axes_base.get_margin(self._native, "y")

    def set_xmargin(self, m):
        """
        Set padding of X data limits prior to autoscaling.

        Parameters
        ----------
        m : float greater than -0.5
        """
        _axes_base.set_margins(self._native, _check_margin(m), None)

    def set_ymargin(self, m):
        """
        Set padding of Y data limits prior to autoscaling.

        Parameters
        ----------
        m : float greater than -0.5
        """
        _axes_base.set_margins(self._native, None, _check_margin(m))

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
        if tight is not True:
            raise NotImplementedError(f"pltrs: margins(tight={tight!r})")
        _axes_base.set_margins(self._native,
                               None if x is None else _check_margin(x),
                               None if y is None else _check_margin(y))

    def relim(self, visible_only=False):
        """
        Recompute the data limits based on current artists.

        Parameters
        ----------
        visible_only : bool, default: False
            Whether to exclude invisible artists.
        """
        _axes_base.relim(self._native, bool(visible_only))

    @property
    def use_sticky_edges(self):
        """
        When autoscaling, whether to obey all `.Artist.sticky_edges`.
        """
        return _axes_base.get_use_sticky_edges(self._native)

    @use_sticky_edges.setter
    def use_sticky_edges(self, b):
        _axes_base.set_use_sticky_edges(self._native, bool(b))
