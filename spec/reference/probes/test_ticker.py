"""Tick locating and tick-label formatting (``matplotlib.ticker``).

Area ``ticker``.

Argument conventions
--------------------
* ``params`` holds constructor keyword arguments, verbatim, except for the
  few *setter* keys listed per op below (e.g. ScalarFormatter's
  ``scientific``/``powerlimits``). Omitted keys take matplotlib's defaults.
* Locators are used **without an axis** unless stated otherwise, so
  ``nbins='auto'`` (MaxNLocator/AutoLocator/LogitLocator) resolves to 9 and
  LogLocator's ``numticks='auto'`` resolves to 9.
* ``rc`` (optional) is a dict of rcParams applied with ``rc_context`` around
  both construction and evaluation. Everything else runs under the reference
  configuration (matplotlib defaults; notably ``axes.unicode_minus=True``,
  ``axes.formatter.limits=[-5, 6]``, ``offset_threshold=4``,
  ``use_mathtext=False``, ``axes.autolimit_mode='data'``,
  ``xtick.minor.ndivs='auto'``).
* Formatters are attached to a dummy axis (``create_dummy_axis()``) whose view
  *and* data intervals are set to ``view`` (``[vmin, vmax]``, order kept);
  then ``format_ticks(np.asarray(locs, float))`` is called and
  ``get_offset()`` read. ``locs`` usually include ticks just outside the view,
  exactly as an Axis passes them.
* Labels contain U+2212 MINUS SIGN where matplotlib applies ``fix_minus``.

Ops: locators
-------------
``ticker.Locator.nonsingular``          {v0, v1} -> [v0, v1] (base implementation,
                                        ``transforms._nonsingular(v0, v1, expander=0.05)``)
``ticker.MaxNLocator.tick_values``      {params{nbins, steps, integer, symmetric, prune, min_n_ticks}, vmin, vmax}
``ticker.MaxNLocator.view_limits``      {params, dmin, dmax, rc?} -> [vmin, vmax]
``ticker.AutoLocator.tick_values``      {vmin, vmax}  (MaxNLocator(nbins='auto', steps=[1, 2, 2.5, 5, 10]))
``ticker.AutoLocator.view_limits``      {dmin, dmax, rc?}
``ticker.MultipleLocator.tick_values``  {params{base, offset}, vmin, vmax}
``ticker.MultipleLocator.view_limits``  {params, dmin, dmax, rc?}
``ticker.FixedLocator.tick_values``     {params{locs, nbins}} (vmin/vmax are ignored -> None)
``ticker.LinearLocator.tick_values``    {params{numticks}, vmin, vmax}
``ticker.LinearLocator.view_limits``    {params, vmin, vmax, rc?}
``ticker.IndexLocator.tick_values``     {params{base, offset}, vmin, vmax}
``ticker.NullLocator.tick_values``      {vmin, vmax} -> []
``ticker.LogLocator.tick_values``       {params{base, subs, numticks}, vmin, vmax, minpos?}
                                        ``minpos`` given -> a dummy axis with that minpos is
                                        attached (used when vmin <= 0)
``ticker.LogLocator.view_limits``       {params, vmin, vmax, minpos, rc?}
``ticker.LogLocator.nonsingular``       {params, vmin, vmax, minpos}
``ticker.SymmetricalLogLocator.tick_values`` {params{base, linthresh, subs, numticks}, vmin, vmax}
                                        (``numticks`` applied with set_params)
``ticker.SymmetricalLogLocator.view_limits`` {params, vmin, vmax, rc?}
``ticker.AsinhLocator.tick_values``     {params{linear_width, numticks, symthresh, base, subs}, vmin, vmax}
``ticker.AsinhLocator.__call__``        {params, view} (dummy axis with that view interval;
                                        near-symmetric views are made symmetric)
``ticker.LogitLocator.tick_values``     {params{minor, nbins}, vmin, vmax}
``ticker.LogitLocator.nonsingular``     {params, vmin, vmax}
``ticker.AutoMinorLocator.__call__``    {params{n}, major_locs, view, axis_name, rc?}: minor ticks
                                        for a linear axis whose major ticks are ``major_locs``

Ops: formatters (all ``format_ticks`` ops return {labels: [...], offset: str})
-----------------------------------------------------------------------------
``ticker.ScalarFormatter.format_ticks`` {params{useOffset, useMathText, useLocale,
                                        scientific, powerlimits}, locs, view, rc?};
                                        ``scientific``/``powerlimits`` go through
                                        set_scientific/set_powerlimits
``ticker.ScalarFormatter.format_data``  {params, value, rc?} -> str
``ticker.ScalarFormatter.format_data_short`` {params, value, view, rc?} -> str
``ticker.EngFormatter.format_ticks``    {params{unit, places, sep, useMathText, useOffset}, locs, view, rc?}
``ticker.EngFormatter.format_data``     {params, value, rc?} -> str
``ticker.LogFormatter.format_ticks``    {params{base, labelOnlyBase, minor_thresholds, linthresh}, locs, view, rc?}
``ticker.LogFormatterExponent.format_ticks``     same params
``ticker.LogFormatterMathtext.format_ticks``     same params
``ticker.LogFormatterSciNotation.format_ticks``  same params
``ticker.LogFormatter.format_data_short`` {value} -> str
``ticker.LogitFormatter.format_ticks``  {params{use_overline, one_half, minor, minor_threshold, minor_number}, locs, view}
``ticker.PercentFormatter.format_ticks`` {params{xmax, decimals, symbol, is_latex}, locs, view, rc?}
``ticker.StrMethodFormatter.format_ticks`` {params{fmt}, locs, view, rc?}
``ticker.FormatStrFormatter.format_ticks`` {params{fmt}, locs, view}
``ticker.FixedFormatter.format_ticks``  {params{seq}, locs, view}
``ticker.NullFormatter.format_ticks``   {locs, view}
"""

import math

import matplotlib as mpl
import matplotlib.ticker as mticker
import numpy as np
import pytest

from spec_support import ProbeSet

probes = ProbeSet("ticker")

INF = float("inf")
NAN = float("nan")

