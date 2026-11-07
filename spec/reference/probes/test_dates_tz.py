"""Dates in time zones (``matplotlib.dates`` with ``tz``).

Area ``dates_tz``: the locators, formatters and conversions of
``matplotlib.dates`` in UTC, fixed offsets and zones with daylight saving
time, given as strings, ``zoneinfo``, ``dateutil.tz`` and
``datetime.timezone`` objects, through an axis and with the dates
themselves, across the changes of the clocks both ways, the year boundary,
leap days, and ranges from microseconds to centuries.

The ops and their argument conventions are in ``_dates_tz.py``, which
pltrs (Python)'s smoke tests replay the goldens with. Every date is fixed:
nothing depends on the wall clock or on the machine's own zone (naive dates
are never mixed with aware ones, which matplotlib would read in it).
"""

import datetime

import matplotlib

from probes._dates_tz import make_ops, tzinfo
from spec_support import ProbeSet

probes = ProbeSet("dates_tz")
OPS = make_ops(matplotlib)

UTC = datetime.timezone.utc


def probe(op, **args):
    probes.probe(op, OPS[op], **args)


def kinds(name):
    """The zone *name* as each kind of ``tz`` matplotlib takes."""
    return [name, {"kind": "zoneinfo", "name": name},
            {"kind": "dateutil", "name": name}]


#: Zones whose clocks change, with instants (UTC) of a change forward and
#: of one back.
CHANGES = {
    "Europe/Berlin": ("2020-03-29T01:00:00", "2020-10-25T01:00:00"),
    "America/New_York": ("2020-03-08T07:00:00", "2020-11-01T06:00:00"),
    # Half an hour, both ways.
    "Australia/Lord_Howe": ("2020-10-03T15:30:00", "2020-04-04T15:00:00"),
    # Southern hemisphere, with the change at midnight.
    "America/Sao_Paulo": ("2018-11-04T03:00:00", "2019-02-17T02:00:00"),
    "Australia/Sydney": ("2020-10-03T16:00:00", "2020-04-04T16:00:00"),
}

#: Zones without changes in the ranges probed.
STEADY = [None, "UTC", {"kind": "utc"}, {"kind": "offset", "seconds": 19800},
          {"kind": "offset", "seconds": -12600}, {"kind": "tzutc"}, "Asia/Kolkata",
          {"kind": "zoneinfo", "name": "Asia/Kathmandu"}]

ZONES = [tz for name in CHANGES for tz in kinds(name)] + STEADY


def instant(iso):
    return datetime.datetime.fromisoformat(iso).replace(tzinfo=UTC)


def local(moment, tz):
    """The date spec of the instant *moment* on the wall clock of *tz*."""
    dt = moment.astimezone(tzinfo(tz))
    return {"at": dt.replace(tzinfo=None).isoformat(), "tz": tz, "fold": dt.fold}


def views(center, span, tz):
    """The views of *span* around the instant *center* (an ISO string in
    UTC), as each ``via`` takes them: ``(via, [start, end])``."""
    start, end = instant(center) - span / 2, instant(center) + span / 2
    naive = [d.replace(tzinfo=None).isoformat() for d in (start, end)]
    yield "axis", naive
    yield "tick_values", naive
    yield "tick_values", [d.isoformat() for d in (start, end)]
    if tz is not None:
        yield "tick_values", [local(d, tz) for d in (start, end)]


HOUR = datetime.timedelta(hours=1)
DAY = datetime.timedelta(days=1)

#: Locators, with the span of the views they are probed on.
LOCATORS = [
    ("HourLocator", {}, 6 * HOUR),
    ("HourLocator", {"interval": 3}, DAY),
    ("HourLocator", {"byhour": [0, 6, 12, 18]}, 2 * DAY),
    ("MinuteLocator", {"byminute": [0, 30]}, 3 * HOUR),
    ("MinuteLocator", {"interval": 15}, 2 * HOUR),
    ("SecondLocator", {"interval": 30}, datetime.timedelta(minutes=4)),
    ("DayLocator", {}, 8 * DAY),
    ("DayLocator", {"bymonthday": [1, 15], "interval": 1}, 60 * DAY),
    ("WeekdayLocator", {"byweekday": 0}, 35 * DAY),
    ("MonthLocator", {}, 365 * DAY),
    ("MonthLocator", {"bymonth": [3, 10], "bymonthday": 25}, 730 * DAY),
    ("YearLocator", {}, 3650 * DAY),
    ("MicrosecondLocator", {"interval": 2}, datetime.timedelta(microseconds=10)),
    ("AutoDateLocator", {}, 3 * HOUR),
    ("AutoDateLocator", {}, DAY),
    ("AutoDateLocator", {}, 10 * DAY),
    ("AutoDateLocator", {}, 120 * DAY),
    ("AutoDateLocator", {"interval_multiples": False}, DAY),
    ("RRuleLocator", {"rule": {"freq": "HOURLY", "interval": 2}}, 8 * HOUR),
    ("RRuleLocator", {"rule": {"freq": "DAILY", "byhour": [9, 17]}}, 4 * DAY),
]


