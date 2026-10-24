"""Tick locators and formatters, ported from matplotlib's own
``tests/test_ticker.py`` with the tests' own inputs.

Area ``mpl_ticker``.

matplotlib runs its tests in the ``classic`` style plus
``_classic_test_patch`` unless a test picks another one, and the upstream
expectations hold there. So every probe records the conditions it ran in:
locators and formatters used on their own carry ``rc=CLASSIC_RC``, the
classic values of the rcParams ``matplotlib.ticker`` reads
(``_internal.classic_mode``, ``axes.autolimit_mode``,
``axes.formatter.limits`` and ``axes.formatter.offset_threshold``; the
other ticker parameters are the same in both styles), plus whatever the
test sets itself; probes on an axes carry the whole ``style``. Tests
decorated with ``style.context('default')`` record the reference
configuration (no ``rc``, ``style=["default"]``).

Ops on locators and formatters
------------------------------
The ``ticker`` area's ops, with its argument conventions (``params`` are
constructor keywords, formatters get a dummy axis showing ``view``), each
evaluated inside ``rc_context(rc)``:

``ticker.MaxNLocator.tick_values`` / ``.view_limits``,
``ticker.MultipleLocator.tick_values`` / ``.view_limits``,
``ticker.IndexLocator.tick_values``, ``ticker.LogLocator.tick_values``,
``ticker.LogitLocator.tick_values`` / ``.nonsingular``,
``ticker.SymmetricalLogLocator.tick_values`` / ``.view_limits``,
``ticker.AsinhLocator.tick_values`` / ``.__call__``,
``ticker.ScalarFormatter.format_data`` / ``.format_data_short``,
``ticker.LogitFormatter.format_ticks``,
``ticker.FormatStrFormatter.format_ticks``.

And these:

``ticker.MaxNLocator._steps`` {steps} -> the steps ``MaxNLocator(steps=...)``
    uses: the given ones padded with 1 and 10.
``ticker.AutoLocator._steps`` {rc} -> the steps of ``AutoLocator()``.
``ticker.LinearLocator.tick_values`` {params{numticks, presets}, vmin, vmax,
    rc}: ``presets`` is a list of ``[[vmin, vmax], locs]`` (a dict keyed by
    limits in matplotlib).
``ticker.SymmetricalLogLocator.__call__`` {params{base, linthresh, subs},
    view, rc} -> the ticks for a dummy axis showing ``view``.
``ticker.LogFormatter.__call__`` / ``ticker.LogFormatterExponent.__call__`` /
``ticker.LogFormatterMathtext.__call__`` /
``ticker.LogFormatterSciNotation.__call__`` {params, values, positions?,
    view?, rc} -> ``[formatter(x, pos), ...]`` for a formatter whose
    ``set_locs`` was never called, on a dummy axis showing ``view`` (no axis
    without ``view``); ``positions`` default to ``None``.
``ticker.LogFormatter._pprint_val`` {value, domain} -> the label of
    ``value`` for an axis spanning ``domain``.
``ticker.LogitFormatter.__call__`` {params, locs?, values} ->
    ``[formatter(x), ...]`` after ``set_locs(locs)`` (never called without
    ``locs``).
``ticker.LogitFormatter.format_data_short`` {values} -> one string per value.
``ticker.EngFormatter.__call__`` {params, value, rc} -> the label.
``ticker.PercentFormatter.format_pct`` {params{xmax, decimals, symbol,
    is_latex}, x, display_range, rc} -> the label.
``ticker.ScalarFormatter.get_useOffset`` {useOffset?, rc} -> {useOffset,
    offset} after ``set_useOffset(useOffset)`` when given.
``ticker.ScalarFormatter.get_useMathText`` {rc} -> bool.
``ticker.NullFormatter.__call__`` / ``.format_data`` / ``.format_data_short``
    {value} -> the label.
``ticker.FixedFormatter.get_offset`` / ``ticker.FuncFormatter.get_offset``
    {offset_string?} -> the offset text after ``set_offset_string`` when
    given (``FixedFormatter(('foo', 'bar'))`` and ``FuncFormatter(lambda a:
    f'val: {a}')``).

Ops on an axes
--------------
These run in ``Figure().add_subplot(*subplot, projection=projection)``
(``subplot`` defaults to ``[1, 1, 1]``) inside ``style.context(style)`` and
``rc_context(rc)``, after setting up the axes in this order:

1. ``plot``: ``ax.plot(plot["y"])`` or ``ax.plot(plot["x"], plot["y"])``;
   ``boxplots``: ``ax.boxplot(b["x"], positions=b["positions"])`` for each.
2. ``minorticks_on``: ``ax.minorticks_on()``.
3. For each axis ``"x"`` and ``"y"`` with a config: ``scale`` (``set_xscale``),
   ``lim`` (``set_xlim``), ``ticks`` (``set_xticks``), ``minorticks``
   (``xaxis.minorticks_on()`` / ``minorticks_off()``), ``major`` and
   ``minor`` locators, ``major_formatter`` and ``minor_formatter``.

Locators are ``["AutoMinorLocator", n?]``, ``["MultipleLocator", base]``,
``["MaxNLocator", nbins]`` and ``["LogLocator", {base, subs}]``; formatters
``["ScalarFormatter", {scientific, powerlimits}]``, ``["LogFormatter",
{labelOnlyBase}]`` and ``["EngFormatter", {unit, useOffset, usetex,
useMathText}]``.

``axis.Axis.get_ticklocs`` {axis, minor, ...} -> the tick locations.
``axis.Axis.get_ticklabels`` {axis, minor, ...} -> {labels, offset} after
    drawing: the label texts and the offset text.
``axis.Formatter.format_data`` / ``axis.Formatter.format_data_short`` {axis,
    value, ...} -> what the major formatter of the axis makes of ``value``
    (stripped).
``axes.Axes.get_yticks`` {data, bins} -> {yticks, ylim}: one entry per axes
    of ``Figure().subplots(len(data), sharex=True, sharey=True)``, each
    with ``hist(d, bins=bins)`` and ``set_yscale('log', nonpositive='clip')``.
``axis.Axis.get_minorticklocs`` {remove_overlapping_locs?} -> {locs,
    labels}: ``test_remove_overlap``'s day-locator date axis, with the
    minor locations kept or removed.

Cases
-----
Named after the upstream tests (``TestMaxNLocator.test_basic`` is
``test_max_n_basic``, and so on). ``TestLogLocator.test_multiple_shared_axes``
draws ``spec_support.rng(19680801).normal(100)`` instead of numpy's random
numbers. ``test_loglocator_properties`` records the tick values its
properties are checked on. A test asserting a property of a result records
the result.

Left out
--------
The ``test_set_params`` tests, ``TestLogitLocator.test_minor_attr``,
``TestAsinhLocator.test_init`` (Python attribute setters; plt-rs configures
locators and formatters with typed builders),
``TestScalarFormatter.test_use_locale`` and ``test_locale_comma`` (the
system locale), ``test_empty_locs`` (a formatter called with no ``set_locs``
state; plt-rs formatters label a whole set of ticks at once),
``test_mathtext_ticks`` and ``test_cmr10_substitutions`` (warnings and log
records about fonts), ``TestStrMethodFormatter.test_basic`` (formats
Python ints with ``{x:05d}``; plt-rs tick values are always floats, and the
``ticker`` area probes the minus signs of float templates),
``TestLogitFormatter.test_logit_deformater`` (tests the test's own parser), ``test_majformatter_type``,
``test_minformatter_type``, ``test_majlocator_type``,
``test_minlocator_type`` and ``test_bad_locator_subs`` (Python type checks;
the ``ticker`` area probes bad ``subs`` shapes).
"""

