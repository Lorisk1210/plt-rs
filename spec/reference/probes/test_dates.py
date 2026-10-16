"""Date conversion, date tick locating and date formatting (``matplotlib.dates``).

Area ``dates``.

Argument conventions
--------------------
* Dates are ISO-8601 strings parsed with ``datetime.fromisoformat``. Strings
  without a UTC offset denote **UTC** (matplotlib treats naive datetimes as
  UTC); strings with an offset are converted to UTC. Nothing depends on the
  wall clock or the local timezone. The epoch is the default
  ``1970-01-01T00:00:00`` and ``rcParams["timezone"]`` is ``"UTC"``.
* Output datetimes are ``num2date(x).isoformat()``: UTC with a ``+00:00``
  suffix, microseconds only when non-zero. ``num2date`` rounds to whole
  microseconds, and to multiples of 20 us when ``|x| > 70 * 365`` days.
* Date locators are attached to a dummy axis whose view interval is
  ``[date2num(vmin), date2num(vmax)]`` and then called (``locator()``), the
  same path an Axis uses. Results are ``{"ticks": [date numbers],
  "iso": [iso strings]}``.
* ``freq`` names are dateutil's: YEARLY, MONTHLY, WEEKLY, DAILY, HOURLY,
  MINUTELY, SECONDLY (and matplotlib's MICROSECONDLY).
* Weekdays are integers, 0 = Monday.

Ops
---
``dates.get_epoch``                {} -> "1970-01-01T00:00:00"
``dates.date2num``                 {d: iso | [iso]} -> float | [float]
``dates.date2num.datetime64``      {d: [str]} -> [float]; ``d`` -> ``np.array(d, dtype="datetime64")``
                                   (nanosecond strings and "NaT" allowed; NaT -> NaN)
``dates.num2date``                 {x: float | [float]} -> iso | [iso]
``dates.num2timedelta``            {x: float | [float]} -> [days, seconds, microseconds] (or a list of them)
``dates.drange``                   {dstart, dend, delta: {days?, hours?, minutes?, seconds?, microseconds?}}
                                   -> [float]
``dates.AutoDateLocator.__call__`` {vmin, vmax, params?{minticks, maxticks, interval_multiples}} ->
                                   {ticks, iso, unit, locator}; ``maxticks`` is an int or
                                   {freq name: int}; ``unit`` is ``_get_unit()`` (days per
                                   tick unit, drives AutoDateFormatter); ``locator`` describes
                                   the delegate chosen by ``get_locator``:
                                   {class, freq, interval, bymonth, bymonthday, byhour,
                                   byminute, bysecond} for RRuleLocator, {class, base, month,
                                   day} for YearLocator, {class, interval} for MicrosecondLocator
``dates.AutoDateLocator.nonsingular`` {vmin: float, vmax: float} -> [float, float]
``dates.AutoDateFormatter.format_ticks`` {vmin, vmax, locator?: AutoDateLocator params,
                                   params?{defaultfmt}} -> labels for the AutoDateLocator ticks
``dates.ConciseDateFormatter.format_ticks`` {vmin, vmax, locator?, params?{formats,
                                   zero_formats, offset_formats, show_offset, usetex}} ->
                                   {labels, offset}
``dates.<Locator>.__call__``       {params, vmin, vmax} -> {ticks, iso} for YearLocator{base, month,
                                   day}, MonthLocator{bymonth, bymonthday, interval},
                                   WeekdayLocator{byweekday, interval}, DayLocator{bymonthday,
                                   interval}, HourLocator{byhour, interval}, MinuteLocator{byminute,
                                   interval}, SecondLocator{bysecond, interval},
                                   MicrosecondLocator{interval}
``dates.RRuleLocator.__call__``    {rule{freq, interval?, bymonth?, bymonthday?, byweekday?, byhour?,
                                   byminute?, bysecond?, byyearday?, bysetpos?}, vmin, vmax} -> {ticks, iso}
``dates.<Locator>.nonsingular``    {params, vmin: float, vmax: float} -> [float, float]
``dates.DateFormatter.__call__``   {fmt, d, usetex?} -> ``DateFormatter(fmt)(date2num(d))``
"""

import datetime

import matplotlib.dates as mdates
import numpy as np
from dateutil import rrule

from spec_support import ProbeSet

probes = ProbeSet("dates")

