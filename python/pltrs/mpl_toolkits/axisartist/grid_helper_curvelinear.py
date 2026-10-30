"""
An experimental support for curvilinear grid.

matplotlib's ``mpl_toolkits.axisartist.grid_helper_curvelinear`` (from
matplotlib 3.11.2, under the Matplotlib License): pltrs (Rust) lays the
grid lines, ticks and axis artists out along the aux transform, asking the
helper's `.GridFinder` (its extreme finder, locators and formatters) for
the range, levels and labels (``_native.axisartist``).
"""

import numpy as np

from pltrs.transforms import Bbox, CompositeGenericTransform
from pltrs._native import axisartist as _axisartist
from .axislines import (
    _FixedAxisArtistHelperBase, _FloatingAxisArtistHelperBase, GridHelperBase)
from .axis_artist import AxisArtist
from .grid_finder import GridFinder


def _point_function(function):
    """*function*, mapping points as arrays, mapping one point ``(x, y)``
    as pltrs (Rust) calls it."""
    def call(x, y):
        with np.errstate(all="ignore"):
            u, v = function(np.float64(x), np.float64(y))
        return float(u), float(v)
    return call


def _native_aux(aux_trans):
    """The native description of the aux transform *aux_trans*: a
    transform, or a ``(forward, inverse)`` pair of functions of ``(x, y)``."""
    if isinstance(aux_trans, tuple):
        forward, inverse = aux_trans
        return ("functions", _point_function(forward), _point_function(inverse))
    if aux_trans.is_affine:
        return ("affine", np.asarray(aux_trans.get_matrix(), dtype=float))
    if isinstance(aux_trans, CompositeGenericTransform):
        return ("then", _native_aux(aux_trans._a), _native_aux(aux_trans._b))
    if type(aux_trans).__name__ == "PolarTransform":
        return ("polar",)
    inverse = aux_trans.inverted()

    def mapped(transform):
        return lambda x, y: tuple(transform.transform(np.array([[x, y]]))[0])
    return ("functions", _point_function(mapped(aux_trans)),
            _point_function(mapped(inverse)))


class FixedAxisArtistHelper(_FixedAxisArtistHelperBase):
    """
    Helper class for a fixed axis.
    """

    def __init__(self, grid_helper, side, nth_coord_ticks=None):
        """
        nth_coord = along which coordinate value varies.
         nth_coord = 0 ->  x axis, nth_coord = 1 -> y axis
        """
        super().__init__(loc=side)
        self.grid_helper = grid_helper
        if nth_coord_ticks is None:
            nth_coord_ticks = self.nth_coord
        self._nth_coord_ticks = nth_coord_ticks
        self._artist_native = None

    def _bind(self, native):
        """Act on the native axis artist *native* from now on."""
        self._artist_native = native
        if self._nth_coord_ticks != self.nth_coord:
            self.nth_coord_ticks = self._nth_coord_ticks

    @property
    def nth_coord_ticks(self):
        """The coordinate whose ticks the axis shows (0 or 1)."""
        return self._nth_coord_ticks

    @nth_coord_ticks.setter
    def nth_coord_ticks(self, nth):
        self._nth_coord_ticks = nth
        if self._artist_native is not None:
            _axisartist.set_nth_coord_ticks(self._artist_native, "xy"[nth])


class FloatingAxisArtistHelper(_FloatingAxisArtistHelperBase):

    def __init__(self, grid_helper, nth_coord, value, axis_direction=None):
        """
        nth_coord = along which coordinate value varies.
         nth_coord = 0 ->  x axis, nth_coord = 1 -> y axis
        """
        super().__init__(nth_coord, value)
        self.value = value
        self.grid_helper = grid_helper
        self._extremes = -np.inf, np.inf
        self._line_num_points = 100  # number of points to create a line
        self._axis_direction = axis_direction or "bottom"
        self._artist_native = None

    def _bind(self, native):
        """Act on the native axis artist *native* from now on."""
        self._artist_native = native

    def set_extremes(self, e1, e2):
        if e1 is None:
            e1 = -np.inf
        if e2 is None:
            e2 = np.inf
        self._extremes = e1, e2
        if self._artist_native is not None:
            _axisartist.set_extremes(self._artist_native, (float(e1), float(e2)))


class GridHelperCurveLinear(GridHelperBase):
    def __init__(self, aux_trans,
                 extreme_finder=None,
                 grid_locator1=None,
                 grid_locator2=None,
                 tick_formatter1=None,
                 tick_formatter2=None):
        """
        Parameters
        ----------
        aux_trans : `.Transform` or tuple[Callable, Callable]
            The transform from curved coordinates to rectilinear coordinate:
            either a `.Transform` instance (which provides also its inverse),
            or a pair of callables ``(trans, inv_trans)`` that define the
            transform and its inverse.  The callables should have signature::

                x_rect, y_rect = trans(x_curved, y_curved)
                x_curved, y_curved = inv_trans(x_rect, y_rect)

        extreme_finder

        grid_locator1, grid_locator2
            Grid locators for each axis.

        tick_formatter1, tick_formatter2
            Tick formatters for each axis.
        """
        super().__init__()
        self._grid_info = None
        self._aux_trans = aux_trans
        self.grid_finder = GridFinder(aux_trans,
                                      extreme_finder,
                                      grid_locator1,
                                      grid_locator2,
                                      tick_formatter1,
                                      tick_formatter2)

    def update_grid_finder(self, aux_trans=None, **kwargs):
        if aux_trans is not None:
            self._aux_trans = aux_trans
            self.grid_finder.set_transform(aux_trans)
        self.grid_finder.update(**kwargs)
        self._old_limits = None  # Force revalidation.

    def _native_finder(self):
        """The parts of the grid finder as pltrs (Rust) calls them; they
        read the grid finder anew at each call."""
        finder = self.grid_finder

        def extremes(x0, y0, x1, y1):
            return tuple(map(float, finder.extreme_finder._find_transformed_bbox(
                finder.get_transform().inverted(),
                Bbox.from_extents(x0, y0, x1, y1)).extents))

        def locator(index):
            def locate(v1, v2):
                levels, n, factor = getattr(finder, f"grid_locator{index}")(v1, v2)
                return ([float(level) for level in np.asarray(levels).ravel()],
                        int(n), float(factor))
            return locate

        def formatter(index):
            def format_ticks(direction, factor, values):
                return [str(label) for label in finder._format_ticks(
                    index, direction, factor, values)]
            return format_ticks

        return (extremes, (locator(1), locator(2)), (formatter(1), formatter(2)))

    def _install(self, axes, zero):
        _axisartist.install_curvilinear(axes._native, _native_aux(self._aux_trans),
                                        self._native_finder())

    def _helper_of(self, axes, name):
        return FixedAxisArtistHelper(self, name)

    def new_fixed_axis(
        self, loc, *, axis_direction=None, offset=None, axes=None, nth_coord=None
    ):
        if axes is None:
            axes = self.axes
        if axis_direction is None:
            axis_direction = loc
        helper = FixedAxisArtistHelper(self, loc, nth_coord_ticks=nth_coord)
        axisline = AxisArtist(axes, helper, axis_direction=axis_direction)
        # Why is clip not set on axisline, unlike in new_floating_axis or in
        # the floating_axig.GridHelperCurveLinear subclass?
        return axisline

    def new_floating_axis(self, nth_coord, value, axes=None, axis_direction="bottom"):
        if axes is None:
            axes = self.axes
        helper = FloatingAxisArtistHelper(
            self, nth_coord, value, axis_direction)
        axisline = AxisArtist(axes, helper, axis_direction=axis_direction)
        return axisline