import itertools

import matplotlib as mpl
import matplotlib.dates as mdates
import matplotlib.ticker as mticker
import numpy as np
import pytest
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from probes.test_ticker import (
    asinh_call, asinh_tick_values, index_tick_values, log_tick_values, logit_nonsingular,
    logit_tick_values, maxn_tick_values, maxn_view_limits, multiple_tick_values,
    multiple_view_limits, scalar_format_data, scalar_format_data_short, symlog_tick_values,
    symlog_view_limits, _formatter_op)
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_ticker")

pytestmark = pytest.mark.filterwarnings("ignore::UserWarning")

CLASSIC_RC = {
    "_internal.classic_mode": True,
    "axes.autolimit_mode": "round_numbers",
    "axes.formatter.limits": [-7, 7],
    "axes.formatter.offset_threshold": 2,
}
CLASSIC_STYLE = ["classic", "_classic_test_patch"]
MINUS = "\N{MINUS SIGN}"


def _in_rc(fn):
    """``fn`` evaluated inside ``rc_context(rc)``."""
    def evaluate(rc=None, **kw):
        with mpl.rc_context(rc or {}):
            return fn(**kw)

    return evaluate


def classic(**extra):
    return {**CLASSIC_RC, **extra}


def probe(op, fn, rc=CLASSIC_RC, **args):
    return probes.probe(op, _in_rc(fn), rc=rc, **args)


# ---------------------------------------------------------------------------
# Locators on their own
# ---------------------------------------------------------------------------


def test_max_n_basic():
    for vmin, vmax in [(20, 100), (0.001, 0.0001), (-1e15, 1e15), (0, 0.85e-50), (-0.85e-50, 0)]:
        probe("ticker.MaxNLocator.tick_values", maxn_tick_values, params={"nbins": 5}, vmin=vmin,
              vmax=vmax)


def test_max_n_integer():
    for vmin, vmax, steps in [(-0.1, 1.1, None), (-0.1, 0.95, None), (1, 55, [1, 1.5, 5, 6, 10])]:
        params = {"nbins": 5, "integer": True, **({} if steps is None else {"steps": steps})}
        probe("ticker.MaxNLocator.tick_values", maxn_tick_values, params=params, vmin=vmin, vmax=vmax)


def test_max_n_errors():
    for params in [{"foo": 0}, {"steps": [2, 1]}, {"steps": 2}, {"steps": [2, 11]}]:
        error = probe("ticker.MaxNLocator.tick_values", maxn_tick_values, params=params, vmin=0,
                      vmax=1)
        assert isinstance(error, (TypeError, ValueError))


def test_max_n_padding():
    for steps in ([1, 2, 10], [2, 10], [1, 2], [2]):
        probes.probe("ticker.MaxNLocator._steps",
                     lambda steps: mticker.MaxNLocator(steps=steps)._steps, steps=steps)


def _linear(params, vmin, vmax):
    params = dict(params)
    presets = params.pop("presets", None)
    if presets is not None:
        params["presets"] = {tuple(limits): locs for limits, locs in presets}
    return mticker.LinearLocator(**params).tick_values(vmin, vmax)