UTC = datetime.timezone.utc
FREQS = {
    "YEARLY": rrule.YEARLY,
    "MONTHLY": rrule.MONTHLY,
    "WEEKLY": rrule.WEEKLY,
    "DAILY": rrule.DAILY,
    "HOURLY": rrule.HOURLY,
    "MINUTELY": rrule.MINUTELY,
    "SECONDLY": rrule.SECONDLY,
    "MICROSECONDLY": mdates.MICROSECONDLY,
}
FREQ_NAMES = {v: k for k, v in FREQS.items()}

LOCATORS = {
    "YearLocator": mdates.YearLocator,
    "MonthLocator": mdates.MonthLocator,
    "WeekdayLocator": mdates.WeekdayLocator,
    "DayLocator": mdates.DayLocator,
    "HourLocator": mdates.HourLocator,
    "MinuteLocator": mdates.MinuteLocator,
    "SecondLocator": mdates.SecondLocator,
    "MicrosecondLocator": mdates.MicrosecondLocator,
}


def _dt(iso):
    value = datetime.datetime.fromisoformat(iso)
    return value if value.tzinfo is not None else value.replace(tzinfo=UTC)


def _iso(num):
    return mdates.num2date(num).isoformat()


def _ticks(ticks):
    ticks = np.asarray(ticks, dtype=float)
    return {"ticks": ticks, "iso": [_iso(t) for t in ticks]}


def _attach(locator, vmin, vmax):
    locator.create_dummy_axis()
    locator.axis.set_view_interval(mdates.date2num(_dt(vmin)), mdates.date2num(_dt(vmax)))
    return locator


def _auto_locator(params):
    params = dict(params or {})
    if isinstance(params.get("maxticks"), dict):
        params["maxticks"] = {FREQS[k]: v for k, v in params["maxticks"].items()}
    return mdates.AutoDateLocator(**params)


def _int_list(value):
    if value is None:
        return None
    if isinstance(value, int):
        return [value]
    return [int(v) for v in value]


def describe_locator(locator):
    if isinstance(locator, mdates.YearLocator):
        construct = locator.rule._construct
        return {"class": "YearLocator", "base": locator.base.step, "month": construct.get("bymonth"),
                "day": construct.get("bymonthday")}
    if isinstance(locator, mdates.RRuleLocator):
        construct = locator.rule._construct
        return {
            "class": type(locator).__name__,
            "freq": FREQ_NAMES[construct["freq"]],
            "interval": construct.get("interval", 1),
            **{key: _int_list(construct.get(key)) for key in
               ["bymonth", "bymonthday", "byhour", "byminute", "bysecond"]},
        }
    return {"class": type(locator).__name__, "interval": locator._interval}


# --------------------------------------------------------------------------
# Adapters
# --------------------------------------------------------------------------


def date2num(d):
    if isinstance(d, list):
        return mdates.date2num([_dt(v) for v in d])
    return mdates.date2num(_dt(d))


def date2num_datetime64(d):
    return mdates.date2num(np.array(d, dtype="datetime64"))


def num2date(x):
    result = mdates.num2date(x)
    if isinstance(result, list):
        return [v.isoformat() for v in result]
    return result.isoformat()


def num2timedelta(x):
    def parts(td):
        return [td.days, td.seconds, td.microseconds]

    result = mdates.num2timedelta(x)
    return [parts(v) for v in result] if isinstance(result, list) else parts(result)


def drange(dstart, dend, delta):
    return mdates.drange(_dt(dstart), _dt(dend), datetime.timedelta(**delta))


def auto_locator_call(vmin, vmax, params=None):
    locator = _attach(_auto_locator(params), vmin, vmax)
    ticks = locator()
    delegate = locator.get_locator(*locator.viewlim_to_dt())
    return {**_ticks(ticks), "unit": locator._get_unit(), "locator": describe_locator(delegate)}


def auto_locator_nonsingular(vmin, vmax):
    return mdates.AutoDateLocator().nonsingular(vmin, vmax)


def auto_formatter(vmin, vmax, locator=None, params=None):
    loc = _attach(_auto_locator(locator), vmin, vmax)
    ticks = loc()
    return mdates.AutoDateFormatter(loc, **(params or {})).format_ticks(ticks)


