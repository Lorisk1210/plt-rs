"""
An experimental support for curvilinear grid.

matplotlib's ``mpl_toolkits.axisartist.floating_axes`` (from matplotlib
3.11.2, under the Matplotlib License): floating axes show a fixed range of
curvilinear coordinates, bounded by axis artists along its edges, which
pltrs (Rust) lays out and fits the axes' limits around
(``_native.axisartist``).
"""

import numpy as np

from pltrs import _api, cbook, patches as mpatches
from pltrs.transforms import Bbox
from pltrs._native import axisartist as _axisartist
from ..axes_grid1.parasite_axes import host_axes_class_factory
from . import axislines, grid_helper_curvelinear
from .axis_artist import AxisArtist
from .grid_finder import ExtremeFinderSimple
from .grid_helper_curvelinear import _native_aux


class FloatingAxisArtistHelper(
        grid_helper_curvelinear.FloatingAxisArtistHelper):
    pass


class FixedAxisArtistHelper(grid_helper_curvelinear.FloatingAxisArtistHelper):

    def __init__(self, grid_helper, side, nth_coord_ticks=None):
        """
        nth_coord = along which coordinate value varies.
         nth_coord = 0 ->  x axis, nth_coord = 1 -> y axis
        """
        lon1, lon2, lat1, lat2 = grid_helper.grid_finder.extreme_finder(*[None] * 5)
        value, nth_coord = _api.getitem_checked(
            dict(left=(lon1, 0), right=(lon2, 0), bottom=(lat1, 1), top=(lat2, 1)),
            side=side)
        super().__init__(grid_helper, nth_coord, value, axis_direction=side)
        if nth_coord_ticks is None:
            nth_coord_ticks = nth_coord
        self.nth_coord_ticks = nth_coord_ticks
        self._loc = side

    def _bind(self, native):
        super()._bind(native)
        if self.nth_coord_ticks != self.nth_coord:
            _axisartist.set_nth_coord_ticks(native, "xy"[self.nth_coord_ticks])

    def _new_axis(self, axes, offset, axis_direction):
        """The native axis artist of the floating axes *axes* along this
        edge of its range."""
        native = _axisartist.new_fixed_axis(
            axes._native, next(axislines._draft_names), self._loc,
            tuple(map(float, offset or (0, 0))))
        if axis_direction != self._loc:
            _axisartist.set_axis_direction(native, axis_direction)
        return native


class ExtremeFinderFixed(ExtremeFinderSimple):
    # docstring inherited

    def __init__(self, extremes):
        """
        This subclass always returns the same bounding box.

        Parameters
        ----------
        extremes : (float, float, float, float)
            The bounding box that this helper always returns.
        """
        x0, x1, y0, y1 = extremes
        self._tbbox = Bbox.from_extents(x0, y0, x1, y1)

    def __call__(self, transform_xy, x1, y1, x2, y2):
        # docstring inherited
        x0, y0, x1, y1 = self._tbbox.extents
        return x0, x1, y0, y1

    def _find_transformed_bbox(self, trans, bbox):
        # docstring inherited
        return self._tbbox