def test_linear_locator():
    probe("ticker.LinearLocator.tick_values", _linear, params={"numticks": 3}, vmin=-0.8, vmax=0.2)
    probe("ticker.LinearLocator.tick_values", _linear, params={"numticks": 0}, vmin=-0.8, vmax=0.2)


def test_linear_presets():
    presets = [[[1, 2], [1, 1.25, 1.75]], [[0, 2], [0.5, 1.5]]]
    for vmin, vmax in [(1, 2), (2, 1), (0, 2), (0.0, 2.0), (0, 1)]:
        probe("ticker.LinearLocator.tick_values", _linear, params={"presets": presets}, vmin=vmin,
              vmax=vmax)


def test_multiple_locator():
    for params in ({"base": 3.147}, {"base": 3.147, "offset": 1.2}):
        probe("ticker.MultipleLocator.tick_values", multiple_tick_values, params=params, vmin=-7,
              vmax=10)


def test_multiple_view_limits():
    probe("ticker.MultipleLocator.view_limits", multiple_view_limits, params={"base": 3.147},
          dmin=-5, dmax=5, rc=classic(**{"axes.autolimit_mode": "data"}))
    for params in ({"base": 3.147}, {"base": 3.147, "offset": 1.3}):
        probe("ticker.MultipleLocator.view_limits", multiple_view_limits, params=params, dmin=-4,
              dmax=4)
    probe("ticker.MaxNLocator.view_limits", maxn_view_limits, params={"nbins": 1}, dmin=-2.3,
          dmax=2.3)


def test_using_all_default_major_steps():
    probes.probe("ticker.AutoLocator._steps", _in_rc(lambda: mticker.AutoLocator()._steps),
                 rc=classic(**{"_internal.classic_mode": False}))


def test_log_locator_basic():
    error = probe("ticker.LogLocator.tick_values", log_tick_values, params={"numticks": 5}, vmin=0,
                  vmax=1000)
    assert isinstance(error, ValueError)
    probe("ticker.LogLocator.tick_values", log_tick_values, params={"numticks": 5}, vmin=0.001,
          vmax=1.1e5)
    probe("ticker.LogLocator.tick_values", log_tick_values, params={"base": 2}, vmin=1, vmax=100)


def test_log_switch_to_autolocator():
    probe("ticker.LogLocator.tick_values", log_tick_values, params={"subs": "all"}, vmin=0.45,
          vmax=0.55)
    ticks = probe("ticker.LogLocator.tick_values", log_tick_values,
                  params={"subs": [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]}, vmin=0.9, vmax=20.0)
    assert 1.0 not in ticks and 10.0 not in ticks
    probe("ticker.LogLocator.tick_values", log_tick_values, params={"subs": "auto"}, vmin=10, vmax=20)


def test_log_tick_values():
    probe("ticker.LogLocator.tick_values", log_tick_values, params={"subs": [1, 2, 5]}, vmin=1,
          vmax=1e7)
    probe("ticker.LogLocator.tick_values", log_tick_values, params={"subs": [1, 2, 5]}, vmin=1,
          vmax=1e8, rc=classic(**{"_internal.classic_mode": False}))


def test_small_range_loglocator():
    for numticks, lims in itertools.product((1, 2, 3, 9), ((.5, 5), (.5, 50), (.5, 500), (.5, 5000))):
        probe("ticker.LogLocator.tick_values", log_tick_values, params={"numticks": numticks},
              vmin=lims[0], vmax=lims[1], rc=None)


def test_loglocator_properties():
    for numticks, (lo, hi) in itertools.product(range(1, 9), itertools.combinations(range(20), 2)):
        probe("ticker.LogLocator.tick_values", log_tick_values, params={"numticks": numticks},
              vmin=float(10 ** lo), vmax=float(10 ** hi), rc=None)


LOGIT_BASIC = [(5 * 10.0 ** -k, 1 - 5 * 10.0 ** -k) for k in range(2, 10)]
LOGIT_MAXN = [(0.4, 0.6), (5e-2, 2e-1), (1 - 2e-1, 1 - 5e-2)]


def logit(params, lims):
    return probe("ticker.LogitLocator.tick_values", logit_tick_values, params=params, vmin=lims[0],
                 vmax=lims[1])


def test_logit_basic_major():
    for lims in LOGIT_BASIC:
        logit({"nbins": 100}, lims)


def test_logit_maxn_major():
    for lims in LOGIT_MAXN:
        for nbins in (4, 8, 16):
            logit({"nbins": nbins}, lims)
            probe("ticker.MaxNLocator.tick_values", maxn_tick_values,
                  params={"nbins": nbins, "steps": [1, 2, 5, 10]}, vmin=lims[0], vmax=lims[1])


def test_logit_nbins_and_minor():
    for lims in LOGIT_BASIC + LOGIT_MAXN:
        basic_needed = int(-np.floor(np.log10(lims[0]))) * 2 + 1
        for nbins in range(basic_needed, 2, -1):
            assert len(logit({"nbins": nbins}, lims)) <= nbins + 2
            if lims in LOGIT_BASIC:
                logit({"nbins": nbins, "minor": True}, lims)


def test_logit_nonsingular():
    acceptable = [*(2.5 ** np.arange(-3, 0)), *(1 - 2.5 ** np.arange(-3, 0))]
    for a, b in itertools.product(acceptable, repeat=2):
        if a != b:
            probe("ticker.LogitLocator.nonsingular", logit_nonsingular, params={}, vmin=a, vmax=b)
    for okval in acceptable:
        for vmin, vmax in ((-1, okval), (okval, 2)):
            probe("ticker.LogitLocator.nonsingular", logit_nonsingular, params={}, vmin=vmin, vmax=vmax)


