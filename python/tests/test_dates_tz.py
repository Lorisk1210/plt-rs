"""
Dates in time zones: pltrs (Python) replays the goldens of the probe area
``dates_tz`` (``spec/goldens/probes/dates_tz.json``, recorded from
matplotlib 3.11) with the area's own operations
(``spec/reference/probes/_dates_tz.py``), which take the package they
run on; then what the goldens cannot hold (Python's objects: pandas'
dates, ``rrulewrapper``'s, the units of an axis), with the values
recorded from matplotlib 3.11.2.

The goldens were recorded with python-dateutil 2.9 and every zone name
the replay asks zoneinfo for. A probe naming a zone zoneinfo lacks here
(the legacy names systems without their legacy zone files and without
the tzdata package lack) expects zoneinfo's error, as the probe makes
the zone before calling matplotlib. Where python-dateutil's zones answer
as zoneinfo's (python-dateutil 4, `build_facts.DATEUTIL_ZONES_AS_ZONEINFO`),
matplotlib places and names the dates of a dateutil zone, given or by
name, as those of zoneinfo's zone of the same name: such a probe
expects its twin's golden (the area's ``as_zoneinfo``) where the goldens
hold one. Where python-dateutil's zones round their files' offsets to
whole minutes (python-dateutil 2.7, `build_facts.DATEUTIL_KEEPS_OFFSETS`),
a probe asking one for a time its golden gives an offset of seconds
(local mean time) has nothing to expect: matplotlib computes with the
rounded offset there, which no golden holds (pltrs asks such a zone
through Python, as matplotlib does). Where python-dateutil's zone database
is out of date (python-dateutil 2.7 on Windows reads its own files of 2018,
`build_facts.DATEUTIL_ZONES_CURRENT`), a probe asking one of its zones has
nothing to expect either: matplotlib computes with the old rules.
"""

import datetime
import importlib.util
import json
import math
import pathlib
import re
import warnings
import zoneinfo

import dateutil.tz
import numpy as np
import pytest

import pltrs
import pltrs.dates as mdates
from pltrs.figure import Figure

import build_facts

ROOT = pathlib.Path(__file__).resolve().parents[2]
GOLDEN = ROOT / "spec" / "goldens" / "probes" / "dates_tz.json"
OPERATIONS = ROOT / "spec" / "reference" / "probes" / "_dates_tz.py"


def _area():
    """The area's module of operations."""
    spec = importlib.util.spec_from_file_location("_dates_tz", OPERATIONS)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _probes():
    """The golden probes, by operation."""
    by_op = {}
    for entry in json.loads(GOLDEN.read_text(encoding="utf-8"))["probes"]:
        by_op.setdefault(entry["op"], []).append(entry)
    return by_op


PROBES = _probes()
AREA = _area()
OPS = AREA.make_ops(pltrs)


def _key(op, args):
    """A probe's operation and arguments, as a key."""
    return op, json.dumps(args, sort_keys=True)


#: Each golden probe's outcome, by `_key`.
GOLDEN = {_key(op, entry["args"]): {k: entry[k] for k in ("value", "error") if k in entry}
          for op, entries in PROBES.items() for entry in entries}


def _zoneinfo_names(args):
    """The names of the zoneinfo zones of *args*."""
    if isinstance(args, dict):
        if args.get("kind") == "zoneinfo":
            return {args["name"]}
        return set().union(*map(_zoneinfo_names, args.values()))
    if isinstance(args, list):
        return set().union(*map(_zoneinfo_names, args))
    return set()


def _has_zone(name):
    """Whether zoneinfo has the zone *name* here."""
    try:
        zoneinfo.ZoneInfo(name)
    except zoneinfo.ZoneInfoNotFoundError:
        return False
    return True


#: An offset from UTC of seconds, as the goldens write date-times.
SECONDS_OFFSET = re.compile(r'[+-]\d\d:\d\d:\d\d"')


def _expected(op, args):
    """What the golden probe of *op* with *args* expects here (see the
    module's description), None for nothing."""
    uses_dateutil = AREA.as_zoneinfo(args) != args
    if (uses_dateutil and not build_facts.DATEUTIL_KEEPS_OFFSETS
            and SECONDS_OFFSET.search(json.dumps(GOLDEN[_key(op, args)]))):
        return None
    if not all(map(_has_zone, _zoneinfo_names(args))):
        return {"error": "ZoneInfoNotFoundError"}
    if uses_dateutil and not build_facts.DATEUTIL_ZONES_CURRENT:
        return None
    if uses_dateutil and build_facts.DATEUTIL_ZONES_AS_ZONEINFO:
        return GOLDEN.get(_key(op, AREA.as_zoneinfo(args)), GOLDEN[_key(op, args)])
    return GOLDEN[_key(op, args)]