def test_locators_across_changes_of_the_clocks():
    # A name resolves to dateutil's zone of that name: the two kinds of
    # dateutil zones (the name and the object) place the same ticks.
    for name, changes in CHANGES.items():
        for tz in kinds(name)[:2]:
            for change in changes:
                for locator, params, span in LOCATORS:
                    for via, view in views(change, span, tz):
                        probe("dates_tz.locate", locator=locator, params=params,
                              tz=tz, view=view, via=via)


def test_locators_in_steady_zones():
    for tz in STEADY:
        for locator, params, span in LOCATORS:
            for via, view in views("2021-01-01T00:00:00", span, tz):
                probe("dates_tz.locate", locator=locator, params=params,
                      tz=tz, view=view, via=via)


#: Spans AutoDateLocator picks every frequency for, around the year boundary
#: and a leap day.
SPANS = [datetime.timedelta(microseconds=6), datetime.timedelta(microseconds=900),
         datetime.timedelta(seconds=2), datetime.timedelta(minutes=3), 2 * HOUR,
         2 * DAY, 20 * DAY, 200 * DAY, 4 * 365 * DAY, 40 * 365 * DAY,
         400 * 365 * DAY]


def test_auto_locator_spans():
    for tz in [None, "Europe/Berlin", {"kind": "zoneinfo", "name": "Australia/Lord_Howe"},
               {"kind": "offset", "seconds": -34200}]:
        for center in ["2021-01-01T00:00:00", "2020-02-29T00:00:00", "2020-02-29T12:00:00"]:
            for span in SPANS:
                for via, view in views(center, span, tz):
                    probe("dates_tz.locate", locator="AutoDateLocator", tz=tz,
                          view=view, via=via)


def test_leap_days_and_year_boundaries():
    for tz in ["America/New_York", {"kind": "zoneinfo", "name": "Pacific/Auckland"},
               {"kind": "offset", "seconds": 3600}]:
        for locator, params, span in [
                ("YearLocator", {"month": 2, "day": 29}, 3650 * DAY),
                ("YearLocator", {"base": 4, "month": 2, "day": 29}, 3650 * DAY),
                ("DayLocator", {"bymonthday": 29}, 365 * DAY),
                ("MonthLocator", {"bymonthday": 31}, 365 * DAY),
                ("YearLocator", {"base": 5}, 100 * 365 * DAY),
                ("YearLocator", {"base": 100}, 1000 * 365 * DAY),
                ("HourLocator", {"byhour": [0]}, 4 * DAY)]:
            for center in ["2021-01-01T00:00:00", "2020-02-29T00:00:00"]:
                for via, view in views(center, span, tz):
                    probe("dates_tz.locate", locator=locator, params=params,
                          tz=tz, view=view, via=via)


def test_rule_zones():
    # The zone of a rule (rrulewrapper's tzinfo) against the locator's.
    for rule_tz in ["UTC", {"kind": "zoneinfo", "name": "Asia/Kolkata"},
                    "America/New_York", {"kind": "offset", "seconds": -3600}]:
        for tz in ["Europe/Berlin", {"kind": "zoneinfo", "name": "Australia/Lord_Howe"}]:
            for freq, extra, span in [("HOURLY", {"interval": 3}, DAY),
                                      ("DAILY", {"byhour": [0, 12]}, 4 * DAY),
                                      ("MINUTELY", {"byminute": [0, 30]}, 3 * HOUR)]:
                rule = {"freq": freq, "tzinfo": rule_tz, **extra}
                for via, view in views("2020-10-25T01:00:00", span, tz):
                    probe("dates_tz.locate", locator="RRuleLocator",
                          params={"rule": rule}, tz=tz, view=view, via=via)


