"""
Polar axes (matplotlib's ``projections.polar``): pltrs (Rust) axes in the
polar projection, made by ``add_subplot(projection='polar')``.

pltrs (Rust) projects, lays out and draws the axes and keeps their
settings (``_native.polar``). The tick helpers `ThetaFormatter`,
`ThetaLocator` and `RadialLocator`, and the transforms `PolarTransform`
and `InvertedPolarTransform`, which matplotlib code uses on their own
(curvilinear grids of ``axisartist``), are matplotlib's (from matplotlib
3.11.2, under the Matplotlib License).
"""

import math

import numpy as np

from .. import _api, _mirrors, _numpy_values, _tick_helpers, cbook, ticker as mticker
from .. import patches as mpatches, transforms as mtransforms
from .._native import artist as _artist, axis as _axis, polar as _polar
from ..axes import Axes
from ..axis import XAxis, XTick, YAxis, YTick
from ..path import Path
from ._polar_trees import PolarTrees, _ThetaShift, _is_full_circle_rad  # noqa: F401


class PolarTransform(mtransforms.Transform):
    r"""
    The base polar transform.

    This transform maps polar coordinates :math:`\theta, r` into Cartesian
    coordinates :math:`x, y = r \cos(\theta), r \sin(\theta)`
    (but does not fully transform into Axes coordinates or
    handle positioning in screen space).

    This transformation is designed to be applied to data after any scaling
    along the radial axis (e.g. log-scaling) has been applied to the input
    data.

    Path segments at a fixed radius are automatically transformed to circular
    arcs as long as ``path._interpolation_steps > 1``.
    """

    input_dims = output_dims = 2

    @_api.delete_parameter('3.11', 'apply_theta_transforms')
    def __init__(self, axis=None, use_rmin=True, *,
                 apply_theta_transforms=False, scale_transform=None):
        """
        Parameters
        ----------
        axis : `~matplotlib.axis.Axis`, optional
            Axis associated with this transform. This is used to get the
            minimum radial limit.
        use_rmin : `bool`, optional
            If ``True``, subtract the minimum radial axis limit before
            transforming to Cartesian coordinates. *axis* must also be
            specified for this to take effect.
        """
        super().__init__()
        self._axis = axis
        self._use_rmin = use_rmin
        self._scale_transform = scale_transform

    __str__ = mtransforms._make_str_method(
        "_axis",
        use_rmin="_use_rmin"
    )

    def _get_rorigin(self):
        # Get lower r limit after being scaled by the radial scale transform
        return self._scale_transform.transform(
            (0, self._axis.get_rorigin()))[1]

    def transform_non_affine(self, values):
        # docstring inherited
        theta, r = np.transpose(values)
        if self._use_rmin and self._axis is not None:
            r = (r - self._get_rorigin()) * self._axis.get_rsign()
        r = np.where(r >= 0, r, np.nan)
        return np.column_stack([r * np.cos(theta), r * np.sin(theta)])

    def transform_path_non_affine(self, path):
        # docstring inherited
        if not len(path) or path._interpolation_steps == 1:
            return Path(self.transform_non_affine(path.vertices), path.codes)
        xys = []
        codes = []
        last_t = last_r = None
        for trs, c in path.iter_segments():
            trs = trs.reshape((-1, 2))
            if c == Path.LINETO:
                (t, r), = trs
                if t == last_t:  # Same angle: draw a straight line.
                    xys.extend(self.transform_non_affine(trs))
                    codes.append(Path.LINETO)
                elif r == last_r:  # Same radius: draw an arc.
                    # The following is complicated by Path.arc() being
                    # "helpful" and unwrapping the angles, but we don't want
                    # that behavior here.
                    last_td, td = np.rad2deg([last_t, t])
                    if self._use_rmin and self._axis is not None:
                        r = ((r - self._get_rorigin())
                             * self._axis.get_rsign())
                    if last_td <= td:
                        while td - last_td > 360:
                            arc = Path.arc(last_td, last_td + 360)
                            xys.extend(arc.vertices[1:] * r)
                            codes.extend(arc.codes[1:])
                            last_td += 360
                        arc = Path.arc(last_td, td)
                        xys.extend(arc.vertices[1:] * r)
                        codes.extend(arc.codes[1:])
                    else:
                        # The reverse version also relies on the fact that all
                        # codes but the first one are the same.
                        while last_td - td > 360:
                            arc = Path.arc(last_td - 360, last_td)
                            xys.extend(arc.vertices[::-1][1:] * r)
                            codes.extend(arc.codes[1:])
                            last_td -= 360
                        arc = Path.arc(td, last_td)
                        xys.extend(arc.vertices[::-1][1:] * r)
                        codes.extend(arc.codes[1:])
                else:  # Interpolate.
                    trs = cbook.simple_linear_interpolation(
                        np.vstack([(last_t, last_r), trs]),
                        path._interpolation_steps)[1:]
                    xys.extend(self.transform_non_affine(trs))
                    codes.extend([Path.LINETO] * len(trs))
            else:  # Not a straight line.
                xys.extend(self.transform_non_affine(trs))
                codes.extend([c] * len(trs))
            last_t, last_r = trs[-1]
        return Path(xys, codes)

    def inverted(self):
        # docstring inherited
        return PolarAxes.InvertedPolarTransform(self._axis, self._use_rmin)


