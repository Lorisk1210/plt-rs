"""The operations of the probe area ``dates_tz`` on a package with
matplotlib's API: ``matplotlib`` itself for the goldens
(``test_dates_tz.py``), ``pltrs`` when pltrs (Python)'s smoke tests replay
them (``python/tests/test_dates_tz.py``).

Argument conventions
--------------------
Zones (``tz``, ``rc``, the zone of a date):

* ``null``: no zone given (``tz=None``: the ``timezone`` rcParam's);
* a string: given as the string (``"UTC"`` and the names
  ``dateutil.tz.gettz`` resolves; as the zone of a date, the same);
* ``{"kind": "utc"}``: ``datetime.timezone.utc``;
* ``{"kind": "offset", "seconds": s}``: ``datetime.timezone(timedelta(seconds=s))``;
* ``{"kind": "zoneinfo", "name": n}``: ``zoneinfo.ZoneInfo(n)``;
* ``{"kind": "dateutil", "name": n}``: ``dateutil.tz.gettz(n)``;
* ``{"kind": "tzutc"}``: ``dateutil.tz.tzutc()``.

Dates:

* an ISO 8601 string: ``datetime.fromisoformat`` (naive, or with a fixed
  offset when the string has one);
* ``{"at": iso, "tz": zone, "fold": 0 | 1}``: the wall-clock time *iso*
  in *zone* (``replace(tzinfo=..., fold=...)``);
* ``{"date": iso}``: a ``datetime.date``;
* ``{"datetime64": iso}``: ``numpy.datetime64(iso)``.

``rc`` (where an op takes it) sets ``rcParams["timezone"]`` for the whole
operation; ``rc_then``, where given, replaces it once the locator,
formatter or plot is made, for using it. A date-time comes back as
``[isoformat, tzname, fold]``.

Ops
---
``dates_tz.date2num``    {d: date | [date]} -> float | [float]
``dates_tz.num2date``    {x: float | [float], tz?, rc?} -> date-time | [date-time]
``dates_tz.drange``      {dstart, dend: date, delta: timedelta keywords} -> [float]
``dates_tz.datetime64``  {counts: [int | null], unit} -> [float]: ``date2num`` of
                         ``numpy.array(counts, dtype=f"datetime64[{unit}]")``, null = NaT
``dates_tz.epoch``       {epoch, d: [date], x: [float]} -> {nums, dates}: ``date2num(d)``
                         and ``num2date(x)`` with ``set_epoch(epoch)``
``dates_tz.locate``      {locator, params?, tz?, rc?, view: [date, date], via} ->
                         {ticks, iso[, unit]}: the locator ``<locator>(**params, tz=tz)``
                         (``RRuleLocator``: ``params = {rule: {freq, tzinfo?, ...}}``) on
                         a view, through an axis (``via="axis"``: the view interval is
                         ``date2num`` of *view*, and the locator is called) or with
                         the dates themselves (``via="tick_values"``); ``iso`` are the
                         ticks in the locator's zone, ``unit`` (``AutoDateLocator``) is
                         ``_get_unit()`` after the call
``dates_tz.format``      {formatter, params?, tz?, rc?, rc_then?, view: [date, date]} ->
                         {labels[, offset, short]}: the labels ``<formatter>`` gives the
                         ticks an ``AutoDateLocator(tz=tz)`` places on the view through
                         an axis (``DateFormatter``: ``params = {fmt}``);
                         ``ConciseDateFormatter`` adds its offset and the
                         ``format_data_short`` of the first tick
``dates_tz.plot``        {d: [date], converter?, rc?, rc_then?} -> {ticks, labels, offset}:
                         ``ax.plot(d, range(len(d)))`` on a new figure with
                         ``rcParams["date.converter"] = converter``, drawn
                         (``draw_without_rendering``): the major tick locations of the
                         x axis, their labels and its offset text
"""

import contextlib
import datetime
import importlib
import zoneinfo

import dateutil.tz
import numpy as np

#: The frequencies of recurrence rules by dateutil's names.
FREQUENCIES = ("YEARLY", "MONTHLY", "WEEKLY", "DAILY", "HOURLY", "MINUTELY",
               "SECONDLY")


def zone(spec):
    """The ``tz`` argument a zone spec stands for (strings and null pass as
    they are)."""
    if spec is None or isinstance(spec, str):
        return spec
    kind = spec["kind"]
    if kind == "utc":
        return datetime.timezone.utc
    if kind == "offset":
        return datetime.timezone(datetime.timedelta(seconds=spec["seconds"]))
    if kind == "zoneinfo":
        return zoneinfo.ZoneInfo(spec["name"])
    if kind == "dateutil":
        return dateutil.tz.gettz(spec["name"])
    if kind == "tzutc":
        return dateutil.tz.tzutc()
    raise ValueError(f"unknown zone kind {kind!r}")


def tzinfo(spec):
    """The `datetime.tzinfo` of a zone spec, strings resolved as
    matplotlib resolves them."""
    tz = zone(spec)
    if tz == "UTC":
        return datetime.timezone.utc
    return dateutil.tz.gettz(tz) if isinstance(tz, str) else tz