def concise_formatter(vmin, vmax, locator=None, params=None):
    loc = _attach(_auto_locator(locator), vmin, vmax)
    ticks = loc()
    formatter = mdates.ConciseDateFormatter(loc, **(params or {}))
    return {"labels": formatter.format_ticks(ticks), "offset": formatter.get_offset()}


def _locator_call(cls_name):
    def call(params, vmin, vmax):
        return _ticks(_attach(LOCATORS[cls_name](**params), vmin, vmax)())

    return call


def _locator_nonsingular(cls_name):
    def nonsingular(params, vmin, vmax):
        return LOCATORS[cls_name](**params).nonsingular(vmin, vmax)

    return nonsingular


def rrule_locator_call(rule, vmin, vmax):
    rule = dict(rule)
    freq = FREQS[rule.pop("freq")]
    return _ticks(_attach(mdates.RRuleLocator(mdates.rrulewrapper(freq, **rule)), vmin, vmax)())


def date_formatter(fmt, d, usetex=None):
    return mdates.DateFormatter(fmt, usetex=usetex)(mdates.date2num(_dt(d)))


# --------------------------------------------------------------------------
# Conversions
# --------------------------------------------------------------------------

ISO_DATES = [
    "1970-01-01T00:00:00",
    "1970-01-01T12:00:00",
    "1970-01-02T00:00:00",
    "1969-12-31T23:59:59.999999",
    "2000-01-01T00:00:00",
    "2000-02-29T12:00:00",
    "2020-02-29T23:59:59.999999",
    "2020-03-05T14:07:09.123456",
    "1900-01-01T00:00:00",
    "1800-01-01T00:00:00.123456",
    "0001-01-01T00:00:00",
    "9999-12-31T23:59:59.999999",
    "2038-01-19T03:14:07",
    "2020-06-15T12:00:00+02:00",
    "2020-06-15T12:00:00-05:30",
    "2020-06-15T12:00:00+00:00",
]


def test_epoch():
    assert probes.probe("dates.get_epoch", mdates.get_epoch) == "1970-01-01T00:00:00"


def test_date2num():
    for d in ISO_DATES:
        probes.probe("dates.date2num", date2num, d=d)
    probes.probe("dates.date2num", date2num, d=ISO_DATES[:6])
    probes.probe("dates.date2num", date2num, d=[])
    assert date2num("1970-01-01T12:00:00") == 0.5
    assert date2num("2020-06-15T12:00:00+02:00") == date2num("2020-06-15T10:00:00")
    for d in [
        ["2020-01-01T00:00:00.123456789", "1970-01-01T00:00:00.000000001", "NaT"],
        ["2020-01-01", "1970-01-02"],
        ["1677-09-22T00:00:00", "2262-04-11T00:00:00"],
        ["0001-01-01T00:00:00", "9999-12-31T23:59:59"],
    ]:
        probes.probe("dates.date2num.datetime64", date2num_datetime64, d=d)


def test_num2date():
    values = [0.0, 0.5, -0.5, 1.0, 1e-6, 1 / 86400, 1 / 86400e6, 0.4 / 86400e6, 0.6 / 86400e6, 18262.5,
              18262.123456789, 25550.0, 25551.000001, -25551.000001, -719162.0, -719163.0, 2932896.9999999,
              2932897.0, 1e10, float("nan"), float("inf")]
    for x in values:
        probes.probe("dates.num2date", num2date, x=x)
    probes.probe("dates.num2date", num2date, x=[0.0, 0.25, 18262.5])
    for d in ISO_DATES:
        probes.probe("dates.num2date", num2date, x=float(date2num(d)))
    assert num2date(0.5) == "1970-01-01T12:00:00+00:00"
    for x in [0.0, 1.5, -0.25, 1e-6, 365.2425, -1e-7, 1e5]:
        probes.probe("dates.num2timedelta", num2timedelta, x=x)
    probes.probe("dates.num2timedelta", num2timedelta, x=[0.5, 2.75])