#: (vmin, vmax) view ranges for linear locators/formatters.
LINEAR_RANGES = [
    (0.0, 1.0),
    (0.0, 10.0),
    (0.0, 100.0),
    (0.0, 1000.0),
    (0.0, 1e4),
    (0.0, 1e5),
    (0.0, 1e6),
    (0.0, 1e7),
    (0.0, 3.0),
    (0.0, 7.0),
    (0.0, 13.0),
    (0.0, 0.0025),
    (-1.0, 1.0),
    (-5.5, 3.3),
    (-1e6, 1e6),
    (-273.15, 100.0),
    (0.1, 0.9),
    (0.1, 0.3),
    (2.5, 7.5),
    (0.001, 0.002),
    (1e-9, 3e-9),
    (1e-7, 5e-7),
    (0.0, 1e-7),
    (-0.0001, 0.0001),
    (1e5, 1e5 + 5),
    (1e5, 1e5 + 1),
    (100000.5, 100001.5),
    (99999.0, 100001.0),
    (9999.5, 10000.5),
    (1e6, 1e6 + 10),
    (1e9, 1e9 + 1),
    (1e9, 1e9 + 100),
    (123456.789, 123457.123),
    (-1e5 - 5, -1e5),
    (-1e9 - 100, -1e9),
    (1.00001, 1.00002),
    (0.99999, 1.00001),
    (1.0, 1.0000001),
    (3.14159, 3.14160),
    (1950.0, 2050.0),
    (2020.5, 2021.5),
    (1e10, 1e11),
    (1e15, 1e16),
    (-1e300, 1e300),
    (1e-300, 3e-300),
    (10.0, 0.0),
    (1.0, -1.0),
    (1.5, 1.5),
    (0.0, 0.0),
    (1e6, 1e6),
    (-INF, INF),
]

LINEAR_SUBSET = [(0.0, 1.0), (0.0, 10.0), (0.0, 7.0), (0.0, 13.0), (-5.5, 3.3), (0.1, 0.3), (1e5, 1e5 + 5),
                 (99999.0, 100001.0), (1e-7, 5e-7), (-1e6, 1e6), (10.0, 0.0), (1.5, 1.5), (0.0, 0.0025)]

LOG_RANGES = [
    (1.0, 10.0),
    (1.0, 100.0),
    (1.0, 1e3),
    (1.0, 1e5),
    (1.0, 1e9),
    (1.0, 1e10),
    (1.0, 1e20),
    (1e-300, 1e300),
    (0.5, 50.0),
    (2.0, 3.0),
    (1.0, 2.0),
    (3.0, 30.0),
    (0.01, 0.02),
    (1e-5, 1e5),
    (5.0, 5.0),
    (100.0, 1.0),
    (0.9, 1.1),
    (1e-7, 1.0),
    (7.0, 8000.0),
]


# --------------------------------------------------------------------------
# Helpers
# --------------------------------------------------------------------------


class _MinposAxis(mticker._DummyAxis):
    """Dummy axis that also answers the shared-axes query LogLocator makes."""

    def _get_shared_axis(self):
        return [self]


class _MinorAxis:
    """The parts of an Axis that AutoMinorLocator consults."""

    def __init__(self, major_locs, view, axis_name):
        self._major_locs = np.asarray(major_locs, dtype=float)
        self._view = tuple(view)
        self.axis_name = axis_name

    def get_scale(self):
        return "linear"

    def get_majorticklocs(self):
        return self._major_locs

    def get_view_interval(self):
        return self._view


def _rc(rc):
    return mpl.rc_context(rc or {})


def _rc_arg(rc):
    """Optional ``rc`` probe argument (omitted from the golden when None)."""
    return {} if rc is None else {"rc": rc}


def _locs(locs):
    return np.asarray(locs, dtype=float)


def _ticks(vmin, vmax):
    """AutoLocator ticks for a view, as a plain list (used to build formatter args)."""
    return [float(v) for v in mticker.AutoLocator().tick_values(vmin, vmax)]


def _format(formatter, locs, view):
    formatter.create_dummy_axis()
    formatter.axis.set_view_interval(*view)
    formatter.axis.set_data_interval(*view)
    labels = formatter.format_ticks(_locs(locs))
    return {"labels": labels, "offset": formatter.get_offset()}


def _scalar_formatter(params):
    params = dict(params)
    scientific = params.pop("scientific", None)
    powerlimits = params.pop("powerlimits", None)
    formatter = mticker.ScalarFormatter(**params)
    if scientific is not None:
        formatter.set_scientific(scientific)
    if powerlimits is not None:
        formatter.set_powerlimits(powerlimits)
    return formatter


# --------------------------------------------------------------------------
# Locator adapters
# --------------------------------------------------------------------------


def locator_nonsingular(v0, v1):
    return mticker.Locator().nonsingular(v0, v1)


def maxn_tick_values(params, vmin, vmax):
    return mticker.MaxNLocator(**params).tick_values(vmin, vmax)


def maxn_view_limits(params, dmin, dmax, rc=None):
    with _rc(rc):
        return mticker.MaxNLocator(**params).view_limits(dmin, dmax)


def auto_tick_values(vmin, vmax):
    return mticker.AutoLocator().tick_values(vmin, vmax)


def auto_view_limits(dmin, dmax, rc=None):
    with _rc(rc):
        return mticker.AutoLocator().view_limits(dmin, dmax)


def multiple_tick_values(params, vmin, vmax):
    return mticker.MultipleLocator(**params).tick_values(vmin, vmax)


def multiple_view_limits(params, dmin, dmax, rc=None):
    with _rc(rc):
        return mticker.MultipleLocator(**params).view_limits(dmin, dmax)


def fixed_tick_values(params):
    return mticker.FixedLocator(**params).tick_values(None, None)


def linear_tick_values(params, vmin, vmax):
    return mticker.LinearLocator(**params).tick_values(vmin, vmax)


def linear_view_limits(params, vmin, vmax, rc=None):
    with _rc(rc):
        return mticker.LinearLocator(**params).view_limits(vmin, vmax)


def index_tick_values(params, vmin, vmax):
    return mticker.IndexLocator(**params).tick_values(vmin, vmax)