class PolarAffine(mtransforms.Affine2DBase):
    r"""
    The affine part of the polar projection.

    Scales the output so that maximum radius rests on the edge of the Axes
    circle and the origin is mapped to (0.5, 0.5). The transform applied is
    the same to x and y components and given by:

    .. math::

        x_{1} = 0.5 \left [ \frac{x_{0}}{(r_{\max} - r_{\min})} + 1 \right ]

    :math:`r_{\min}, r_{\max}` are the minimum and maximum radial limits after
    any scaling (e.g. log scaling) has been removed.
    """
    def __init__(self, scale_transform, limits):
        """
        Parameters
        ----------
        scale_transform : `~matplotlib.transforms.Transform`
            Scaling transform for the data. This is used to remove any scaling
            from the radial view limits.
        limits : `~matplotlib.transforms.BboxBase`
            View limits of the data. The only part of its bounds that is used
            is the y limits (for the radius limits).
        """
        super().__init__()
        self._scale_transform = scale_transform
        self._limits = limits
        self.set_children(scale_transform, limits)
        self._mtx = None

    __str__ = mtransforms._make_str_method("_scale_transform", "_limits")

    def get_matrix(self):
        # docstring inherited
        if self._invalid:
            limits_scaled = self._limits.transformed(self._scale_transform)
            yscale = limits_scaled.ymax - limits_scaled.ymin
            affine = mtransforms.Affine2D() \
                .scale(0.5 / yscale) \
                .translate(0.5, 0.5)
            self._mtx = affine.get_matrix()
            self._inverted = None
            self._invalid = 0
        return self._mtx


class InvertedPolarTransform(mtransforms.Transform):
    """
    The inverse of the polar transform, mapping Cartesian
    coordinate space *x* and *y* back to *theta* and *r*.
    """
    input_dims = output_dims = 2

    @_api.delete_parameter('3.11', 'apply_theta_transforms')
    def __init__(self, axis=None, use_rmin=True,
                 *, apply_theta_transforms=False):
        """
        Parameters
        ----------
        axis : `~matplotlib.axis.Axis`, optional
            Axis associated with this transform. This is used to get the
            minimum radial limit.
        use_rmin : `bool`, optional
            If ``True``, add the minimum radial axis limit after
            transforming from Cartesian coordinates. *axis* must also be
            specified for this to take effect.
        """
        super().__init__()
        self._axis = axis
        self._use_rmin = use_rmin

    __str__ = mtransforms._make_str_method(
        "_axis",
        use_rmin="_use_rmin")

    def transform_non_affine(self, values):
        # docstring inherited
        x, y = values.T
        r = np.hypot(x, y)
        theta = np.arctan2(y, x) % (2 * np.pi)
        if self._use_rmin and self._axis is not None:
            r += self._axis.get_rorigin()
            r *= self._axis.get_rsign()
        return np.column_stack([theta, r])

    def inverted(self):
        # docstring inherited
        return PolarAxes.PolarTransform(self._axis, self._use_rmin)