def test_drange():
    cases = [
        ("2020-01-01T00:00:00", "2020-01-02T00:00:00", {"hours": 6}),
        ("2020-01-01T00:00:00", "2020-01-01T01:00:00", {"minutes": 7}),
        ("2020-01-01T00:00:00", "2020-01-01T00:00:00.000010", {"microseconds": 1}),
        ("2020-01-01T00:00:00", "2020-03-01T00:00:00", {"days": 7}),
        ("2020-01-01T00:00:00", "2020-01-01T00:00:00", {"days": 1}),
        ("2020-01-01T00:00:00", "2019-12-25T00:00:00", {"days": 1}),
        ("2020-01-01T00:00:00", "2020-01-02T00:00:00", {"seconds": 86400}),
        ("1900-01-01T00:00:00", "2100-01-01T00:00:00", {"days": 3653}),
    ]
    for dstart, dend, delta in cases:
        probes.probe("dates.drange", drange, dstart=dstart, dend=dend, delta=delta)
    assert list(drange("2020-01-01T00:00:00", "2020-01-02T00:00:00", {"hours": 6})) == [
        18262.0, 18262.25, 18262.5, 18262.75]


# --------------------------------------------------------------------------
# AutoDateLocator and the automatic formatters
# --------------------------------------------------------------------------

BASE = "2020-01-01T00:00:00"

#: (vmin, vmax) spans from microseconds to millennia.
SPANS = [
    (BASE, "2020-01-01T00:00:00.000050"),
    (BASE, "2020-01-01T00:00:00.000500"),
    (BASE, "2020-01-01T00:00:00.001000"),
    (BASE, "2020-01-01T00:00:00.010000"),
    (BASE, "2020-01-01T00:00:00.100000"),
    (BASE, "2020-01-01T00:00:01"),
    (BASE, "2020-01-01T00:00:02"),
    (BASE, "2020-01-01T00:00:05"),
    (BASE, "2020-01-01T00:00:10"),
    (BASE, "2020-01-01T00:00:30"),
    (BASE, "2020-01-01T00:00:59"),
    (BASE, "2020-01-01T00:01:00"),
    (BASE, "2020-01-01T00:02:00"),
    (BASE, "2020-01-01T00:05:00"),
    (BASE, "2020-01-01T00:10:00"),
    (BASE, "2020-01-01T00:30:00"),
    (BASE, "2020-01-01T00:59:00"),
    (BASE, "2020-01-01T01:00:00"),
    (BASE, "2020-01-01T02:00:00"),
    (BASE, "2020-01-01T04:00:00"),
    (BASE, "2020-01-01T06:00:00"),
    (BASE, "2020-01-01T12:00:00"),
    (BASE, "2020-01-01T23:00:00"),
    (BASE, "2020-01-02T00:00:00"),
    (BASE, "2020-01-03T00:00:00"),
    (BASE, "2020-01-04T00:00:00"),
    (BASE, "2020-01-06T00:00:00"),
    (BASE, "2020-01-08T00:00:00"),
    (BASE, "2020-01-11T00:00:00"),
    (BASE, "2020-01-15T00:00:00"),
    (BASE, "2020-01-21T00:00:00"),
    (BASE, "2020-02-01T00:00:00"),
    (BASE, "2020-03-01T00:00:00"),
    (BASE, "2020-04-01T00:00:00"),
    (BASE, "2020-07-01T00:00:00"),
    (BASE, "2020-10-01T00:00:00"),
    (BASE, "2021-01-01T00:00:00"),
    (BASE, "2022-01-01T00:00:00"),
    (BASE, "2023-01-01T00:00:00"),
    (BASE, "2025-01-01T00:00:00"),
    (BASE, "2030-01-01T00:00:00"),
    (BASE, "2040-01-01T00:00:00"),
    (BASE, "2070-01-01T00:00:00"),
    (BASE, "2120-01-01T00:00:00"),
    ("1800-01-01T00:00:00", "2200-01-01T00:00:00"),
    ("1000-01-01T00:00:00", "2000-01-01T00:00:00"),
    ("0001-01-01T00:00:00", "3000-01-01T00:00:00"),
    ("0001-01-01T00:00:00", "9999-12-31T00:00:00"),
    ("0001-01-01T00:00:00", "0002-01-01T00:00:00"),
    ("9998-01-01T00:00:00", "9999-12-31T00:00:00"),
    ("2020-01-15T13:47:12", "2020-03-02T00:00:00"),
    ("2020-02-27T05:00:00", "2020-03-02T19:00:00"),
    ("2019-12-31T22:00:00", "2020-01-01T03:00:00"),
    ("2019-12-31T23:59:58", "2020-01-01T00:00:03"),
    ("2020-01-01T10:17:42.5", "2020-01-01T10:17:43.25"),
    ("1969-12-31T18:00:00", "1970-01-01T06:00:00"),
    ("2017-06-14T00:00:00", "2019-03-02T00:00:00"),
    ("1999-10-01T00:00:00", "2000-03-01T00:00:00"),
    ("2020-01-02T00:00:00", BASE),
    (BASE, BASE),
]

