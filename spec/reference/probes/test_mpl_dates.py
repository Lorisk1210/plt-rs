"""Date conversion, date locators and date formatters, ported from
matplotlib's own ``tests/test_dates.py`` with the tests' own inputs.

Area ``mpl_dates``.

Arguments follow the ``dates`` area (``test_dates.py``): dates are ISO 8601
strings, naive ones in UTC; locators run on a dummy axis showing ``[vmin,
vmax]`` and give ``{ticks, iso}``. As in matplotlib's test suite, probes on
an axes run in the ``classic`` style plus ``_classic_test_patch`` unless a
test picks another style, and record it in ``style``. ``epoch`` (optional)
is set with ``set_epoch`` for the duration of the op, as the tests'
``_new_epoch_decorator`` does; without it the epoch is the default
``1970-01-01T00:00:00``. Time zones are IANA names. ``rc`` (optional) is
in effect for the whole op, ``made`` (optional) is the ``timezone``
rcParam while the op makes its locator or formatter, and ``iso`` strings
are ``num2date(x).isoformat()``, in the zone of ``timezone``.

Ops on their own
----------------
``dates.date2num`` {d} -> date number(s) (the ``dates`` area's op).
``dates.date2num.datetime64`` {d, unit?, epoch?} -> ``date2num(np.array(d,
    dtype=f"datetime64[{unit}]"))`` (``datetime64`` without ``unit``); ``d``
    is a string or a list of strings, ``"NaT"`` gives NaN.
``dates.num2date`` {x, tz?, rc?} -> ``num2date(x, tz).isoformat()`` (a
    list for a list).
``dates.num2timedelta`` {x} -> ``[days, seconds, microseconds]`` (or a list
    of them).
``dates.drange`` {dstart, dend, delta} -> date numbers.
``dates.datestr2num`` {s, default?} -> ``datestr2num(s, default)``;
    ``default`` is a date (``YYYY-MM-DD``).
``dates.set_epoch`` {epoch, used} -> null, or an error when the epoch was
    already used (``used``: ``get_epoch()`` was called after resetting it).
``dates.AutoDateLocator.__call__`` {vmin, vmax, params?, epoch?, rc?} ->
    {ticks, iso, unit, locator} (the ``dates`` area's op); ``params`` may
    also hold ``tz``.
``dates.AutoDateLocator.maxticks`` {params?} -> ``{frequency name: count}``.
``dates.AutoDateFormatter.format_ticks`` {vmin, vmax, locator?, params?,
    rc?} -> labels; ``params`` may hold ``defaultfmt`` and ``usetex``.
``dates.AutoDateFormatter.__call__`` {x, made?, rc?} -> the label of
    ``AutoDateFormatter(HourLocator())`` without an axis.
``dates.ConciseDateFormatter.format_ticks`` {vmin, vmax, locator?, params?,
    rc?} -> {labels, offset} (the ``dates`` area's op), or {locs, locator?}
    -> labels of ``locs`` for a formatter without an axis.
``dates.ConciseDateFormatter.__call__`` / ``.format_data_short`` {x, made?,
    rc?} -> the label of ``ConciseDateFormatter(AutoDateLocator())``
    without an axis.
``dates.YearLocator.__call__`` {params, vmin, vmax} -> {ticks, iso}.
``dates.DayLocator.tick_values`` {vmin, vmax} -> {ticks, iso} of
    ``DayLocator().tick_values(vmin, vmax)`` (date-time arguments).
``dates.RRuleLocator.tick_values`` {rule, vmin, vmax} -> {ticks, iso} of
    ``RRuleLocator(rrulewrapper(freq, **rule)).tick_values(vmin, vmax)``.
``dates.DateLocator.nonsingular`` {vmin, vmax} -> ``[vmin, vmax]``.
``dates.DateLocator.datalim_to_dt`` {data?, tz?, made?, rc?} -> the ISO
    pair of ``DateLocator(tz=tz).datalim_to_dt()`` on a dummy axis whose
    data interval is ``date2num(data)`` (the dummy default without
    ``data``); an unknown time zone is an error.
``dates.DateFormatter.__call__`` {fmt, d, tz?, made?, rc?} -> the label of
    ``d`` by ``DateFormatter(fmt)``, given ``set_tzinfo(tz)``.

Ops on an axes
--------------
``Figure().subplots()`` in ``style.context(style)`` and ``rc_context(rc)``,
set up in this order: ``converter`` (``"dates"``:
``ax.xaxis.set_converter(DateConverter())``), ``date_first`` (``{tz}``:
``ax.xaxis_date(tz)``, ``tz`` a zone name or ``null``), ``axhline``
(``ax.axhline(y)``), ``plot`` (``{x, y}`` of numbers or ISO strings, or
``{days: [start, stop]}`` for ``np.arange(start, stop,
dtype='datetime64[D]')`` against ``0, 1, ...``, and ``{hours: [start,
stop]}`` likewise with ``datetime64[h]``), ``replot`` (``{rc, plot}``: sets
the rcParams ``rc`` for the rest of the op, then plots ``plot`` in the same
forms), ``xaxis_date`` (``ax.xaxis_date()``), ``axis_date``
(``ax.xaxis.axis_date()``), then per axis ``"x"``/``"y"``: ``lim`` (ISO
strings), ``major``/``minor`` locators (``["AutoDateLocator",
{interval_multiples}]``, ``["MinuteLocator", [minutes]]``, ``["DayLocator"]``,
``["NullLocator"]``), ``major``/``minor`` formatters
(``["ConciseDateFormatter", {formats, zero_formats, tz}]``,
``["AutoDateFormatter", locator]``), ``minor`` again when ``minor2`` is
given, and ``invert``.

``axes.Axes.get_xlim`` {...} -> the x limits after drawing without
    rendering.
``axis.Axis.get_ticklabels`` {axis, minor, ...} -> {labels, offset} after
    drawing.
``axis.Axis.get_major_locator`` {...} -> the number of ticks the x major
    locator gives (``test_too_many_date_ticks``) and the x limits.
``axis.Axis.major`` {...} -> {locator, formatter}: the class names of the
    x axis' major locator and formatter.
``axis.Axis.get_units`` {...} -> the x axis' unit: ``null``, or the
    abbreviation of its zone on 2020-01-01 (``units.tzname(...)``).
``axis.ConciseDateFormatter.get_offset`` {xlims} -> the offset text after
    drawing at each of the successive x limits (``test_offset_changes``).

Cases
-----
Named after the upstream tests. ``test_date2num_dst`` records the
Brussels-time date-times it converts as ISO strings with their offsets.
``Canada/Pacific`` and ``Iceland`` are spelled ``America/Vancouver`` and
``Atlantic/Reykjavik`` (the system zone database has only the canonical
names).

The ``test_timezone_rc_*`` cases are plt-rs' own: the ``timezone``
rcParam as ``Europe/Berlin`` and ``America/New_York`` across their 2020
clock changes, read when matplotlib reads it (``made`` against ``rc``: when
locators and ``DateFormatter`` are made, whenever ``AutoDateFormatter`` and
``ConciseDateFormatter`` format), and a name no zone has. The figures plot
``datetime64`` values, which carry no zone, so their ticks follow the
rcParam (UTC-aware date-times would make UTC the axis unit).

Under the same two zones, in the ``default`` style (whose margins show the
autoscaling a unit change does), the ``test_timezone_rc_*`` cases also pin
how ``update_units`` sets a date axis up: ``xaxis_date(tz)`` makes a given
zone the axis unit, which later plots keep, while ``tz=None`` leaves the
unit unset; date ticks come only with a converter new to the axis, so the
first ``date.converter``'s formatter stays through later plots and
``xaxis_date()``, and an explicitly set ``DateConverter`` brings none
until ``xaxis_date`` gives it a zone.

Left out
--------
The image tests (``test_date_axhspan``, ``test_date_axvspan``,
``test_date_axhline``, ``test_date_axvline``, ``test_RRuleLocator``,
``test_DateFormatter``, ``test_date_inverted_limit``: figure cases of the
``mpl_misc`` group), ``test_DayLocator`` (the ``dates`` area probes the
invalid intervals plt-rs can express; negative and fractional ones are
type errors), ``test_date2num_masked`` and ``test_datetime_masked`` (numpy
masked arrays), ``test_concise_converter_stays`` (the unit converter
registry), ``test_date_formatter_callable`` (a Python callable as a
format), ``test_warn_notintervals`` (mutates a private table and checks a
warning), ``test_tz_utc`` (Python's ``tzinfo`` API), ``test_rrulewrapper``
(the time zones dateutil rules attach; plt-rs rules run on local time
inside ``RRuleLocator``, probed by ``test_auto_date_locator_intmult_tz``),
``test_rrulewrapper_pytz`` and ``test_yearlocator_pytz`` (pytz),
``test_date2num_dst_pandas`` (pandas), and the ``'boo'`` half of
``test_change_converter`` (the ``rcsetup`` area validates
``date.converter``).
"""

