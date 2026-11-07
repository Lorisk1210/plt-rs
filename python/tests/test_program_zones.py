"""
Time zones pltrs (Rust) has no model of (`pltrs._program_zones`):
``dateutil.tz.tzlocal()``, ``dateutil.tz.tzstr``, ``tzinfo`` classes a
program writes, and the :rc:`timezone` names only dateutil reads. pltrs
(Rust) asks them what it needs, as matplotlib calls their methods, and no
more often. The expected values are real matplotlib 3.11.2's for the same
programs (the programs of the probe area ``dates_tz`` cover the classes a
program writes in depth).
"""

import collections
import datetime
import io
import pickle
import time
import zoneinfo

import dateutil.tz
import numpy as np
import pytest

import pltrs
import pltrs.dates as mdates
import pltrs.pyplot as plt

ZERO = datetime.timedelta(0)
HOUR = datetime.timedelta(hours=1)


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


@pytest.fixture
def chicago(monkeypatch):
    """The machine's local zone is Chicago's while the test runs. Some
    Pythons cannot change it (python-build-standalone's macOS x86_64 builds,
    cross-compiled, have no ``time.tzset``)."""
    if not hasattr(time, "tzset"):
        pytest.skip("this Python cannot change the local time zone (no time.tzset)")
    monkeypatch.setenv("TZ", "America/Chicago")
    time.tzset()
    yield
    monkeypatch.undo()
    time.tzset()


class Fixed(datetime.tzinfo):
    """A fixed offset with a name of its own."""

    def __init__(self, hours, name):
        self._offset = datetime.timedelta(hours=hours)
        self._name = name

    def utcoffset(self, dt):
        return self._offset

    def dst(self, dt):
        return ZERO

    def tzname(self, dt):
        return self._name

    def __getinitargs__(self):
        return self._offset / HOUR, self._name


def _first_sunday_on_or_after(dt):
    return dt + datetime.timedelta(days=6 - dt.weekday())


class Eastern(datetime.tzinfo):
    """The United States' Eastern time since 2007, written in Python
    (Python's documentation's ``USTimeZone``), counting the calls of its
    methods."""

    def __init__(self):
        self.calls = collections.Counter()

    def _range(self, year):
        return (_first_sunday_on_or_after(datetime.datetime(year, 3, 8, 2)),
                _first_sunday_on_or_after(datetime.datetime(year, 11, 1, 2)))

    def tzname(self, dt):
        self.calls["tzname"] += 1
        return "EDT" if self._dst(dt) else "EST"

    def utcoffset(self, dt):
        self.calls["utcoffset"] += 1
        return -5 * HOUR + self._dst(dt)

    def dst(self, dt):
        self.calls["dst"] += 1
        return self._dst(dt)

    def _dst(self, dt):
        if dt is None or dt.tzinfo is None:
            return ZERO
        start, end = self._range(dt.year)
        dt = dt.replace(tzinfo=None)
        if start + HOUR <= dt < end - HOUR:
            return HOUR
        if end - HOUR <= dt < end:
            return ZERO if dt.fold else HOUR
        if start <= dt < start + HOUR:
            return HOUR if dt.fold else ZERO
        return ZERO

    def fromutc(self, dt):
        self.calls["fromutc"] += 1
        start, end = (d.replace(tzinfo=self) for d in self._range(dt.year))
        std_time = dt - 5 * HOUR
        dst_time = std_time + HOUR
        if end <= dst_time < end + HOUR:
            return std_time.replace(fold=1)
        if std_time < start or dst_time >= end:
            return std_time
        return dst_time


def _rounded(values):
    return [round(float(v), 6) for v in values]


def _observed(tz):
    """What matplotlib's dates do in the zone *tz* across the change of the
    clocks of 2021-03-14 in the United States."""
    start = datetime.datetime(2021, 3, 13, 12, tzinfo=tz)
    end = datetime.datetime(2021, 3, 15, 0, tzinfo=tz)
    seen = {
        "date2num": _rounded(mdates.date2num([start, end])),
        "num2date": [(d.isoformat(), d.tzname())
                     for d in mdates.num2date([18700.25, 18701.5], tz=tz)],
        "rule": _rounded(mdates.HourLocator(byhour=range(0, 24, 6), tz=tz)
                         .tick_values(start, end)),
        "auto": _rounded(mdates.AutoDateLocator(tz=tz).tick_values(start, end)),
        "year": _rounded(mdates.YearLocator(tz=tz).tick_values(
            start, start + datetime.timedelta(days=900))),
        "format": mdates.DateFormatter("%Y-%m-%d %H:%M %Z %z", tz=tz)
        .format_ticks([18700.25, 18701.5]),
    }
    fig, ax = plt.subplots()
    ax.plot([start, end], [0, 1])
    fig.canvas.draw()
    locator = ax.xaxis.get_major_locator()
    seen["plot"] = (_rounded(ax.get_xticks()),
                    [t.get_text() for t in ax.get_xticklabels()],
                    ax.xaxis.get_units() is tz, locator.tz is tz, locator._freq)
    concise = mdates.ConciseDateFormatter(locator, tz=tz)
    ax.xaxis.set_major_formatter(concise)
    fig.canvas.draw()
    seen["concise"] = ([t.get_text() for t in ax.get_xticklabels()],
                       concise.get_offset())
    fig, ax = plt.subplots()
    ax.xaxis.set_major_locator(mdates.HourLocator(interval=12, tz=tz))
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%d %H:%M %Z", tz=tz))
    ax.plot([start, end], [0, 1])
    fig.canvas.draw()
    seen["hours"] = [t.get_text() for t in ax.get_xticklabels()]
    return seen