AUTO_PARAMS = [
    None,
    {"minticks": 3},
    {"maxticks": 5},
    {"interval_multiples": False},
    {"minticks": 3, "maxticks": {"MONTHLY": 6, "DAILY": 5, "HOURLY": 4}},
]


def _auto_args(params):
    return {} if params is None else {"params": params}


def test_auto_date_locator():
    for vmin, vmax in SPANS:
        for params in AUTO_PARAMS:
            probes.probe("dates.AutoDateLocator.__call__", auto_locator_call, vmin=vmin, vmax=vmax,
                         **_auto_args(params))
    result = auto_locator_call(BASE, "2020-01-02T00:00:00")
    # 24 hours with at most 12 hourly ticks -> every 3rd hour (2 * 11 < 24 <= 3 * 11).
    assert result["locator"] == {"class": "RRuleLocator", "freq": "HOURLY", "interval": 1, "bymonth": None,
                                 "bymonthday": None, "byhour": [0, 3, 6, 9, 12, 15, 18, 21], "byminute": [0],
                                 "bysecond": [0]}
    for vmin, vmax in [(0.0, 1.0), (5.0, 5.0), (3.0, 1.0), (float("-inf"), float("inf")),
                       (float("nan"), 1.0)]:
        probes.probe("dates.AutoDateLocator.nonsingular", auto_locator_nonsingular, vmin=vmin, vmax=vmax)


def test_auto_date_formatter():
    for vmin, vmax in SPANS:
        probes.probe("dates.AutoDateFormatter.format_ticks", auto_formatter, vmin=vmin, vmax=vmax)
    for vmin, vmax in SPANS[::6]:
        probes.probe("dates.AutoDateFormatter.format_ticks", auto_formatter, vmin=vmin, vmax=vmax,
                     locator={"interval_multiples": False}, params={"defaultfmt": "%d/%m"})
    assert auto_formatter(BASE, "2020-01-02T00:00:00")[:2] == ["01-01 00", "01-01 03"]


def test_concise_date_formatter():
    variants = [
        None,
        {"show_offset": False},
        {"formats": ["%y", "%B", "%d.", "%Hh", "%Mm", "%Ss"]},
        {"zero_formats": ["", "%Y", "%B", "%d %b", "%Hh", "%Mm"]},
        {"offset_formats": ["", "%y", "%y/%m", "%y/%m/%d", "%y/%m/%d", "%y/%m/%d %H"]},
        {"usetex": True},
    ]
    for vmin, vmax in SPANS:
        probes.probe("dates.ConciseDateFormatter.format_ticks", concise_formatter, vmin=vmin, vmax=vmax)
    for params in variants[1:]:
        for vmin, vmax in SPANS[::4]:
            probes.probe("dates.ConciseDateFormatter.format_ticks", concise_formatter, vmin=vmin, vmax=vmax,
                         params=params)
    for vmin, vmax in SPANS[::5]:
        probes.probe("dates.ConciseDateFormatter.format_ticks", concise_formatter, vmin=vmin, vmax=vmax,
                     locator={"minticks": 3, "maxticks": 5})
    for params in [{"formats": ["%Y"]}, {"zero_formats": ["%Y"]}, {"offset_formats": ["%Y"]}]:
        probes.probe("dates.ConciseDateFormatter.format_ticks", concise_formatter, vmin=BASE,
                     vmax="2021-01-01T00:00:00", params=params)
    result = concise_formatter("2020-01-15T13:47:12", "2020-03-02T00:00:00")
    assert result == {"labels": ["22", "Feb", "08", "15", "22", "Mar"], "offset": "2020-Mar"}


# --------------------------------------------------------------------------
# Fixed-frequency locators
# --------------------------------------------------------------------------


YEAR_SPANS = [("2000-03-15T00:00:00", "2031-01-01T00:00:00"), ("1999-12-31T00:00:00", "2001-01-02T00:00:00"),
              ("2020-06-01T00:00:00", "2020-07-01T00:00:00")]
MONTH_SPANS = [("2020-01-01T00:00:00", "2021-01-01T00:00:00"), ("2019-11-20T00:00:00", "2020-03-10T00:00:00"),
               ("2020-01-05T00:00:00", "2020-01-25T00:00:00")]
