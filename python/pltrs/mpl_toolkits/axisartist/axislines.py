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

import numpy as np

from pltrs import _api, _handles
import pltrs.axes as maxes
from pltrs.path import Path
from pltrs.transforms import Bbox, _interval_contains_close
from pltrs._native import axisartist as _axisartist
from ..axes_grid1 import mpl_axes
from .axis_artist import AxisArtist, GridlinesCollection

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

    def _to_xy(self, values, const):
        """
        Create a (*values.shape, 2)-shape array representing (x, y) pairs.

        The other coordinate is filled with the constant *const*.
        """
        if self.nth_coord == 0:
            return np.stack(np.broadcast_arrays(values, const), axis=-1)
        elif self.nth_coord == 1:
            return np.stack(np.broadcast_arrays(const, values), axis=-1)
        else:
            raise ValueError("Unexpected nth_coord")

    def _rectilinear_ticks(self, axes, const, inside):
        """The major and minor ticks of ``self.axis`` on this line at the
        constant other coordinate *const*, those whose tick coordinates
        pass *inside*, as matplotlib's rectilinear ``get_tick_iterators``
        yield them."""
        angle_normal, angle_tangent = {0: (90, 0), 1: (0, 90)}[self.nth_coord]

        def ticks(level):
            locs = level.locator()
            labels = level.formatter.format_ticks(locs)
            for loc, label in zip(locs, labels):
                c = self._to_xy(loc, const=const)
                if inside(c):
                    yield c, angle_normal, angle_tangent, label

        return ticks(self.axis.major), ticks(self.axis.minor)


class _FixedAxisArtistHelperBase(_AxisArtistHelperBase):
    """Helper class for a fixed (in the axes coordinate) axis."""

    def __init__(self, loc):
        """``nth_coord = 0``: x-axis; ``nth_coord = 1``: y-axis."""
        super().__init__(_api.getitem_checked(
            {"bottom": 0, "top": 0, "left": 1, "right": 1}, loc=loc))
        self._loc = loc
        self._pos = {"bottom": 0, "top": 1, "left": 0, "right": 1}[loc]
        # axis line in transAxes
        self._path = Path(self._to_xy((0, 1), const=self._pos))

    # LINE

    def get_line(self, axes):
        return self._path

    def get_line_transform(self, axes):
        return axes.transAxes

    # LABEL

    def get_axislabel_transform(self, axes):
        return axes.transAxes

    def get_axislabel_pos_angle(self, axes):
        """
        Return the label reference position in transAxes.

        get_label_transform() returns a transform of (transAxes+offset)
        """
        return dict(left=((0., 0.5), 90),  # (position, angle_tangent)
                    right=((1., 0.5), 90),
                    bottom=((0.5, 0.), 0),
                    top=((0.5, 1.), 0))[self._loc]

    # TICK

    def get_tick_transform(self, axes):
        return [axes.get_xaxis_transform(), axes.get_yaxis_transform()][self.nth_coord]

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

    def get_line(self, axes):
        raise RuntimeError("get_line method should be defined by the derived class")

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

    # TICK

    def get_tick_iterators(self, axes):
        """tick_loc, tick_angle, tick_label"""
        tick_to_axes = self.get_tick_transform(axes) - axes.transAxes
        return self._rectilinear_ticks(
            axes, self._pos, lambda c: _interval_contains_close(
                (0, 1), tick_to_axes.transform(c)[self.nth_coord]))


