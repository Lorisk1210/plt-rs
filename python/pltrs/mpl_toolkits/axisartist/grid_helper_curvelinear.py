"""
An experimental support for curvilinear grid.

matplotlib's ``mpl_toolkits.axisartist.grid_helper_curvelinear`` (from
matplotlib 3.11.2, under the Matplotlib License): pltrs (Rust) lays the
grid lines, ticks and axis artists out along the aux transform, asking the
helper's `.GridFinder` (its extreme finder, locators and formatters) for
the range, levels and labels (``_native.axisartist``).
"""

import numbers
import weakref

import numpy as np

from pltrs import _unusable
from pltrs.path import Path
from pltrs.transforms import (
    Affine2D, Bbox, CompositeGenericTransform, IdentityTransform)
from pltrs._native import axisartist as _axisartist
from .axislines import (
    _FixedAxisArtistHelperBase, _FloatingAxisArtistHelperBase, GridHelperBase)
from .axis_artist import AxisArtist
from .grid_finder import GridFinder


def _xy_function(function):
    """*function*, mapping the arrays ``(x, y)``, as pltrs (Rust) calls it:
    once with all the points a grid line or a sampling of the plane needs,
    answering with a ``(2, N)`` array."""
    def call(x, y):
        with np.errstate(all="ignore"):
            u, v = function(x, y)
        return np.array(np.broadcast_arrays(u, v), dtype=float)
    return call


def _native_aux(aux_trans):
    """The native description of the aux transform *aux_trans*: a
    transform, or a ``(forward, inverse)`` pair of functions of ``(x, y)``."""
    if isinstance(aux_trans, tuple):
        forward, inverse = aux_trans
        return ("functions", _xy_function(forward), _xy_function(inverse))
    if aux_trans.is_affine:
        return ("affine", np.asarray(aux_trans.get_matrix(), dtype=float))
    if isinstance(aux_trans, CompositeGenericTransform):
        return ("then", _native_aux(aux_trans._a), _native_aux(aux_trans._b))
    if type(aux_trans).__name__ == "PolarTransform":
        return ("polar",)
    inverse = aux_trans.inverted()

    def mapped(transform):
        return lambda x, y: transform.transform(np.column_stack([x, y])).T
    return ("functions", _xy_function(mapped(aux_trans)),
            _xy_function(mapped(inverse)))


def _reads(axes, name):
    """Read the attribute *name* of *axes*, as matplotlib's helpers read it
    first (failing as theirs do for no Axes)."""
    getattr(axes, name)


# What the helper of an axis artist says of its ticks, line and label
# (matplotlib's ``get_tick_iterators``, ``get_line`` and
# ``get_axislabel_pos_angle``): pltrs (Rust) computes them for the axis artist
# the helper serves, in its axes' current state. The helper classes define the
# public methods again as their own, as matplotlib's do; no docstring, so that
# ``help`` shows their bases'.
class _NativeHelper:

    def _served(self):
        """The native axis artist the helper serves."""
        if self._artist_native is None:
            raise RuntimeError(
                "the helper serves no axis artist yet (new_fixed_axis and "
                "new_floating_axis make the axis artists of helpers)")
        return self._artist_native

    def get_tick_iterators(self, axes):
        # tick_loc, tick_angle, tick_label
        major, minor = _axisartist.helper_ticks(self._served())

        def ticks(level):
            for xy, normal, tangent, label in level:
                yield list(xy), normal, tangent, label

        # matplotlib's curvilinear helpers have no minor ticks to yield.
        return ticks(major), iter(list(ticks(minor)))


def _coordinate_name(nth):
    """The name of the coordinate *nth* (``"x"`` for 0, ``"y"`` for 1,
    numpy integers and truth values too), None for any other value."""
    if isinstance(nth, (numbers.Integral, np.bool_)) and nth in (0, 1):
        return "xy"[int(nth)]
    return None


def _keep_ticks_coordinate(artist, nth):
    """Have the axis artist *artist* show the ticks of the coordinate
    *nth*, kept as given: matplotlib's draws (and measures) of the artist
    fail on any other value than 0 or 1, picking the tick information of
    the coordinate and of the other one (``["lon", "lat"][nth]``)."""
    name = _coordinate_name(nth)
    if name is not None:
        _axisartist.set_nth_coord_ticks(artist._native, name)
        _unusable.drop(artist, "nth_coord_ticks")
        return

    def use(nth):
        for coordinate in (nth, 1 - nth):
            ["lon", "lat"][coordinate]

    _unusable.keep(artist, "nth_coord_ticks", nth, use)


class FixedAxisArtistHelper(_NativeHelper, _FixedAxisArtistHelperBase):
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

    def _bind(self, artist):
        """Act on the axis artist *artist* from now on."""
        self._artist_native = artist._native
        self._artist = weakref.ref(artist)
        if self._nth_coord_ticks != self.nth_coord:
            self.nth_coord_ticks = self._nth_coord_ticks

    def __getstate__(self):
        # The weak reference to the axis artist pickles as the artist.
        state = self.__dict__.copy()
        if "_artist" in state:
            state["_artist"] = state["_artist"]()
        return state

    def __setstate__(self, state):
        artist = state.pop("_artist", None)
        self.__dict__.update(state)
        if artist is not None:
            self._artist = weakref.ref(artist)

    @property
    def nth_coord_ticks(self):
        """The coordinate whose ticks the axis shows (0 or 1)."""
        return self._nth_coord_ticks

    @nth_coord_ticks.setter
    def nth_coord_ticks(self, nth):
        self._nth_coord_ticks = nth
        if self._artist_native is not None:
            _keep_ticks_coordinate(self._artist(), nth)

    def get_tick_transform(self, axes):
        return axes.transData

    def update_lim(self, axes):
        self.grid_helper.update_lim(axes)

    def get_tick_iterators(self, axes):
        """tick_loc, tick_angle, tick_label"""
        # matplotlib's reads the limits along the axis first.
        (axes.get_ylim if self.nth_coord == 0 else axes.get_xlim)()
        return super().get_tick_iterators(axes)