class GridHelperCurveLinear(grid_helper_curvelinear.GridHelperCurveLinear):

    def __init__(self, aux_trans, extremes,
                 grid_locator1=None,
                 grid_locator2=None,
                 tick_formatter1=None,
                 tick_formatter2=None):
        # docstring inherited
        super().__init__(aux_trans,
                         extreme_finder=ExtremeFinderFixed(extremes),
                         grid_locator1=grid_locator1,
                         grid_locator2=grid_locator2,
                         tick_formatter1=tick_formatter1,
                         tick_formatter2=tick_formatter2)
        self._extremes = tuple(map(float, extremes))

    def _install(self, axes, zero):
        _axisartist.install_floating(axes._native, _native_aux(self._aux_trans),
                                     self._extremes, self._native_finder())

    def _helper_of(self, axes, name):
        return FixedAxisArtistHelper(self, name)

    def new_fixed_axis(
            self, loc, nth_coord=None, axis_direction=None, offset=None, axes=None):
        if axes is None:
            axes = self.axes
        if axis_direction is None:
            axis_direction = loc
        # This is not the same as the FixedAxisArtistHelper class used by
        # grid_helper_curvelinear.GridHelperCurveLinear.new_fixed_axis!
        helper = FixedAxisArtistHelper(
            self, loc, nth_coord_ticks=nth_coord)
        return AxisArtist(axes, helper, axis_direction=axis_direction)

    def _update_grid(self, bbox):
        if self._grid_info is None:
            self._grid_info = dict()

        grid_info = self._grid_info

        grid_finder = self.grid_finder
        tbbox = grid_finder.extreme_finder._find_transformed_bbox(
            grid_finder.get_transform().inverted(), bbox)

        lon_min, lat_min, lon_max, lat_max = tbbox.extents
        grid_info["extremes"] = tbbox

        lon_levs, lon_n, lon_factor = grid_finder.grid_locator1(lon_min, lon_max)
        lon_levs = np.asarray(lon_levs)
        lat_levs, lat_n, lat_factor = grid_finder.grid_locator2(lat_min, lat_max)
        lat_levs = np.asarray(lat_levs)

        grid_info["lon_info"] = lon_levs, lon_n, lon_factor
        grid_info["lat_info"] = lat_levs, lat_n, lat_factor

        grid_info["lon_labels"] = grid_finder._format_ticks(
            1, "bottom", lon_factor, lon_levs)
        grid_info["lat_labels"] = grid_finder._format_ticks(
            2, "bottom", lat_factor, lat_levs)

        lon_values = lon_levs[:lon_n] / lon_factor
        lat_values = lat_levs[:lat_n] / lat_factor

        lon_lines, lat_lines = grid_finder._get_raw_grid_lines(
            lon_values[(lon_min < lon_values) & (lon_values < lon_max)],
            lat_values[(lat_min < lat_values) & (lat_values < lat_max)],
            tbbox)

        grid_info["lon_lines"] = lon_lines
        grid_info["lat_lines"] = lat_lines

        lon_lines, lat_lines = grid_finder._get_raw_grid_lines(
            tbbox.intervalx, tbbox.intervaly, tbbox)

        grid_info["lon_lines0"] = lon_lines
        grid_info["lat_lines0"] = lat_lines

    def get_gridlines(self, which="major", axis="both"):
        self._update_for_served_axes()
        grid_lines = []
        if axis in ["both", "x"]:
            grid_lines.extend(map(np.transpose, self._grid_info["lon_lines"]))
        if axis in ["both", "y"]:
            grid_lines.extend(map(np.transpose, self._grid_info["lat_lines"]))
        return grid_lines


class FloatingAxesBase:

    def __init__(self, *args, grid_helper, **kwargs):
        _api.check_isinstance(GridHelperCurveLinear, grid_helper=grid_helper)
        super().__init__(*args, grid_helper=grid_helper, **kwargs)

    def _gen_axes_patch(self):
        # docstring inherited
        # The region of the extremes, in the coordinates of the grid
        # transform, as matplotlib's floating axes place their patch.
        grid_finder = self.get_grid_helper().grid_finder
        x0, x1, y0, y1 = grid_finder.extreme_finder(*[None] * 5)
        patch = mpatches.Polygon([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
        patch.get_path()._interpolation_steps = 100
        patch.set_transform(grid_finder.get_transform() + self.transData)
        return patch

    def clear(self):
        # docstring inherited
        _axisartist.clear_floating(self._native)
        self._load_axislines()

    def adjust_axes_lim(self):
        """Fit the limits around the range the axes shows."""
        _axisartist.adjust_floating_lim(self._native)


floatingaxes_class_factory = cbook._make_class_factory(FloatingAxesBase, "Floating{}")
FloatingAxes = floatingaxes_class_factory(host_axes_class_factory(axislines.Axes))
FloatingSubplot = FloatingAxes