def test_index_tick_values_not_exceeding_vmax():
    for params, vmax in [({"base": 1, "offset": 0}, 4), ({"base": 1, "offset": 0.5}, 4),
                         ({"base": 2, "offset": 0}, 5)]:
        probe("ticker.IndexLocator.tick_values", index_tick_values, params=params, vmin=0, vmax=vmax)


def _symlog_call(params, view):
    locator = mticker.SymmetricalLogLocator(**params)
    locator.create_dummy_axis()
    locator.axis.set_view_interval(*view)
    return locator()


def test_symlog_locator():
    params = {"base": 10, "linthresh": 1}
    for vmin, vmax in [(0, 1), (-1, 1)]:
        probe("ticker.SymmetricalLogLocator.tick_values", symlog_tick_values, params=params, vmin=vmin,
              vmax=vmax)
    probe("ticker.SymmetricalLogLocator.__call__", _symlog_call, params={**params, "subs": [2.0, 4.0]},
          view=[-10, 10])
    for view in ([8, 9], [8, 12]):
        probe("ticker.SymmetricalLogLocator.__call__", _symlog_call, params=params, view=view)
    for vmin, vmax in [(10, 10), (-10, -10), (0, 0)]:
        probe("ticker.SymmetricalLogLocator.view_limits", symlog_view_limits, params=params,
              vmin=vmin, vmax=vmax)


def test_asinh_locator():
    for params, ranges in [
        ({"linear_width": 100, "numticks": 11, "base": 0}, [(-1, 1), (-0.1, 0.1), (-0.01, 0.01)]),
        ({"linear_width": 0.1, "numticks": 11, "base": 0}, [(-100, 100), (-1000, 1000)]),
        ({"linear_width": 100, "numticks": 3, "base": 0}, [(-1.1, 0.9)]),
        ({"linear_width": 1.0, "numticks": 11}, [(101, 102)]),
        ({"linear_width": 1, "numticks": 8, "base": 10, "subs": [1, 3, 5]}, [(-110, 110)]),
        ({"linear_width": 1, "numticks": 20, "base": 5}, [(-1050, 1050)]),
    ]:
        for vmin, vmax in ranges:
            probe("ticker.AsinhLocator.tick_values", asinh_tick_values, params=params, vmin=vmin,
                  vmax=vmax)
    params = {"linear_width": 1, "numticks": 3, "symthresh": 0.25, "base": 0}
    for view in ([-1, 2], [-1, 0.9], [-0.85, 1.05], [1, 1.1]):
        probe("ticker.AsinhLocator.__call__", asinh_call, params=params, view=view)


# ---------------------------------------------------------------------------
# Formatters on their own
# ---------------------------------------------------------------------------


def test_scalar_use_offset():
    def get_use_offset(useOffset=None):
        formatter = mticker.ScalarFormatter()
        if useOffset is not None:
            formatter.set_useOffset(useOffset)
        return {"useOffset": formatter.get_useOffset(), "offset": formatter.offset}

    for use_offset in (True, False):
        probe("ticker.ScalarFormatter.get_useOffset", get_use_offset,
              rc=classic(**{"axes.formatter.useoffset": use_offset}))
    for value in (0.5, True, False, 1):
        probe("ticker.ScalarFormatter.get_useOffset", get_use_offset, useOffset=value)
    for mathtext in (True, False):
        probe("ticker.ScalarFormatter.get_useMathText",
              lambda: mticker.ScalarFormatter().get_useMathText(),
              rc=classic(**{"axes.formatter.use_mathtext": mathtext}))


def test_scalar_format_data():
    for value in (.1, .11, 1e8, 1.1e8):
        probe("ticker.ScalarFormatter.format_data", scalar_format_data, params={}, value=value,
              rc=classic(**{"axes.unicode_minus": False}))


CURSOR_DATA = [0., 0.0123, 0.123, 1.23, 12.3]


def test_cursor_dummy_axis():
    for value in CURSOR_DATA:
        probe("ticker.ScalarFormatter.format_data_short", scalar_format_data_short, params={},
              value=value, view=[0, 10])


def _log_call(cls):
    def call(params, values, positions=None, view=None):
        formatter = cls(**params)
        if view is not None:
            formatter.create_dummy_axis()
            formatter.axis.set_view_interval(*view)
        positions = positions or [None] * len(values)
        return [formatter(x, pos) for x, pos in zip(values, positions)]

    return call


def log_call(name, rc=CLASSIC_RC, **args):
    probe(f"ticker.{name}.__call__", _log_call(getattr(mticker, name)), rc=rc, **args)


def test_log_formatter_exponent():
    cases = [
        (True, 4, np.arange(-3, 4.0), list(range(7))),
        (False, 10, np.array([0.1, 0.00001, np.pi, 0.2, -0.2, -0.00001]), list(range(6))),
        (False, 50, np.array([3, 5, 12, 42], dtype=float), list(range(4))),
    ]
    for label_only_base, exponent, locs, positions in cases:
        for base in (2.0, 5.0, 10.0, np.pi, np.e):
            log_call("LogFormatterExponent", params={"base": base, "labelOnlyBase": label_only_base},
                     values=(base ** locs).tolist(), positions=positions, view=[1, base ** exponent])
    log_call("LogFormatterExponent", params={"base": 10, "labelOnlyBase": True},
             values=[10 ** 0.1], view=[1, 10])