import contextlib
import datetime

import matplotlib as mpl
import matplotlib.dates as mdates
import matplotlib.style
import matplotlib.ticker as mticker
import numpy as np
import pytest
from dateutil import rrule
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from probes.test_dates import (
    FREQS, _dt, _iso, _ticks, auto_formatter, auto_locator_call, concise_formatter, date2num,
    drange, num2timedelta, _locator_call)
from spec_support import ProbeSet

probes = ProbeSet("mpl_dates")

pytestmark = pytest.mark.filterwarnings("ignore::UserWarning")

CLASSIC_STYLE = ["classic", "_classic_test_patch"]
UTC = datetime.timezone.utc


@contextlib.contextmanager
def _epoch(epoch):
    if epoch is None:
        yield
        return
    mdates._reset_epoch_test_example()
    mdates.set_epoch(epoch)
    try:
        yield
    finally:
        mdates._reset_epoch_test_example()


def _in_epoch(fn):
    def evaluate(epoch=None, **kw):
        with _epoch(epoch):
            return fn(**kw)

    return evaluate


def _plus(iso, **delta):
    return (_dt(iso) + datetime.timedelta(**delta)).isoformat()


# ---------------------------------------------------------------------------
# Conversions
# ---------------------------------------------------------------------------


def _date2num64(d, unit=None):
    dtype = f"datetime64[{unit}]" if unit else "datetime64"
    result = mdates.date2num(np.array(d, dtype=dtype))
    return result.tolist() if isinstance(result, np.ndarray) else float(result)