def null_tick_values(vmin, vmax):
    return mticker.NullLocator().tick_values(vmin, vmax)


def _log_locator(params, minpos=None):
    locator = mticker.LogLocator(**params)
    if minpos is not None:
        locator.set_axis(_MinposAxis(minpos=minpos))
    return locator


def log_tick_values(params, vmin, vmax, minpos=None):
    return _log_locator(params, minpos).tick_values(vmin, vmax)


def log_view_limits(params, vmin, vmax, minpos, rc=None):
    with _rc(rc):
        return _log_locator(params, minpos).view_limits(vmin, vmax)


def log_nonsingular(params, vmin, vmax, minpos):
    return _log_locator(params, minpos).nonsingular(vmin, vmax)


def _symlog_locator(params):
    params = dict(params)
    numticks = params.pop("numticks", None)
    locator = mticker.SymmetricalLogLocator(**params)
    if numticks is not None:
        locator.set_params(numticks=numticks)
    return locator


def symlog_tick_values(params, vmin, vmax):
    return _symlog_locator(params).tick_values(vmin, vmax)


def symlog_view_limits(params, vmin, vmax, rc=None):
    with _rc(rc):
        return _symlog_locator(params).view_limits(vmin, vmax)


def asinh_tick_values(params, vmin, vmax):
    return mticker.AsinhLocator(**params).tick_values(vmin, vmax)


def asinh_call(params, view):
    locator = mticker.AsinhLocator(**params)
    locator.create_dummy_axis()
    locator.axis.set_view_interval(*view)
    return locator()


def logit_tick_values(params, vmin, vmax):
    return mticker.LogitLocator(**params).tick_values(vmin, vmax)


def logit_nonsingular(params, vmin, vmax):
    return mticker.LogitLocator(**params).nonsingular(vmin, vmax)


def auto_minor_call(params, major_locs, view, axis_name, rc=None):
    with _rc(rc):
        locator = mticker.AutoMinorLocator(**params)
        locator.set_axis(_MinorAxis(major_locs, view, axis_name))
        return locator()


# --------------------------------------------------------------------------
# Formatter adapters
# --------------------------------------------------------------------------


def scalar_format_ticks(params, locs, view, rc=None):
    with _rc(rc):
        return _format(_scalar_formatter(params), locs, view)


def scalar_format_data(params, value, rc=None):
    with _rc(rc):
        return _scalar_formatter(params).format_data(value)


def scalar_format_data_short(params, value, view, rc=None):
    with _rc(rc):
        formatter = _scalar_formatter(params)
        formatter.create_dummy_axis()
        formatter.axis.set_view_interval(*view)
        return formatter.format_data_short(value)


def eng_format_ticks(params, locs, view, rc=None):
    with _rc(rc):
        return _format(mticker.EngFormatter(**params), locs, view)


def eng_format_data(params, value, rc=None):
    with _rc(rc):
        return mticker.EngFormatter(**params).format_data(value)


def _formatter_op(cls):
    def format_ticks(params, locs, view, rc=None):
        with _rc(rc):
            return _format(cls(**params), locs, view)

    return format_ticks


def null_format_ticks(locs, view):
    return _format(mticker.NullFormatter(), locs, view)


def log_format_data_short(value):
    return mticker.LogFormatter().format_data_short(value)


# --------------------------------------------------------------------------
# Locators
# --------------------------------------------------------------------------


def test_locator_nonsingular():
    for v0, v1 in [(0.0, 1.0), (1.0, 0.0), (0.0, 0.0), (1.0, 1.0), (-1.0, -1.0), (1e6, 1e6), (1e-20, 1e-20),
                   (-1e-20, 1e-20), (1e300, 1e300), (-INF, INF), (NAN, 1.0), (0.0, INF), (1.0, 1.0 + 1e-15)]:
        probes.probe("ticker.Locator.nonsingular", locator_nonsingular, v0=v0, v1=v1)


