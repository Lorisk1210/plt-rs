"""
Axislines includes modified implementation of the Axes class. The
biggest difference is that the artists responsible for drawing the axis spine,
ticks, ticklabels and axis labels are separated out from Matplotlib's Axis
class. Originally, this change was motivated to support curvilinear
grid.

matplotlib's ``mpl_toolkits.axisartist.axislines`` (from matplotlib 3.11.2,
under the Matplotlib License): pltrs (Rust) hides the axes' spines and axes
and draws an axis artist on each edge (``_native.axisartist``); the axis
helpers say where a new axis artist goes, and a grid helper how the grid
lines run.
"""

import itertools

from pltrs import _api
import pltrs.axes as maxes
from pltrs.transforms import Bbox
from pltrs._native import axisartist as _axisartist
from ..axes_grid1 import mpl_axes
from .axis_artist import AxisArtist

#: Names for axis artists made before an axes holds them under a key.
_draft_names = (f"_pltrs_draft{n}" for n in itertools.count())


class _AxisArtistHelperBase:
    """
    Base class for axis helper.

    Subclasses should define the methods listed below.  The *axes*
    argument will be the ``.axes`` attribute of the caller artist.
    """

    def __init__(self, nth_coord):
        self.nth_coord = nth_coord

    def update_lim(self, axes):
        pass

    def get_nth_coord(self):
        return self.nth_coord


class _FixedAxisArtistHelperBase(_AxisArtistHelperBase):
    """Helper class for a fixed (in the axes coordinate) axis."""

    def __init__(self, loc):
        """``nth_coord = 0``: x-axis; ``nth_coord = 1``: y-axis."""
        super().__init__(_api.getitem_checked(
            {"bottom": 0, "top": 0, "left": 1, "right": 1}, loc=loc))
        self._loc = loc

    def _new_axis(self, axes, offset, axis_direction):
        """The native axis artist of *axes* along this edge."""
        native = _axisartist.new_fixed_axis(
            axes._native, next(_draft_names), self._loc,
            tuple(map(float, offset or (0, 0))))
        if axis_direction != self._loc:
            _axisartist.set_axis_direction(native, axis_direction)
        return native


class _FloatingAxisArtistHelperBase(_AxisArtistHelperBase):
    def __init__(self, nth_coord, value):
        self._value = value
        super().__init__(nth_coord)

    def _new_axis(self, axes, offset, axis_direction):
        """The native axis artist of *axes* along this line."""
        return _axisartist.new_floating_axis(
            axes._native, next(_draft_names),
            ("xy"[self.nth_coord], float(self._value)), axis_direction)


class FixedAxisArtistHelperRectilinear(_FixedAxisArtistHelperBase):

    def __init__(self, axes, loc):
        """
        nth_coord = along which coordinate value varies
        in 2D, nth_coord = 0 ->  x axis, nth_coord = 1 -> y axis
        """
        super().__init__(loc)
        self.axis = [axes.xaxis, axes.yaxis][self.nth_coord]


class FloatingAxisArtistHelperRectilinear(_FloatingAxisArtistHelperBase):

    def __init__(self, axes, nth_coord,
                 passingthrough_point, axis_direction="bottom"):
        super().__init__(nth_coord, passingthrough_point)
        self._axis_direction = axis_direction
        self.axis = [axes.xaxis, axes.yaxis][self.nth_coord]


class AxisArtistHelper:  # Backcompat.
    Fixed = _FixedAxisArtistHelperBase
    Floating = _FloatingAxisArtistHelperBase


class AxisArtistHelperRectlinear:  # Backcompat.
    Fixed = FixedAxisArtistHelperRectilinear
    Floating = FloatingAxisArtistHelperRectilinear


class GridHelperBase:

    def __init__(self):
        self._old_limits = None
        super().__init__()

    def update_lim(self, axes):
        x1, x2 = axes.get_xlim()
        y1, y2 = axes.get_ylim()
        if self._old_limits != (x1, x2, y1, y2):
            self._update_grid(Bbox.from_extents(x1, y1, x2, y2))
            self._old_limits = (x1, x2, y1, y2)

    def _update_grid(self, bbox):
        """Cache relevant computations when the axes limits have changed."""

    def get_gridlines(self, which, axis):
        """
        Return list of grid lines as a list of paths (list of points).

        Parameters
        ----------
        which : {"both", "major", "minor"}
        axis : {"both", "x", "y"}
        """
        return []

    def _install(self, axes, zero):
        """Make *axes* an axes drawn by axis artists through this helper
        (with the hidden zero axes of `AxesZero` when *zero*)."""
        _axisartist.install(axes._native, zero)

    def _helper_of(self, axes, name):
        """The axis helper of the axis artist *name* pltrs (Rust) made."""
        if name in ("xzero", "yzero"):
            nth = 0 if name == "xzero" else 1
            return FloatingAxisArtistHelperRectilinear(
                axes, nth, 0., "bottom" if nth == 0 else "left")
        return FixedAxisArtistHelperRectilinear(axes, name)