def date2num64(**args):
    return probes.probe("dates.date2num.datetime64", _in_epoch(_date2num64), **args)


def test_date_numpyx():
    days = [f"2017-01-0{day}T00:00:00" for day in (1, 2, 3)]
    probes.probe("dates.date2num", date2num, d=days)
    date2num64(d=days, unit="ns")


def test_date_date2num_numpy():
    times = ["2017-01-01T00:01:01", "2017-01-01T01:01:01", "2017-01-01T02:01:01",
             "2017-01-01T03:01:01"]
    probes.probe("dates.date2num", date2num, d=times)
    for unit in ("s", "us", "ms", "ns"):
        date2num64(d=times[0], unit=unit)
        date2num64(d=times, unit=unit)


def test_date2num_NaT():
    for unit in ("s", "ms", "us", "ns"):
        date2num64(d=["2017-01-01T00:01:01", "NaT"], unit=unit)


def test_date2num_NaT_scalar():
    for unit in ("s", "ms", "us", "ns"):
        date2num64(d="NaT", unit=unit)


def test_datetime64_in_list():
    date2num64(d=["2000-01-01", "2001-01-01"])


def test_change_epoch():
    for epoch in ("1970-01-01", "0000-12-31", "1970-01-01T01:00:00"):
        date2num64(d="2000-01-01", epoch=epoch)
    date2num64(d="1970-01-01T12:00:00", epoch="1970-01-01T00:00:00")

    def set_epoch(epoch, used):
        mdates._reset_epoch_test_example()
        try:
            if used:
                mdates.get_epoch()
            return mdates.set_epoch(epoch)
        finally:
            mdates._reset_epoch_test_example()

    for used in (True, False):
        probes.probe("dates.set_epoch", set_epoch, epoch="0000-01-01", used=used)


def _num2date(x, tz=None):
    result = mdates.num2date(x, tz=tz)
    return [d.isoformat() for d in result] if isinstance(result, list) else result.isoformat()


def test_num2date_error():
    for x in (-1000000, 10000000):
        assert isinstance(probes.probe("dates.num2date", _num2date, x=x), ValueError)


def test_num2date_roundoff():
    for x in (100000.0000578702, 100000.0000578703):
        probes.probe("dates.num2date", _num2date, x=x)


def test_num2timedelta():
    for x in (1, [1, 1.5]):
        probes.probe("dates.num2timedelta", num2timedelta, x=x)


def test_drange():
    start, end = "2011-01-01T00:00:00+00:00", "2011-01-02T00:00:00+00:00"
    for dend, delta in [(end, {"hours": 1}), (_plus(end, microseconds=-1), {"hours": 1}),
                        (_plus(end, microseconds=1), {"hours": 1}), (end, {"hours": 4})]:
        probes.probe("dates.drange", drange, dstart=start, dend=dend, delta=delta)


def test_date2num_dst():
    brussels = _tz("Europe/Brussels")
    start = datetime.datetime(2014, 3, 30, 0, 0, tzinfo=UTC)
    interval = datetime.timedelta(minutes=33, seconds=45)
    times = [(start + i * interval).astimezone(brussels).isoformat() for i in range(8)]
    probes.probe("dates.date2num", date2num, d=times)


def _datestr2num(s, default=None):
    if default is not None:
        default = datetime.date.fromisoformat(default)
    result = mdates.datestr2num(s, default)
    return result.tolist() if isinstance(result, np.ndarray) else result


def test_datestr2num():
    probes.probe("dates.datestr2num", _datestr2num, s="2022-01-10")
    probes.probe("dates.datestr2num", _datestr2num, s="2022-01", default="2022-01-10")
    probes.probe("dates.datestr2num", _datestr2num, s=["2022-01", "2022-02"], default="2022-01-10")
    probes.probe("dates.datestr2num", _datestr2num, s=[])
    probes.probe("dates.datestr2num", _datestr2num, s=[], default="2022-01-10")


# ---------------------------------------------------------------------------
# Locators and formatters on their own
# ---------------------------------------------------------------------------

D1990 = "1990-01-01T00:00:00"
D1997 = "1997-01-01T00:00:00"
DELTAS = [{"weeks": 52 * 200}, {"weeks": 52}, {"days": 141}, {"days": 40}, {"hours": 40},
          {"minutes": 20}, {"seconds": 40}, {"microseconds": 1500}]


def _auto_date_locator(interval_multiples, d1):
    for delta in DELTAS:
        probes.probe("dates.AutoDateLocator.__call__", _in_epoch(auto_locator_call), vmin=d1,
                     vmax=_plus(d1, **delta), params={"interval_multiples": interval_multiples},
                     epoch="2000-01-01")


def test_auto_date_locator():
    _auto_date_locator(False, D1990)


def test_auto_date_locator_intmult():
    _auto_date_locator(True, D1997)


def _maxticks(params=None):
    params = dict(params or {})
    if isinstance(params.get("maxticks"), dict):
        params["maxticks"] = {FREQS[k]: v for k, v in params["maxticks"].items()}
    names = {v: k for k, v in FREQS.items()}
    return {names[freq]: count for freq, count in mdates.AutoDateLocator(**params).maxticks.items()}