def test_auto_locator():
    for vmin, vmax in LINEAR_RANGES:
        probes.probe("ticker.AutoLocator.tick_values", auto_tick_values, vmin=vmin, vmax=vmax)
    for dmin, dmax in LINEAR_SUBSET:
        for rc in [None, {"axes.autolimit_mode": "round_numbers"}]:
            probes.probe("ticker.AutoLocator.view_limits", auto_view_limits, dmin=dmin, dmax=dmax, **_rc_arg(rc))
    assert list(auto_tick_values(0.0, 10.0)) == [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
    assert list(auto_tick_values(0.0, 1.0)) == [0.0, 0.2, 0.4, 0.6000000000000001, 0.8, 1.0]


MAXN_VARIANTS = [
    {},
    {"nbins": "auto"},
    {"nbins": 1},
    {"nbins": 3},
    {"nbins": 5},
    {"nbins": 4, "steps": [1, 2, 5, 10]},
    {"nbins": 5, "steps": [1, 2, 2.5, 5, 10]},
    {"nbins": 6, "steps": [1, 3]},
    {"nbins": 6, "steps": [2, 4, 6]},
    {"nbins": 5, "integer": True},
    {"nbins": 5, "integer": True, "min_n_ticks": 1},
    {"nbins": 5, "symmetric": True},
    {"nbins": 5, "prune": "lower"},
    {"nbins": 5, "prune": "upper"},
    {"nbins": 5, "prune": "both"},
    {"nbins": 2, "min_n_ticks": 5},
    {"nbins": 9, "min_n_ticks": 0},
]


def test_max_n_locator():
    for vmin, vmax in LINEAR_RANGES:
        probes.probe("ticker.MaxNLocator.tick_values", maxn_tick_values, params={}, vmin=vmin, vmax=vmax)
    for params in MAXN_VARIANTS[1:]:
        for vmin, vmax in LINEAR_SUBSET + [(0.0, 2.0), (0.2, 3.7), (-0.4, 0.4), (1.0, 1.3)]:
            probes.probe("ticker.MaxNLocator.tick_values", maxn_tick_values, params=params, vmin=vmin, vmax=vmax)
    for params in [{"steps": [0.5, 1]}, {"steps": [1, 11]}, {"steps": [5, 2]}, {"steps": 5}, {"prune": "middle"},
                   {"nbins": "many"}, {"bogus": 1}]:
        probes.probe("ticker.MaxNLocator.tick_values", maxn_tick_values, params=params, vmin=0.0, vmax=1.0)
    for params in [{}, {"nbins": 5}, {"nbins": 5, "symmetric": True}, {"nbins": 4, "integer": True}]:
        for dmin, dmax in LINEAR_SUBSET:
            for rc in [None, {"axes.autolimit_mode": "round_numbers"}]:
                probes.probe("ticker.MaxNLocator.view_limits", maxn_view_limits, params=params, dmin=dmin,
                             dmax=dmax, **_rc_arg(rc))
    # At most nbins + 1 ticks fall inside the view; one more on each side is kept
    # beyond the limits (to support autoscaling).
    assert list(maxn_tick_values({"nbins": 4}, 0.1, 0.9)) == [0.0, 0.2, 0.4, 0.6000000000000001, 0.8, 1.0]


def test_multiple_locator():
    ranges = [(0.0, 10.0), (-3.3, 7.7), (0.1, 0.9), (10.0, 0.0), (5.0, 5.0), (1e6, 1e6 + 10), (-1e-3, 1e-3),
              (0.3, 0.30000001), (0.0, 1.0)]
    for params in [{}, {"base": 0.5}, {"base": 0.1}, {"base": 2.5}, {"base": 1e-3}, {"base": 1e6}, {"base": 0.2},
                   {"base": 1.0, "offset": 0.25}, {"base": 0.5, "offset": -0.1}, {"base": 2.0, "offset": 0.05},
                   {"base": 0.1, "offset": 0.05}]:
        for vmin, vmax in ranges:
            if abs(vmax - vmin) / params.get("base", 1.0) > 200:
                continue  # keep goldens small: thousands of ticks add nothing
            probes.probe("ticker.MultipleLocator.tick_values", multiple_tick_values, params=params, vmin=vmin,
                         vmax=vmax)
        for dmin, dmax in [(0.1, 9.3), (-3.3, 7.7), (2.0, 2.0), (0.0, 1.0)]:
            for rc in [None, {"axes.autolimit_mode": "round_numbers"}]:
                probes.probe("ticker.MultipleLocator.view_limits", multiple_view_limits, params=params,
                             dmin=dmin, dmax=dmax, **_rc_arg(rc))
    for params in [{"base": 0.0}, {"base": -1.0}]:
        probes.probe("ticker.MultipleLocator.tick_values", multiple_tick_values, params=params, vmin=0.0, vmax=1.0)
    assert list(multiple_tick_values({"base": 2.5}, 0.0, 10.0)) == [-2.5, 0.0, 2.5, 5.0, 7.5, 10.0, 12.5]


def test_fixed_index_null_locators():
    locs = [float(v) for v in range(11)]
    symmetric = [float(v) for v in range(-5, 6)]
    for params in [{"locs": locs}, {"locs": locs, "nbins": 2}, {"locs": locs, "nbins": 3}, {"locs": locs, "nbins": 5},
                   {"locs": locs, "nbins": 0}, {"locs": symmetric, "nbins": 3}, {"locs": symmetric, "nbins": 4},
                   {"locs": [3.0, 1.0, 2.0]}, {"locs": []}, {"locs": [[1.0, 2.0]]},
                   {"locs": [0.5, -0.25, 7.0, -3.0, 0.1], "nbins": 2}]:
        probes.probe("ticker.FixedLocator.tick_values", fixed_tick_values, params=params)
    for params in [{"base": 1, "offset": 0}, {"base": 5, "offset": 0}, {"base": 2.5, "offset": 0.5},
                   {"base": 3, "offset": 1}]:
        for vmin, vmax in [(0.0, 10.0), (0.0, 9.99), (2.0, 37.0), (0.5, 0.6), (10.0, 0.0)]:
            probes.probe("ticker.IndexLocator.tick_values", index_tick_values, params=params, vmin=vmin, vmax=vmax)
    probes.probe("ticker.NullLocator.tick_values", null_tick_values, vmin=0.0, vmax=1.0)


def test_linear_locator():
    for params in [{}, {"numticks": 0}, {"numticks": 1}, {"numticks": 2}, {"numticks": 5}, {"numticks": 11}]:
        for vmin, vmax in [(0.0, 1.0), (0.0, 10.0), (5.0, 5.0), (10.0, 0.0), (1e-15, 2e-15), (-3.3, 7.7),
                           (0.0, 0.0), (-INF, INF)]:
            probes.probe("ticker.LinearLocator.tick_values", linear_tick_values, params=params, vmin=vmin,
                         vmax=vmax)
        for vmin, vmax in [(0.13, 0.87), (3.0, 3.0), (-7.0, 123.0), (10.0, 0.0)]:
            for rc in [None, {"axes.autolimit_mode": "round_numbers"}]:
                probes.probe("ticker.LinearLocator.view_limits", linear_view_limits, params=params, vmin=vmin,
                             vmax=vmax, **_rc_arg(rc))


def test_log_locator():
    variants = [
        {},
        {"subs": "auto"},
        {"subs": "all"},
        {"subs": None},
        {"subs": [2.0, 5.0]},
        {"subs": [1.0, 2.0, 5.0]},
        {"numticks": 3},
        {"numticks": 5},
        {"numticks": 15},
        {"base": 2.0},
        {"base": 2.0, "subs": "auto"},
        {"base": math.e},
        {"base": 3.0, "subs": "auto"},
        {"base": 16.0, "subs": "all"},
    ]
    for params in variants:
        for vmin, vmax in LOG_RANGES:
            probes.probe("ticker.LogLocator.tick_values", log_tick_values, params=params, vmin=vmin, vmax=vmax)
    for params in [{}, {"subs": "auto"}]:
        for vmin, vmax, minpos in [(0.0, 100.0, 0.1), (-5.0, 100.0, 2.0), (0.0, 100.0, 0.0), (0.0, 100.0, INF),
                                   (-1.0, -0.5, 0.01)]:
            probes.probe("ticker.LogLocator.tick_values", log_tick_values, params=params, vmin=vmin, vmax=vmax,
                         minpos=minpos)
        probes.probe("ticker.LogLocator.tick_values", log_tick_values, params=params, vmin=0.0, vmax=100.0)
    for vmin, vmax, minpos in [(1.0, 100.0, 1.0), (0.3, 70.0, 0.3), (0.0, 70.0, 0.5), (5.0, 5.0, 1.0),
                               (10.0, 10.0, 1.0), (-INF, INF, 1.0), (-10.0, -1.0, 1.0), (100.0, 1.0, 1.0),
                               (0.0, 70.0, INF)]:
        probes.probe("ticker.LogLocator.nonsingular", log_nonsingular, params={}, vmin=vmin, vmax=vmax, minpos=minpos)
        for rc in [None, {"axes.autolimit_mode": "round_numbers"}]:
            probes.probe("ticker.LogLocator.view_limits", log_view_limits, params={}, vmin=vmin, vmax=vmax,
                         minpos=minpos, **_rc_arg(rc))
    probes.probe("ticker.LogLocator.view_limits", log_view_limits, params={"base": 2.0}, vmin=3.0, vmax=50.0,
                 minpos=1.0, rc={"axes.autolimit_mode": "round_numbers"})
    for params in [{"subs": "some"}, {"subs": [[1.0, 2.0]]}, {"base": 1.0}]:
        probes.probe("ticker.LogLocator.tick_values", log_tick_values, params=params, vmin=1.0, vmax=100.0)
    assert list(log_tick_values({}, 1.0, 1e3)) == [0.1, 1.0, 10.0, 100.0, 1000.0, 10000.0]


def test_symlog_locator():
    ranges = [(-100.0, 100.0), (-1.0, 1.0), (-0.5, 0.5), (0.0, 1000.0), (-1e5, 10.0), (0.2, 0.7), (5.0, 500.0),
              (-500.0, -5.0), (-0.5, 1e6), (100.0, -100.0), (-1e10, 1e10), (0.0, 0.5)]
    for params in [
        {"linthresh": 1.0, "base": 10.0},
        {"linthresh": 1.0, "base": 10.0, "subs": [1.0, 2.0, 5.0]},
        {"linthresh": 1.0, "base": 10.0, "numticks": 5},
        {"linthresh": 0.01, "base": 10.0},
        {"linthresh": 2.0, "base": 2.0},
        {"linthresh": 10.0, "base": 10.0, "subs": [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]},
    ]:
        for vmin, vmax in ranges:
            probes.probe("ticker.SymmetricalLogLocator.tick_values", symlog_tick_values, params=params, vmin=vmin,
                         vmax=vmax)
        for vmin, vmax in [(-37.0, 420.0), (3.0, 3.0), (0.0, 0.0), (-5.0, -0.2)]:
            for rc in [None, {"axes.autolimit_mode": "round_numbers"}]:
                probes.probe("ticker.SymmetricalLogLocator.view_limits", symlog_view_limits, params=params,
                             vmin=vmin, vmax=vmax, **_rc_arg(rc))
    probes.probe("ticker.SymmetricalLogLocator.tick_values", symlog_tick_values, params={"linthresh": 1.0},
                 vmin=0.0, vmax=1.0)  # base missing -> ValueError


def test_asinh_locator():
    ranges = [(-100.0, 100.0), (0.0, 1000.0), (-5.0, 1e6), (0.1, 0.2), (-1.0, 1.0), (1e-3, 1e3), (-1e6, -1.0),
              (-3.0, 2.0), (0.0, 0.0), (-80.0, 100.0)]
    for params in [
        {"linear_width": 1.0},
        {"linear_width": 0.1},
        {"linear_width": 10.0, "numticks": 7},
        {"linear_width": 1.0, "base": 2},
        {"linear_width": 1.0, "base": 0},
        {"linear_width": 1.0, "subs": [2, 5]},
        {"linear_width": 1.0, "base": 2, "subs": [2, 4, 8]},
    ]:
        for vmin, vmax in ranges:
            probes.probe("ticker.AsinhLocator.tick_values", asinh_tick_values, params=params, vmin=vmin, vmax=vmax)
        for view in [[-80.0, 100.0], [-100.0, 70.0], [-50.0, 100.0], [10.0, 100.0]]:
            probes.probe("ticker.AsinhLocator.__call__", asinh_call, params=params, view=view)
    probes.probe("ticker.AsinhLocator.__call__", asinh_call, params={"linear_width": 1.0, "symthresh": 0.5},
                 view=[-50.0, 100.0])


def test_logit_locator():
    ranges = [(1e-4, 1 - 1e-4), (0.1, 0.9), (0.3, 0.7), (0.01, 0.99), (1e-9, 0.5), (0.5, 1 - 1e-9), (0.45, 0.55),
              (1e-3, 0.2), (0.0, 1.0), (-1.0, 2.0), (0.2, 0.2), (0.9, 0.1), (1e-12, 1 - 1e-12), (0.6, 0.65)]
    for params in [{}, {"minor": True}, {"nbins": 3}, {"nbins": 20}, {"minor": True, "nbins": 3}]:
        for vmin, vmax in ranges:
            probes.probe("ticker.LogitLocator.tick_values", logit_tick_values, params=params, vmin=vmin, vmax=vmax)
    for vmin, vmax in [(0.0, 1.0), (-INF, INF), (2.0, 3.0), (0.3, 0.3), (0.7, 0.2), (0.1, 0.9)]:
        probes.probe("ticker.LogitLocator.nonsingular", logit_nonsingular, params={}, vmin=vmin, vmax=vmax)


def test_auto_minor_locator():
    cases = [
        ([0.0, 0.2, 0.4, 0.6, 0.8, 1.0], [0.0, 1.0]),
        ([0.0, 0.2, 0.4, 0.6, 0.8, 1.0], [0.05, 0.93]),
        ([0.0, 2.0, 4.0, 6.0, 8.0, 10.0], [0.0, 10.0]),
        ([0.0, 2.5, 5.0, 7.5, 10.0], [-1.0, 11.0]),
        ([0.0, 5.0, 10.0], [0.0, 10.0]),
        ([0.0, 3.0, 6.0, 9.0], [0.0, 9.0]),
        ([-1.5, 0.0, 1.5], [-2.0, 2.0]),
        ([100000.0, 100001.0, 100002.0], [100000.0, 100002.0]),
        ([1e-7, 2e-7, 3e-7], [1e-7, 3e-7]),
        ([0.0, 1.0], [1.0, 0.0]),
        ([1.0, 1.0, 2.0], [0.0, 3.0]),
        ([5.0], [0.0, 10.0]),
        ([], [0.0, 10.0]),
    ]
    for major_locs, view in cases:
        for params in [{}, {"n": "auto"}, {"n": 2}, {"n": 4}, {"n": 5}]:
            probes.probe("ticker.AutoMinorLocator.__call__", auto_minor_call, params=params, major_locs=major_locs,
                         view=view, axis_name="x")
    for axis_name, rc in [("y", {"ytick.minor.ndivs": 3}), ("x", {"ytick.minor.ndivs": 3}),
                          ("x", {"xtick.minor.ndivs": 2})]:
        probes.probe("ticker.AutoMinorLocator.__call__", auto_minor_call, params={},
                     major_locs=[0.0, 1.0, 2.0], view=[0.0, 2.0], axis_name=axis_name, **_rc_arg(rc))


# --------------------------------------------------------------------------
# ScalarFormatter
# --------------------------------------------------------------------------

SCALAR_VARIANTS = [
    ({}, None),
    ({"useMathText": True}, None),
    ({"useOffset": False}, None),
    ({"scientific": False}, None),
    ({"useOffset": False, "scientific": False}, None),
    ({"powerlimits": [-3, 4]}, None),
    ({"powerlimits": [0, 0]}, None),
    ({"powerlimits": [3, 3]}, None),
    ({"powerlimits": [-2, -2], "useMathText": True}, None),
    ({}, {"axes.formatter.offset_threshold": 2}),
    ({}, {"axes.formatter.offset_threshold": 6}),
    ({}, {"axes.unicode_minus": False}),
    ({}, {"axes.formatter.useoffset": False}),
    ({}, {"axes.formatter.use_mathtext": True}),
    ({}, {"axes.formatter.limits": [-3, 3]}),
]


def test_scalar_formatter_auto_ticks():
    """Default AutoLocator ticks for many views, formatted the way an Axis would."""
    for vmin, vmax in LINEAR_RANGES:
        if not (math.isfinite(vmin) and math.isfinite(vmax)):
            continue
        locs = _ticks(vmin, vmax)
        view = [vmin, vmax]
        for params, rc in SCALAR_VARIANTS:
            probes.probe("ticker.ScalarFormatter.format_ticks", scalar_format_ticks, params=params, locs=locs,
                         view=view, **_rc_arg(rc))
    labels = scalar_format_ticks({}, _ticks(1e5, 1e5 + 5), [1e5, 1e5 + 5])
    assert labels == {"labels": ["0", "1", "2", "3", "4", "5"], "offset": "+1e5"}
    labels = scalar_format_ticks({}, _ticks(-1e5 - 5, -1e5), [-1e5 - 5, -1e5])
    assert labels["offset"] == "\N{MINUS SIGN}1e5"
    assert scalar_format_ticks({}, _ticks(1e-7, 5e-7), [1e-7, 5e-7])["offset"] == "1e\N{MINUS SIGN}7"


def test_scalar_formatter_explicit_locs():
    cases = [
        ([5.0], [0.0, 10.0]),
        ([], [0.0, 10.0]),
        ([0.0], [0.0, 0.0]),
        ([0.0, 0.0], [-1.0, 1.0]),
        ([1e5], [1e5 - 1, 1e5 + 1]),
        ([20.0, 30.0], [0.0, 10.0]),
        ([1.0, 2.0, 3.0], [3.0, 1.0]),
        ([1e-9, 0.0, -1e-9, 1.0], [-1.0, 1.0]),
        ([0.1, 0.2, 0.30000000000000004], [0.0, 0.4]),
        ([1.0, 1.5, 2.25, 3.125], [1.0, 3.125]),
        ([-0.5, 0.0, 0.5], [-0.5, 0.5]),
        ([999999.0, 1000000.0, 1000001.0], [999999.0, 1000001.0]),
        ([1e5, 1e5 + 0.001, 1e5 + 0.002], [1e5, 1e5 + 0.002]),
        ([12345.0, 12345.5, 12346.0], [12345.0, 12346.0]),
        ([123456.0, 123457.0, 123458.0], [123456.0, 123458.0]),
        ([-2.0, -1.0, 0.0, 1.0], [-2.0, 1.0]),
        ([1e100, 2e100, 3e100], [1e100, 3e100]),
        ([3e-100, 4e-100], [3e-100, 4e-100]),
        ([NAN, 1.0, 2.0], [0.0, 3.0]),
    ]
    for locs, view in cases:
        for params, rc in [({}, None), ({"useMathText": True}, None), ({"useOffset": False}, None),
                           ({"powerlimits": [0, 0]}, None)]:
            probes.probe("ticker.ScalarFormatter.format_ticks", scalar_format_ticks, params=params, locs=locs,
                         view=view, **_rc_arg(rc))
    for offset in [1000.0, 100000.0, -5.0, 0.5, 0.0]:
        for params in [{"useOffset": offset}, {"useOffset": offset, "useMathText": True}]:
            probes.probe("ticker.ScalarFormatter.format_ticks", scalar_format_ticks, params=params,
                         locs=[100000.0, 100002.0, 100004.0], view=[100000.0, 100004.0])
    probes.probe("ticker.ScalarFormatter.format_ticks", scalar_format_ticks, params={"useLocale": False},
                 locs=[1000.0, 2000.0], view=[1000.0, 2000.0])
    probes.probe("ticker.ScalarFormatter.format_ticks", scalar_format_ticks, params={"powerlimits": [1, 2, 3]},
                 locs=[1.0], view=[0.0, 1.0])


def test_scalar_formatter_format_data():
    values = [1.0, 10.0, 100.0, 1e5, 1e6, 123456.0, 1.5e-7, -2.5e10, 0.001, -0.001, 3.14159265358979, 1e-300,
              1e300, 2.0, 0.5, -1.0]
    for value in values:
        for params in [{}, {"useMathText": True}]:
            probes.probe("ticker.ScalarFormatter.format_data", scalar_format_data, params=params, value=value)
    probes.probe("ticker.ScalarFormatter.format_data", scalar_format_data, params={}, value=-3e-5,
                 rc={"axes.unicode_minus": False})
    probes.probe("ticker.ScalarFormatter.format_data", scalar_format_data, params={}, value=0.0)
    for value, view in [(3.14159265, [0.0, 10.0]), (3.14159265, [0.0, 0.001]), (1234567.0, [0.0, 1e7]),
                        (-0.5, [-1.0, 1.0]), (1e-7, [0.0, 1e-6]), (100000.123456, [100000.0, 100001.0]),
                        (2.0, [0.0, 1e10]), (0.0, [-1.0, 1.0]), (5, [0.0, 10.0])]:
        probes.probe("ticker.ScalarFormatter.format_data_short", scalar_format_data_short, params={}, value=value,
                     view=view)


# --------------------------------------------------------------------------
# Log formatters
# --------------------------------------------------------------------------

LOG_FORMATTERS = {
    "LogFormatter": mticker.LogFormatter,
    "LogFormatterExponent": mticker.LogFormatterExponent,
    "LogFormatterMathtext": mticker.LogFormatterMathtext,
    "LogFormatterSciNotation": mticker.LogFormatterSciNotation,
}


def _log_cases():
    """(locs, view) pairs: major and minor LogLocator ticks for several views."""
    cases = []
    for vmin, vmax in [(1.0, 1e3), (1.0, 10.0), (1.0, 5.0), (0.5, 50.0), (1e-5, 1e5), (2.0, 3.0), (1.0, 1e10),
                       (0.01, 0.2), (20.0, 2000.0), (3e4, 7e4), (1e-3, 1.0)]:
        for subs in [(1.0,), "auto"]:
            locs = [float(v) for v in mticker.LogLocator(subs=subs).tick_values(vmin, vmax)]
            cases.append((locs, [vmin, vmax]))
    cases.append(([-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0], [-100.0, 100.0]))
    cases.append(([1.0, 2.0, 4.0, 8.0, 16.0], [1.0, 16.0]))
    cases.append(([1e-310, 1e300], [1e-310, 1e300]))
    return cases


def test_log_formatters():
    param_variants = [
        {},
        {"labelOnlyBase": True},
        {"minor_thresholds": [INF, INF]},
        {"minor_thresholds": [2, 0.5]},
        {"minor_thresholds": [0, 0]},
    ]
    for cls_name, cls in LOG_FORMATTERS.items():
        op = f"ticker.{cls_name}.format_ticks"
        for locs, view in _log_cases():
            for params in param_variants:
                probes.probe(op, _formatter_op(cls), params=params, locs=locs, view=view)
        for params, locs, view in [
            ({"base": 2.0}, [0.5, 1.0, 2.0, 4.0, 8.0, 3.0, 6.0], [0.5, 8.0]),
            ({"base": 2.0, "labelOnlyBase": True}, [0.5, 1.0, 2.0, 4.0, 8.0, 3.0], [0.5, 8.0]),
            ({"base": math.e}, [1.0, math.e, math.e ** 2, 5.0], [1.0, 10.0]),
            ({"base": 16.0}, [1.0, 16.0, 256.0, 4096.0], [1.0, 4096.0]),
            ({"linthresh": 1.0}, [-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0, 20.0, -20.0], [-100.0, 100.0]),
            ({"linthresh": 1.0}, [0.0, 1.0, 2.0, 5.0, 10.0], [0.0, 10.0]),
            ({}, [1.0, 10.0, 100.0], [100.0, 1.0]),
            ({}, [1.0, 10.0, 100.0], [-1.0, 100.0]),
        ]:
            probes.probe(op, _formatter_op(cls), params=params, locs=locs, view=view)
        for rc in [{"axes.formatter.min_exponent": 2}, {"axes.unicode_minus": False},
                   {"_internal.classic_mode": True}]:
            probes.probe(op, _formatter_op(cls), params={}, locs=[1e-3, 0.01, 0.1, 1.0, 10.0, 100.0, 1e3, 5.0],
                         view=[1e-3, 1e3], **_rc_arg(rc))
    for value in [1.0, 1234.5678, 1e-7, -3.25, 123456789.0, 0.0]:
        probes.probe("ticker.LogFormatter.format_data_short", log_format_data_short, value=value)
    result = _formatter_op(mticker.LogFormatterSciNotation)({}, [1.0, 10.0, 100.0, 1000.0, 2.0], [1.0, 1000.0])
    assert result["labels"][:2] == [r"$\mathdefault{10^{0}}$", r"$\mathdefault{10^{1}}$"]


def test_logit_formatter():
    cases = []
    for vmin, vmax in [(1e-4, 1 - 1e-4), (0.1, 0.9), (0.3, 0.7), (0.01, 0.99), (1e-9, 0.5), (0.45, 0.55),
                       (1e-3, 0.2)]:
        major = [float(v) for v in mticker.LogitLocator().tick_values(vmin, vmax)]
        minor = [float(v) for v in mticker.LogitLocator(minor=True).tick_values(vmin, vmax)]
        cases.append((major, minor, [vmin, vmax]))
    for major, minor, view in cases:
        for params in [{}, {"use_overline": True}, {"one_half": "1/2"}]:
            probes.probe("ticker.LogitFormatter.format_ticks", _formatter_op(mticker.LogitFormatter), params=params,
                         locs=major, view=view)
        for params in [{"minor": True}, {"minor": True, "minor_threshold": 5}, {"minor": True, "minor_number": 2}]:
            probes.probe("ticker.LogitFormatter.format_ticks", _formatter_op(mticker.LogitFormatter), params=params,
                         locs=minor, view=view)
    for locs in [[0.0, 0.5, 1.0, -0.1, 1.1], [0.001, 0.002, 0.003], [0.997, 0.998, 0.999], [0.25, 0.5, 0.75],
                 [0.05, 0.95], [0.123]]:
        probes.probe("ticker.LogitFormatter.format_ticks", _formatter_op(mticker.LogitFormatter), params={},
                     locs=locs, view=[0.001, 0.999])


# --------------------------------------------------------------------------
# Engineering, percent and string formatters
# --------------------------------------------------------------------------


def test_eng_formatter():
    values = [0.0, 1.0, -1.0, 999.9, 999.9999, 1000.0, 1e-6, -1.5e-6, 1e-31, 1e-30, 1e-33, 1e30, 1e33, 4.56e7,
              0.000123, 123.456, 1e3 - 1e-9, 0.1, 999999.0, NAN, INF]
    for params in [{}, {"unit": "Hz"}, {"places": 0}, {"places": 1}, {"places": 3, "unit": "m"},
                   {"sep": "", "unit": "V"}, {"sep": "\N{THIN SPACE}", "unit": "s"}, {"useMathText": True},
                   {"useMathText": True, "unit": "W", "places": 2}]:
        for value in values:
            probes.probe("ticker.EngFormatter.format_data", eng_format_data, params=params, value=value)
    for vmin, vmax in [(0.0, 1e4), (0.0, 1.0), (0.0, 1e-6), (-5e9, 5e9), (1e6, 1e6 + 10), (0.0, 3e-12), (1e5, 1e5 + 5)]:
        locs = _ticks(vmin, vmax)
        for params in [{}, {"unit": "Hz"}, {"places": 1}, {"useOffset": True}, {"useOffset": True, "unit": "Hz"},
                       {"useMathText": True, "useOffset": True}]:
            probes.probe("ticker.EngFormatter.format_ticks", eng_format_ticks, params=params, locs=locs,
                         view=[vmin, vmax])
    probes.probe("ticker.EngFormatter.format_data", eng_format_data, params={}, value=-2e-3,
                 rc={"axes.unicode_minus": False})
    assert eng_format_data({"unit": "Hz"}, 1e7) == "10 MHz"
    assert eng_format_data({"places": 1}, 999.99) == "1.0 k"


def test_percent_formatter():
    for params in [{}, {"xmax": 1.0}, {"xmax": 50}, {"decimals": 0}, {"decimals": 2}, {"symbol": None},
                   {"symbol": ""}, {"symbol": " pct"}, {"xmax": 1.0, "decimals": 1, "symbol": "#"}]:
        for locs, view in [([0.0, 25.0, 50.0, 75.0, 100.0], [0.0, 100.0]), ([0.0, 2.0, 4.0], [0.0, 4.0]),
                           ([0.0, 0.2, 0.4], [0.0, 0.4]), ([0.0, 0.02, 0.04], [0.0, 0.04]),
                           ([0.0, 0.2, 0.4, 0.6, 0.8, 1.0], [0.0, 1.0]), ([-50.0, 0.0, 50.0], [-50.0, 50.0]),
                           ([34.5, 34.56], [30.0, 40.0]), ([1.0, 2.0], [1.0, 1.0]), ([1e-7, 2e-7], [1e-7, 2e-7])]:
            probes.probe("ticker.PercentFormatter.format_ticks", _formatter_op(mticker.PercentFormatter),
                         params=params, locs=locs, view=view)
    for params in [{}, {"symbol": "50% $"}, {"symbol": "50% $", "is_latex": True}]:
        probes.probe("ticker.PercentFormatter.format_ticks", _formatter_op(mticker.PercentFormatter), params=params,
                     locs=[10.0, 20.0], view=[0.0, 100.0], rc={"text.usetex": True})


def test_string_formatters():
    locs = [-1500.25, -1.0, -1e-5, 0.0, 1e-5, 0.5, 1.0, 2.0, 1500.25, 1e7]
    view = [-1500.25, 1e7]
    for fmt in ["{x}", "{x:.2f}", "{x:,.0f}", "{x:+.1e}", "{x:.1e}", "{x} km", "{pos}:{x:g}", "{x:.0%}", "{x:g}",
                "{x:>8.3f}", "{x:#x}", "{y}"]:
        probes.probe("ticker.StrMethodFormatter.format_ticks", _formatter_op(mticker.StrMethodFormatter),
                     params={"fmt": fmt}, locs=locs, view=view)
    probes.probe("ticker.StrMethodFormatter.format_ticks", _formatter_op(mticker.StrMethodFormatter),
                 params={"fmt": "{x:.1f}"}, locs=locs, view=view, rc={"axes.unicode_minus": False})
    for fmt in ["%1.2f", "%d", "%.3e", "%s", "%g", "%5.1f%%", "%+.1f", "%x"]:
        probes.probe("ticker.FormatStrFormatter.format_ticks", _formatter_op(mticker.FormatStrFormatter),
                     params={"fmt": fmt}, locs=locs, view=view)
    for seq in [["a", "b", "c"], ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"], []]:
        probes.probe("ticker.FixedFormatter.format_ticks", _formatter_op(mticker.FixedFormatter),
                     params={"seq": seq}, locs=locs, view=view)
    probes.probe("ticker.NullFormatter.format_ticks", null_format_ticks, locs=locs, view=view)
    assert _formatter_op(mticker.StrMethodFormatter)({"fmt": "{x:.1e}"}, [-1e-5], [0, 1])["labels"] == [
        "\N{MINUS SIGN}1.0e\N{MINUS SIGN}05"]
    assert _formatter_op(mticker.FormatStrFormatter)({"fmt": "%d"}, [-1.0], [0, 1])["labels"] == ["-1"]


@pytest.mark.parametrize("vmin, vmax", [(0.0, 1.0), (1e5, 1e5 + 5), (-1e-7, 5e-7)])
def test_auto_ticks_round_trip(vmin, vmax):
    """The AutoLocator ticks used as formatter args are themselves recorded."""
    locs = probes.probe("ticker.AutoLocator.tick_values", auto_tick_values, vmin=vmin, vmax=vmax)
    assert [float(v) for v in locs] == _ticks(vmin, vmax)