def _is_full_circle_deg(thetamin, thetamax):
    """
    Determine if a wedge (in degrees) spans the full circle.

    The condition is derived from :class:`~matplotlib.patches.Wedge`.
    """
    return abs(abs(thetamax - thetamin) - 360.0) < 1e-12


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

    def _spec(self):
        """The native description of the locator (that of its base, in
        degrees), None when the base computes in Python."""
        base = _tick_helpers._spec_of(self.base)
        return None if base is None else {"kind": "theta", "base": base}


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
        if self._keeps_off_origin():
            rorigin = self._rorigin()
            return [tick for tick in self.base() if tick > rorigin]
        return self.base()

    def _rorigin(self):
        """The radius of the origin, signed as the radial axis goes."""
        ax = self.base.axis.axes
        return ax.get_rorigin() * ax.get_rsign()

    def _keeps_off_origin(self):
        """Whether the ticks leave out those at and inside the origin: on a
        full circle that is not annular, as matplotlib's do (a list of
        them)."""
        ax = self.base.axis.axes
        return (_is_full_circle_rad(*ax.get_xlim())
                and ax.get_rmin() <= self._rorigin())

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

    def _spec(self):
        """The native description of the locator: that of its base, which
        pltrs (Rust) wraps on the radial axis; None when the base computes
        in Python."""
        return _tick_helpers._spec_of(self.base)


class ThetaTick(XTick):
    """
    A theta-axis tick.

    This subclass of `.XTick` provides angular ticks with some small
    modification to their re-positioning such that ticks are rotated based on
    tick location. This results in ticks that are correctly perpendicular to
    the arc spine.

    When 'auto' rotation is enabled, labels are also rotated to be parallel to
    the spine. The label padding is also applied here since it's not possible
    to use a generic axes transform to produce tick-specific padding.
    """
    # Its marks are perpendicular to the arc spine and, with 'auto' rotation,
    # its labels parallel to it, padded from the spine: pltrs (Rust) places
    # and turns them as matplotlib's ``ThetaTick.update_position`` does.

    def __init__(self, axes, *args, **kwargs):
        super().__init__(axes, *args, **kwargs)
        self.label1.set_rotation_mode('anchor')
        self.label2.set_rotation_mode('anchor')

    def update_position(self, loc):
        super().update_position(loc)
        # matplotlib's angles of the tick, with which it turns its marks
        # (pltrs (Rust) turns them as it draws): Python's and numpy's
        # arithmetic on *loc*, which fails and warns as they do.
        axes = self.axes
        angle = loc * axes.get_theta_direction() + axes.get_theta_offset()
        np.rad2deg(angle) % 360  # noqa: B018
        turned = {mmarkers.TICKUP, mmarkers.TICKDOWN, '|'}
        if {self.tick1line.get_marker(), self.tick2line.get_marker()} & turned:
            mtransforms.Affine2D().rotate(angle - np.pi / 2)


class ThetaAxis(XAxis):
    """
    A theta Axis.

    This overrides certain properties of an `.XAxis` to provide special-casing
    for an angular axis.
    """
    # pltrs (Rust) wraps its default locator and formatter for angles
    # (`ThetaLocator`, `ThetaFormatter`).
    __name__ = 'thetaaxis'
    _tick_class = ThetaTick

    def _wrap_locator_formatter(self):
        locator = self.get_major_locator()
        if not isinstance(locator, ThetaLocator):
            self.set_major_locator(ThetaLocator(locator))
        self.set_major_formatter(ThetaFormatter())
        _axis.mark_default_helpers(*self._args())

    def clear(self):
        # docstring inherited
        super().clear()
        self.set_ticks_position('none')
        self._wrap_locator_formatter()

    def _set_axes_scale(self, value, **kwargs):
        # docstring inherited; the angles stay linear (matplotlib's
        # ``ThetaAxis._set_scale``).
        if value != 'linear':
            raise NotImplementedError(
                "The xscale cannot be set on a polar plot")
        super()._set_axes_scale(value, **kwargs)