def test_auto_date_locator_maxticks():
    for params in ({"interval_multiples": False}, {"maxticks": {"MONTHLY": 5}}, {"maxticks": 5}):
        probes.probe("dates.AutoDateLocator.maxticks", _maxticks, params=params)


def test_auto_date_locator_intmult_tz():
    tz = "America/Vancouver"
    d1 = "1997-01-01T00:00:00-08:00"
    for delta in DELTAS[:-1]:
        vmax = (datetime.datetime(1997, 1, 1, tzinfo=_tz(tz)) + datetime.timedelta(**delta)).isoformat()
        result = probes.probe("dates.AutoDateLocator.__call__", _in_rc(auto_locator_call),
                              vmin=d1, vmax=vmax, params={"interval_multiples": True, "tz": tz},
                              rc={"_internal.classic_mode": False})
        probes.probe("dates.num2date", _num2date, x=result["ticks"].tolist(), tz=tz)


def _tz(name):
    from dateutil import tz
    return tz.gettz(name)


def _in_rc(fn):
    def evaluate(rc=None, **kw):
        with mpl.rc_context(rc or {}):
            return fn(**kw)

    return evaluate


def _made_in(made):
    """The ``timezone`` rcParam ``made`` (unchanged when ``None``), while an op
    makes its locator or formatter."""
    return mpl.rc_context({} if made is None else {"timezone": made})


def test_date_formatter_usetex():
    with matplotlib.style.context("default"):
        for delta in ({"weeks": 52 * 200}, {"days": 30}, {"hours": 20}, {"minutes": 10}):
            probes.probe("dates.AutoDateFormatter.format_ticks", auto_formatter, vmin=D1990,
                         vmax=_plus(D1990, **delta), locator={"interval_multiples": False},
                         params={"usetex": True})


def _concise_locs(locs, locator=None):
    loc = mdates.AutoDateLocator(**(locator or {}))
    return mdates.ConciseDateFormatter(loc).format_ticks(locs)


def test_concise_formatter_subsecond():
    year_1996 = 9861.0
    locs = [year_1996, year_1996 + 500 / mdates.MUSECONDS_PER_DAY,
            year_1996 + 900 / mdates.MUSECONDS_PER_DAY]
    probes.probe("dates.ConciseDateFormatter.format_ticks", _concise_locs, locs=locs,
                 locator={"interval_multiples": True})


def test_concise_formatter_usetex():
    for delta in ({"weeks": 52 * 200}, {"days": 40}, {"hours": 40}, {"seconds": 2}):
        probes.probe("dates.ConciseDateFormatter.format_ticks", concise_formatter, vmin=D1997,
                     vmax=_plus(D1997, **delta), locator={"interval_multiples": True},
                     params={"usetex": True})


def test_concise_formatter_exceptions():
    for kwarg in ("formats", "zero_formats", "offset_formats"):
        error = probes.probe("dates.ConciseDateFormatter.format_ticks", concise_formatter,
                             vmin=D1997, vmax=_plus(D1997, days=40), params={kwarg: ["", "%Y"]})
        assert isinstance(error, ValueError)


def _concise_text(method):
    def label(x, made=None):
        with _made_in(made):
            formatter = mdates.ConciseDateFormatter(mdates.AutoDateLocator())
        return getattr(formatter, method)(x)

    return _in_rc(label)


def test_concise_formatter_call():
    probes.probe("dates.ConciseDateFormatter.__call__", _concise_text("__call__"), x=19002.0)
    probes.probe("dates.ConciseDateFormatter.format_data_short", _concise_text("format_data_short"),
                 x=19002.0)


def test_year_locator():
    for weeks, params in [(52 * 200, {"base": 20, "month": 1, "day": 1}),
                          (52 * 200, {"base": 20, "month": 5, "day": 16}),
                          (52 * 5, {"base": 20, "month": 9, "day": 25})]:
        probes.probe("dates.YearLocator.__call__", _locator_call("YearLocator"), params=params,
                     vmin=D1990, vmax=_plus(D1990, weeks=weeks))


def test_RRuleLocator_dayrange():
    probes.probe("dates.DayLocator.tick_values",
                 lambda vmin, vmax: _ticks(mdates.DayLocator().tick_values(_dt(vmin), _dt(vmax))),
                 vmin="0001-01-01T00:00:00+00:00", vmax="0001-01-16T00:00:00+00:00")


def _rrule_tick_values(rule, vmin, vmax):
    rule = dict(rule)
    freq = FREQS[rule.pop("freq")]
    locator = mdates.RRuleLocator(mdates.rrulewrapper(freq, **rule))
    return _ticks(locator.tick_values(_dt(vmin), _dt(vmax)))


def test_RRuleLocator_close_minmax():
    probes.probe("dates.RRuleLocator.tick_values", _rrule_tick_values,
                 rule={"freq": "SECONDLY", "interval": 5}, vmin="2020-01-01T00:00:00",
                 vmax="2020-01-01T00:00:00.000001")