class FloatingAxisArtistHelper(_NativeHelper, _FloatingAxisArtistHelperBase):

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

    def _bind(self, artist):
        """Act on the axis artist *artist* from now on."""
        self._artist_native = artist._native

    def set_extremes(self, e1, e2):
        if e1 is None:
            e1 = -np.inf
        if e2 is None:
            e2 = np.inf
        self._extremes = e1, e2
        if self._artist_native is not None:
            _axisartist.set_extremes(self._artist_native, (float(e1), float(e2)))

    def get_axislabel_transform(self, axes):
        return Affine2D()  # axes.transData

    def get_axislabel_pos_angle(self, axes):
        _reads(axes, "transData")
        position = _axisartist.helper_label_position(self._served())
        if position is None:
            return None, None
        xy, angle = position
        return np.array(xy), np.float64(angle)

    def get_tick_transform(self, axes):
        return IdentityTransform()  # axes.transData

    def get_line_transform(self, axes):
        return axes.transData

    def get_line(self, axes):
        self.update_lim(axes)
        vertices, codes = _axisartist.helper_line(self._served())
        return Path(vertices, codes)

    def update_lim(self, axes):
        self.grid_helper.update_lim(axes)
        # matplotlib's finds the range of the line in the limits, then the
        # points of the line along its coordinate (none but 0 or 1).
        axes.get_xlim()
        axes.get_ylim()
        if _coordinate_name(self.nth_coord) is None:
            raise UnboundLocalError(
                "cannot access local variable 'xys' where it is not associated "
                "with a value")

    def get_tick_iterators(self, axes):
        """tick_loc, tick_angle, tick_label, (optionally) tick_label"""
        _reads(axes, "transData")
        return super().get_tick_iterators(axes)


class _FinderPart:
    """A part of a grid finder as pltrs (Rust) calls it, reading the grid
    finder anew at each call (*index*: of the coordinate it serves); an
    object rather than a closure, so that it pickles with its figure."""

    def __init__(self, finder, index=None):
        self.finder = finder
        self.index = index


class _Extremes(_FinderPart):
    """The range of the curvilinear coordinates covering a box: the view
    of the Axes pltrs (Rust) lays the grid of *helper* out over, which the
    helper records as matplotlib's ``update_lim`` does (``_old_limits``)."""

    def __init__(self, finder, helper):
        super().__init__(finder)
        self.helper = helper

    def __call__(self, x0, y0, x1, y1):
        finder = self.finder
        helper = self.helper
        helper._old_limits = tuple(map(np.float64, (x0, x1, y0, y1)))
        # Computed for those limits when the program reads it.
        helper._grid_info = None
        return tuple(map(float, finder.extreme_finder._find_transformed_bbox(
            finder.get_transform().inverted(),
            Bbox.from_extents(x0, y0, x1, y1)).extents))


class _Levels(_FinderPart):
    """The levels of a coordinate: ``(levels, n, factor)``."""

    def __call__(self, v1, v2):
        levels, n, factor = getattr(self.finder, f"grid_locator{self.index}")(v1, v2)
        return ([float(level) for level in np.asarray(levels).ravel()],
                int(n), float(factor))


class _Labels(_FinderPart):
    """The labels of the levels of a coordinate."""

    def __call__(self, direction, factor, values):
        return [str(label) for label in self.finder._format_ticks(
            self.index, direction, factor, values)]


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
        try:
            self.grid_finder.update(**kwargs)
            self._old_limits = None  # Force revalidation.
        finally:
            # The transform stays updated when a property fails after it.
            axes = self.__dict__.get("_served_axes")
            if axes is not None:
                _axisartist.update_grid_finder(
                    axes._native, None if aux_trans is None else _native_aux(aux_trans))

    def _native_finder(self):
        """The parts of the grid finder as pltrs (Rust) calls them; they
        read the grid finder anew at each call."""
        finder = self.grid_finder
        return (_Extremes(finder, self), (_Levels(finder, 1), _Levels(finder, 2)),
                (_Labels(finder, 1), _Labels(finder, 2)))

    def _install(self, axes, zero):
        _axisartist.install_curvilinear(axes._native, _native_aux(self._aux_trans),
                                        self._native_finder())

    def _helper_of(self, axes, name):
        return FixedAxisArtistHelper(self, name)

    def _update_grid(self, bbox):
        self._grid_info = self.grid_finder.get_grid_info(bbox)

    def get_gridlines(self, which="major", axis="both"):
        self._update_for_served_axes()
        if self._grid_info is None and self._old_limits is not None:
            # The limits pltrs (Rust) laid the grid out for.
            x1, x2, y1, y2 = self._old_limits
            self._update_grid(Bbox.from_extents(x1, y1, x2, y2))
        grid_lines = []
        if axis in ["both", "x"]:
            grid_lines.extend([gl.T for gl in self._grid_info["lon"]["lines"]])
        if axis in ["both", "y"]:
            grid_lines.extend([gl.T for gl in self._grid_info["lat"]["lines"]])
        return grid_lines

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
        # As matplotlib's: the axis artist faces the default direction
        # (pltrs (Rust) clips its line to the axes box).
        axisline = AxisArtist(axes, helper)
        return axisline


# What matplotlib's module also has from its imports, which programs
# import from it.
import functools  # noqa: E402, F401
import pltrs as mpl  # noqa: E402, F401
