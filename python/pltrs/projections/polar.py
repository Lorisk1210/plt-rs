"""
Polar axes (matplotlib's ``projections.polar``): pltrs (Rust) axes in the
polar projection, made by ``add_subplot(projection='polar')``.

pltrs (Rust) projects, lays out and draws the axes and keeps their
settings (``_native.polar``). The tick helpers `ThetaFormatter`,
`ThetaLocator` and `RadialLocator`, which matplotlib code uses on their
own, are matplotlib's (from matplotlib 3.11.2, under the Matplotlib
License).
"""

import numpy as np

from .. import _api, ticker as mticker, transforms as mtransforms
from .._native import polar as _polar
from ..axes import Axes


def _is_full_circle_deg(thetamin, thetamax):
    """
    Determine if a wedge (in degrees) spans the full circle.

    The condition is derived from :class:`~matplotlib.patches.Wedge`.
    """
    return abs(abs(thetamax - thetamin) - 360.0) < 1e-12


def _is_full_circle_rad(thetamin, thetamax):
    """
    Determine if a wedge (in radians) spans the full circle.

    The condition is derived from :class:`~matplotlib.patches.Wedge`.
    """
    return abs(abs(thetamax - thetamin) - 2 * np.pi) < 1.74e-14


class ThetaFormatter(mticker.Formatter):
    """
    Used to format the *theta* tick labels.  Converts the native
    unit of radians into degrees and adds a degree symbol.
    """

    def __call__(self, x, pos=None):
        vmin, vmax = self.axis.get_view_interval()
        d = np.rad2deg(abs(vmax - vmin))
        digits = max(-int(np.log10(d) - 1.5), 0)
        return f"{np.rad2deg(x):0.{digits}f}\N{DEGREE SIGN}"


class _AxisWrapper:
    def __init__(self, axis):
        self._axis = axis

    def get_view_interval(self):
        return np.rad2deg(self._axis.get_view_interval())

    def set_view_interval(self, vmin, vmax):
        self._axis.set_view_interval(*np.deg2rad((vmin, vmax)))

    def get_minpos(self):
        return np.rad2deg(self._axis.get_minpos())

    def get_data_interval(self):
        return np.rad2deg(self._axis.get_data_interval())

    def set_data_interval(self, vmin, vmax):
        self._axis.set_data_interval(*np.deg2rad((vmin, vmax)))

    def get_tick_space(self):
        return self._axis.get_tick_space()


class ThetaLocator(mticker.Locator):
    """
    Used to locate theta ticks.

    This will work the same as the base locator except in the case that the
    view spans the entire circle. In such cases, the previously used default
    locations of every 45 degrees are returned.
    """

    def __init__(self, base):
        self.base = base
        self.axis = self.base.axis = _AxisWrapper(self.base.axis)

    def set_axis(self, axis):
        self.axis = _AxisWrapper(axis)
        self.base.set_axis(self.axis)

    def __call__(self):
        lim = self.axis.get_view_interval()
        if _is_full_circle_deg(lim[0], lim[1]):
            return np.deg2rad(min(lim)) + np.arange(8) * 2 * np.pi / 8
        else:
            return np.deg2rad(self.base())

    def view_limits(self, vmin, vmax):
        vmin, vmax = np.rad2deg((vmin, vmax))
        return np.deg2rad(self.base.view_limits(vmin, vmax))


class RadialLocator(mticker.Locator):
    """
    Used to locate radius ticks.

    Ensures that all ticks are strictly positive.  For all other tasks, it
    delegates to the base `.Locator` (which may be different depending on the
    scale of the *r*-axis).
    """

    @_api.delete_parameter("3.11", "axes")
    def __init__(self, base, axes=None):
        self.base = base
        self._axes = axes

    def set_axis(self, axis):
        self.base.set_axis(axis)

    def __call__(self):
        # Ensure previous behaviour with full circle non-annular views.
        ax = self.base.axis.axes
        if _is_full_circle_rad(*ax.get_xlim()):
            rorigin = ax.get_rorigin() * ax.get_rsign()
            if ax.get_rmin() <= rorigin:
                return [tick for tick in self.base() if tick > rorigin]
        return self.base()

    def _zero_in_bounds(self):
        """
        Return True if zero is within the valid values for the
        scale of the radial axis.
        """
        vmin, vmax = self.base.axis._scale.limit_range_for_scale(0, 1, 1e-5)
        return vmin == 0

    def nonsingular(self, vmin, vmax):
        # docstring inherited
        if self._zero_in_bounds() and (vmin, vmax) == (-np.inf, np.inf):
            # Initial view limits
            return (0, 1)
        else:
            return self.base.nonsingular(vmin, vmax)

    def view_limits(self, vmin, vmax):
        vmin, vmax = self.base.view_limits(vmin, vmax)
        if self._zero_in_bounds() and vmax > vmin:
            # this allows inverted r/y-lims
            vmin = min(0, vmin)
        return mtransforms._nonsingular(vmin, vmax)