def test_date_locator():
    for vmin, vmax in [(0, np.inf), (0, 1), (1, 0), (0, 0)]:
        probes.probe("dates.DateLocator.nonsingular",
                     lambda vmin, vmax: list(mdates.DateLocator().nonsingular(vmin, vmax)),
                     vmin=vmin, vmax=vmax)

    datalim_to_dt = _in_rc(_datalim_to_dt)
    reykjavik = "Atlantic/Reykjavik"
    probes.probe("dates.DateLocator.datalim_to_dt", datalim_to_dt)
    probes.probe("dates.DateLocator.datalim_to_dt", datalim_to_dt,
                 data=["2022-01-10", "2022-01-08"], tz=reykjavik)
    probes.probe("dates.DateLocator.datalim_to_dt", datalim_to_dt,
                 data=["2022-01-10", "2022-01-08"], rc={"timezone": reykjavik})
    error = probes.probe("dates.DateLocator.datalim_to_dt", datalim_to_dt, tz="Aiceland")
    assert isinstance(error, ValueError)


def _datalim_to_dt(data=None, tz=None, made=None):
    with _made_in(made):
        locator = mdates.DateLocator(tz=tz)
    locator.create_dummy_axis()
    if data is not None:
        locator.axis.set_data_interval(*mdates.date2num(data))
    return [d.isoformat() for d in locator.datalim_to_dt()]


@_in_rc
def date_formatter(fmt, d, tz=None, made=None):
    with _made_in(made):
        formatter = mdates.DateFormatter(fmt)
    if tz is not None:
        formatter.set_tzinfo(tz)
    return formatter(mdates.date2num(_dt(d)))


def test_DateFormatter_settz():
    for tz in (None, "Pacific/Kiritimati"):
        probes.probe("dates.DateFormatter.__call__", date_formatter, fmt="%Y-%b-%d %H:%M",
                     d="2011-01-01T00:00:00+00:00", **({} if tz is None else {"tz": tz}))


# ---------------------------------------------------------------------------
# On an axes
# ---------------------------------------------------------------------------

#: The ``plot`` forms of evenly spaced ``datetime64`` values, and their unit.
SPACINGS = {"days": "D", "hours": "h"}

LOCATORS = {
    "AutoDateLocator": lambda kw=None: mdates.AutoDateLocator(**(kw or {})),
    "MinuteLocator": mdates.MinuteLocator,
    "DayLocator": mdates.DayLocator,
    "NullLocator": mticker.NullLocator,
}


def _locator(spec):
    name, *args = spec
    return LOCATORS[name](*[tuple(a) if isinstance(a, list) else a for a in args])


def _formatter(spec, axis_locator):
    name, arg = spec
    if name == "AutoDateFormatter":
        return mdates.AutoDateFormatter(_locator(arg))
    params = dict(arg)
    if "tz" in params:
        params["tz"] = datetime.timezone(datetime.timedelta(hours=params["tz"]))
    return mdates.ConciseDateFormatter(axis_locator, **params)


def _values(values):
    return [_dt(v) if isinstance(v, str) else v for v in values]


def _configure(ax, name, config):
    axis = getattr(ax, f"{name}axis")
    if "lim" in config:
        getattr(ax, f"set_{name}lim")(*_values(config["lim"]))
    for key, setter in (("major", axis.set_major_locator), ("minor", axis.set_minor_locator)):
        if key in config:
            setter(_locator(config[key]))
    for key, level in (("major_formatter", "major"), ("minor_formatter", "minor")):
        if key in config:
            formatter = _formatter(config[key], getattr(axis, level).locator)
            getattr(axis, f"set_{level}_formatter")(formatter)
    if "minor2" in config:
        axis.set_minor_locator(_locator(config["minor2"]))
    if config.get("invert"):
        axis.set_inverted(True)


#: The ``converter`` an axes may be given explicitly.
CONVERTERS = {"dates": mdates.DateConverter}


def _plot(ax, plot):
    spacing = next((key for key in SPACINGS if key in plot), None)
    if spacing is not None:
        times = np.arange(*plot[spacing], dtype=f"datetime64[{SPACINGS[spacing]}]")
        ax.plot(times, np.arange(len(times)))
    else:
        ax.plot(_values(plot["x"]), _values(plot["y"]))


def _on_axes(query):
    def evaluate(style, rc=None, epoch=None, converter=None, date_first=None, axhline=None,
                 plot=None, replot=None, xaxis_date=False, axis_date=False, x=None, y=None, **kw):
        with _epoch(epoch), matplotlib.style.context(style), mpl.rc_context(rc or {}):
            fig = Figure()
            FigureCanvasAgg(fig)
            ax = fig.subplots()
            if converter is not None:
                ax.xaxis.set_converter(CONVERTERS[converter]())
            if date_first is not None:
                ax.xaxis_date(date_first["tz"])
            if axhline is not None:
                ax.axhline(axhline)
            if plot is not None:
                _plot(ax, plot)
            if replot is not None:
                mpl.rcParams.update(replot.get("rc", {}))
                _plot(ax, replot["plot"])
            if xaxis_date:
                ax.xaxis_date()
            if axis_date:
                ax.xaxis.axis_date()
            for name, config in (("x", x), ("y", y)):
                if config is not None:
                    _configure(ax, name, config)
            return query(fig, ax, **kw)

    return evaluate


def on_axes(op, query, style=CLASSIC_STYLE, **args):
    return probes.probe(op, _on_axes(query), style=style, **args)