def test_log_formatter_mathtext_min_exponent():
    for min_exponent, value in [(0, 1), (0, 1e-2), (0, 1e2), (3, 1), (3, 1e-2), (3, 1e2), (3, 1e-3),
                                (3, 1e3)]:
        log_call("LogFormatterMathtext", params={}, values=[value],
                 rc=classic(**{"axes.formatter.min_exponent": min_exponent}))


def test_log_formatter_sci_notation():
    for base, value in [(2, 0.03125), (2, 1), (2, 32), (2, 0.0375), (2, 1.2), (2, 38.4), (10, -1),
                        (10, 1e-05), (10, 1), (10, 100000), (10, 2e-05), (10, 2), (10, 200000),
                        (10, 5e-05), (10, 5), (10, 500000)]:
        log_call("LogFormatterSciNotation", params={"base": base}, values=[value],
                 rc={"text.usetex": False})


PPRINT_VALUES = [3.141592654e-05, 0.0003141592654, 0.003141592654, 0.03141592654, 0.3141592654,
                 3.141592654, 31.41592654, 314.1592654, 3141.592654, 31415.92654, 314159.2654,
                 1e-05, 0.0001, 0.001, 0.01, 0.1, 1, 10, 100, 1000, 10000, 100000]


def test_log_formatter_pprint():
    for domain in (0.001, 0.015, 0.5, 5, 100, 1000000.0):
        for value in PPRINT_VALUES:
            probe("ticker.LogFormatter._pprint_val",
                  lambda value, domain: mticker.LogFormatter()._pprint_val(value, domain),
                  value=value, domain=domain)


def test_log_formatter_call():
    for value in (1, 10, 100, 1000, 1e-323, 2e-323, 10e-323, 11e-323):
        log_call("LogFormatter", params={}, values=[value], view=[1, 10])


DECADES = sorted([10 ** (-i) for i in range(1, 10)] + [1 - 10 ** (-i) for i in range(1, 10)] + [1 / 2])


def _logit_call(params, values, locs=None):
    formatter = mticker.LogitFormatter(**params)
    if locs is not None:
        formatter.set_locs(locs)
    return [formatter(x) for x in values]


def logit_call(**args):
    probe("ticker.LogitFormatter.__call__", _logit_call, **args)


def test_logit_formatter_basic_and_invalid():
    params = {"use_overline": False}
    probe("ticker.LogitFormatter.format_ticks", _formatter_op(mticker.LogitFormatter), params=params,
          locs=DECADES, view=[DECADES[0], DECADES[-1]])
    logit_call(params=params, locs=DECADES, values=[-1, -0.5, -0.1, 1.1, 1.5, 2])


def test_logit_formatter_variable_length():
    for x in 1 / (1 + np.exp(-np.linspace(-7, 7, 10))):
        for n in (10, 20, 50, 100, 200, 1000, 2000, 5000, 10000):
            if x + 1 / n < 1:
                logit_call(params={"use_overline": False}, locs=[x - 1 / n, x, x + 1 / n],
                           values=[x, x + 1 / n])


def test_logit_formatter_minor_vs_major():
    cases = [
        (True, (5e-8, 1 - 5e-8), (25, 75)),
        (True, (5e-5, 1 - 5e-5), (25, 75)),
        (True, (5e-2, 1 - 5e-2), (25, 75)),
        (False, (0.75, 0.76, 0.77), (7, 25, 75)),
    ]
    for method, lims, thresholds in cases:
        locs = logit({"minor": True}, lims) if method else np.array(lims)
        for threshold in thresholds:
            probe("ticker.LogitFormatter.format_ticks", _formatter_op(mticker.LogitFormatter),
                  params={"minor": True, "minor_threshold": threshold}, locs=locs.tolist(),
                  view=[0.0, 1.0])


def test_logit_formatter_minor_number():
    locs = logit({"minor": True}, (5e-2, 1 - 5e-2)).tolist()
    for minor_number in (2, 4, 8, 16):
        probe("ticker.LogitFormatter.format_ticks", _formatter_op(mticker.LogitFormatter),
              params={"minor": True, "minor_number": minor_number}, locs=locs, view=[0.0, 1.0])


def test_logit_formatter_overline_and_one_half():
    for use_overline in (False, True):
        logit_call(params={"use_overline": use_overline}, values=[1 - 1e-2])
    for one_half in (None, "1/2", "one half"):
        logit_call(params={} if one_half is None else {"one_half": one_half}, values=[1 / 2])


def test_logit_formatter_format_data_short():
    for n in (100, 253, 754):
        probe("ticker.LogitFormatter.format_data_short",
              lambda values: [mticker.LogitFormatter().format_data_short(x) for x in values],
              values=np.linspace(0, 1, n)[1:-1].tolist())


def test_format_str_formatter():
    probe("ticker.FormatStrFormatter.format_ticks", _formatter_op(mticker.FormatStrFormatter),
          params={"fmt": "%05d"}, locs=[2], view=[0, 10])




ENG_DATA = [
    (False, -1234.56789), (True, -1234.56789), (False, -1.23456789), (True, -1.23456789),
    (False, -0.123456789), (True, -0.123456789), (False, -0.00123456789), (True, -0.00123456789),
    (True, -0.0), (True, -0), (True, 0), (True, 1.23456789e-6), (True, 0.123456789), (True, 0.1),
    (True, 1), (True, 1.23456789), (True, 999.9), (True, 999.9999), (False, -999.9999),
    (True, -999.9999), (True, 1000), (True, 1001), (True, 100001), (True, 987654.321),
    (True, 1.23e33),
]