def when(spec):
    """The date a date spec stands for."""
    if isinstance(spec, str):
        return datetime.datetime.fromisoformat(spec)
    if "date" in spec:
        return datetime.date.fromisoformat(spec["date"])
    if "datetime64" in spec:
        return np.datetime64(spec["datetime64"])
    local = datetime.datetime.fromisoformat(spec["at"])
    return local.replace(tzinfo=tzinfo(spec["tz"]), fold=spec.get("fold", 0))


def dates(spec):
    """One date or a list of them."""
    return [when(s) for s in spec] if isinstance(spec, list) else when(spec)


def described(dt):
    """A date-time as ``[isoformat, tzname, fold]``."""
    return [dt.isoformat(), dt.tzname(), dt.fold]


def make_ops(package):
    """The operations of the area on *package* (``matplotlib`` or
    ``pltrs``), by name."""
    mdates = importlib.import_module(package.__name__ + ".dates")
    importlib.import_module(package.__name__ + ".figure")
    rc_params = package.rcParams

    @contextlib.contextmanager
    def rc_zone(rc):
        """``rcParams["timezone"] = rc`` for the block, when given."""
        saved = rc_params["timezone"]
        if rc is not None:
            rc_params["timezone"] = rc
        try:
            yield
        finally:
            rc_params["timezone"] = saved

    def tz_keyword(tz):
        return {} if tz is None else {"tz": zone(tz)}

    def locator_of(name, params, tz):
        params = dict(params or {})
        if name == "RRuleLocator":
            rule = dict(params.pop("rule"))
            freq = FREQUENCIES.index(rule.pop("freq"))
            for key in ("dtstart", "until"):
                if key in rule:
                    rule[key] = when(rule[key])
            if "tzinfo" in rule:
                rule["tzinfo"] = tzinfo(rule["tzinfo"])
            params["o"] = mdates.rrulewrapper(freq, **rule)
        return getattr(mdates, name)(**params, **tz_keyword(tz))

    def on_axis(locator, view):
        locator.create_dummy_axis()
        locator.axis.set_view_interval(*(mdates.date2num(when(v)) for v in view))
        return locator

    def date2num(d):
        result = mdates.date2num(dates(d))
        return np.asarray(result).tolist()

    def num2date(x, tz=None, rc=None):
        with rc_zone(rc):
            result = mdates.num2date(x, **tz_keyword(tz))
        if isinstance(result, list):
            return [described(dt) for dt in result]
        return described(result)

    def drange(dstart, dend, delta):
        return mdates.drange(when(dstart), when(dend),
                             datetime.timedelta(**delta)).tolist()

    def datetime64(counts, unit):
        values = np.array(counts, dtype=f"datetime64[{unit}]")
        return mdates.date2num(values).tolist()

    def epoch(epoch, d, x):
        mdates._reset_epoch_test_example()
        try:
            mdates.set_epoch(epoch)
            return {"nums": date2num(d),
                    "dates": [described(dt) for dt in mdates.num2date(x)]}
        finally:
            mdates._reset_epoch_test_example()

    def locate(locator, view, via, params=None, tz=None, rc=None, rc_then=None):
        with rc_zone(rc):
            loc = locator_of(locator, params, tz)
        with rc_zone(rc if rc_then is None else rc_then):
            if via == "axis":
                ticks = on_axis(loc, view)()
            else:
                ticks = loc.tick_values(*(when(v) for v in view))
            ticks = np.asarray(ticks, dtype=float)
            result = {"ticks": ticks.tolist(),
                      "iso": [described(dt) for dt in mdates.num2date(ticks, loc.tz)]}
            if locator == "AutoDateLocator":
                result["unit"] = loc._get_unit()
        return result

    def format(formatter, view, params=None, tz=None, rc=None, rc_then=None):
        with rc_zone(rc):
            loc = mdates.AutoDateLocator(**tz_keyword(tz))
            params = dict(params or {})
            if formatter == "DateFormatter":
                made = mdates.DateFormatter(params.pop("fmt"), **params,
                                            **tz_keyword(tz))
            else:
                made = getattr(mdates, formatter)(loc, **params, **tz_keyword(tz))
        with rc_zone(rc if rc_then is None else rc_then):
            ticks = on_axis(loc, view)()
            result = {"labels": made.format_ticks(ticks)}
            if formatter == "ConciseDateFormatter":
                result["offset"] = made.get_offset()
                result["short"] = made.format_data_short(ticks[0])
        return result

    def plot(d, converter=None, rc=None, rc_then=None):
        settings = {} if converter is None else {"date.converter": converter}
        with rc_zone(rc), package.rc_context(settings):
            fig = package.figure.Figure()
            ax = fig.add_subplot()
            ax.plot(dates(d), range(len(d)))
        with rc_zone(rc if rc_then is None else rc_then):
            fig.draw_without_rendering()
            return {"ticks": ax.xaxis.get_majorticklocs().tolist(),
                    "labels": [t.get_text() for t in ax.get_xticklabels()],
                    "offset": ax.xaxis.get_offset_text().get_text()}

    return {"dates_tz.date2num": date2num, "dates_tz.num2date": num2date,
            "dates_tz.drange": drange, "dates_tz.datetime64": datetime64,
            "dates_tz.epoch": epoch, "dates_tz.locate": locate,
            "dates_tz.format": format, "dates_tz.plot": plot}
