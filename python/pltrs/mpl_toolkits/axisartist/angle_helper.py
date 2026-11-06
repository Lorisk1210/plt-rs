"""
Grid levels and labels for angles in degrees or hours, with minutes and
seconds, and the extreme finder of cyclic coordinates (matplotlib's
``mpl_toolkits.axisartist.angle_helper``, from matplotlib 3.11.2, under the
Matplotlib License, with the steps, labels and extremes computed by pltrs
(Rust)).
"""

from pltrs._native import axisartist as _axisartist
from pltrs.transforms import Bbox
from pltrs.mpl_toolkits.axisartist.grid_finder import (
    ExtremeFinderSimple, _transformed_grid)


def select_step_degree(dv):
    return _axisartist.select_step_degree(float(dv))


def select_step_hour(dv):
    return _axisartist.select_step_hour(float(dv))


def select_step_sub(dv):
    return _axisartist.select_step_sub(float(dv))


def select_step(v1, v2, nv, hour=False, include_last=True,
                threshold_factor=3600.):
    levs, n, factor = _axisartist.select_step(
        float(v1), float(v2), int(nv), bool(hour), bool(include_last),
        float(threshold_factor))
    return levs, n, factor


def select_step24(v1, v2, nv, include_last=True, threshold_factor=3600):
    return _axisartist.select_step24(
        float(v1), float(v2), int(nv), bool(include_last),
        float(threshold_factor))


def select_step360(v1, v2, nv, include_last=True, threshold_factor=3600):
    return _axisartist.select_step360(
        float(v1), float(v2), int(nv), bool(include_last),
        float(threshold_factor))


class LocatorBase:
    def __init__(self, nbins, include_last=True):
        self.nbins = nbins
        self._include_last = include_last

    def set_params(self, nbins=None):
        if nbins is not None:
            self.nbins = int(nbins)


class LocatorHMS(LocatorBase):
    def __call__(self, v1, v2):
        return select_step24(v1, v2, self.nbins, self._include_last)


class LocatorHM(LocatorBase):
    def __call__(self, v1, v2):
        return select_step24(v1, v2, self.nbins, self._include_last,
                             threshold_factor=60)


class LocatorH(LocatorBase):
    def __call__(self, v1, v2):
        return select_step24(v1, v2, self.nbins, self._include_last,
                             threshold_factor=1)


class LocatorDMS(LocatorBase):
    def __call__(self, v1, v2):
        return select_step360(v1, v2, self.nbins, self._include_last)


class LocatorDM(LocatorBase):
    def __call__(self, v1, v2):
        return select_step360(v1, v2, self.nbins, self._include_last,
                              threshold_factor=60)


class LocatorD(LocatorBase):
    def __call__(self, v1, v2):
        return select_step360(v1, v2, self.nbins, self._include_last,
                              threshold_factor=1)


class FormatterDMS:
    deg_mark = r"^{\circ}"
    min_mark = r"^{\prime}"
    sec_mark = r"^{\prime\prime}"

    fmt_d = "$%d" + deg_mark + "$"
    fmt_ds = r"$%d.%s" + deg_mark + "$"

    # %s for sign
    fmt_d_m = r"$%s%d" + deg_mark + r"\,%02d" + min_mark + "$"
    fmt_d_ms = r"$%s%d" + deg_mark + r"\,%02d.%s" + min_mark + "$"

    fmt_d_m_partial = "$%s%d" + deg_mark + r"\,%02d" + min_mark + r"\,"
    fmt_s_partial = "%02d" + sec_mark + "$"
    fmt_ss_partial = "%02d.%s" + sec_mark + "$"

    _format = staticmethod(_axisartist.format_dms)

    def __call__(self, direction, factor, values):
        return self._format(float(factor), [float(v) for v in values])


class FormatterHMS(FormatterDMS):
    deg_mark = r"^\mathrm{h}"
    min_mark = r"^\mathrm{m}"
    sec_mark = r"^\mathrm{s}"

    fmt_d = "$%d" + deg_mark + "$"
    fmt_ds = r"$%d.%s" + deg_mark + "$"

    # %s for sign
    fmt_d_m = r"$%s%d" + deg_mark + r"\,%02d" + min_mark+"$"
    fmt_d_ms = r"$%s%d" + deg_mark + r"\,%02d.%s" + min_mark+"$"

    fmt_d_m_partial = "$%s%d" + deg_mark + r"\,%02d" + min_mark + r"\,"
    fmt_s_partial = "%02d" + sec_mark + "$"
    fmt_ss_partial = "%02d.%s" + sec_mark + "$"

    _format = staticmethod(_axisartist.format_hms)


class ExtremeFinderCycle(ExtremeFinderSimple):
    # docstring inherited

    def __init__(self, nx, ny,
                 lon_cycle=360., lat_cycle=None,
                 lon_minmax=None, lat_minmax=(-90, 90)):
        """
        This subclass handles the case where one or both coordinates should be
        taken modulo 360, or be restricted to not exceed a specific range.

        Parameters
        ----------
        nx, ny : int
            The number of samples in each direction.

        lon_cycle, lat_cycle : 360 or None
            If not None, values in the corresponding direction are taken modulo
            *lon_cycle* or *lat_cycle*; in theory this can be any number but
            the implementation actually assumes that it is 360 (if not None);
            other values give nonsensical results.

            This is done by "unwrapping" the transformed grid coordinates so
            that jumps are less than a half-cycle; then normalizing the span to
            no more than a full cycle.

            For example, if values are in the union of the [0, 2] and
            [358, 360] intervals (typically, angles measured modulo 360), the
            values in the second interval are normalized to [-2, 0] instead so
            that the values now cover [-2, 2].  If values are in a range of
            [5, 1000], this gets normalized to [5, 365].

        lon_minmax, lat_minmax : (float, float) or None
            If not None, the computed bounding box is clipped to the given
            range in the corresponding direction.
        """
        self.nx, self.ny = nx, ny
        self.lon_cycle, self.lat_cycle = lon_cycle, lat_cycle
        self.lon_minmax = lon_minmax
        self.lat_minmax = lat_minmax

    def _find_transformed_bbox(self, trans, bbox):
        # docstring inherited
        lon, lat = _transformed_grid(trans, bbox, self.nx, self.ny).T
        return Bbox.from_extents(*_axisartist.cycle_extent(
            lon, lat, (self.nx, self.ny), (self.lon_cycle, self.lat_cycle),
            (self.lon_minmax, self.lat_minmax)))