def _eng_call(params, value):
    return mticker.EngFormatter(**params)(value)


def test_eng_formatter_params():
    for unicode_minus, value in ENG_DATA:
        for places in (None, 0, 2):
            for unit, sep in itertools.product((None, "s"),
                                               (None, "", "\N{NARROW NO-BREAK SPACE}", "@_@")):
                params = {k: v for k, v in (("places", places), ("unit", unit), ("sep", sep))
                          if v is not None}
                probe("ticker.EngFormatter.__call__", _eng_call, params=params, value=value,
                      rc=classic(**{"axes.unicode_minus": unicode_minus}))


def _format_pct(params, x, display_range):
    return mticker.PercentFormatter(**params).format_pct(x, display_range)


def test_percent_formatter():
    for xmax, decimals, symbol, x, display_range in [
        (100, 0, '%', 120, 100), (100, 0, '%', 100, 90), (100, 0, '%', 90, 50),
        (100, 0, '%', -1.7, 40), (100, 1, '%', 90.0, 100), (100, 1, '%', 80.1, 90),
        (100, 1, '%', 70.23, 50), (100, 1, '%', -60.554, 40), (100, None, '%', 95, 1),
        (1.0, None, '%', 3, 6), (17.0, None, '%', 1, 8.5), (17.0, None, '%', 1, 8.4),
        (5, None, '%', -100, 0.000001), (1.0, 2, None, 1.2, 100), (75, 3, '', 50, 100),
        (42, None, '^^Foobar$$', 21, 12),
    ]:
        probe("ticker.PercentFormatter.format_pct", _format_pct,
              params={"xmax": xmax, "decimals": decimals, "symbol": symbol}, x=x,
              display_range=display_range, rc=classic(**{"text.usetex": False}))
    for is_latex, usetex in itertools.product((False, True), repeat=2):
        probe("ticker.PercentFormatter.format_pct", _format_pct,
              params={"symbol": "\\{t}%", "is_latex": is_latex}, x=50, display_range=100,
              rc=classic(**{"text.usetex": usetex}))


def test_null_formatter():
    formatter = mticker.NullFormatter()
    for name, fn in (("__call__", formatter), ("format_data", formatter.format_data),
                     ("format_data_short", formatter.format_data_short)):
        probe(f"ticker.NullFormatter.{name}", lambda value, _fn=fn: _fn(value), value=1.0)


def test_set_offset_string():
    for name, make in (("FuncFormatter", lambda: mticker.FuncFormatter(lambda a: f'val: {a}')),
                       ("FixedFormatter", lambda: mticker.FixedFormatter(('foo', 'bar')))):
        def get_offset(offset_string=None, _make=make):
            formatter = _make()
            if offset_string is not None:
                formatter.set_offset_string(offset_string)
            return formatter.get_offset()

        probe(f"ticker.{name}.get_offset", get_offset)
        probe(f"ticker.{name}.get_offset", get_offset, offset_string="mpl")


# ---------------------------------------------------------------------------
# On an axes
# ---------------------------------------------------------------------------

LOCATORS = {
    "AutoMinorLocator": mticker.AutoMinorLocator,
    "MultipleLocator": mticker.MultipleLocator,
    "MaxNLocator": mticker.MaxNLocator,
    "LogLocator": lambda kw: mticker.LogLocator(**kw),
}
FORMATTERS = {
    "ScalarFormatter": mticker.ScalarFormatter,
    "LogFormatter": mticker.LogFormatter,
    "EngFormatter": mticker.EngFormatter,
}


def _locator(spec):
    name, *args = spec
    return LOCATORS[name](*args)


def _formatter(spec):
    name, params = spec
    params = dict(params)
    scientific = params.pop("scientific", None)
    powerlimits = params.pop("powerlimits", None)
    formatter = FORMATTERS[name](**params)
    if scientific is not None:
        formatter.set_scientific(scientific)
    if powerlimits is not None:
        formatter.set_powerlimits(powerlimits)
    return formatter


def _configure(axes, name, config):
    axis = getattr(axes, f"{name}axis")
    if "scale" in config:
        getattr(axes, f"set_{name}scale")(config["scale"])
    if "lim" in config:
        getattr(axes, f"set_{name}lim")(*config["lim"])
    if "ticks" in config:
        getattr(axes, f"set_{name}ticks")(config["ticks"])
    if "minorticks" in config:
        (axis.minorticks_on if config["minorticks"] else axis.minorticks_off)()
    if "major" in config:
        axis.set_major_locator(_locator(config["major"]))
    if "minor" in config:
        axis.set_minor_locator(_locator(config["minor"]))
    if "major_formatter" in config:
        axis.set_major_formatter(_formatter(config["major_formatter"]))
    if "minor_formatter" in config:
        axis.set_minor_formatter(_formatter(config["minor_formatter"]))


