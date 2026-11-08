"""
The computations of the locators pltrs (Rust) places ticks for, as
matplotlib's ``ticker`` writes them (from matplotlib 3.11.2, under the
Matplotlib License).

matplotlib's locators keep their parameters as given and compute with
them, and with the limits they are asked for, in Python. pltrs (Rust)
computes from numbers it holds; where a parameter or a limit is none
(a string, None, a tuple: see `.ticker._NativeLocator`), the locator
computes as matplotlib's does, with these, which fail (or compute) as
matplotlib's do. Each takes the locator as its first argument, as the
method of matplotlib's class it is.
"""

import math

import numpy as np

from . import _api


def _nonsingular(*args, **kwargs):
    """``mtransforms._nonsingular`` (imported when first needed)."""
    from .transforms import _nonsingular
    return _nonsingular(*args, **kwargs)


def _rc(key):
    """The rcParam *key* (imported when first needed)."""
    from . import rcParams
    return rcParams[key]


def _decade_less_equal(x, base):
    """The largest integer power of *base* that is <= *x*."""
    return (x if x == 0 else
            -_decade_greater_equal(-x, base) if x < 0 else
            base ** np.floor(np.log(x) / np.log(base)))


def _decade_greater_equal(x, base):
    """The smallest integer power of *base* that is >= *x*."""
    return (x if x == 0 else
            -_decade_less_equal(-x, base) if x < 0 else
            base ** np.ceil(np.log(x) / np.log(base)))


def _decade_less(x, base):
    """The largest integer power of *base* that is < *x*."""
    if x < 0:
        return -_decade_greater(-x, base)
    less = _decade_less_equal(x, base)
    if less == x:
        less /= base
    return less


def _decade_greater(x, base):
    """The smallest integer power of *base* that is > *x*."""
    if x < 0:
        return -_decade_less(-x, base)
    greater = _decade_greater_equal(x, base)
    if greater == x:
        greater *= base
    return greater


def view_call(locator):
    """``__call__`` of most locators: the ticks of the axis' view."""
    vmin, vmax = locator.axis.get_view_interval()
    return locator.tick_values(vmin, vmax)


def index_call(locator):
    """``IndexLocator.__call__``: the ticks of the axis' data."""
    dmin, dmax = locator.axis.get_data_interval()
    return locator.tick_values(dmin, dmax)


def index_tick_values(locator, vmin, vmax):
    """``IndexLocator.tick_values``."""
    tick_values = np.arange(vmin + locator.offset, vmax + 1e-12, locator._base)
    return locator.raise_if_exceeds(tick_values)


def linear_tick_values(locator, vmin, vmax):
    """``LinearLocator.tick_values``."""
    vmin, vmax = _nonsingular(vmin, vmax, expander=0.05)
    if (vmin, vmax) in locator.presets:
        return locator.presets[(vmin, vmax)]
    if locator.numticks == 0:
        return []
    ticklocs = np.linspace(vmin, vmax, locator.numticks)
    return locator.raise_if_exceeds(ticklocs)


def linear_view_limits(locator, vmin, vmax):
    """``LinearLocator.view_limits``."""
    if vmax < vmin:
        vmin, vmax = vmax, vmin
    if vmin == vmax:
        vmin -= 1
        vmax += 1
    if _rc('axes.autolimit_mode') == 'round_numbers':
        exponent, remainder = divmod(
            math.log10(vmax - vmin), math.log10(max(locator.numticks - 1, 1)))
        exponent -= (remainder < .5)
        scale = max(locator.numticks - 1, 1) ** (-exponent)
        vmin = math.floor(scale * vmin) / scale
        vmax = math.ceil(scale * vmax) / scale
    return _nonsingular(vmin, vmax)


def multiple_tick_values(locator, vmin, vmax):
    """``MultipleLocator.tick_values``."""
    if vmax < vmin:
        vmin, vmax = vmax, vmin
    step = locator._edge.step
    vmin -= locator._offset
    vmax -= locator._offset
    vmin = locator._edge.ge(vmin) * step
    n = (vmax - vmin + 0.001 * step) // step
    locs = vmin - step + np.arange(n + 3) * step + locator._offset
    return locator.raise_if_exceeds(locs)