PLOT_LABELS = ['03-13 12', '03-13 16', '03-13 20', '03-14 00', '03-14 04',
               '03-14 08', '03-14 12', '03-14 16', '03-14 20', '03-15 00']
CONCISE = (['12:00', '16:00', '20:00', 'Mar-14', '04:00', '08:00', '12:00', '16:00',
            '20:00', 'Mar-15'], '2021-Mar-15')

EXPECTED = {
    'class': {
        'auto': [18699.708333, 18699.875, 18700.041667, 18700.208333, 18700.333333,
                 18700.5, 18700.666667, 18700.833333, 18701.0, 18701.166667],
        'concise': CONCISE,
        'date2num': [18699.708333, 18701.166667],
        'format': ['2021-03-14 01:00 EST -0500', '2021-03-15 08:00 EDT -0400'],
        'hours': ['13 18:00 EST', '14 06:00 EDT', '14 18:00 EDT'],
        'num2date': [('2021-03-14T01:00:00-05:00', 'EST'),
                     ('2021-03-15T08:00:00-04:00', 'EDT')],
        'plot': ([18699.708333, 18699.875, 18700.041667, 18700.208333, 18700.333333,
                  18700.5, 18700.666667, 18700.833333, 18701.0, 18701.166667],
                 PLOT_LABELS, True, True, 4),
        'rule': [18699.708333, 18699.958333, 18700.208333, 18700.416667, 18700.666667,
                 18700.916667, 18701.166667],
        'year': [18628.208333, 18993.208333, 19358.208333],
    },
    'fixed': {
        'auto': [18699.270833, 18699.4375, 18699.604167, 18699.770833, 18699.9375,
                 18700.104167, 18700.270833, 18700.4375, 18700.604167, 18700.770833],
        'concise': CONCISE,
        'date2num': [18699.270833, 18700.770833],
        'format': ['2021-03-14 11:30 IST! +0530', '2021-03-15 17:30 IST! +0530'],
        'hours': ['13 18:00 IST!', '14 06:00 IST!', '14 18:00 IST!'],
        'num2date': [('2021-03-14T11:30:00+05:30', 'IST!'),
                     ('2021-03-15T17:30:00+05:30', 'IST!')],
        'plot': ([18699.270833, 18699.4375, 18699.604167, 18699.770833, 18699.9375,
                  18700.104167, 18700.270833, 18700.4375, 18700.604167, 18700.770833],
                 PLOT_LABELS, True, True, 4),
        'rule': [18699.270833, 18699.520833, 18699.770833, 18700.020833, 18700.270833,
                 18700.520833, 18700.770833],
        'year': [18627.770833, 18992.770833, 19357.770833],
    },
    'tzlocal': {
        'auto': [18699.75, 18699.916667, 18700.083333, 18700.25, 18700.375,
                 18700.541667, 18700.708333, 18700.875, 18701.041667, 18701.208333],
        'concise': CONCISE,
        'date2num': [18699.75, 18701.208333],
        'format': ['2021-03-14 00:00 CST -0600', '2021-03-15 07:00 CDT -0500'],
        'hours': ['13 18:00 CST', '14 06:00 CDT', '14 18:00 CDT'],
        'num2date': [('2021-03-14T00:00:00-06:00', 'CST'),
                     ('2021-03-15T07:00:00-05:00', 'CDT')],
        'plot': ([18699.75, 18699.916667, 18700.083333, 18700.25, 18700.375,
                  18700.541667, 18700.708333, 18700.875, 18701.041667, 18701.208333],
                 PLOT_LABELS, True, True, 4),
        'rule': [18699.75, 18700.0, 18700.25, 18700.458333, 18700.708333, 18700.958333,
                 18701.208333],
        'year': [18628.25, 18993.25, 19358.25],
    },
    'tzstr': {
        'auto': [18699.708333, 18699.875, 18700.041667, 18700.208333, 18700.375,
                 18700.541667, 18700.708333, 18700.875, 18701.041667, 18701.208333],
        'concise': CONCISE,
        'date2num': [18699.708333, 18701.208333],
        'format': ['2021-03-14 01:00 EST -0500', '2021-03-15 07:00 EST -0500'],
        'hours': ['13 18:00 EST', '14 06:00 EST', '14 18:00 EST'],
        'num2date': [('2021-03-14T01:00:00-05:00', 'EST'),
                     ('2021-03-15T07:00:00-05:00', 'EST')],
        'plot': ([18699.708333, 18699.875, 18700.041667, 18700.208333, 18700.375,
                  18700.541667, 18700.708333, 18700.875, 18701.041667, 18701.208333],
                 PLOT_LABELS, True, True, 4),
        'rule': [18699.708333, 18699.958333, 18700.208333, 18700.458333, 18700.708333,
                 18700.958333, 18701.208333],
        'year': [18628.208333, 18993.208333, 19358.208333],
    },
}