WEEK_SPANS = [("2020-01-01T00:00:00", "2020-02-01T00:00:00"), ("2020-02-25T00:00:00", "2020-03-06T00:00:00")]
DAY_SPANS = [("2020-01-01T00:00:00", "2020-03-01T00:00:00"), ("2020-02-20T06:00:00", "2020-03-04T18:00:00")]
HOUR_SPANS = [("2020-01-01T00:00:00", "2020-01-02T00:00:00"), ("2020-01-01T05:30:00", "2020-01-01T09:10:00"),
              ("2020-03-28T22:00:00", "2020-03-29T04:00:00")]
MINUTE_SPANS = [("2020-01-01T00:00:00", "2020-01-01T02:00:00"), ("2020-01-01T10:03:20", "2020-01-01T10:20:00")]
SECOND_SPANS = [("2020-01-01T00:00:00", "2020-01-01T00:02:00"), ("2020-01-01T10:03:20.5", "2020-01-01T10:03:50")]

#: {locator: [(params, spans)]}, spans chosen to keep tick counts moderate.
FIXED_CASES = {
    "YearLocator": [
        ({}, YEAR_SPANS),
        ({"base": 5}, YEAR_SPANS),
        ({"base": 2, "month": 7, "day": 4}, YEAR_SPANS),
        ({"base": 1, "month": 2, "day": 29}, YEAR_SPANS),
        ({"base": 100}, [("0001-01-01T00:00:00", "3000-01-01T00:00:00"), ("1000-01-01T00:00:00", "2000-01-01T00:00:00"),
                         ("9000-01-01T00:00:00", "9999-12-31T00:00:00")]),
    ],
    "MonthLocator": [(params, MONTH_SPANS) for params in [
        {}, {"bymonth": [1, 4, 7, 10]}, {"bymonth": 6}, {"interval": 3}, {"bymonthday": 15}, {"bymonthday": 31},
        {"bymonth": [2], "bymonthday": 30}]],
    "WeekdayLocator": [(params, WEEK_SPANS) for params in [
        {}, {"byweekday": 0}, {"byweekday": [0, 2, 4]}, {"byweekday": 6, "interval": 2}]],
    "DayLocator": [(params, DAY_SPANS) for params in [
        {}, {"bymonthday": [1, 15]}, {"interval": 3}, {"bymonthday": 31}, {"bymonthday": [29, 30, 31]}]],
    "HourLocator": [(params, HOUR_SPANS) for params in [
        {}, {"byhour": [0, 12]}, {"interval": 6}, {"byhour": 3, "interval": 1}]],
    "MinuteLocator": [(params, MINUTE_SPANS) for params in [
        {}, {"byminute": [0, 30]}, {"interval": 15}, {"byminute": 7}]],
    "SecondLocator": [(params, SECOND_SPANS) for params in [
        {}, {"bysecond": [0, 30]}, {"interval": 10}, {"bysecond": 59}]],
    "MicrosecondLocator": [
        ({}, [("2020-01-01T00:00:00", "2020-01-01T00:00:00.000050"),
              ("2020-01-01T23:59:59.999990", "2020-01-02T00:00:00.000020")]),
        ({"interval": 10}, [("2020-01-01T00:00:00", "2020-01-01T00:00:00.002000"),
                            ("1970-01-01T00:00:00", "1970-01-01T00:00:00.000100")]),
        ({"interval": 500}, [("2020-01-01T00:00:00", "2020-01-01T00:00:00.002000"),
                             ("1970-01-01T00:00:00", "1970-01-01T00:00:00.1")]),
        ({"interval": 250000}, [("1970-01-01T00:00:00", "1970-01-01T00:00:01"),
                                ("2020-01-01T00:00:00", "2020-01-01T00:00:10")]),
    ],
}