def _xlim(fig, ax):
    fig.draw_without_rendering()
    return list(ax.get_xlim())


def _ticklabels(fig, ax, axis, minor):
    fig.canvas.draw()
    axis = getattr(ax, f"{axis}axis")
    return {"labels": [t.get_text() for t in axis.get_ticklabels(minor=minor)],
            "offset": axis.get_offset_text().get_text()}


def ticklabels(axis="x", minor=False, **scene):
    return on_axes("axis.Axis.get_ticklabels", _ticklabels, axis=axis, minor=minor, **scene)


def test_date_empty():
    for epoch in (None, "0000-12-31"):
        on_axes("axes.Axes.get_xlim", _xlim, xaxis_date=True, **({} if epoch is None else {"epoch": epoch}))


def test_date_not_empty():
    on_axes("axes.Axes.get_xlim", _xlim, plot={"x": [50, 70], "y": [1, 2]}, axis_date=True)


def test_axhline():
    for epoch in (None, "0000-12-31"):
        on_axes("axes.Axes.get_xlim", _xlim, axhline=1.5,
                plot={"x": ["2016-01-01", "2016-01-02"], "y": [1, 2]},
                **({} if epoch is None else {"epoch": epoch}))


def _too_many_ticks(fig, ax):
    return {"count": len(ax.xaxis.get_major_locator()()), "xlim": list(ax.get_xlim())}


def test_too_many_date_ticks():
    t0 = "2000-01-20T00:00:00"

    def query(fig, ax):
        ax.set_xlim(_dt(t0), _dt(t0), auto=True)
        ax.plot([], [])
        ax.xaxis.set_major_locator(mdates.DayLocator())
        return _too_many_ticks(fig, ax)

    on_axes("axis.Axis.get_major_locator", query)


def test_locator_set_formatter():
    rc = {"date.autoformatter.minute": "%d %H:%M"}
    plot = {"x": ["2018-09-30T08:00:00", "2018-09-30T08:59:00", "2018-09-30T10:30:00"],
            "y": [2, 3, 1]}
    ticklabels(rc=rc, plot=plot, x={"major": ["MinuteLocator", [0, 30]]})
    ticklabels(minor=True, rc=rc, plot=plot, x={
        "major": ["NullLocator"], "minor": ["MinuteLocator", [5, 55]],
        "minor_formatter": ["AutoDateFormatter", ["MinuteLocator", [12, 27]]],
        "minor2": ["MinuteLocator", [15, 45]]})


def concise(d1=D1997, **delta):
    return {"lim": [d1, _plus(d1, **delta)], "major": ["AutoDateLocator", {"interval_multiples": True}]}


CONCISE_DELTAS = DELTAS[:-1] + [{"seconds": 2}]


def _concise_labels(formatter):
    for delta in CONCISE_DELTAS:
        ticklabels(axis="y", y={**concise(**delta), "major_formatter": ["ConciseDateFormatter", formatter]})


def test_concise_formatter():
    _concise_labels({})


def test_concise_formatter_formats():
    _concise_labels({"formats": ["%Y", "%m/%Y", "day: %d", "%H hr %M min", "%H hr %M min", "%S.%f sec"]})


def test_concise_formatter_zformats():
    _concise_labels({"zero_formats": ["", "'%y", "%B", "%m-%d", "%S", "%S.%f"]})


def test_concise_formatter_tz():
    d1 = "1997-01-01T00:00:00+00:00"
    for delta in ({"hours": 40}, {"minutes": 20}, {"seconds": 40}, {"seconds": 2}):
        ticklabels(axis="y", y={**concise(d1=d1, **delta),
                                "major_formatter": ["ConciseDateFormatter", {"tz": 3}]})


def _offset_labels(delta, **x):
    ticklabels(plot={"x": [D1997, _plus(D1997, **delta)], "y": [0, 0]},
               x={"major": ["AutoDateLocator"], "major_formatter": ["ConciseDateFormatter", {}], **x})


def test_concise_formatter_show_offset():
    for delta in ({"seconds": 0.01}, {"minutes": 1}, {"hours": 1}, {"days": 1}, {"weeks": 1},
                  {"weeks": 26}, {"weeks": 520}):
        _offset_labels(delta)


def test_concise_formatter_show_offset_inverted():
    _offset_labels({"days": 60}, invert=True)


def _offsets(fig, ax, xlims):
    offsets = []
    for lim in xlims:
        if lim is not None:
            ax.set_xlim(*_values(lim))
        fig.draw_without_rendering()
        offsets.append(ax.xaxis.get_major_formatter().get_offset())
    return offsets


def test_offset_changes():
    d2 = _plus(D1997, weeks=520)
    xlims = [None, [D1997, _plus(D1997, weeks=3)], [_plus(D1997, weeks=7), _plus(D1997, weeks=30)],
             [D1997, d2]]
    on_axes("axis.ConciseDateFormatter.get_offset", _offsets, xlims=xlims,
            plot={"x": [D1997, d2], "y": [0, 0]},
            x={"major": ["AutoDateLocator"], "major_formatter": ["ConciseDateFormatter", {}]})


def test_change_converter():
    for converter in ("concise", "auto"):
        ticklabels(rc={"date.converter": converter}, plot={"days": ["2020-01-01", "2020-05-01"]})