@pytest.mark.parametrize("name", sorted(EXPECTED))
def test_zones_pltrs_has_no_model_of(name, chicago):
    tz = {"tzlocal": dateutil.tz.tzlocal, "tzstr": lambda: dateutil.tz.tzstr("EST5EDT"),
          "fixed": lambda: Fixed(5.5, "IST!"), "class": Eastern}[name]()
    assert _observed(tz) == EXPECTED[name]


def test_a_year_of_hours_asks_the_zone_no_more_than_matplotlib():
    tz = Eastern()
    start = datetime.datetime(2021, 1, 1, tzinfo=tz)
    end = datetime.datetime(2022, 1, 1, tzinfo=tz)
    # matplotlib logs that the ticks exceed Locator.MAXTICKS.
    ticks = mdates.HourLocator(tz=tz).tick_values(start, end)
    assert len(ticks) == 8761
    # matplotlib's date2num asks each tick's offset once.
    assert dict(tz.calls) == {"utcoffset": 8761}
    tz.calls.clear()
    fig, ax = plt.subplots()
    ax.plot([start, end], [0, 1])
    fig.canvas.draw()
    assert len(ax.get_xticklabels()) == 7
    matplotlib = {"fromutc": 48, "utcoffset": 37, "dst": 28}
    assert all(tz.calls[name] <= matplotlib.get(name, 0) for name in tz.calls), tz.calls


def test_the_rc_timezone_is_read_as_dateutil_reads_it():
    with pltrs.rc_context({"timezone": "EST5EDT,M3.2.0,M11.1.0"}):
        fig, ax = plt.subplots()
        ax.plot([datetime.datetime(2021, 3, 13, 12), datetime.datetime(2021, 3, 15)],
                [0, 1])
        fig.canvas.draw()
        assert [t.get_text() for t in ax.get_xticklabels()] == [
            '03-13 08', '03-13 12', '03-13 16', '03-13 20', '03-14 00', '03-14 04',
            '03-14 08', '03-14 12', '03-14 16', '03-14 20']
        assert mdates.num2date(18700.25).isoformat() == '2021-03-14T01:00:00-05:00'
        assert type(mdates.AutoDateLocator().tz) is dateutil.tz.tzstr


@pytest.mark.parametrize("tz", [
    datetime.timezone.utc, zoneinfo.ZoneInfo("Europe/Berlin"),
    dateutil.tz.gettz("Europe/Berlin"), dateutil.tz.tzstr("EST5EDT"),
    Fixed(-3, "M3")], ids=["utc", "zoneinfo", "dateutil", "tzstr", "fixed"])
def test_figures_with_zoned_date_axes_pickle(tz):
    fig, ax = plt.subplots()
    ax.plot([datetime.datetime(2021, 1, 1, tzinfo=tz),
             datetime.datetime(2021, 1, 3, tzinfo=tz)], [0, 1])
    fig.canvas.draw()
    labels = [t.get_text() for t in ax.get_xticklabels()]
    again = pickle.loads(pickle.dumps(fig))
    again.canvas.draw()
    assert [t.get_text() for t in again.axes[0].get_xticklabels()] == labels
    assert type(again.axes[0].xaxis.get_units()) is type(tz)


def test_a_zone_that_raises_fails_the_locator():
    class Broken(datetime.tzinfo):
        def utcoffset(self, dt):
            if dt is not None and dt.day == 14:
                raise RuntimeError("no offset on the 14th")
            return HOUR

        def dst(self, dt):
            return ZERO

        def tzname(self, dt):
            return "BRK"

    tz = Broken()
    with pytest.raises(RuntimeError, match="no offset on the 14th"):
        mdates.HourLocator(byhour=[0, 12], tz=tz).tick_values(
            datetime.datetime(2021, 3, 13, tzinfo=tz),
            datetime.datetime(2021, 3, 15, tzinfo=tz))


def test_an_axis_of_dates_in_a_program_zone():
    fig, ax = plt.subplots()
    ax.xaxis_date(Eastern())
    ax.plot([18700.0, 18702.0], [0, 1])
    fig.canvas.draw()
    assert [t.get_text() for t in ax.get_xticklabels()] == [
        '03-13 18', '03-14 00', '03-14 06', '03-14 12', '03-14 18', '03-15 00',
        '03-15 06', '03-15 12', '03-15 18']
    fig.savefig(io.BytesIO(), format="svg")