def _on_axes(query):
    def evaluate(axis, style, rc=None, subplot=(1, 1, 1), projection=None, plot=None,
                 boxplots=(), minorticks_on=False, x=None, y=None, **kw):
        with mpl.style.context(style), mpl.rc_context(rc or {}):
            fig = Figure()
            FigureCanvasAgg(fig)
            ax = fig.add_subplot(*subplot, projection=projection)
            if plot is not None:
                ax.plot(*([plot["x"]] if "x" in plot else []), plot["y"])
            for box in boxplots:
                ax.boxplot(box["x"], positions=box["positions"])
            if minorticks_on:
                ax.minorticks_on()
            for name, config in (("x", x), ("y", y)):
                if config is not None:
                    _configure(ax, name, config)
            return query(fig, getattr(ax, f"{axis}axis"), **kw)

    return evaluate


def _ticklocs(fig, axis, minor):
    return axis.get_ticklocs(minor=minor)


def _ticklabels(fig, axis, minor):
    fig.canvas.draw()
    labels = axis.get_ticklabels(minor=minor)
    return {"labels": [label.get_text() for label in labels],
            "offset": axis.get_offset_text().get_text()}


def _format_data(method):
    def query(fig, axis, value):
        return getattr(axis.get_major_formatter(), method)(value).strip()

    return query


def on_axes(op, query, style=CLASSIC_STYLE, **args):
    return probes.probe(op, _on_axes(query), style=style, **args)


def ticklocs(axis="x", minor=True, **scene):
    return on_axes("axis.Axis.get_ticklocs", _ticklocs, axis=axis, minor=minor, **scene)


def ticklabels(axis="x", minor=False, **scene):
    return on_axes("axis.Axis.get_ticklabels", _ticklabels, axis=axis, minor=minor, **scene)


def format_data(method, **scene):
    return on_axes(f"axis.Formatter.{method}", _format_data(method), axis="x", **scene)


def test_auto_minor_basic():
    ticklocs(x={"lim": [0, 1.39]}, minorticks_on=True)
    for lim in ([-1.9, 1.9], [-5, 5]):
        ticklocs(x={"lim": lim, "minor": ["AutoMinorLocator"]})


def test_auto_minor_low_number_of_majorticks():
    for count in (0, 1):
        ticklocs(x={"lim": [0, 5], "ticks": np.linspace(0, 5, count).tolist(),
                    "minor": ["AutoMinorLocator"]}, minorticks_on=True)


def test_auto_minor_number_of_minor_ticks():
    for step in (1, 2, 2.5, 5, 10):
        ticklocs(x={"lim": [0, step], "ticks": [0, step], "minor": ["AutoMinorLocator"]},
                 minorticks_on=True)


def test_auto_minor_additional():
    for lim in [(0, 1.39), (0, 0.139), (0, 0.11e-19), (0, 0.112e-12), (-2.0e-07, -3.3e-08),
                (1.20e-06, 1.42e-06), (-1.34e-06, -1.44e-06), (-8.76e-07, -1.51e-06)]:
        ticklocs(axis="y", y={"lim": list(lim)}, minorticks_on=True)


def test_auto_minor_ndivs():
    for use_rcparam in (False, True):
        for lim in ((0, 1.39), (0, 0.139)):
            rc = {"xtick.minor.ndivs": "auto", "ytick.minor.ndivs": "auto"} if use_rcparam else None
            minor = ["AutoMinorLocator"] if use_rcparam else ["AutoMinorLocator", "auto"]
            config = {"lim": list(lim), "minor": minor}
            for axis in ("x", "y"):
                ticklocs(axis=axis, rc=rc, x=config, y=config)
        for n, lim in ((2, (0, 4)), (4, (0, 2)), (10, (0, 1))):
            rc = {"xtick.minor.ndivs": n, "ytick.minor.ndivs": n} if use_rcparam else None
            minor = ["AutoMinorLocator"] if use_rcparam else ["AutoMinorLocator", n]
            config = {"lim": list(lim), "major": ["MultipleLocator", 1], "minor": minor}
            for axis in ("x", "y"):
                ticklocs(axis=axis, rc=rc, x=config, y=config)


def test_log_polar_axes():
    ticklocs(axis="y", minor=False, projection="polar", y={"scale": "log", "lim": [1, 100]})


def _shared_hist(data, bins):
    with mpl.style.context(CLASSIC_STYLE):
        axes = Figure().subplots(len(data), sharex=True, sharey=True)
        for ax, d in zip(axes.flatten(), data):
            ax.hist(d, bins=bins)
            ax.set_yscale("log", nonpositive="clip")
        return {"yticks": [ax.get_yticks() for ax in axes], "ylim": [ax.get_ylim() for ax in axes]}


def test_log_multiple_shared_axes():
    result = probes.probe("axes.Axes.get_yticks", _shared_hist, data=[rng(19680801).normal(100).tolist(),
                                                                      [], []], bins=10)
    assert all((ticks == result["yticks"][0]).all() for ticks in result["yticks"])


def test_scalar_unicode_minus():
    for unicode_minus in (True, False):
        format_data("format_data_short", value=-1, rc={"axes.unicode_minus": unicode_minus})


OFFSET_DATA = [
    (123, 189), (-189, -123), (12341, 12349), (-12349, -12341), (99999.5, 100010.5),
    (-100010.5, -99999.5), (99990.5, 100000.5), (-100000.5, -99990.5), (1233999, 1234001),
    (-1234001, -1233999), (1, 1), (123, 123), (.4538, .4578), (3789.12, 3783.1),
    (45124.3, 45831.75), (0.000721, 0.0007243), (12592.82, 12591.43), (9., 12.), (900., 1200.),
    (1900., 1200.), (0.99, 1.01), (9.99, 10.01), (99.99, 100.01), (5.99, 6.01), (15.99, 16.01),
    (-0.452, 0.492), (-0.492, 0.492), (12331.4, 12350.5), (-12335.3, 12335.3),
]