class RadialTick(YTick):
    """
    A radial-axis tick.

    This subclass of `.YTick` provides radial ticks with some small
    modification to their re-positioning such that ticks are rotated based on
    axes limits.  This results in ticks that are correctly perpendicular to
    the spine. Labels are also rotated to be perpendicular to the spine, when
    'auto' rotation is enabled.
    """
    # Its marks and, with 'auto' rotation, its labels are perpendicular to the
    # spine: pltrs (Rust) places and turns them as matplotlib's
    # ``RadialTick.update_position`` does.

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.label1.set_rotation_mode('anchor')
        self.label2.set_rotation_mode('anchor')

    def update_position(self, loc):
        super().update_position(loc)


class RadialAxis(YAxis):
    """
    A radial Axis.

    This overrides certain properties of a `.YAxis` to provide special-casing
    for a radial axis.
    """
    __name__ = 'radialaxis'
    _tick_class = RadialTick

    def set_major_locator(self, locator):
        # docstring inherited
        super().set_major_locator(self._as_major_locator(locator))

    def _as_major_locator(self, locator):
        # A radial axis wraps its major locators.
        return locator if isinstance(locator, RadialLocator) else RadialLocator(locator)

    def _wrap_locator_formatter(self):
        self.set_major_locator(self.get_major_locator())
        _axis.mark_default_helpers(*self._args())

    def clear(self):
        # docstring inherited
        super().clear()
        self.set_ticks_position('none')
        self._wrap_locator_formatter()


class _WedgeBbox(mtransforms.Bbox):
    """
    Transform (theta, r) wedge Bbox into Axes bounding box.

    Parameters
    ----------
    center : (float, float)
        Center of the wedge
    viewLim : `~matplotlib.transforms.Bbox`
        Bbox determining the boundaries of the wedge
    originLim : `~matplotlib.transforms.Bbox`
        Bbox determining the origin for the wedge, if different from *viewLim*
    """
    def __init__(self, center, viewLim, originLim, **kwargs):
        super().__init__([[0, 0], [1, 1]], **kwargs)
        self._center = center
        self._viewLim = viewLim
        self._originLim = originLim
        self.set_children(viewLim, originLim)

    __str__ = mtransforms._make_str_method("_center", "_viewLim", "_originLim")

    def get_points(self):
        # docstring inherited
        if self._invalid:
            from ..patches import Wedge
            points = self._viewLim.get_points().copy()
            # Scale angular limits to work with Wedge.
            points[:, 0] *= 180 / np.pi
            if points[0, 0] > points[1, 0]:
                points[:, 0] = points[::-1, 0]

            # Scale radial limits based on origin radius.
            points[:, 1] -= self._originLim.y0

            # Scale radial limits to match axes limits.
            rscale = 0.5 / points[1, 1]
            points[:, 1] *= rscale
            width = min(points[1, 1] - points[0, 1], 0.5)

            # Generate bounding box for wedge.
            wedge = Wedge(self._center, points[1, 1],
                          points[0, 0], points[1, 0],
                          width=width)
            self.update_from_path(wedge.get_path())

            # Ensure equal aspect ratio.
            w, h = self._points[1] - self._points[0]
            deltah = max(w - h, 0) / 2
            deltaw = max(h - w, 0) / 2
            self._points += np.array([[-deltaw, -deltah], [deltaw, deltah]])

            self._invalid = 0

        return self._points


#: matplotlib's default angle of the radial labels, in radians.
_DEFAULT_RLABEL = np.deg2rad(22.5)