def multiple_view_limits(locator, dmin, dmax):
    """``MultipleLocator.view_limits``."""
    if _rc('axes.autolimit_mode') == 'round_numbers':
        edge, offset = locator._edge, locator._offset
        vmin = edge.le(dmin - offset) * edge.step + offset
        vmax = edge.ge(dmax - offset) * edge.step + offset
        if vmin == vmax:
            vmin -= 1
            vmax += 1
    else:
        vmin = dmin
        vmax = dmax
    return _nonsingular(vmin, vmax)


def symlog_tick_values(locator, vmin, vmax):
    """``SymmetricalLogLocator.tick_values``."""
    linthresh = locator._linthresh
    if vmax < vmin:
        vmin, vmax = vmax, vmin
    if -linthresh <= vmin < vmax <= linthresh:
        # only the linear range is present
        return sorted({vmin, 0, vmax})
    has_a = (vmin < -linthresh)
    has_c = (vmax > linthresh)
    has_b = (has_a and vmax > -linthresh) or (has_c and vmin < linthresh)
    base = locator._base

    def get_log_range(lo, hi):
        lo = np.floor(np.log(lo) / np.log(base))
        hi = np.ceil(np.log(hi) / np.log(base))
        return lo, hi

    a_lo, a_hi = (0, 0)
    if has_a:
        a_upper_lim = min(-linthresh, vmax)
        a_lo, a_hi = get_log_range(abs(a_upper_lim), abs(vmin) + 1)
    c_lo, c_hi = (0, 0)
    if has_c:
        c_lower_lim = max(linthresh, vmin)
        c_lo, c_hi = get_log_range(c_lower_lim, vmax + 1)
    total_ticks = (a_hi - a_lo) + (c_hi - c_lo)
    if has_b:
        total_ticks += 1
    stride = max(total_ticks // (locator.numticks - 1), 1)
    decades = []
    if has_a:
        decades.extend(-1 * (base ** (np.arange(a_lo, a_hi, stride)[::-1])))
    if has_b:
        decades.append(0.0)
    if has_c:
        decades.extend(base ** (np.arange(c_lo, c_hi, stride)))
    subs = np.asarray(locator._subs)
    if len(subs) > 1 or subs[0] != 1.0:
        ticklocs = []
        for decade in decades:
            if decade == 0:
                ticklocs.append(decade)
            else:
                ticklocs.extend(subs * decade)
    else:
        ticklocs = decades
    return locator.raise_if_exceeds(np.array(ticklocs))


def symlog_view_limits(locator, vmin, vmax):
    """``SymmetricalLogLocator.view_limits``."""
    b = locator._base
    if vmax < vmin:
        vmin, vmax = vmax, vmin
    if _rc('axes.autolimit_mode') == 'round_numbers':
        vmin = _decade_less_equal(vmin, b)
        vmax = _decade_greater_equal(vmax, b)
        if vmin == vmax:
            vmin = _decade_less(vmin, b)
            vmax = _decade_greater(vmax, b)
    return _nonsingular(vmin, vmax)


def asinh_call(locator):
    """``AsinhLocator.__call__``: a view almost symmetric about zero is
    ticked symmetrically."""
    vmin, vmax = locator.axis.get_view_interval()
    if (vmin * vmax) < 0 and abs(1 + vmax / vmin) < locator.symthresh:
        bound = max(abs(vmin), abs(vmax))
        return locator.tick_values(-bound, bound)
    return locator.tick_values(vmin, vmax)


def asinh_tick_values(locator, vmin, vmax):
    """``AsinhLocator.tick_values``."""
    width = locator.linear_width
    ymin, ymax = width * np.arcsinh(np.array([vmin, vmax]) / width)
    ys = np.linspace(ymin, ymax, locator.numticks)
    zero_dev = abs(ys / (ymax - ymin))
    if ymin * ymax < 0:
        # Ensure that the zero tick-mark is included, if the axis straddles zero.
        ys = np.hstack([ys[(zero_dev > 0.5 / locator.numticks)], 0.0])
    xs = width * np.sinh(ys / width)
    zero_xs = (ys == 0)
    with np.errstate(divide="ignore"):  # base ** log(0) = base ** -inf = 0.
        if locator.base > 1:
            pows = (np.sign(xs)
                    * locator.base ** np.floor(np.log(abs(xs)) / math.log(locator.base)))
            qs = np.outer(pows, locator.subs).flatten() if locator.subs else pows
        else:  # No need to adjust sign(pows), as it cancels out when computing qs.
            pows = np.where(zero_xs, 1, 10**np.floor(np.log10(abs(xs))))
            qs = pows * np.round(xs / pows)
    ticks = np.array(sorted(set(qs)))
    return ticks if len(ticks) >= 2 else np.linspace(vmin, vmax, locator.numticks)


def log_tick_values(locator, vmin, vmax):
    """``LogLocator.tick_values``."""
    from .ticker import AutoLocator
    n_request = (
        locator.numticks if locator.numticks != "auto" else
        np.clip(locator.axis.get_tick_space(), 2, 9) if locator.axis is not None else
        9)
    b = locator._base
    if vmin <= 0.0:
        if locator.axis is not None:
            vmin = locator.axis.get_minpos()
        if vmin <= 0.0 or not np.isfinite(vmin):
            raise ValueError(
                "Data cannot be log-scaled because all values are <= 0.")
    if vmax < vmin:
        vmin, vmax = vmax, vmin
    efmin, efmax = (np.log10([vmin, vmax]) if b == 10 else
                    np.log2([vmin, vmax]) if b == 2 else
                    np.log([vmin, vmax]) / np.log(b))
    emin = math.ceil(efmin)
    emax = math.floor(efmax)
    n_avail = emax - emin + 1  # Total number of decade ticks available.
    if isinstance(locator._subs, str):
        if n_avail >= 10 or b < 3:
            if locator._subs == 'auto':
                return np.array([])  # no minor or major ticks
            subs = np.array([1.0])  # major ticks
        else:
            _first = 2.0 if locator._subs == 'auto' else 1.0
            subs = np.arange(_first, b)
    else:
        subs = locator._subs
    if _rc("_internal.classic_mode"):  # keep historic formulas
        stride = max(math.ceil((n_avail - 1) / (n_request - 1)), 1)
        decades = np.arange(emin - stride, emax + stride + 1, stride)
    else:
        stride = n_avail // (n_request + 1) + 1
        nr = math.ceil(n_avail / stride)
        if nr <= n_request:
            n_request = nr
        if n_request == 0:  # No tick in bounds; two ticks just outside.
            decades = [emin - 1, emax + 1]
            stride = decades[1] - decades[0]
        elif n_request == 1:  # A single tick close to center.
            mid = round((efmin + efmax) / 2)
            stride = max(mid - (emin - 1), (emax + 1) - mid)
            decades = [mid - stride, mid, mid + stride]
        else:
            stride = (n_avail - 1) // (n_request - 1)
            if stride < n_avail / n_request:  # fallback to second case
                stride = n_avail // n_request
            olo = max(n_avail - stride * n_request, 0)
            ohi = min(n_avail - stride * (n_request - 1), stride)
            offset = (-emin) % stride
            if not olo <= offset < ohi:
                offset = olo
            decades = range(emin + offset - stride, emax + stride + 1, stride)
    is_minor = len(subs) > 1 or (len(subs) == 1 and subs[0] != 1.0)
    if is_minor:
        if stride == 1 or n_avail <= 1:
            # Minor ticks start in the decade preceding the first major tick.
            ticklocs = np.concatenate([
                subs * b**decade for decade in range(emin - 1, emax + 1)])
        else:
            ticklocs = np.array([])
    else:
        ticklocs = b ** np.array(decades)
    if (len(subs) > 1
            and stride == 1
            and (len(decades) - 2  # major
                 + ((vmin <= ticklocs) & (ticklocs <= vmax)).sum())  # minor
                 <= 1):
        return AutoLocator().tick_values(vmin, vmax)
    return locator.raise_if_exceeds(ticklocs)


def logit_tick_values(locator, vmin, vmax):
    """``LogitLocator.tick_values``, its linear ticks those of
    `maxn_tick_values`."""
    if hasattr(locator.axis, "axes") and locator.axis.axes.name == "polar":
        raise NotImplementedError("Polar axis cannot be logit scaled yet")
    if locator._nbins == "auto":
        if locator.axis is not None:
            nbins = locator.axis.get_tick_space()
            if nbins < 2:
                nbins = 2
        else:
            nbins = 9
    else:
        nbins = locator._nbins

    def ideal_ticks(x):
        return 10 ** x if x < 0 else 1 - (10 ** (-x)) if x > 0 else 0.5

    vmin, vmax = locator.nonsingular(vmin, vmax)
    binf = int(
        np.floor(np.log10(vmin)) if vmin < 0.5
        else 0 if vmin < 0.9
        else -np.ceil(np.log10(1 - vmin)))
    bsup = int(
        np.ceil(np.log10(vmax)) if vmax <= 0.5
        else 1 if vmax <= 0.9
        else -np.floor(np.log10(1 - vmax)))
    numideal = bsup - binf - 1
    if numideal >= 2:
        if numideal > nbins:
            subsampling_factor = math.ceil(numideal / nbins)
            if locator._minor:
                ticklocs = [ideal_ticks(b) for b in range(binf, bsup + 1)
                            if (b % subsampling_factor) != 0]
            else:
                ticklocs = [ideal_ticks(b) for b in range(binf, bsup + 1)
                            if (b % subsampling_factor) == 0]
            return locator.raise_if_exceeds(np.array(ticklocs))
        if locator._minor:
            ticklocs = []
            for b in range(binf, bsup):
                if b < -1:
                    ticklocs.extend(np.arange(2, 10) * 10 ** b)
                elif b == -1:
                    ticklocs.extend(np.arange(2, 5) / 10)
                elif b == 0:
                    ticklocs.extend(np.arange(6, 9) / 10)
                else:
                    ticklocs.extend(1 - np.arange(2, 10)[::-1] * 10 ** (-b - 1))
            return locator.raise_if_exceeds(np.array(ticklocs))
        ticklocs = [ideal_ticks(b) for b in range(binf, bsup + 1)]
        return locator.raise_if_exceeds(np.array(ticklocs))
    if locator._minor:
        return []
    return maxn_tick_values(locator, vmin, vmax)


def maxn_tick_values(locator, vmin, vmax):
    """``MaxNLocator.tick_values``, around the locator's ``_raw_ticks``."""
    if locator._symmetric:
        vmax = max(abs(vmin), abs(vmax))
        vmin = -vmax
    vmin, vmax = _nonsingular(vmin, vmax, expander=1e-13, tiny=1e-14)
    locs = locator._raw_ticks(vmin, vmax)
    prune = locator._prune
    if prune == 'lower':
        locs = locs[1:]
    elif prune == 'upper':
        locs = locs[:-1]
    elif prune == 'both':
        locs = locs[1:-1]
    return locator.raise_if_exceeds(locs)


def maxn_view_limits(locator, dmin, dmax):
    """``MaxNLocator.view_limits``."""
    if locator._symmetric:
        dmax = max(abs(dmin), abs(dmax))
        dmin = -dmax
    dmin, dmax = _nonsingular(dmin, dmax, expander=1e-12, tiny=1e-13)
    if _rc('axes.autolimit_mode') == 'round_numbers':
        return locator._raw_ticks(dmin, dmax)[[0, -1]]
    return dmin, dmax


def warn_outside_logit(vmin, vmax):
    """The warning of ``LogitLocator.nonsingular`` for finite limits
    holding no value between 0 and 1, the limits compared as it compares
    them."""
    if vmin > vmax:
        vmin, vmax = vmax, vmin
    if np.isfinite(vmin) and np.isfinite(vmax) and (vmax <= 0 or vmin >= 1):
        _api.warn_external(
            "Data has no values between 0 and 1, and therefore cannot be "
            "logit-scaled.")