class PolarAxes(Axes):
    """
    A polar graph projection, where the input dimensions are *theta*, *r*.

    Theta starts pointing east and goes anti-clockwise.
    """
    name = 'polar'

    def set_thetamax(self, thetamax):
        """Set the maximum theta limit in degrees."""
        _polar.set_thetamax(self._native, float(thetamax))

    def get_thetamax(self):
        """Return the maximum theta limit in degrees."""
        return _polar.get_thetalim(self._native)[1]

    def set_thetamin(self, thetamin):
        """Set the minimum theta limit in degrees."""
        _polar.set_thetamin(self._native, float(thetamin))

    def get_thetamin(self):
        """Get the minimum theta limit in degrees."""
        return _polar.get_thetalim(self._native)[0]

    def set_thetalim(self, *args, **kwargs):
        r"""
        Set the minimum and maximum theta values.

        Can take the following signatures:

        - ``set_thetalim(minval, maxval)``: Set the limits in radians.
        - ``set_thetalim(thetamin=minval, thetamax=maxval)``: Set the limits
          in degrees.

        where minval and maxval are the minimum and maximum limits. Values are
        wrapped in to the range :math:`[0, 2\pi]` (in radians), so for example
        it is possible to do ``set_thetalim(-np.pi / 2, np.pi / 2)`` to have
        an axis symmetric around 0. A ValueError is raised if the absolute
        angle difference is larger than a full circle.
        """
        orig_lim = self.get_xlim()  # in radians
        if 'thetamin' in kwargs:
            kwargs['left'] = np.deg2rad(kwargs.pop('thetamin'))
        if 'thetamax' in kwargs:
            kwargs['right'] = np.deg2rad(kwargs.pop('thetamax'))
        new_min, new_max = self.set_xlim(*args, **kwargs)
        # Parsing all permutations of *args, **kwargs is tricky; it is simpler
        # to let set_xlim() do it and then validate the limits.
        if abs(new_max - new_min) > 2 * np.pi:
            self.set_xlim(orig_lim)  # un-accept the change
            raise ValueError("The angle range must be less than a full circle")
        return tuple(np.rad2deg((new_min, new_max)))

    def set_theta_offset(self, offset):
        """
        Set the offset for the location of 0 in radians.
        """
        _polar.set_theta_offset(self._native, float(offset))

    def get_theta_offset(self):
        """
        Get the offset for the location of 0 in radians.
        """
        return _polar.get_theta_offset(self._native)

    def set_theta_zero_location(self, loc, offset=0.0):
        """
        Set the location of theta's zero.

        Parameters
        ----------
        loc : str
            May be one of "N", "NW", "W", "SW", "S", "SE", "E", or "NE".
        offset : float, default: 0
            An offset in degrees to apply from the specified *loc*. **Note:**
            this offset is *always* applied counter-clockwise regardless of
            the direction setting.
        """
        _polar.set_theta_zero_location(self._native, loc, float(offset))

    def set_theta_direction(self, direction):
        """
        Set the direction in which theta increases.

        clockwise, -1:
           Theta increases in the clockwise direction

        counterclockwise, anticlockwise, 1:
           Theta increases in the counterclockwise direction
        """
        if direction in ('clockwise', -1):
            _polar.set_theta_direction(self._native, True)
        elif direction in ('counterclockwise', 'anticlockwise', 1):
            _polar.set_theta_direction(self._native, False)
        else:
            _api.check_in_list(
                [-1, 1, 'clockwise', 'counterclockwise', 'anticlockwise'],
                direction=direction)

    def get_theta_direction(self):
        """
        Get the direction in which theta increases.

        -1:
           Theta increases in the clockwise direction

        1:
           Theta increases in the counterclockwise direction
        """
        return _polar.get_theta_direction(self._native)

    def set_rmax(self, rmax):
        """
        Set the outer radial limit.

        Parameters
        ----------
        rmax : float
        """
        _polar.set_rmax(self._native, float(rmax))

    def get_rmax(self):
        """
        Returns
        -------
        float
            Outer radial limit.
        """
        return _polar.get_rlim(self._native)[1]

    def set_rmin(self, rmin):
        """
        Set the inner radial limit.

        Parameters
        ----------
        rmin : float
        """
        _polar.set_rmin(self._native, float(rmin))

    def get_rmin(self):
        """
        Returns
        -------
        float
            The inner radial limit.
        """
        return _polar.get_rlim(self._native)[0]

    def set_rorigin(self, rorigin):
        """
        Update the radial origin.

        Parameters
        ----------
        rorigin : float
        """
        _polar.set_rorigin(self._native, None if rorigin is None else float(rorigin))

    def get_rorigin(self):
        """
        Returns
        -------
        float
        """
        return _polar.get_rorigin(self._native)

    def get_rsign(self):
        """The sign of the radial direction: 1 outwards, -1 inwards."""
        return np.sign(self.get_rmax() - self.get_rorigin())

    def set_rlim(self, bottom=None, top=None, *,
                 emit=True, auto=False, **kwargs):
        """
        Set the radial axis view limits.

        This function behaves like `.Axes.set_ylim`, but additionally supports
        *rmin* and *rmax* as aliases for *bottom* and *top*.

        See Also
        --------
        .Axes.set_ylim
        """
        if 'rmin' in kwargs:
            if bottom is None:
                bottom = kwargs.pop('rmin')
            else:
                raise ValueError('Cannot supply both positional "bottom"'
                                 'argument and kwarg "rmin"')
        if 'rmax' in kwargs:
            if top is None:
                top = kwargs.pop('rmax')
            else:
                raise ValueError('Cannot supply both positional "top"'
                                 'argument and kwarg "rmax"')
        if kwargs or not emit or auto:
            raise NotImplementedError(
                "pltrs: set_rlim(emit=..., auto=..., ymin/ymax=...)")
        return self.set_ylim(bottom, top)

    def get_rlabel_position(self):
        """
        Returns
        -------
        float
            The theta position of the radius labels in degrees.
        """
        return _polar.get_rlabel_position(self._native)

    def set_rlabel_position(self, value):
        """
        Update the theta position of the radius labels.

        Parameters
        ----------
        value : number
            The angular position of the radius labels in degrees.
        """
        _polar.set_rlabel_position(self._native, float(value))

    def set_rscale(self, value, **kwargs):
        """Set the scale of the radial axis (`.Axes.set_yscale`)."""
        if kwargs:
            raise NotImplementedError("pltrs: set_rscale(**kwargs)")
        _polar.set_rscale(self._native, value)

    def set_rticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """Set the radial ticks (`.Axes.set_yticks`)."""
        if labels is not None or minor or kwargs:
            return self.set_yticks(ticks, labels, minor=minor, **kwargs)
        _polar.set_rticks(self._native, [float(t) for t in np.atleast_1d(ticks)])

    def set_thetagrids(self, angles, labels=None, fmt=None, **kwargs):
        """
        Set the theta gridlines in a polar plot.

        Parameters
        ----------
        angles : tuple with floats, degrees
            The angles of the theta gridlines.
        labels : tuple with strings or None
            The labels to use at each theta gridline. The
            `.projections.polar.ThetaFormatter` will be used if None.
        fmt : str or None
            Format string used in `matplotlib.ticker.FormatStrFormatter`.
            For example '%f'. Note that the angle that is used is in
            radians.

        Returns
        -------
        lines : list of `.lines.Line2D`
            The theta gridlines.
        labels : list of `.text.Text`
            The tick labels.
        """
        _polar.set_thetagrids(self._native, [float(a) for a in np.atleast_1d(angles)])
        if labels is not None:
            self.set_xticklabels(labels)
        elif fmt is not None:
            self.xaxis.set_major_formatter(mticker.FormatStrFormatter(fmt))
        for t in self.xaxis.get_ticklabels():
            t._internal_update(kwargs)
        return self.xaxis.get_ticklines(), self.xaxis.get_ticklabels()

    def set_rgrids(self, radii, labels=None, angle=None, fmt=None, **kwargs):
        """
        Set the radial gridlines on a polar plot.

        Parameters
        ----------
        radii : tuple with floats
            The radii for the radial gridlines
        labels : tuple with strings or None
            The labels to use at each radial gridline. The
            `matplotlib.ticker.ScalarFormatter` will be used if None.
        angle : float
            The angular position of the radius labels in degrees.
        fmt : str or None
            Format string used in `matplotlib.ticker.FormatStrFormatter`.
            For example '%f'.

        Returns
        -------
        lines : list of `.lines.Line2D`
            The radial gridlines.
        labels : list of `.text.Text`
            The tick labels.
        """
        _polar.set_rgrids(self._native, [float(r) for r in np.atleast_1d(radii)],
                          None if angle is None else float(angle))
        if labels is not None:
            self.set_yticklabels(labels)
        elif fmt is not None:
            self.yaxis.set_major_formatter(mticker.FormatStrFormatter(fmt))
        for t in self.yaxis.get_ticklabels():
            t._internal_update(kwargs)
        return self.yaxis.get_gridlines(), self.yaxis.get_ticklabels()

    def get_data_ratio(self):
        """
        Return the aspect ratio of the data itself.  For a polar plot,
        this should always be 1.0
        """
        return 1.0

    def can_zoom(self):
        """
        Return whether this Axes supports the zoom box button functionality.

        A polar Axes does not support zoom boxes.
        """
        return False

    def can_pan(self):
        """
        Return whether this Axes supports the pan/zoom button functionality.

        For a polar Axes, this is slightly misleading. Both panning and
        zooming are performed by the same button. Panning is performed
        in azimuth while zooming is done along the radial.
        """
        return True