def test_scalar_offset_value():
    for left, right in OFFSET_DATA:
        for lim in ([left, right], [right, left]):
            ticklabels(x={"lim": lim})


def test_scalar_scilimits():
    for sci_type, scilimits, lim, fewticks in [
        (False, (0, 0), (10.0, 20.0), False), (True, (-2, 2), (-10, 20), False),
        (True, (-2, 2), (-20, 10), False), (True, (-2, 2), (-110, 120), False),
        (True, (-2, 2), (-120, 110), False), (True, (-2, 2), (-.001, 0.002), False),
        (True, (-7, 7), (0.18e10, 0.83e10), True), (True, (0, 0), (-1e5, 1e5), False),
        (True, (6, 6), (-1e5, 1e5), False),
    ]:
        config = {"lim": list(lim), "major_formatter": ["ScalarFormatter", {
            "scientific": sci_type, "powerlimits": list(scilimits)}]}
        if fewticks:
            config["major"] = ["MaxNLocator", 4]
        ticklabels(axis="y", y=config)


def test_scalar_cursor_precision():
    for value in CURSOR_DATA:
        format_data("format_data_short", value=value, x={"lim": [-1, 1]})


def test_log_formatter_format_data():
    for value in (0.0, 0, -1.0, 2e-10, 1e10):
        for method in ("format_data", "format_data_short"):
            format_data(method, value=value, x={"scale": "log"})


def test_log_formatter_sublabel():
    config = {
        "scale": "log",
        "major": ["LogLocator", {"base": 10, "subs": []}],
        "minor": ["LogLocator", {"base": 10, "subs": list(range(2, 10))}],
        "major_formatter": ["LogFormatter", {"labelOnlyBase": True}],
        "minor_formatter": ["LogFormatter", {"labelOnlyBase": False}],
    }
    for lim in ([1, 1e4], [1, 800], [1, 80], [.8, 9], [1, 8], [0.5, 0.9]):
        for minor in (False, True):
            ticklabels(minor=minor, style=["default"], x={**config, "lim": lim})


def test_engformatter_usetex_use_math_text():
    for params in ({"usetex": True}, {"useMathText": True}):
        ticklabels(plot={"x": [0, 500, 1000], "y": [0, 500, 1000]},
                   x={"ticks": [0, 500, 1000], "major_formatter": ["EngFormatter", params]})


def test_engformatter_offset_oom():
    for data_offset, noise in [
        (271_490_000_000.0, 10), (27_149_000_000_000.0, 10_000_000), (27.149, 0.01),
        (2_714.9, 0.01), (271_490.0, 0.001), (271.49, 0.001), (27_149_000_000.0, 100),
        (27.149, 1e-07), (271.49, 0.0001), (27.149, 0.0001), (27_149.0, 10_000), (27.149, 10_000),
        (27.149, 1_000), (27.149, 100), (27.149, 10),
    ]:
        ydata = data_offset + np.arange(-5, 7, dtype=float) * noise
        ticklabels(axis="y", plot={"y": ydata.tolist()},
                   y={"major_formatter": ["EngFormatter", {"useOffset": True, "unit": "eV"}]})


def test_minorticks_rc():
    for index, (xminor, yminor) in enumerate(itertools.product((False, True), repeat=2), start=1):
        rc = {"xtick.minor.visible": xminor, "ytick.minor.visible": yminor}
        for axis in ("x", "y"):
            ticklocs(axis=axis, rc=rc, subplot=[2, 2, index])


def test_minorticks_toggle():
    for scale in ("linear", "log", "asinh", "logit"):
        for index, (xminor, yminor) in enumerate([(False, False), (True, False), (False, True),
                                                  (True, True)], start=1):
            for axis in ("x", "y"):
                ticklocs(axis=axis, subplot=[2, 2, index], x={"scale": scale, "minorticks": xminor},
                         y={"scale": scale, "minorticks": yminor})


def _remove_overlap(remove_overlapping_locs=None):
    with mpl.style.context(CLASSIC_STYLE):
        t = np.arange("2018-11-03", "2018-11-06", dtype="datetime64")
        ax = Figure().subplots()
        ax.plot(t, np.ones(len(t)))
        ax.xaxis.set_major_locator(mdates.DayLocator())
        ax.xaxis.set_major_formatter(mdates.DateFormatter('\n%a'))
        ax.xaxis.set_minor_locator(mdates.HourLocator((0, 6, 12, 18)))
        ax.xaxis.set_minor_formatter(mdates.DateFormatter('%H:%M'))
        if remove_overlapping_locs is not None:
            ax.xaxis.remove_overlapping_locs = remove_overlapping_locs
        return {"locs": ax.xaxis.get_minorticklocs(),
                "labels": [label.get_text() for label in ax.xaxis.get_minorticklabels()]}


def test_remove_overlap():
    for remove in (True, None, False):
        result = probes.probe("axis.Axis.get_minorticklocs", _remove_overlap,
                              **({} if remove is None else {"remove_overlapping_locs": remove}))
        assert len(result["locs"]) == (9 if remove is False else 6)


def test_minorticks_on_multi_fig():
    boxes = [{"x": list(range(10)), "positions": [p]} for p in (0, 0, 1)]
    ticklocs(boxplots=boxes, minorticks_on=True)