class PolarAxes(PolarTrees, Axes):
    """
    A polar graph projection, where the input dimensions are *theta*, *r*.

    Theta starts pointing east and goes anti-clockwise.
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    name = 'polar'

    def __init__(self, *args,
                 theta_offset=0, theta_direction=1, rlabel_position=22.5,
                 **kwargs):
        # docstring inherited
        self._default_theta_offset = theta_offset
        self._default_theta_direction = theta_direction
        self._default_rlabel_position = np.deg2rad(rlabel_position)
        super().__init__(*args, **kwargs)
        self.use_sticky_edges = True
        self.set_aspect('equal', adjustable='box', anchor='C')
        self.clear()

    # What ``help`` shows, as matplotlib's (the mixin before `.Axes` in the
    # MRO would hide it).
    __init__.__doc__ = Axes.__init__.__doc__

    def _init_axes_state(self):
        super()._init_axes_state()
        self.xaxis = ThetaAxis(self)
        self.yaxis = RadialAxis(self)

    def _gen_axes_spines(self):
        # docstring inherited; pltrs (Rust) draws the outer and inner arcs
        # and the sector's edges as matplotlib places them.
        from ..spines import Spine
        return {name: Spine._of_axes(self, name)
                for name in ['polar', 'start', 'end', 'inner']}

    def _init_axis(self):
        # docstring inherited
        self.xaxis = ThetaAxis(self, clear=False)
        self.yaxis = RadialAxis(self, clear=False)
        self.spines['polar'].register_axis(self.yaxis)

    def clear(self):
        # docstring inherited
        super().clear()
        # pltrs (Rust) clears to matplotlib's defaults; those given to the
        # constructor are the Axes' own.
        defaults = self.__dict__
        if defaults.get('_default_theta_offset', 0) != 0:
            self.set_theta_offset(self._default_theta_offset)
        if defaults.get('_default_theta_direction', 1) != 1:
            self.set_theta_direction(self._default_theta_direction)
        rlabel = defaults.get('_default_rlabel_position', _DEFAULT_RLABEL)
        if rlabel != _DEFAULT_RLABEL:
            self.set_rlabel_position(np.rad2deg(rlabel))

    def _gen_axes_patch(self):
        # docstring inherited
        return mpatches.Wedge((0.5, 0.5), 0.5, 0.0, 360.0)

    def _fit_patch(self, patch):
        """
        Give a `.Wedge` background what matplotlib's draw gives it: its
        centre, angles, radius and width fitting the view the last draw
        showed (pltrs (Rust) fits the background it draws itself).
        """
        drawn = _artist.draw_count(self._native)
        if (not isinstance(patch, mpatches.Wedge) or not drawn
                or self.__dict__.get("_patch_fitted") == drawn):
            return
        self._patch_fitted = drawn
        thetamin, thetamax = np.rad2deg(self._realViewLim.intervalx)
        if thetamin > thetamax:
            thetamin, thetamax = thetamax, thetamin
        rmin, rmax = ((self._realViewLim.intervaly - self.get_rorigin()) *
                      self.get_rsign())
        center = self.transWedge.transform((0.5, 0.5))
        edge, _ = self.transWedge.transform((1, 0))
        radius = edge - center[0]
        with _mirrors.detached(patch):
            patch.set_center(center)
            patch.set_theta1(thetamin)
            patch.set_theta2(thetamax)
            patch.set_radius(radius)
            patch.set_width(min(radius * (rmax - rmin) / rmax, radius))

    def format_coord(self, theta, r):
        # docstring inherited
        screen_xy = self.transData.transform((theta, r))
        screen_xys = screen_xy + np.stack(
            np.meshgrid([-1, 0, 1], [-1, 0, 1])).reshape((2, -1)).T
        ts, rs = self.transData.inverted().transform(screen_xys).T
        delta_t = abs((ts - theta + np.pi) % (2 * np.pi) - np.pi).max()
        delta_t_halfturns = delta_t / np.pi
        delta_t_degrees = delta_t_halfturns * 180
        delta_r = abs(rs - r).max()
        if theta < 0:
            theta += 2 * np.pi
        theta_halfturns = theta / np.pi
        theta_degrees = theta_halfturns * 180

        # See ScalarFormatter.format_data_short.  For r, use #g-formatting
        # (as for linear axes), but for theta, use f-formatting as scientific
        # notation doesn't make sense and the trailing dot is ugly.
        def format_sig(value, delta, opt, fmt):
            # For "f", only count digits after decimal point.
            prec = (max(0, -math.floor(math.log10(delta))) if fmt == "f" else
                    cbook._g_sig_digits(value, delta))
            return f"{value:-{opt}.{prec}{fmt}}"

        # In case fmt_xdata was not specified, resort to default

        if self.fmt_ydata is None:
            r_label = format_sig(r, delta_r, "#", "g")
        else:
            r_label = self.format_ydata(r)

        if self.fmt_xdata is None:
            return ('\N{GREEK SMALL LETTER THETA}={}\N{GREEK SMALL LETTER PI} '
                    '({}\N{DEGREE SIGN}), r={}').format(
                    format_sig(theta_halfturns, delta_t_halfturns, "", "f"),
                    format_sig(theta_degrees, delta_t_degrees, "", "f"),
                    r_label
                )
        else:
            return '\N{GREEK SMALL LETTER THETA}={}, r={}'.format(
                        self.format_xdata(theta),
                        r_label
                        )

    def set_thetamax(self, thetamax):
        """Set the maximum theta limit in degrees."""
        _polar.set_thetamax(self._native, _numpy_values.degrees(thetamax))

    def get_thetamax(self):
        """Return the maximum theta limit in degrees."""
        return np.float64(_polar.get_thetalim(self._native)[1])

    def set_thetamin(self, thetamin):
        """Set the minimum theta limit in degrees."""
        _polar.set_thetamin(self._native, _numpy_values.degrees(thetamin))

    def get_thetamin(self):
        """Get the minimum theta limit in degrees."""
        return np.float64(_polar.get_thetalim(self._native)[0])

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
        _polar.set_theta_offset(self._native, _numpy_values.element(offset))

    def get_theta_offset(self):
        """
        Get the offset for the location of 0 in radians.
        """
        return np.float64(_polar.get_theta_offset(self._native))

    def set_theta_zero_location(self, loc, offset=0.0):
        """
        Set the location of theta's zero.

        This simply calls `set_theta_offset` with the correct value in radians.

        Parameters
        ----------
        loc : str
            May be one of "N", "NW", "W", "SW", "S", "SE", "E", or "NE".
        offset : float, default: 0
            An offset in degrees to apply from the specified *loc*. **Note:**
            this offset is *always* applied counter-clockwise regardless of
            the direction setting.
        """
        mapping = {
            'N': np.pi * 0.5,
            'NW': np.pi * 0.75,
            'W': np.pi,
            'SW': np.pi * 1.25,
            'S': np.pi * 1.5,
            'SE': np.pi * 1.75,
            'E': 0,
            'NE': np.pi * 0.25}
        return self.set_theta_offset(mapping[loc] + np.deg2rad(offset))

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
        return np.float64(_polar.get_theta_direction(self._native))

    def set_rmax(self, rmax):
        """
        Set the outer radial limit.

        Parameters
        ----------
        rmax : float
        """
        _polar.set_rmax(self._native, _numpy_values.element(rmax))

    def get_rmax(self):
        """
        Returns
        -------
        float
            Outer radial limit.
        """
        return np.float64(_polar.get_rlim(self._native)[1])

    def set_rmin(self, rmin):
        """
        Set the inner radial limit.

        Parameters
        ----------
        rmin : float
        """
        _polar.set_rmin(self._native, _numpy_values.element(rmin))

    def get_rmin(self):
        """
        Returns
        -------
        float
            The inner radial limit.
        """
        return np.float64(_polar.get_rlim(self._native)[0])

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
        if self.__dict__.get('_answering_from_trees'):
            return self._originViewLim.y0
        return np.float64(_polar.get_rorigin(self._native))

    def get_rsign(self):
        if self.__dict__.get('_answering_from_trees'):
            # What matplotlib computes, from the limits the projection of a
            # program's subclass is asked about while pltrs (Rust) draws.
            return np.sign(self._originViewLim.y1 - self._originViewLim.y0)
        return np.sign(self.viewLim.y1 - self.get_rorigin())

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
        return self.set_ylim(bottom, top, emit=emit, auto=auto, **kwargs)

    def get_rlabel_position(self):
        """
        Returns
        -------
        float
            The theta position of the radius labels in degrees.
        """
        return np.float64(_polar.get_rlabel_position(self._native))

    def set_rlabel_position(self, value):
        """
        Update the theta position of the radius labels.

        Parameters
        ----------
        value : number
            The angular position of the radius labels in degrees.
        """
        _polar.set_rlabel_position(self._native, _numpy_values.degrees(value))

    def set_rscale(self, *args, **kwargs):
        return Axes.set_yscale(self, *args, **kwargs)

    def set_rticks(self, *args, **kwargs):
        return Axes.set_yticks(self, *args, **kwargs)

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

        Other Parameters
        ----------------
        **kwargs
            *kwargs* are optional `.Text` properties for the labels.

            .. warning::

                This only sets the properties of the current ticks.
                Ticks are not guaranteed to be persistent. Various operations
                can create, delete and modify the Tick instances. There is an
                imminent risk that these settings can get lost if you work on
                the figure further (including also panning/zooming on a
                displayed figure).

                Use `.set_tick_params` instead if possible.

        See Also
        --------
        .PolarAxes.set_rgrids
        .Axis.get_gridlines
        .Axis.get_ticklabels
        """
        # Make sure we take into account unitized data
        angles = self.convert_yunits(angles)
        angles = np.deg2rad(angles)
        self.set_xticks(angles)
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

        Other Parameters
        ----------------
        **kwargs
            *kwargs* are optional `.Text` properties for the labels.

            .. warning::

                This only sets the properties of the current ticks.
                Ticks are not guaranteed to be persistent. Various operations
                can create, delete and modify the Tick instances. There is an
                imminent risk that these settings can get lost if you work on
                the figure further (including also panning/zooming on a
                displayed figure).

                Use `.set_tick_params` instead if possible.

        See Also
        --------
        .PolarAxes.set_thetagrids
        .Axis.get_gridlines
        .Axis.get_ticklabels
        """
        # Make sure we take into account unitized data
        radii = self.convert_xunits(radii)
        radii = np.asarray(radii)

        self.set_yticks(radii)
        if labels is not None:
            self.set_yticklabels(labels)
        elif fmt is not None:
            self.yaxis.set_major_formatter(mticker.FormatStrFormatter(fmt))
        if angle is None:
            angle = self.get_rlabel_position()
        self.set_rlabel_position(angle)
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

    # pltrs (Rust) draws the polar Axes, and pans and zooms them in azimuth
    # and along the radius as matplotlib's do.

    def draw(self, renderer):
        super().draw(renderer)

    def start_pan(self, x, y, button):
        super().start_pan(x, y, button)

    def end_pan(self):
        super().end_pan()

    def drag_pan(self, button, key, x, y):
        super().drag_pan(button, key, x, y)


PolarAxes.ThetaFormatter = ThetaFormatter
PolarAxes.RadialLocator = RadialLocator
PolarAxes.ThetaLocator = ThetaLocator
PolarAxes.PolarTransform = PolarTransform
PolarAxes.PolarAffine = PolarAffine
PolarAxes.InvertedPolarTransform = InvertedPolarTransform


cbook._own_mixin_methods(PolarAxes, Axes)


# What matplotlib's module also has from its imports, which programs
# import from it.
import math  # noqa: E402, F401
import types  # noqa: E402, F401
from .. import axis as maxis, markers as mmarkers  # noqa: E402, F401
from ..spines import Spine  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