def test_rc_timezone():
    for rc in ["Europe/Berlin", "America/Sao_Paulo", "Asia/Kolkata", "UTC"]:
        for locator, params, span in LOCATORS[:3] + LOCATORS[13:15]:
            for via, view in views("2020-10-25T01:00:00", span, None):
                probe("dates_tz.locate", locator=locator, params=params,
                      view=view, via=via, rc=rc)
        probe("dates_tz.num2date", x=[18560.0, 18560.0417, 18560.5], rc=rc)
        for formatter in ["AutoDateFormatter", "ConciseDateFormatter"]:
            probe("dates_tz.format", formatter=formatter, rc=rc,
                  view=["2020-10-24T20:00:00", "2020-10-25T06:00:00"])
    probe("dates_tz.num2date", x=0.0, rc="Nowhere/Atlantis")
    probe("dates_tz.locate", locator="HourLocator", rc="Nowhere/Atlantis",
          view=["2020-01-01T00:00:00", "2020-01-01T03:00:00"], via="axis")


#: Wall-clock times repeated (``fold`` matters) and skipped by the clocks.
AMBIGUOUS = {"Europe/Berlin": "2020-10-25T02:30:00",
             "America/New_York": "2020-11-01T01:30:00",
             "Australia/Lord_Howe": "2020-04-05T01:45:00",
             "America/Sao_Paulo": "2019-02-16T23:30:00"}
SKIPPED = {"Europe/Berlin": "2020-03-29T02:30:00",
           "America/New_York": "2020-03-08T02:30:00",
           "Australia/Lord_Howe": "2020-10-04T02:15:00",
           "America/Sao_Paulo": "2018-11-04T00:30:00"}


def test_date2num_of_aware_dates():
    for name in AMBIGUOUS:
        for tz in kinds(name)[1:]:
            for at in (AMBIGUOUS[name], SKIPPED[name], "2020-07-01T12:00:00",
                       "2020-01-01T00:00:00", "1960-06-01T00:00:00",
                       "2037-07-01T00:00:00", "2045-07-01T00:00:00"):
                for fold in (0, 1):
                    probe("dates_tz.date2num", d={"at": at, "tz": tz, "fold": fold})
            probe("dates_tz.date2num", d=[{"at": AMBIGUOUS[name], "tz": tz, "fold": fold}
                                          for fold in (0, 1)])
    for d in ["2020-01-01T00:00:00+05:30", "2020-01-01T00:00:00-00:01",
              "2020-06-30T23:59:59.999999+14:00", ["2020-01-01T00:00:00+01:00",
                                                   "2020-01-01T00:00:00Z"],
              {"at": "2020-01-01T00:00:00", "tz": {"kind": "tzutc"}},
              {"date": "2020-02-29"}, [{"date": "2020-02-29"}, {"date": "2021-03-01"}],
              {"datetime64": "2020-01-01T00:00:00.123456789"}, []]:
        probe("dates_tz.date2num", d=d)


def test_num2date_in_zones():
    xs = [18560.0, 18560.04166666666, 18560.0625, 18560.08333333333, 18560.125,
          18346.25, 18346.3, 18346.34, -3652.0, 25567.5, 27000.25]
    for tz in ZONES:
        probe("dates_tz.num2date", x=xs, tz=tz)
        probe("dates_tz.num2date", x=18560.0625, tz=tz)
    for name, (forward, back) in CHANGES.items():
        for tz in kinds(name):
            for change in (forward, back):
                center = datetime.datetime.fromisoformat(change)
                x = (center - datetime.datetime(1970, 1, 1)) / DAY
                probe("dates_tz.num2date", tz=tz,
                      x=[x + k / 96 for k in range(-6, 7)])
    for x in [0.5, 2932896.9999999, -719162.0, -719163.0, 2932897.0, 1e-7, 1e6 + 1e-6]:
        probe("dates_tz.num2date", x=x, tz="America/New_York")