def test_fixed_frequency_locators():
    for cls_name, cases in FIXED_CASES.items():
        for params, spans in cases:
            for vmin, vmax in spans:
                probes.probe(f"dates.{cls_name}.__call__", _locator_call(cls_name), params=params, vmin=vmin,
                             vmax=vmax)
        for vmin, vmax in [(18262.0, 18262.0), (18263.0, 18262.0), (0.0, 1e-7), (float("nan"), 1.0)]:
            probes.probe(f"dates.{cls_name}.nonsingular", _locator_nonsingular(cls_name), params=cases[1][0],
                         vmin=vmin, vmax=vmax)
    probes.probe("dates.DayLocator.__call__", _locator_call("DayLocator"), params={"interval": 0},
                 vmin=BASE, vmax="2020-02-01T00:00:00")
    probes.probe("dates.DayLocator.__call__", _locator_call("DayLocator"), params={"interval": 1.5},
                 vmin=BASE, vmax="2020-02-01T00:00:00")
    years = _locator_call("YearLocator")({"base": 5}, "2000-03-15T00:00:00", "2031-01-01T00:00:00")
    assert years["iso"][0] == "2000-01-01T00:00:00+00:00"


def test_rrule_locator():
    year = ("2020-01-01T00:00:00", "2021-01-01T00:00:00")
    days = ("2020-02-10T00:00:00", "2020-02-12T00:00:00")
    instant = ("2020-01-01T00:00:00", "2020-01-01T00:00:00.5")
    cases = [
        ({"freq": "YEARLY", "bymonth": [1, 7], "bymonthday": 1}, [year, days, instant]),
        ({"freq": "MONTHLY", "bymonthday": -1}, [year, days, instant]),
        ({"freq": "MONTHLY", "byweekday": [4], "bysetpos": -1}, [year, days]),
        ({"freq": "WEEKLY", "byweekday": [0], "interval": 2}, [year, days]),
        ({"freq": "DAILY", "interval": 10}, [year, days]),
        ({"freq": "DAILY", "byyearday": [1, 100, 200, 300]}, [year, days]),
        ({"freq": "HOURLY", "byhour": [9, 17], "byminute": 0, "bysecond": 0}, [days, instant]),
        ({"freq": "MINUTELY", "interval": 90}, [days, instant]),
        ({"freq": "SECONDLY", "interval": 3600}, [days, instant]),
    ]
    for rule, spans in cases:
        for vmin, vmax in spans:
            probes.probe("dates.RRuleLocator.__call__", rrule_locator_call, rule=rule, vmin=vmin, vmax=vmax)


# --------------------------------------------------------------------------
# DateFormatter
# --------------------------------------------------------------------------

FORMAT_DATES = [
    "2020-03-05T14:07:09.123456",
    "2021-01-01T00:00:00",
    "1999-12-31T23:59:59.999999",
    "2000-02-29T12:00:00",
    "1970-01-01T00:00:00",
    "1969-07-20T20:17:40",
    "2010-06-15T00:30:00",
    "2010-06-15T12:30:00",
    "2024-12-30T00:00:00",
    "2021-01-03T00:00:00",
    "1800-01-01T00:00:00.123456",
    "0001-01-01T00:00:00",
    "0099-05-04T03:02:01",
    "9999-12-31T23:59:59",
]

STRFTIME_CODES = ["%a", "%A", "%b", "%B", "%C", "%d", "%D", "%e", "%F", "%g", "%G", "%h", "%H", "%I", "%j", "%m",
                  "%M", "%n", "%p", "%R", "%S", "%t", "%T", "%u", "%U", "%V", "%w", "%W", "%y", "%Y", "%z", "%Z",
                  "%%", "%f"]


def test_date_formatter():
    for code in STRFTIME_CODES:
        for d in FORMAT_DATES:
            probes.probe("dates.DateFormatter.__call__", date_formatter, fmt=code, d=d)
    composite = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M:%S.%f", "%d %b %Y", "%H:%M", "%I:%M %p", "%A, %B %d",
                 "week %W/%U", "%Y-%jT%H", "literal text", "", "%Y%m%d%H%M%S", "%b-%d\n%Y"]
    for fmt in composite:
        for d in FORMAT_DATES[:6]:
            probes.probe("dates.DateFormatter.__call__", date_formatter, fmt=fmt, d=d)
    for fmt in ["%Y-%m-%d %H:%M:%S", "%d %b", "%H:%M:%S.%f", "Jan-%d"]:
        probes.probe("dates.DateFormatter.__call__", date_formatter, fmt=fmt, d=FORMAT_DATES[0], usetex=True)
    assert date_formatter("%Y-%m-%d", "2020-03-05T14:07:09") == "2020-03-05"
    assert date_formatter("%f", "1800-01-01T00:00:00.123456") == "123460"