class GridHelperRectlinear(GridHelperBase):

    def __init__(self, axes):
        super().__init__()
        self.axes = axes

    def new_fixed_axis(
        self, loc, *, axis_direction=None, offset=None, axes=None
    ):
        if axes is None:
            _api.warn_external(
                "'new_fixed_axis' explicitly requires the axes keyword.")
            axes = self.axes
        if axis_direction is None:
            axis_direction = loc
        return AxisArtist(axes, FixedAxisArtistHelperRectilinear(axes, loc),
                          offset=offset, axis_direction=axis_direction)

    def new_floating_axis(self, nth_coord, value, axis_direction="bottom", axes=None):
        if axes is None:
            _api.warn_external(
                "'new_floating_axis' explicitly requires the axes keyword.")
            axes = self.axes
        helper = FloatingAxisArtistHelperRectilinear(
            axes, nth_coord, value, axis_direction)
        return AxisArtist(axes, helper, axis_direction=axis_direction)


class _AxisDict(mpl_axes.Axes.AxisDict):
    """The axis artists of an axes by name: holding one under a name makes
    pltrs (Rust) draw it there, in place of the one held before."""

    def __setitem__(self, k, v):
        _axisartist.assign(self.axes._native, k, v._native)
        super().__setitem__(k, v)


class Axes(maxes.Axes):

    def __init__(self, *args, grid_helper=None, **kwargs):
        self._axisline_on = True
        self._grid_helper = grid_helper if grid_helper else GridHelperRectlinear(self)
        super().__init__(*args, **kwargs)

    @classmethod
    def _make_native(cls, figure, kwargs, *, spec=None, rect=None):
        return cls._constructed_native(figure, kwargs, spec=spec, rect=rect)

    _zero_axes = False

    def _init_axes_state(self):
        super()._init_axes_state()
        if "_grid_helper" not in self.__dict__:
            self._axisline_on = True
            self._grid_helper = GridHelperRectlinear(self)
        self._grid_helper._install(self, self._zero_axes)
        self._grid_on = False
        self._load_axislines()

    def _load_axislines(self):
        """Hold the axis artists pltrs (Rust) made for the axes."""
        self._axislines = _AxisDict(self)
        for name, native in _axisartist.axis_artists(self._native):
            helper = self._grid_helper._helper_of(self, name)
            direction = getattr(helper, "_axis_direction", name)
            dict.__setitem__(self._axislines, name,
                             AxisArtist._of_axes(self, native, helper, direction))

    def _aux_parasite_native(self, parent_axes, aux_transform, viewlim_mode):
        """The native parasite of *parent_axes*, an axes drawn by axis
        artists, plotting in the coordinates *aux_transform* maps to its
        data coordinates (``get_aux_axes(tr)``)."""
        from .grid_helper_curvelinear import _native_aux
        return _axisartist.aux_axes(parent_axes._native, _native_aux(aux_transform))

    def toggle_axisline(self, b=None):
        if b is None:
            b = not self._axisline_on
        if b:
            self._axisline_on = True
            self.spines[:].set_visible(False)
            self.xaxis.set_visible(False)
            self.yaxis.set_visible(False)
        else:
            self._axisline_on = False
            self.spines[:].set_visible(True)
            self.xaxis.set_visible(True)
            self.yaxis.set_visible(True)

    @property
    def axis(self):
        return self._axislines

    def clear(self):
        # docstring inherited
        _axisartist.clear(self._native)
        self._load_axislines()

    def get_grid_helper(self):
        return self._grid_helper

    def grid(self, visible=None, which='major', axis="both", **kwargs):
        """
        Toggle the gridlines, and optionally set the properties of the lines.
        """
        _api.check_in_list(["major", "minor", "both"], which=which)
        _api.check_in_list(["x", "y", "both"], axis=axis)
        if visible is None:
            visible = True if kwargs else not self._grid_on
        self._grid_on = bool(visible)
        style = {"color": None, "alpha": None, "linewidth": None,
                 "linestyle": None}
        for key, value in kwargs.items():
            key = {"c": "color", "lw": "linewidth", "ls": "linestyle"}.get(key, key)
            _api.check_in_list(list(style), keyword=key)
            style[key] = value
        _axisartist.grid(self._native, bool(visible), (which, axis),
                         tuple(style.values()))

    def new_fixed_axis(self, loc, offset=None):
        return self.get_grid_helper().new_fixed_axis(loc, offset=offset, axes=self)

    def new_floating_axis(self, nth_coord, value, axis_direction="bottom"):
        return self.get_grid_helper().new_floating_axis(
            nth_coord, value, axis_direction=axis_direction, axes=self)


class AxesZero(Axes):

    _zero_axes = True

    def clear(self):
        super().clear()
        for name, nth, direction in (("xzero", 0, "bottom"),
                                     ("yzero", 1, "left")):
            self._axislines[name] = self.new_floating_axis(
                nth_coord=nth, value=0., axis_direction=direction)
            self._axislines[name].set_visible(False)


Subplot = Axes
SubplotZero = AxesZero