def test_drange():
    for tz in kinds("Europe/Berlin")[1:] + [{"kind": "offset", "seconds": 3600}]:
        for start, end, delta in [
                ("2020-03-28T12:00:00", "2020-03-30T00:00:00", {"hours": 6}),
                ("2020-10-24T22:00:00", "2020-10-25T05:00:00", {"hours": 1}),
                ("2020-10-25T00:00:00", "2020-10-26T00:00:00", {"days": 1}),
                ("2020-01-01T00:00:00", "2020-01-01T00:00:01", {"microseconds": 250000})]:
            probe("dates_tz.drange", dstart={"at": start, "tz": tz},
                  dend={"at": end, "tz": tz}, delta=delta)
    probe("dates_tz.drange", dstart="2020-10-25T00:00:00+02:00",
          dend="2020-10-25T06:00:00+01:00", delta={"hours": 1})


def test_datetime64_units():
    counts = {"Y": [0, 50, -1, 30], "M": [0, 1, 601, -13], "W": [0, 1, 2639, -1],
              "D": [0, 18321, -1, 2932896], "h": [0, 439704, -25], "m": [0, 26382240, -1],
              "s": [0, 1582934400, -86401], "ms": [0, 1582934400123, -1],
              "us": [0, 1582934400123456, -1], "ns": [0, 1582934400123456789, -1, 1],
              "ps": [0, 1, 1000000, -1000000000], "fs": [0, 1000000000, -5],
              "as": [0, 1, 1000000000000, -1]}
    for unit, values in counts.items():
        probe("dates_tz.datetime64", counts=values, unit=unit)
        probe("dates_tz.datetime64", counts=[values[1], None, values[0]], unit=unit)
    probe("dates_tz.datetime64", counts=[None], unit="D")
    probe("dates_tz.datetime64", counts=[], unit="s")


def test_epochs():
    d = ["2020-01-01T00:00:00", "2000-01-01T12:00:00", "1970-01-01T00:00:00.000001",
         {"at": "2020-10-25T02:30:00", "tz": {"kind": "zoneinfo", "name": "Europe/Berlin"},
          "fold": 1}]
    for epoch in ["0000-12-31T00:00:00", "2000-01-01", "1970-01-01T00:00:00",
                  "2020-02-29T12:30:00.5", "1900-01-01T00:00:00", "-0001-01-01"]:
        probe("dates_tz.epoch", epoch=epoch, d=d, x=[0.0, 1.5, -0.25, 1e-6])


def test_formatters_in_zones():
    for name, (forward, back) in CHANGES.items():
        for tz in kinds(name)[:2]:
            for change in (forward, back):
                for span in [3 * HOUR, DAY, 10 * DAY]:
                    start = instant(change) - span / 2
                    view = [d.replace(tzinfo=None).isoformat() for d in (start, start + span)]
                    for formatter, params in [
                            ("DateFormatter", {"fmt": "%Y-%m-%d %H:%M:%S %Z %z"}),
                            ("AutoDateFormatter", {}),
                            ("ConciseDateFormatter", {}),
                            ("ConciseDateFormatter", {"show_offset": False})]:
                        probe("dates_tz.format", formatter=formatter, params=params,
                              tz=tz, view=view)
    for tz in STEADY:
        for view in [["2020-12-31T22:00:00", "2021-01-01T02:00:00"],
                     ["2020-12-31T23:59:59.999990", "2021-01-01T00:00:00.000010"],
                     ["2000-01-01T00:00:00", "2100-01-01T00:00:00"]]:
            for formatter, params in [("DateFormatter", {"fmt": "%c %Z"}),
                                      ("AutoDateFormatter", {}),
                                      ("ConciseDateFormatter", {})]:
                probe("dates_tz.format", formatter=formatter, params=params,
                      tz=tz, view=view)


def test_rc_timezone_changed_after_making():
    # Locators keep the zone the rcParam named when they were made;
    # AutoDateFormatter and ConciseDateFormatter read it as they format.
    view = ["2020-10-24T20:00:00", "2020-10-25T06:00:00"]
    for rc, rc_then in [("Europe/Berlin", "UTC"), ("UTC", "America/New_York"),
                        ("Asia/Kolkata", "Australia/Lord_Howe")]:
        for locator, params in [("HourLocator", {"interval": 2}), ("AutoDateLocator", {})]:
            probe("dates_tz.locate", locator=locator, params=params, view=view,
                  via="axis", rc=rc, rc_then=rc_then)
        for formatter, params in [("DateFormatter", {"fmt": "%H:%M %Z"}),
                                  ("AutoDateFormatter", {}), ("ConciseDateFormatter", {})]:
            probe("dates_tz.format", formatter=formatter, params=params, view=view,
                  rc=rc, rc_then=rc_then)


