"""
Grid levels and labels for angles in degrees or hours, with minutes and
seconds, and the extreme finder of cyclic coordinates (matplotlib's
``mpl_toolkits.axisartist.angle_helper``, from matplotlib 3.11.2, under the
Matplotlib License, with the steps, labels and extremes computed by pltrs
(Rust); labels of values that are no finite numbers matplotlib's own
formatting computes, failing where it fails).
"""

import math
from numbers import Real

import numpy as np

from pltrs._native import axisartist as _axisartist
from pltrs.transforms import Bbox
from pltrs.mpl_toolkits.axisartist.grid_finder import (
    ExtremeFinderSimple, _transformed_grid)


def _plain(*numbers):
    """Whether *numbers* are all finite real numbers, which pltrs (Rust)
    computes the steps for in double precision; matplotlib's own
    computation (failing where it fails, and comparing numpy floats of
    another precision in theirs) takes anything else."""
    return all(isinstance(n, Real) and np.isfinite(n)
               and not (isinstance(n, np.floating) and n.dtype != np.float64)
               for n in numbers)


def select_step_degree(dv):
    if _plain(dv) and dv <= _DEGREE_STEPS[0][-1]:
        return _axisartist.select_step_degree(float(dv))
    return _select_step_from(_DEGREE_STEPS, dv)


def select_step_hour(dv):
    if _plain(dv) and dv <= _HOUR_STEPS[0][-1]:
        return _axisartist.select_step_hour(float(dv))
    return _select_step_from(_HOUR_STEPS, dv)


def select_step_sub(dv):
    if _plain(dv) and dv > 0:
        return _axisartist.select_step_sub(float(dv))
    # subarcsec or degree
    tmp = 10.**(int(math.log10(dv))-1.)
    factor = 1./tmp
    if 1.5*tmp >= dv:
        step = 1
    elif 3.*tmp >= dv:
        step = 2
    elif 7.*tmp >= dv:
        step = 5
    else:
        step = 1
        factor = 0.1*factor
    return step, factor


def _steps(limits, steps, minsec_limits, minsec_steps):
    """matplotlib's table of the steps of a unit (degrees, hours) and of its
    minutes and seconds: their limits, steps and factors."""
    minute_limits_ = np.array(minsec_limits) / 60
    second_limits_ = np.array(minsec_limits) / 3600
    return ([*second_limits_, *minute_limits_, *limits],
            [*minsec_steps, *minsec_steps, *steps],
            [*[3600.] * len(second_limits_), *[60.] * len(minute_limits_),
             *[1.] * len(steps)])


_DEGREE_STEPS = _steps([1.5, 3, 7, 13, 20, 40, 70, 120, 270, 520],
                       [1, 2, 5, 10, 15, 30, 45, 90, 180, 360],
                       [1.5, 2.5, 3.5, 8, 11, 18, 25, 45],
                       [1, 2, 3, 5, 10, 15, 20, 30])
_HOUR_STEPS = _steps([1.5, 2.5, 3.5, 5, 7, 10, 15, 21, 36],
                     [1, 2, 3, 4, 6, 8, 12, 18, 24],
                     [1.5, 2.5, 3.5, 4.5, 5.5, 8, 11, 14, 18, 25, 45],
                     [1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30])


def _select_step_from(table, dv):
    """matplotlib's step and factor for intervals of *dv* in *table*."""
    limits, steps, factors = table
    n = np.searchsorted(limits, dv)
    step = steps[n]
    factor = factors[n]
    return step, factor


def select_step(v1, v2, nv, hour=False, include_last=True,
                threshold_factor=3600.):
    if _steps_found(v1, v2, nv, hour, threshold_factor):
        levs, n, factor = _axisartist.select_step(
            float(v1), float(v2), int(nv), bool(hour), bool(include_last),
            float(threshold_factor))
        return levs, n, factor
    return _select_step_as_given(v1, v2, nv, hour, include_last, threshold_factor)


def _steps_found(v1, v2, nv, hour, threshold_factor):
    """Whether pltrs (Rust) finds the steps for these arguments: finite
    numbers, a whole positive number of intervals, and a step in the tables
    (or a positive sub-unit step)."""
    if not (_plain(v1, v2, nv, threshold_factor) and nv > 0
            and float(nv).is_integer() and threshold_factor != 0):
        return False
    dv = abs(v2 - v1) / nv
    if dv > 1 / threshold_factor:
        return dv <= (_HOUR_STEPS if hour else _DEGREE_STEPS)[0][-1]
    return dv * threshold_factor > 0