def test_change_interval_multiples():
    for multiples in (False, "True"):
        ticklabels(rc={"date.interval_multiples": multiples},
                   plot={"days": ["2020-01-10", "2020-05-01"]})


# ---------------------------------------------------------------------------
# The timezone rcParam (plt-rs' own cases)
# ---------------------------------------------------------------------------

ZONES = ["Europe/Berlin", "America/New_York"]
UNKNOWN_ZONE = {"timezone": "Nowhere/Atlantis"}

#: Views across the 2020 clock changes of both zones (Berlin's at 01:00 UTC
#: on March 29 and October 25, New York's at 07:00 UTC on March 8 and 06:00
#: UTC on November 1): hours, minutes, days and the whole year.
ZONE_SPANS = [
    ("2020-03-28T22:00:00", "2020-03-29T04:00:00"),
    ("2020-10-24T22:00:00", "2020-10-25T04:00:00"),
    ("2020-03-29T00:40:00", "2020-03-29T01:20:00"),
    ("2020-03-26T00:00:00", "2020-04-01T00:00:00"),
    ("2020-03-08T04:00:00", "2020-03-08T10:00:00"),
    ("2020-11-01T03:00:00", "2020-11-01T09:00:00"),
    ("2020-03-08T06:40:00", "2020-03-08T07:20:00"),
    ("2020-03-05T00:00:00", "2020-03-11T00:00:00"),
    ("2020-01-01T00:00:00", "2021-01-01T00:00:00"),
]

#: Instants on either side of each change (UTC).
ZONE_INSTANTS = [
    "2020-03-29T00:59:59", "2020-03-29T01:00:00", "2020-10-25T00:30:00", "2020-10-25T01:30:00",
    "2020-03-08T06:59:59", "2020-03-08T07:00:00", "2020-11-01T05:30:00", "2020-11-01T06:30:00",
]

#: Plotted ``datetime64`` hours across each zone's changes; days across both
#: springs.
ZONE_PLOTS = {
    "Europe/Berlin": [{"hours": ["2020-03-28T22:00", "2020-03-29T04:00"]},
                      {"hours": ["2020-10-24T22:00", "2020-10-25T04:00"]}],
    "America/New_York": [{"hours": ["2020-03-08T04:00", "2020-03-08T10:00"]},
                         {"hours": ["2020-11-01T03:00", "2020-11-01T09:00"]}],
}
DAYS_ACROSS_SPRING = {"days": ["2020-03-01", "2020-04-15"]}


def test_timezone_rc_locators_and_formatters():
    for zone in ZONES:
        rc = {"timezone": zone}
        for vmin, vmax in ZONE_SPANS:
            probes.probe("dates.AutoDateLocator.__call__", _in_rc(auto_locator_call), vmin=vmin, vmax=vmax,
                         rc=rc)
            probes.probe("dates.AutoDateFormatter.format_ticks", _in_rc(auto_formatter), vmin=vmin, vmax=vmax,
                         rc=rc)
            probes.probe("dates.ConciseDateFormatter.format_ticks", _in_rc(concise_formatter), vmin=vmin,
                         vmax=vmax, rc=rc)
    berlin = _in_rc(auto_locator_call)(vmin=ZONE_SPANS[0][0], vmax=ZONE_SPANS[0][1],
                                       rc={"timezone": "Europe/Berlin"})
    # The rule's 02:00, which Berlin skips, lands on 01:00 CET as dateutil resolves it.
    assert berlin["iso"][2:4] == ["2020-03-29T01:00:00+01:00"] * 2


def test_timezone_rc_conversions():
    numbers = [float(date2num(d)) for d in ZONE_INSTANTS]
    for zone in ZONES:
        rc = {"timezone": zone}
        probes.probe("dates.num2date", _in_rc(_num2date), x=numbers, rc=rc)
        for d in ZONE_INSTANTS:
            probes.probe("dates.DateFormatter.__call__", date_formatter, fmt="%Y-%m-%d %H:%M:%S %Z%z", d=d, rc=rc)
    assert _in_rc(_num2date)(x=numbers[1], rc={"timezone": "Europe/Berlin"}) == "2020-03-29T03:00:00+02:00"


def test_timezone_rc_figures():
    for zone, plots in ZONE_PLOTS.items():
        for plot in plots + [DAYS_ACROSS_SPRING]:
            for converter in ("auto", "concise"):
                ticklabels(rc={"timezone": zone, "date.converter": converter}, plot=plot)


def _auto_hour_label(x, made=None):
    with _made_in(made):
        formatter = mdates.AutoDateFormatter(mdates.HourLocator())
    return formatter(x)


def test_timezone_rc_read_when_made_or_used():
    berlin, new_york = ZONES
    instant = ZONE_INSTANTS[1]
    x = float(date2num(instant))
    for made, used in [(berlin, new_york), (new_york, berlin), ("UTC", berlin), (berlin, "UTC")]:
        rc = {"timezone": used}
        probes.probe("dates.DateFormatter.__call__", date_formatter, fmt="%H:%M %Z", d=instant, made=made, rc=rc)
        probes.probe("dates.DateLocator.datalim_to_dt", _in_rc(_datalim_to_dt),
                     data=[instant, ZONE_INSTANTS[0]], made=made, rc=rc)
        probes.probe("dates.AutoDateFormatter.__call__", _in_rc(_auto_hour_label), x=x, made=made, rc=rc)
        probes.probe("dates.ConciseDateFormatter.format_data_short", _concise_text("format_data_short"), x=x,
                     made=made, rc=rc)
    assert date_formatter(fmt="%H:%M", d=instant, made=berlin, rc={"timezone": "UTC"}) == "03:00"
    concise = _concise_text("format_data_short")(x=x, made=berlin, rc={"timezone": "UTC"})
    assert concise == "2020-03-29 01:00:00"