#: Zone names dateutil and zoneinfo find in other places than the
#: system's database here: dateutil's own zones and the tzdata package's.
ELSEWHERE = ["US/Pacific", "EST5EDT", "GMT", "Etc/GMT+5"]


def test_zones_found_elsewhere():
    for name in ELSEWHERE:
        for tz in kinds(name):
            probe("dates_tz.num2date", x=[18346.25, 18560.25], tz=tz)
            for via, view in views("2020-11-01T09:00:00", 6 * HOUR, tz):
                probe("dates_tz.locate", locator="HourLocator", params={"interval": 2},
                      tz=tz, view=view, via=via)


#: Views at the ends of the dates matplotlib represents.
EXTREMES = [
    ("HourLocator", {}, ["0001-01-01T00:00:00", "0001-01-01T06:00:00"]),
    ("HourLocator", {}, ["9999-12-31T12:00:00", "9999-12-31T23:59:59"]),
    ("DayLocator", {}, ["0001-01-01T00:00:00", "0001-01-10T00:00:00"]),
    ("DayLocator", {}, ["9999-12-20T00:00:00", "9999-12-31T00:00:00"]),
    ("MonthLocator", {}, ["0001-01-01T00:00:00", "0002-06-01T00:00:00"]),
    ("YearLocator", {"base": 1000}, ["0001-01-01T00:00:00", "9999-12-31T00:00:00"]),
    ("AutoDateLocator", {}, ["0001-01-01T00:00:00", "9999-12-31T00:00:00"]),
    ("AutoDateLocator", {}, ["0001-01-01T00:00:00", "0001-01-01T12:00:00"]),
    ("AutoDateLocator", {}, ["9999-12-31T00:00:00", "9999-12-31T12:00:00"]),
]


def test_extreme_views():
    for tz in [None, "Europe/Berlin", {"kind": "offset", "seconds": 19800},
               {"kind": "offset", "seconds": -12600},
               {"kind": "zoneinfo", "name": "America/New_York"}]:
        for locator, params, view in EXTREMES:
            for via in ("axis", "tick_values"):
                probe("dates_tz.locate", locator=locator, params=params, tz=tz,
                      view=view, via=via)


def _hours(start, count, tz, step=1):
    """*count* date specs every *step* hours from the instant *start* on
    the wall clock of *tz*."""
    return [local(instant(start) + k * step * HOUR, tz) for k in range(count)]


def test_plotted_dates_take_their_zone():
    for name, (forward, back) in CHANGES.items():
        for tz in kinds(name)[1:]:
            for change in (forward, back):
                for converter in (None, "concise"):
                    for count, step in [(7, 1), (10, 6), (40, 24)]:
                        start = (instant(change) - count * step * HOUR / 2)
                        probe("dates_tz.plot", converter=converter,
                              d=_hours(start.replace(tzinfo=None).isoformat(),
                                       count, tz, step))
    naive = [f"2020-10-2{day}T0{hour}:00:00" for day in (4, 5) for hour in (0, 3, 6)]
    for rc in [None, "Europe/Berlin", "Australia/Lord_Howe"]:
        for converter in (None, "concise"):
            probe("dates_tz.plot", d=naive, rc=rc, converter=converter)
            probe("dates_tz.plot", d=naive, rc=rc, rc_then="UTC", converter=converter)
            probe("dates_tz.plot", d=[{"date": "2020-10-24"}, {"date": "2020-10-26"}],
                  rc=rc, converter=converter)
            probe("dates_tz.plot", d=[{"datetime64": "2020-10-24T12:00"},
                                      {"datetime64": "2020-10-25T12:00"}],
                  rc=rc, converter=converter)
    # The zone of the first date is the axis'.
    probe("dates_tz.plot", d=[{"at": "2020-06-01T00:00:00", "tz": "Asia/Kolkata"},
                              {"at": "2020-06-01T12:00:00", "tz": "America/New_York"}])
    probe("dates_tz.plot", d=["2020-06-01T00:00:00+05:30", "2020-06-02T00:00:00-04:00"])