class FloatingAxisArtistHelperRectilinear(_FloatingAxisArtistHelperBase):

    def __init__(self, axes, nth_coord,
                 passingthrough_point, axis_direction="bottom"):
        super().__init__(nth_coord, passingthrough_point)
        self._axis_direction = axis_direction
        self.axis = [axes.xaxis, axes.yaxis][self.nth_coord]

    def _fixed_axes_coord(self, axes):
        """Where the line runs across, in axes coordinates."""
        data_to_axes = axes.transData - axes.transAxes
        p = data_to_axes.transform([self._value, self._value])
        return p[1 - self.nth_coord]

    def get_line(self, axes):
        return Path(self._to_xy((0, 1), const=self._fixed_axes_coord(axes)))

    def get_line_transform(self, axes):
        return axes.transAxes

    def get_axislabel_transform(self, axes):
        return axes.transAxes

    def get_axislabel_pos_angle(self, axes):
        """
        Return the label reference position in transAxes.

        get_label_transform() returns a transform of (transAxes+offset)
        """
        angle = [0, 90][self.nth_coord]
        verts = self._to_xy(0.5, const=self._fixed_axes_coord(axes))
        return ((verts, angle) if 0 <= verts[1 - self.nth_coord] <= 1
                else (None, None))

    def get_tick_transform(self, axes):
        return axes.transData

    def get_tick_iterators(self, axes):
        """tick_loc, tick_angle, tick_label"""
        data_to_axes = axes.transData - axes.transAxes

        def inside(c):
            c1, c2 = data_to_axes.transform(c)
            return 0 <= c1 <= 1 and 0 <= c2 <= 1

        return self._rectilinear_ticks(axes, self._value, inside)


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

    def _update_for_served_axes(self):
        """Update the grid for the limits of the axes the helper serves
        (``update_lim``), as matplotlib's helpers do whenever they lay an
        axis artist out: pltrs (Rust) lays them out without asking."""
        axes = self.__dict__.get("_served_axes")
        if axes is not None:
            self.update_lim(axes)

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

    def get_gridlines(self, which="major", axis="both"):
        """
        Return list of gridline coordinates in data coordinates.

        Parameters
        ----------
        which : {"both", "major", "minor"}
        axis : {"both", "x", "y"}
        """
        _api.check_in_list(["both", "major", "minor"], which=which)
        _api.check_in_list(["both", "x", "y"], axis=axis)
        gridlines = []

        if axis in ("both", "x"):
            locs = []
            y1, y2 = self.axes.get_ylim()
            if which in ("both", "major"):
                locs.extend(self.axes.xaxis.major.locator())
            if which in ("both", "minor"):
                locs.extend(self.axes.xaxis.minor.locator())
            gridlines.extend([[x, x], [y1, y2]] for x in locs)

        if axis in ("both", "y"):
            x1, x2 = self.axes.get_xlim()
            locs = []
            if self.axes.yaxis._major_tick_kw["gridOn"]:
                locs.extend(self.axes.yaxis.major.locator())
            if self.axes.yaxis._minor_tick_kw["gridOn"]:
                locs.extend(self.axes.yaxis.minor.locator())
            gridlines.extend([[x1, x2], [y, y]] for y in locs)

        return gridlines


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
        self._grid_helper._served_axes = self
        self._grid_on = False
        self._load_axislines()

    def _load_axislines(self):
        """Hold the axis artists and the gridlines pltrs (Rust) made for
        the axes."""
        # Known to the figure, as one of the axes' children.
        _gridlines_of(self)
        self._axislines = mpl_axes.Axes.AxisDict(self)
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

    def _hold_axis_artist(self, name, artist):
        """Have pltrs (Rust) draw *artist* as the axis artist *name*, in
        place of the one held there (``ax.axis[name] = artist``)."""
        _axisartist.assign(self._native, name, artist._native)

    def get_children(self):
        if self._axisline_on:
            children = [*self._axislines.values(), self.gridlines]
        else:
            children = []
        # pltrs (Rust) keeps the gridlines among the axes' own artists.
        children.extend(child for child in super().get_children()
                        if not isinstance(child, GridlinesCollection))
        return children

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
        # The style pltrs (Rust) gives the lines of each level and axis;
        # the other properties are the collection's (``gridlines.set``).
        style = {"color": None, "alpha": None, "linewidth": None,
                 "linestyle": None}
        rest = {}
        for key, value in kwargs.items():
            key = {"c": "color", "lw": "linewidth", "ls": "linestyle"}.get(key, key)
            (style if key in style else rest)[key] = value
        gridlines = self.gridlines
        if rest and gridlines is not None:
            gridlines.set(**rest)
        _axisartist.grid(self._native, bool(visible), (which, axis),
                         tuple(style.values()))

    @property
    def gridlines(self):
        """The `.GridlinesCollection` drawing the gridlines, None once
        removed."""
        return _gridlines_of(self)

    def new_fixed_axis(self, loc, offset=None):
        return self.get_grid_helper().new_fixed_axis(loc, offset=offset, axes=self)

    def new_floating_axis(self, nth_coord, value, axis_direction="bottom"):
        return self.get_grid_helper().new_floating_axis(
            nth_coord, value, axis_direction=axis_direction, axes=self)


def _gridlines_of(axes):
    """The `.GridlinesCollection` drawing the gridlines of *axes*, None once
    removed."""
    native = _axisartist.gridlines(axes._native)
    return (None if native is None
            else _handles.wrap(native, GridlinesCollection, axes.figure))


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
