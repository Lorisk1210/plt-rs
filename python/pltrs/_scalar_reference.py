"""
matplotlib's arithmetic of `.ScalarFormatter` (from matplotlib 3.11.2,
under the Matplotlib License): the order of magnitude and the format of
the labels of the ticks at given locations, for power limits pltrs (Rust)
does not hold (no integers: NaN, fractions, text), which matplotlib
compares as they are.
"""

import math

import numpy as np


def order_of_magnitude(formatter, locs):
    """The order of magnitude *formatter* writes the ticks at *locs* with
    (matplotlib's ``ScalarFormatter._set_order_of_magnitude``)."""
    if not formatter._scientific:
        return 0
    if formatter._powerlimits[0] == formatter._powerlimits[1] != 0:
        # fixed scaling when lower power limit = upper <> 0.
        return formatter._powerlimits[0]
    # restrict to visible ticks
    vmin, vmax = sorted(formatter.axis.get_view_interval())
    locs = np.asarray(locs)
    locs = locs[(vmin <= locs) & (locs <= vmax)]
    locs = np.abs(locs)
    if not len(locs):
        return 0
    if formatter.offset:
        oom = math.floor(math.log10(vmax - vmin))
    else:
        val = locs.max()
        if val == 0:
            oom = 0
        else:
            oom = math.floor(math.log10(val))
    if oom <= formatter._powerlimits[0]:
        return oom
    elif oom >= formatter._powerlimits[1]:
        return oom
    else:
        return 0


def label_format(formatter, locs):
    """The format string of the labels of the ticks at *locs* (matplotlib's
    ``ScalarFormatter._set_format``), for the order of magnitude and the
    offset *formatter* holds."""
    if len(locs) < 2:
        # Temporarily augment the locations with the axis end points.
        _locs = [*locs, *formatter.axis.get_view_interval()]
    else:
        _locs = locs
    values = (np.asarray(_locs) - formatter.offset) / 10. ** formatter._orderOfMagnitude
    loc_range = np.ptp(values)
    # Curvilinear coordinates can yield two identical points.
    if loc_range == 0:
        loc_range = np.max(np.abs(values))
    # Both points might be zero.
    if loc_range == 0:
        loc_range = 1
    if len(locs) < 2:
        # We needed the end points only for the loc_range calculation.
        values = values[:-2]
    loc_range_oom = int(math.floor(math.log10(loc_range)))
    # first estimate:
    sigfigs = max(0, 3 - loc_range_oom)
    # refined estimate:
    thresh = 1e-3 * 10 ** loc_range_oom
    while sigfigs >= 0:
        if np.abs(values - np.round(values, decimals=sigfigs)).max() < thresh:
            sigfigs -= 1
        else:
            break
    sigfigs += 1
    label = f'%1.{sigfigs}f'
    if formatter._usetex or formatter._useMathText:
        label = r'$\mathdefault{%s}$' % label
    return label