def test_timezone_rc_unknown():
    instant = ZONE_INSTANTS[1]
    x = float(date2num(instant))
    vmin, vmax = ZONE_SPANS[0]
    results = [
        probes.probe("dates.num2date", _in_rc(_num2date), x=x, rc=UNKNOWN_ZONE),
        probes.probe("dates.DateFormatter.__call__", date_formatter, fmt="%H:%M", d=instant, rc=UNKNOWN_ZONE),
        probes.probe("dates.AutoDateLocator.__call__", _in_rc(auto_locator_call), vmin=vmin, vmax=vmax,
                     rc=UNKNOWN_ZONE),
        probes.probe("dates.AutoDateFormatter.format_ticks", _in_rc(auto_formatter), vmin=vmin, vmax=vmax,
                     rc=UNKNOWN_ZONE),
        probes.probe("dates.ConciseDateFormatter.format_ticks", _in_rc(concise_formatter), vmin=vmin, vmax=vmax,
                     rc=UNKNOWN_ZONE),
        probes.probe("dates.ConciseDateFormatter.format_data_short", _concise_text("format_data_short"), x=x,
                     made="UTC", rc=UNKNOWN_ZONE),
        probes.probe("dates.DateLocator.datalim_to_dt", _in_rc(_datalim_to_dt), rc=UNKNOWN_ZONE),
        ticklabels(rc=UNKNOWN_ZONE, plot=DAYS_ACROSS_SPRING),
    ]
    assert all(isinstance(result, ValueError) for result in results)
    # A zone given to the constructor never reads the rcParam.
    given = probes.probe("dates.DateLocator.datalim_to_dt", _in_rc(_datalim_to_dt), tz="Europe/Berlin",
                         rc=UNKNOWN_ZONE)
    assert given == ["1970-01-01T01:00:00+01:00", "1970-01-02T01:00:00+01:00"]


# ---------------------------------------------------------------------------
# How a date axis takes its zone and ticks (plt-rs' own cases)
# ---------------------------------------------------------------------------

DEFAULT_STYLE = ["default"]

#: Hourly ``datetime64`` values from 12:00 to 18:00 UTC on 2020-01-01.
NOON_HOURS = {"hours": ["2020-01-01T12:00", "2020-01-01T19:00"]}


def _major(fig, ax):
    major = ax.xaxis.major
    return {"locator": type(major.locator).__name__, "formatter": type(major.formatter).__name__}


def major(**scene):
    return on_axes("axis.Axis.major", _major, style=DEFAULT_STYLE, **scene)


def _units(fig, ax):
    units = ax.xaxis.get_units()
    return None if units is None else units.tzname(datetime.datetime(2020, 1, 1))


def units(**scene):
    return on_axes("axis.Axis.get_units", _units, style=DEFAULT_STYLE, **scene)


def test_timezone_rc_axis_date_first():
    for zone in ZONES:
        rc = {"timezone": zone}
        for tz in ("UTC", None, zone):
            scene = {"rc": rc, "date_first": {"tz": tz}}
            labels = ticklabels(**scene, style=DEFAULT_STYLE, plot=NOON_HOURS)
            units(**scene, plot=NOON_HOURS)
            xlim = on_axes("axes.Axes.get_xlim", _xlim, **scene, style=DEFAULT_STYLE)
            # A given zone becomes the unit, whose change autoscales the view.
            assert xlim == ([0.0, 1.0] if tz is None else [-0.05, 1.05])
            if zone == "Europe/Berlin":
                assert labels["labels"][0] == ("01-01 12" if tz == "UTC" else "01-01 13")


def test_timezone_rc_first_converter_stays():
    for zone in ZONES:
        for first, second in (("auto", "concise"), ("concise", "auto")):
            scene = {"rc": {"timezone": zone, "date.converter": first}, "plot": NOON_HOURS,
                     "replot": {"rc": {"date.converter": second}, "plot": NOON_HOURS}}
            ticklabels(**scene, style=DEFAULT_STYLE)
            for later in ({}, {"xaxis_date": True}):
                kept = major(**scene, **later)
                assert kept["formatter"] == f"{first.capitalize()}DateFormatter"


def test_timezone_rc_explicit_converter():
    for zone in ZONES:
        explicit = {"rc": {"timezone": zone}, "converter": "dates"}
        plain = major(**explicit, plot=NOON_HOURS)
        assert plain == {"locator": "AutoLocator", "formatter": "ScalarFormatter"}
        ticklabels(**explicit, style=DEFAULT_STYLE, plot=NOON_HOURS)
        for tz in (None, "UTC"):
            major(**explicit, date_first={"tz": tz}, plot=NOON_HOURS)
            units(**explicit, date_first={"tz": tz})