UTC = datetime.timezone.utc
NEW_YORK = zoneinfo.ZoneInfo("America/New_York")
BERLIN = zoneinfo.ZoneInfo("Europe/Berlin")


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield


def _plain(value):
    """A result as the goldens record it (tuples as lists, no -0.0)."""
    if isinstance(value, dict):
        return {str(k): _plain(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [_plain(v) for v in value]
    if isinstance(value, float) and value == 0:
        return 0.0
    return value


def _same(expected, actual, rel=1e-9, abs_tol=1e-12):
    """Whether *actual* is the golden value *expected* (floats within the
    harness' tolerances, NaN equal to NaN)."""
    if isinstance(expected, float) or isinstance(actual, float):
        if not isinstance(actual, (int, float)) or isinstance(actual, bool):
            return False
        if math.isnan(expected) or math.isnan(actual):
            return math.isnan(expected) and math.isnan(actual)
        return math.isclose(expected, actual, rel_tol=rel, abs_tol=abs_tol)
    if isinstance(expected, list):
        return (isinstance(actual, list) and len(expected) == len(actual)
                and all(_same(e, a) for e, a in zip(expected, actual)))
    if isinstance(expected, dict):
        return (isinstance(actual, dict) and expected.keys() == actual.keys()
                and all(_same(expected[k], actual[k]) for k in expected))
    return expected == actual


def _outcome(op, args):
    """What the operation gives pltrs: ``{"value": ...}`` or ``{"error":
    type name}``."""
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            return {"value": _plain(OPS[op](**args))}
    except Exception as exc:  # noqa: BLE001 - errors are part of the spec
        return {"error": type(exc).__name__}


@pytest.mark.parametrize("op", sorted(PROBES))
def test_goldens(op):
    mismatches = []
    for entry in PROBES[op]:
        expected = _expected(op, entry["args"])
        if expected is None:
            continue
        got = _outcome(op, entry["args"])
        if got.keys() != expected.keys() or not _same(
                next(iter(expected.values())), next(iter(got.values()))):
            mismatches.append(f"{json.dumps(entry['args'])}\n"
                              f"    expected {json.dumps(expected)}\n"
                              f"    got      {json.dumps(got)}")
    assert not mismatches, (f"{len(mismatches)} of {len(PROBES[op])} differ:\n"
                            + "\n".join(mismatches[:20]))


def test_tick_values_run_the_rule_in_the_zone_of_the_dates():
    # Naive dates tick on their own (UTC) hours, not the locator's: no
    # second tick at midnight UTC where Berlin skips 02:00, and every six
    # hours from midnight UTC rather than Berlin's.
    hours = mdates.HourLocator(tz="Europe/Berlin")
    start, end = datetime.datetime(2020, 3, 29), datetime.datetime(2020, 3, 29, 3)
    assert list(hours.tick_values(start, end)) == [
        18350.0, 18350.041666666668, 18350.083333333332, 18350.125]
    # Where the clocks skip 02:00, matplotlib's rule, run in the zone of the
    # dates, repeats the tick after the change in zoneinfo's zone (and
    # python-dateutil 4's) and the one before it in python-dateutil 2's.
    zoneinfo_ticks = [18350.0, 18350.041666666668, 18350.041666666668, 18350.083333333332,
                      18350.125]
    for zone, expected in [
            (BERLIN, zoneinfo_ticks),
            (dateutil.tz.gettz("Europe/Berlin"),
             zoneinfo_ticks if build_facts.DATEUTIL_ZONES_AS_ZONEINFO else
             [18350.0, 18350.0, 18350.041666666668, 18350.083333333332, 18350.125])]:
        in_berlin = [d.replace(tzinfo=UTC).astimezone(zone) for d in (start, end)]
        assert list(hours.tick_values(*in_berlin)) == expected
    six = mdates.HourLocator(byhour=[0, 6, 12, 18], tz="Europe/Berlin")
    assert list(six.tick_values(datetime.datetime(2020, 1, 1),
                                datetime.datetime(2020, 1, 2))) == [
        18262.0, 18262.25, 18262.5, 18262.75, 18263.0]


def test_dateutil_offsets_without_names():
    off = dateutil.tz.tzoffset(None, -18000)
    assert [(d.isoformat(), d.tzname()) for d in mdates.num2date([18262.0, 18262.5], off)] == [
        ("2019-12-31T19:00:00-05:00", None), ("2020-01-01T07:00:00-05:00", None)]
    assert mdates.DateFormatter("%H:%M %Z|%z", tz=off)(18262.25) == "01:00 |-0500"
    ticks = mdates.HourLocator(interval=6, tz=off).tick_values(
        datetime.datetime(2020, 1, 1, tzinfo=off), datetime.datetime(2020, 1, 2, tzinfo=off))
    assert list(ticks) == [18262.208333333332, 18262.458333333332, 18262.708333333332,
                           18262.958333333332, 18263.208333333332]


def test_date2num_reads_the_zone_of_the_first_date():
    with pytest.raises(AttributeError, match="no attribute 'astimezone'"):
        mdates.date2num([datetime.datetime(2020, 1, 1, tzinfo=UTC), datetime.date(2020, 1, 2)])
    assert mdates.date2num(np.array([], dtype=object)).dtype == object
    with pytest.raises(OverflowError, match="conversion factor"):
        mdates.date2num(np.array([1], dtype="datetime64[as]"))


def test_zones_refused():
    with pytest.raises(TypeError, match="tz must be string or tzinfo subclass, not 5."):
        mdates.HourLocator(tz=5)
    with pytest.raises(ValueError, match="Nowhere/Atlantis is not a valid timezone"):
        mdates.num2date(0.0, tz="Nowhere/Atlantis")


def test_rrulewrapper_in_a_zone():
    rule = mdates.rrulewrapper(mdates.HOURLY, tzinfo=NEW_YORK,
                               dtstart=datetime.datetime(2020, 11, 1))
    found = rule.between(datetime.datetime(2020, 11, 1, 0, 30, tzinfo=NEW_YORK),
                         datetime.datetime(2020, 11, 1, 3, tzinfo=NEW_YORK), inc=True)
    assert [d.isoformat() for d in found] == [
        "2020-11-01T01:00:00-04:00", "2020-11-01T02:00:00-05:00", "2020-11-01T03:00:00-05:00"]
    assert rule.after(datetime.datetime(2020, 11, 1, 4, tzinfo=UTC)).isoformat() == \
        "2020-11-01T01:00:00-04:00"
    assert rule.before(datetime.datetime(2020, 11, 1, 7, tzinfo=UTC), inc=True).isoformat() == \
        "2020-11-01T02:00:00-05:00"
    started = mdates.rrulewrapper(
        mdates.DAILY, dtstart=datetime.datetime(2020, 3, 28, 12, tzinfo=BERLIN), count=3)
    assert [d.isoformat() for d in started.between(
        datetime.datetime(2020, 3, 28), datetime.datetime(2020, 3, 31), inc=True)] == [
        "2020-03-28T12:00:00+01:00", "2020-03-29T12:00:00+02:00", "2020-03-30T12:00:00+02:00"]
    naive = mdates.rrulewrapper(mdates.DAILY, dtstart=datetime.datetime(2020, 1, 1))
    with pytest.raises(TypeError, match="can't compare offset-naive and offset-aware"):
        naive.between(datetime.datetime(2020, 1, 1, tzinfo=UTC),
                      datetime.datetime(2020, 1, 3, tzinfo=UTC))
    with pytest.raises(ValueError, match="until cannot be aware if dtstart is naive"):
        mdates.rrulewrapper(mdates.DAILY, dtstart=datetime.datetime(2020, 1, 1),
                            until=datetime.datetime(2020, 1, 3, tzinfo=UTC))


def test_microseconds_far_from_the_epoch_warn():
    start = datetime.datetime(2100, 1, 1)
    with pytest.warns(UserWarning, match="Plotting microsecond time intervals for dates far"):
        ticks = mdates.AutoDateLocator().tick_values(
            start, start + datetime.timedelta(microseconds=50))
    assert list(ticks[:2]) == [47481.99999999988, 47482.0]


#: The x tick labels of the dates of `test_pandas_dates_plot_in_their_zone`.
BERLIN_HOURS = ["10-24 22", "10-24 23", "10-25 00", "10-25 01", "10-25 02", "10-25 03"]


def _drawn(plotted):
    """The unit, the major locator's zone and the tick labels of the x
    axis of *plotted* against counts."""
    fig = Figure()
    ax = fig.add_subplot()
    ax.plot(plotted, range(len(plotted)))
    fig.draw_without_rendering()
    return (str(ax.xaxis.get_units()), str(ax.xaxis.get_major_locator().tz),
            [t.get_text() for t in ax.get_xticklabels()])


def test_pandas_dates_plot_in_their_zone():
    pd = pytest.importorskip("pandas")
    stamp = pd.Timestamp("2020-03-29 03:30:00.000000123", tz="Europe/Berlin")
    assert mdates.date2num(stamp) == 18350.062500000004
    assert mdates.date2num(pd.Timestamp("2020-03-29 03:30:00.000000123")) == 18350.145833333336
    index = pd.date_range("2020-10-24 22:00", periods=4, freq="2h", tz="Europe/Berlin")
    numbers = [18559.833333333332, 18559.916666666668, 18560.0, 18560.083333333332]
    for dates in (index, pd.Series(index), list(index)):
        assert list(mdates.date2num(dates)) == numbers
    with pytest.raises(ValueError, match="NaTType does not support astimezone"):
        mdates.date2num(pd.Series([index[0], pd.NaT]))
    for dates in (index, pd.Series(index), list(index.to_pydatetime())):
        assert _drawn(dates) == ("Europe/Berlin", "Europe/Berlin", BERLIN_HOURS)


def test_axis_units_given_a_zone():
    days = [datetime.datetime(2020, 1, 1), datetime.datetime(2020, 1, 2)]
    tokyo = ["01-01 09", "01-01 12", "01-01 15", "01-01 18", "01-01 21", "01-02 00",
             "01-02 03", "01-02 06", "01-02 09"]
    for units_first in (False, True):
        fig = Figure()
        ax = fig.add_subplot()
        if units_first:
            ax.xaxis.set_units(zoneinfo.ZoneInfo("Asia/Tokyo"))
        ax.plot(days, [0, 1])
        ax.xaxis.set_units(zoneinfo.ZoneInfo("Asia/Tokyo"))
        fig.draw_without_rendering()
        assert str(ax.xaxis.get_units()) == "Asia/Tokyo"
        assert [t.get_text() for t in ax.get_xticklabels()] == tokyo
    fig = Figure()
    ax = fig.add_subplot()
    ax.plot([18262.0, 18263.0], [0, 1])
    ax.xaxis_date("Asia/Kolkata")
    fig.draw_without_rendering()
    assert ax.xaxis.get_units() is dateutil.tz.gettz("Asia/Kolkata")
    assert [t.get_text() for t in ax.get_xticklabels()] == [
        "01-01 06", "01-01 09", "01-01 12", "01-01 15", "01-01 18", "01-01 21",
        "01-02 00", "01-02 03", "01-02 06"]


TOKYO_DAYS = [datetime.datetime(2020, 1, 1, tzinfo=zoneinfo.ZoneInfo("Asia/Tokyo")),
              datetime.datetime(2020, 1, 2, tzinfo=zoneinfo.ZoneInfo("Asia/Tokyo"))]


@pytest.mark.parametrize("plotting, first_labels", [
    (lambda ax, d: ax.plot(d, [0, 1]), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.scatter(d, [0, 1]), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.bar(d, [1, 2], width=0.5), ["12-31 20", "01-01 00"]),
    (lambda ax, d: ax.fill_between(d, [0, 1]), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.axvline(d[0]), ["2018-01", "2018-07"]),
    (lambda ax, d: ax.axvspan(d[0], d[1]), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.set_xlim(d[0], d[1]), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.vlines(d, 0, 1), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.errorbar(d, [0, 1], yerr=0.1), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.stem(d, [0, 1]), ["01-01 00", "01-01 03"]),
    (lambda ax, d: ax.hist(d), ["01-01 00", "01-01 03"]),
])
def test_plotted_dates_give_the_axis_their_zone(plotting, first_labels):
    fig = Figure()
    ax = fig.add_subplot()
    plotting(ax, TOKYO_DAYS)
    fig.draw_without_rendering()
    assert ax.xaxis.get_units() is TOKYO_DAYS[0].tzinfo
    assert [t.get_text() for t in ax.get_xticklabels()][:2] == first_labels