def _select_step_as_given(v1, v2, nv, hour, include_last, threshold_factor):
    """matplotlib's steps themselves, for what pltrs (Rust) does not take:
    computed as matplotlib's computes them, failing where they fail."""
    if v1 > v2:
        v1, v2 = v2, v1

    dv = (v2 - v1) / nv

    if hour:
        _select_step = select_step_hour
        cycle = 24.
    else:
        _select_step = select_step_degree
        cycle = 360.

    # for degree
    if dv > 1 / threshold_factor:
        step, factor = _select_step(dv)
    else:
        step, factor = select_step_sub(dv*threshold_factor)

        factor = factor * threshold_factor

    levs = np.arange(np.floor(v1 * factor / step),
                     np.ceil(v2 * factor / step) + 0.5,
                     dtype=int) * step

    n = len(levs)

    if factor == 1. and levs[-1] >= levs[0] + cycle:  # check for cycle
        nv = int(cycle / step)
        if include_last:
            levs = levs[0] + np.arange(0, nv+1, 1) * step
        else:
            levs = levs[0] + np.arange(0, nv, 1) * step

        n = len(levs)

    return np.array(levs), n, factor


def select_step24(v1, v2, nv, include_last=True, threshold_factor=3600):
    v1, v2 = v1 / 15, v2 / 15
    levs, n, factor = select_step(v1, v2, nv, hour=True,
                                  include_last=include_last,
                                  threshold_factor=threshold_factor)
    return levs * 15, n, factor


def select_step360(v1, v2, nv, include_last=True, threshold_factor=3600):
    return select_step(v1, v2, nv, hour=False,
                       include_last=include_last,
                       threshold_factor=threshold_factor)


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

    def _get_number_fraction(self, factor):
        ## check for fractional numbers
        number_fraction = None
        # check for 60

        for threshold in [1, 60, 3600]:
            if factor <= threshold:
                break

            d = factor // threshold
            int_log_d = int(np.floor(np.log10(d)))
            if 10**int_log_d == d and d != 1:
                number_fraction = int_log_d
                factor = factor // 10**int_log_d
                return factor, number_fraction

        return factor, number_fraction

    def __call__(self, direction, factor, values):
        return self._labels(factor, values, values)

    def _labels(self, factor, values, given):
        """The labels of *values* (in this formatter's unit) at *factor*:
        pltrs (Rust) formats finite numbers (from *given*, the values as
        the program gave them); anything else matplotlib formats as it
        computes with it, failing where it fails (`_labels_as_given`)."""
        if len(values) == 0:
            return []
        if (isinstance(factor, Real) and np.isfinite(factor)
                and np.asarray(values).dtype.kind in "iuf"
                and np.isfinite(values).all() and factor in (1, 60, 3600)):
            return self._format(float(factor), [float(v) for v in given])
        return self._labels_as_given(factor, values)

    def _labels_as_given(self, factor, values):
        """matplotlib's labels of *values* at *factor* themselves."""
        ss = np.sign(values)
        signs = ["-" if v < 0 else "" for v in values]

        factor, number_fraction = self._get_number_fraction(factor)

        values = np.abs(values)

        if number_fraction is not None:
            values, frac_part = divmod(values, 10 ** number_fraction)
            frac_fmt = "%%0%dd" % (number_fraction,)
            frac_str = [frac_fmt % (f1,) for f1 in frac_part]

        if factor == 1:
            if number_fraction is None:
                return [self.fmt_d % (s * int(v),) for s, v in zip(ss, values)]
            else:
                return [self.fmt_ds % (s * int(v), f1)
                        for s, v, f1 in zip(ss, values, frac_str)]
        elif factor == 60:
            deg_part, min_part = divmod(values, 60)
            if number_fraction is None:
                return [self.fmt_d_m % (s1, d1, m1)
                        for s1, d1, m1 in zip(signs, deg_part, min_part)]
            else:
                return [self.fmt_d_ms % (s, d1, m1, f1)
                        for s, d1, m1, f1
                        in zip(signs, deg_part, min_part, frac_str)]

        elif factor == 3600:
            if ss[-1] == -1:
                inverse_order = True
                values = values[::-1]
                signs = signs[::-1]
            else:
                inverse_order = False

            l_hm_old = ""
            r = []

            deg_part, min_part_ = divmod(values, 3600)
            min_part, sec_part = divmod(min_part_, 60)

            if number_fraction is None:
                sec_str = [self.fmt_s_partial % (s1,) for s1 in sec_part]
            else:
                sec_str = [self.fmt_ss_partial % (s1, f1)
                           for s1, f1 in zip(sec_part, frac_str)]

            for s, d1, m1, s1 in zip(signs, deg_part, min_part, sec_str):
                l_hm = self.fmt_d_m_partial % (s, d1, m1)
                if l_hm != l_hm_old:
                    l_hm_old = l_hm
                    l = l_hm + s1
                else:
                    l = "$" + s + s1
                r.append(l)

            if inverse_order:
                return r[::-1]
            else:
                return r

        else:  # factor > 3600.
            return [r"$%s^{\circ}$" % v for v in ss*values]


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

    def __call__(self, direction, factor, values):  # hour
        # (pltrs (Rust) formats the values given as hours, ``values / 15``.)
        return self._labels(factor, np.asarray(values) / 15, values)


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


