"""
The date tick locators of `.dates`, the recurrence rules they place ticks
with (`rrulewrapper`) and `AutoDateFormatter`.

The classes keep matplotlib's names, parameters and attributes (from
matplotlib 3.11.2, under the Matplotlib License); pltrs (Rust) computes the
occurrences of the rules and the ticks (``pltrs::dates``), where matplotlib
uses ``dateutil``'s ``rrule``: the frequency constants are defined here,
numbered as ``dateutil`` numbers them, and the weekdays ``MO`` ... ``SU``
are ``dateutil``'s, as matplotlib's ``dates`` re-exports them.
"""

import datetime
import functools
import importlib.resources
import math
import os

from dateutil.relativedelta import relativedelta
from dateutil.rrule import MO, TU, WE, TH, FR, SA, SU  # noqa: F401  (`.dates` has them)
import numpy as np

from . import _api, _tick_state, ticker
from ._native import dates as _dates
from ._program_zones import ProgramZone

# The frequencies of recurrence rules, numbered as dateutil numbers them.
YEARLY, MONTHLY, WEEKLY, DAILY, HOURLY, MINUTELY, SECONDLY = range(7)
MICROSECONDLY = SECONDLY + 1

# The units of time `.dates` counts in.
HOURS_PER_DAY = 24.
MIN_PER_HOUR = 60.
SEC_PER_MIN = 60.
MONTHS_PER_YEAR = 12.

DAYS_PER_WEEK = 7.
DAYS_PER_MONTH = 30.
DAYS_PER_YEAR = 365.0

MINUTES_PER_DAY = MIN_PER_HOUR * HOURS_PER_DAY

SEC_PER_HOUR = SEC_PER_MIN * MIN_PER_HOUR
SEC_PER_DAY = SEC_PER_HOUR * HOURS_PER_DAY
SEC_PER_WEEK = SEC_PER_DAY * DAYS_PER_WEEK

MUSECONDS_PER_DAY = 1e6 * SEC_PER_DAY


def _get_tzinfo(tz):
    """`.dates._get_tzinfo` (imported when first needed: `.dates` imports
    this module)."""
    from .dates import _get_tzinfo
    return _get_tzinfo(tz)


def _date2num(d):
    """`.dates.date2num`."""
    from .dates import date2num
    return date2num(d)


def _num2date(x, tz):
    """`.dates.num2date`."""
    from .dates import num2date
    return num2date(x, tz)


#: The classes of fixed offsets from UTC, by module and name, which pltrs
#: (Rust) models as long as they name themselves as `datetime.timezone`
#: names an offset.
_FIXED_OFFSETS = {('datetime', 'timezone'), ('dateutil.tz.tz', 'tzutc'),
                  ('dateutil.tz.tz', 'tzoffset')}


def _native_tz(tz):
    """
    The time zone *tz* as pltrs (Rust) takes it: the IANA name of a zone
    of `zoneinfo` (with the data it reads, as a pair), of ``pytz`` or of
    ``dateutil``'s files; the offset from UTC in seconds of a fixed one;
    else the `._program_zones.ProgramZone` pltrs (Rust) asks it through.
    """
    if tz is datetime.timezone.utc:
        return 'UTC'
    kind = (type(tz).__module__, type(tz).__qualname__)
    key = getattr(tz, 'key', None)
    if kind == ('zoneinfo', 'ZoneInfo') and isinstance(key, str):
        data = _zoneinfo_data(key)
        return key if data is None else (key, data)
    zone = getattr(tz, 'zone', None)
    if kind[0].partition('.')[0] == 'pytz' and isinstance(zone, str):
        return zone
    filename = getattr(tz, '_filename', None)
    if kind == ('dateutil.tz.tz', 'tzfile') and isinstance(filename, str):
        return filename
    if kind in _FIXED_OFFSETS:
        offset = tz.utcoffset(None)
        if (offset % datetime.timedelta(seconds=1) == datetime.timedelta(0)
                and tz.tzname(None) == datetime.timezone(offset).tzname(None)):
            return int(offset.total_seconds())
    return ProgramZone(tz)


@functools.cache
def _zoneinfo_data(key):
    """
    The ``TZif`` data `zoneinfo` reads for the zone *key*: the first file
    of `zoneinfo.TZPATH`, else the `tzdata` package's (where a system's
    database lacks old names such as ``US/Pacific``), None for neither.
    """
    # Imported here, as matplotlib's pyplot leaves zoneinfo unimported
    # (finding its search path reads the interpreter's build settings).
    import zoneinfo
    for directory in zoneinfo.TZPATH:
        path = os.path.join(directory, key)
        if os.path.isfile(path):
            with open(path, 'rb') as file:
                return file.read()
    *packages, resource = key.split('/')
    try:
        return importlib.resources.files(
            '.'.join(['tzdata.zoneinfo', *packages])).joinpath(resource).read_bytes()
    except (ImportError, OSError):
        return None


def _parts(dt):
    """The calendar parts of a naive `datetime.datetime`."""
    return ((dt.year, dt.month, dt.day),
            (dt.hour, dt.minute, dt.second, dt.microsecond))


def _zoned(dt):
    """
    The date-time *dt* as pltrs (Rust) takes it: its calendar parts, its
    zone (None when naive) and its fold; a `datetime.date` is its
    midnight, as ``relativedelta`` reads it.
    """
    if not isinstance(dt, datetime.datetime):
        dt = datetime.datetime.combine(dt, datetime.time())
    tz = dt.tzinfo
    return _parts(dt), None if tz is None else _native_tz(tz), bool(dt.fold)


def _from_parts(parts):
    """The naive `datetime.datetime` of calendar parts."""
    (year, month, day), (hour, minute, second, microsecond) = parts
    return datetime.datetime(year, month, day, hour, minute, second,
                             microsecond)


def _int_list(values):
    """A ``by*`` keyword of a rule as a list of ints (dateutil takes one
    int or several), None when not given."""
    if values is None:
        return None
    if isinstance(values, (int, np.integer)):
        return [int(values)]
    return [int(value) for value in values]


def _weekdays(values):
    """``byweekday`` as ``(index, n)`` pairs: ints or weekday objects."""
    if values is None:
        return None
    if isinstance(values, (int, np.integer)) or hasattr(values, 'weekday'):
        values = [values]
    return [(int(value), None) if isinstance(value, (int, np.integer))
            else (value.weekday, value.n) for value in values]


#: The keywords of a dateutil ``rrule`` pltrs (Rust) takes.
_RULE_KEYWORDS = {'freq', 'dtstart', 'interval', 'wkst', 'count', 'until',
                  'bysetpos', 'bymonth', 'bymonthday', 'byyearday',
                  'byweekno', 'byweekday', 'byhour', 'byminute', 'bysecond',
                  'byeaster', 'cache'}


#: The rule keyword dateutil reduces with the interval, and its base, per
#: frequency.
_GCD_BY = {HOURLY: ('byhour', 24), MINUTELY: ('byminute', 60),
           SECONDLY: ('bysecond', 60)}


def _check_rule(construct):
    """Fail as dateutil's ``rrule`` fails to be made of the keywords
    *construct* (the values it keeps fail where they are used)."""
    unknown = set(construct) - _RULE_KEYWORDS
    if unknown:
        raise TypeError(f"rrule got an unexpected keyword argument "
                        f"{unknown.pop()!r}")
    by = _GCD_BY.get(construct['freq'])
    if by is not None and construct.get(by[0]) is not None:
        # dateutil reduces the hours (minutes, seconds) of a rule repeating
        # every so many of them with the gcd of the interval and the base,
        # which takes integers only.
        math.gcd(construct.get('interval', 1), by[1])


def _rule(construct, started):
    """The native description of the rule of dateutil's keywords, starting
    at *started* when they give no start."""
    dtstart = construct.get('dtstart')
    if dtstart is None:
        dtstart = started
    until = construct.get('until')
    wkst = construct.get('wkst')
    return {
        'freq': int(construct['freq']),
        'interval': int(construct.get('interval', 1)),
        'dtstart': _parts(dtstart),
        'until': None if until is None else _parts(until),
        'count': construct.get('count'),
        'wkst': 0 if wkst is None else int(getattr(wkst, 'weekday', wkst)),
        'byweekday': _weekdays(construct.get('byweekday')),
        **{key: _int_list(construct.get(key))
           for key in ('bysetpos', 'bymonth', 'bymonthday', 'byyearday',
                       'byweekno', 'byhour', 'byminute', 'bysecond',
                       'byeaster')},
    }


def _refuse_non_dates(dmin, dmax):
    """Fail as matplotlib's rule locators fail on what is no date-time: on
    the first step they compute, the calendar difference of *dmin* and
    *dmax* (``relativedelta(dmax, dmin)``, which also refuses a naive and
    an aware one)."""
    relativedelta(dmax, dmin)


def _zoned_datetime(dt):
    """`_zoned` of what a rule starts or stops at: a `datetime.datetime`
    (a `datetime.date` has no ``tzinfo`` for matplotlib's
    `rrulewrapper`)."""
    dt.tzinfo  # noqa: B018  (raises for dates, as the rule's start does)
    return _zoned(dt)


class rrulewrapper:
    """
    A simple wrapper around a `dateutil.rrule` allowing flexible
    date tick specifications.
    """
    def __init__(self, freq, tzinfo=None, **kwargs):
        """
        Parameters
        ----------
        freq : {YEARLY, MONTHLY, WEEKLY, DAILY, HOURLY, MINUTELY, SECONDLY}
            Tick frequency. These constants are defined in `dateutil.rrule`,
            but they are accessible from `matplotlib.dates` as well.
        tzinfo : `datetime.tzinfo`, optional
            Time zone information. The default is None.
        **kwargs
            Additional keyword arguments are passed to the `dateutil.rrule`.
        """
        kwargs['freq'] = freq
        self._base_tzinfo = tzinfo

        self._update_rrule(**kwargs)

    def set(self, **kwargs):
        """Set parameters for an existing wrapper."""
        self._construct.update(kwargs)

        self._update_rrule(**self._construct)

    def _update_rrule(self, **kwargs):
        tzinfo = self._base_tzinfo

        # rrule does not play nicely with timezones - especially pytz time
        # zones, it's best to use naive zones and attach timezones once the
        # datetimes are returned
        if 'dtstart' in kwargs:
            dtstart = kwargs['dtstart']
            if dtstart.tzinfo is not None:
                if tzinfo is None:
                    tzinfo = dtstart.tzinfo
                else:
                    dtstart = dtstart.astimezone(tzinfo)

                kwargs['dtstart'] = dtstart.replace(tzinfo=None)

        if 'until' in kwargs:
            until = kwargs['until']
            if until.tzinfo is not None:
                if tzinfo is not None:
                    until = until.astimezone(tzinfo)
                else:
                    raise ValueError('until cannot be aware if dtstart '
                                     'is naive and tzinfo is None')

                kwargs['until'] = until.replace(tzinfo=None)

        self._construct = kwargs.copy()
        self._tzinfo = tzinfo
        _check_rule(self._construct)
        # dateutil starts a rule without a start when it is made, to the
        # second.
        self._started = datetime.datetime.now().replace(microsecond=0)
        self.__dict__.pop('_native_rule', None)
        self.__dict__.pop('_dateutil_rule', None)

    @property
    def _rule(self):
        """The rule as pltrs (Rust) computes its occurrences, made when
        first needed (dateutil keeps the values it is given, failing where
        it uses them)."""
        rule = self.__dict__.get('_native_rule')
        if rule is None:
            rule = self.__dict__['_native_rule'] = _rule(self._construct, self._started)
        return rule

    @property
    def _rrule(self):
        """
        The `dateutil.rrule.rrule` of the parameters: matplotlib's wrapper
        holds one and hands it what the wrapper itself lacks (pltrs (Rust)
        computes the occurrences the locators ask for).
        """
        rule = self.__dict__.get('_dateutil_rule')
        if rule is None:
            from dateutil.rrule import rrule
            rule = self.__dict__['_dateutil_rule'] = rrule(
                **{'dtstart': self._started, **self._construct})
        return rule

    def __getattr__(self, name):
        if '_construct' not in self.__dict__:
            raise AttributeError(
                f"{type(self).__name__!r} object has no attribute {name!r}")
        f = getattr(self._rrule, name)
        if name in {'xafter', 'xbefore'} and self._tzinfo is not None:
            @functools.wraps(f)
            def aware(*args, **kwargs):
                args = tuple(self._naive(arg) for arg in args)
                kwargs = {kw: self._naive(arg) for kw, arg in kwargs.items()}
                return [self._attach_tzinfo(dt, self._tzinfo)
                        for dt in f(*args, **kwargs)]
            return aware
        return f

    def __setstate__(self, state):
        self.__dict__.update(state)

    @property
    def _freq(self):
        """The frequency (dateutil's ``rrule._freq``)."""
        return self._construct['freq']

    @property
    def _interval(self):
        """The interval (dateutil's ``rrule._interval``)."""
        return self._construct.get('interval', 1)

    def _attach_tzinfo(self, dt, tzinfo):
        # pytz zones are attached by "localizing" the datetime
        if hasattr(tzinfo, 'localize'):
            return tzinfo.localize(dt, is_dst=True)

        return dt.replace(tzinfo=tzinfo)

    def _naive(self, arg):
        """A date-time argument in the rule's zone, without the zone."""
        if isinstance(arg, datetime.datetime) and arg.tzinfo is not None:
            if self._tzinfo is None:
                # A naive rule compares its occurrences with the argument.
                raise TypeError(
                    "can't compare offset-naive and offset-aware datetimes")
            if arg.tzinfo is not self._tzinfo:
                arg = arg.astimezone(self._tzinfo)
            return arg.replace(tzinfo=None)
        return arg

    def _aware(self, parts):
        """An occurrence, with the rule's zone attached when it has one."""
        dt = _from_parts(parts)
        return dt if self._tzinfo is None else self._attach_tzinfo(
            dt, self._tzinfo)

    def between(self, after, before, inc=False):
        """
        The occurrences between *after* and *before*, including them when
        *inc* is true.
        """
        found = _dates.rrule_between(self._rule, _parts(self._naive(after)),
                                     _parts(self._naive(before)), bool(inc))
        return [self._aware(parts) for parts in found]

    def after(self, dt, inc=False):
        """The first occurrence after *dt* (or at it, if *inc*), or None."""
        found = _dates.rrule_after(self._rule, _parts(self._naive(dt)),
                                   bool(inc))
        return None if found is None else self._aware(found)

    def before(self, dt, inc=False):
        """The last occurrence before *dt* (or at it, if *inc*), or None."""
        found = _dates.rrule_before(self._rule, _parts(self._naive(dt)),
                                    bool(inc))
        return None if found is None else self._aware(found)


class DateLocator(ticker.Locator):
    """
    Determines the tick locations when plotting dates.

    This class is subclassed by other Locators and
    is not meant to be used on its own.
    """
    hms0d = {'byhour': 0, 'byminute': 0, 'bysecond': 0}

    def __init__(self, tz=None):
        """
        Parameters
        ----------
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        self.tz = _get_tzinfo(tz)

    def set_tzinfo(self, tz):
        """
        Set timezone info.

        Parameters
        ----------
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        self.tz = _get_tzinfo(tz)

    def datalim_to_dt(self):
        """Convert axis data interval to datetime objects."""
        dmin, dmax = self.axis.get_data_interval()
        if dmin > dmax:
            dmin, dmax = dmax, dmin

        return _num2date(dmin, self.tz), _num2date(dmax, self.tz)

    def viewlim_to_dt(self):
        """Convert the view interval to datetime objects."""
        vmin, vmax = self.axis.get_view_interval()
        if vmin > vmax:
            vmin, vmax = vmax, vmin
        return _num2date(vmin, self.tz), _num2date(vmax, self.tz)

    def _get_unit(self):
        """
        Return how many days a unit of the locator is; used for
        intelligent autoscaling.
        """
        return 1

    def _get_interval(self):
        """
        Return the number of units for each tick.
        """
        return 1

    def nonsingular(self, vmin, vmax):
        """
        Given the proposed upper and lower extent, adjust the range
        if it is too close to being singular (i.e. a range of ~0).
        """
        if not np.isfinite(vmin) or not np.isfinite(vmax):
            # Except if there is no data, then use 1970 as default.
            return (_date2num(datetime.date(1970, 1, 1)),
                    _date2num(datetime.date(1970, 1, 2)))
        if vmax < vmin:
            vmin, vmax = vmax, vmin
        unit = self._get_unit()
        interval = self._get_interval()
        if abs(vmax - vmin) < 1e-6:
            vmin -= 2 * unit * interval
            vmax += 2 * unit * interval
        return vmin, vmax


class RRuleLocator(DateLocator):
    def __init__(self, o, tz=None):
        super().__init__(tz)
        self.rule = o

    def __call__(self):
        # if no data have been set, this will tank with a ValueError
        try:
            dmin, dmax = self.viewlim_to_dt()
        except ValueError:
            return []

        return self.tick_values(dmin, dmax)

    def _spec(self):
        """The native description of the locator."""
        rule_tz = self.rule._base_tzinfo
        return {"kind": "date_rrule", "rule": self.rule._rule,
                "tz": _native_tz(self.tz),
                "rule_tz": None if rule_tz is None else _native_tz(rule_tz)}

    def tick_values(self, vmin, vmax):
        # The rule runs on the wall clock of the dates' own zone (naive
        # ones: naive), not of the locator's, as matplotlib's does.
        self._create_rrule(vmin, vmax)
        return self._ticks_between(vmin, vmax)

    def _create_rrule(self, vmin, vmax):
        # set appropriate rrule dtstart and until and return
        # start and end
        delta = relativedelta(vmax, vmin)

        # We need to cap at the endpoints of valid datetime
        try:
            start = vmin - delta
        except (ValueError, OverflowError):
            # cap
            start = datetime.datetime(1, 1, 1, 0, 0, 0,
                                      tzinfo=datetime.timezone.utc)

        try:
            stop = vmax + delta
        except (ValueError, OverflowError):
            # cap
            stop = datetime.datetime(9999, 12, 31, 23, 59, 59,
                                     tzinfo=datetime.timezone.utc)

        self.rule.set(dtstart=start, until=stop)

        return vmin, vmax

    def _ticks_between(self, vmin, vmax):
        """The ticks of the rule between the date-times *vmin* and *vmax*,
        else the two (matplotlib's ``tick_values`` once the dates are
        known to be some)."""
        ticks = _dates.rule_tick_values(self._spec(), _zoned_datetime(vmin),
                                        _zoned_datetime(vmax))
        if ticks is None:
            return _date2num([vmin, vmax])
        return self.raise_if_exceeds(ticks)

    def _get_unit(self):
        # docstring inherited
        return self.get_unit_generic(self.rule._freq)

    @staticmethod
    def get_unit_generic(freq):
        if freq == YEARLY:
            return DAYS_PER_YEAR
        elif freq == MONTHLY:
            return DAYS_PER_MONTH
        elif freq == WEEKLY:
            return DAYS_PER_WEEK
        elif freq == DAILY:
            return 1.0
        elif freq == HOURLY:
            return 1.0 / HOURS_PER_DAY
        elif freq == MINUTELY:
            return 1.0 / MINUTES_PER_DAY
        elif freq == SECONDLY:
            return 1.0 / SEC_PER_DAY
        else:
            # error
            return -1  # or should this just return '1'?

    def _get_interval(self):
        return self.rule._interval


class AutoDateLocator(DateLocator):
    """
    On autoscale, this class picks the best `DateLocator` to set the view
    limits and the tick locations.

    Attributes
    ----------
    intervald : dict

        Mapping of tick frequencies to multiples allowed for that ticking.
        The default is ::

            self.intervald = {
                YEARLY  : [1, 2, 4, 5, 10, 20, 40, 50, 100, 200, 400, 500,
                           1000, 2000, 4000, 5000, 10000],
                MONTHLY : [1, 2, 3, 4, 6],
                DAILY   : [1, 2, 3, 7, 14, 21],
                HOURLY  : [1, 2, 3, 4, 6, 12],
                MINUTELY: [1, 5, 10, 15, 30],
                SECONDLY: [1, 5, 10, 15, 30],
                MICROSECONDLY: [1, 2, 5, 10, 20, 50, 100, 200, 500,
                                1000, 2000, 5000, 10000, 20000, 50000,
                                100000, 200000, 500000, 1000000],
            }

        where the keys are defined in `dateutil.rrule`.

        The interval is used to specify multiples that are appropriate for
        the frequency of ticking. For instance, every 7 days is sensible
        for daily ticks, but for minutes/seconds, 15 or 30 make sense.

        When customizing, you should only modify the values for the existing
        keys. You should not add or delete entries.

        Example for forcing ticks every 3 hours::

            locator = AutoDateLocator()
            locator.intervald[HOURLY] = [3]  # only show every 3 hours
    """

    # The frequency the locator picked last, which a locator pltrs (Rust)
    # ticks an axis with natively picks again from the axis' ticks.
    _freq = _tick_state.ticked(YEARLY)

    def __init__(self, tz=None, minticks=5, maxticks=None,
                 interval_multiples=True):
        """
        Parameters
        ----------
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        minticks : int
            The minimum number of ticks desired; controls whether ticks occur
            yearly, monthly, etc.
        maxticks : int
            The maximum number of ticks desired; controls the interval between
            ticks (ticking every other, every 3, etc.).  For fine-grained
            control, this can be a dictionary mapping individual rrule
            frequency constants (YEARLY, MONTHLY, etc.) to their own maximum
            number of ticks.  This can be used to keep the number of ticks
            appropriate to the format chosen in `AutoDateFormatter`. Any
            frequency not specified in this dictionary is given a default
            value.
        interval_multiples : bool, default: True
            Whether ticks should be chosen to be multiple of the interval,
            locking them to 'nicer' locations.  For example, this will force
            the ticks to be at hours 0, 6, 12, 18 when hourly ticking is done
            at 6 hour intervals.
        """
        super().__init__(tz=tz)
        self._freq = YEARLY
        self._freqs = [YEARLY, MONTHLY, DAILY, HOURLY, MINUTELY,
                       SECONDLY, MICROSECONDLY]
        self.minticks = minticks

        self.maxticks = {YEARLY: 11, MONTHLY: 12, DAILY: 11, HOURLY: 12,
                         MINUTELY: 11, SECONDLY: 11, MICROSECONDLY: 8}
        if maxticks is not None:
            try:
                self.maxticks.update(maxticks)
            except TypeError:
                # Assume we were given an integer. Use this as the maximum
                # number of ticks for every frequency and create a
                # dictionary for this
                self.maxticks = dict.fromkeys(self._freqs, maxticks)
        self.interval_multiples = interval_multiples
        self.intervald = {
            YEARLY:   [1, 2, 4, 5, 10, 20, 40, 50, 100, 200, 400, 500,
                       1000, 2000, 4000, 5000, 10000],
            MONTHLY:  [1, 2, 3, 4, 6],
            DAILY:    [1, 2, 3, 7, 14, 21],
            HOURLY:   [1, 2, 3, 4, 6, 12],
            MINUTELY: [1, 5, 10, 15, 30],
            SECONDLY: [1, 5, 10, 15, 30],
            MICROSECONDLY: [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000,
                            5000, 10000, 20000, 50000, 100000, 200000, 500000,
                            1000000],
                            }
        if interval_multiples:
            # Swap "3" for "4" in the DAILY list; If we use 3 we get bad
            # tick loc for months w/ 31 days: 1, 4, ..., 28, 31, 1
            # If we use 4 then we get: 1, 5, ... 25, 29, 1
            self.intervald[DAILY] = [1, 2, 4, 7, 14]

    def __call__(self):
        # docstring inherited
        dmin, dmax = self.viewlim_to_dt()
        locator = self.get_locator(dmin, dmax)
        return locator()

    def tick_values(self, vmin, vmax):
        return self.get_locator(vmin, vmax).tick_values(vmin, vmax)

    def nonsingular(self, vmin, vmax):
        # whatever is thrown at us, we can scale the unit.
        # But default nonsingular date plots at an ~4 year period.
        if not np.isfinite(vmin) or not np.isfinite(vmax):
            # Except if there is no data, then use 1970 as default.
            return (_date2num(datetime.date(1970, 1, 1)),
                    _date2num(datetime.date(1970, 1, 2)))
        if vmax < vmin:
            vmin, vmax = vmax, vmin
        if vmin == vmax:
            vmin = vmin - DAYS_PER_YEAR * 2
            vmax = vmax + DAYS_PER_YEAR * 2
        return vmin, vmax

    def _get_unit(self):
        if self._freq in [MICROSECONDLY]:
            return 1. / MUSECONDS_PER_DAY
        else:
            return RRuleLocator.get_unit_generic(self._freq)

    def _spec(self):
        """The native description of the locator's parameters."""
        return {
            "tz": _native_tz(self.tz),
            "minticks": int(self.minticks),
            "maxticks": [(int(freq), int(count))
                         for freq, count in self.maxticks.items()],
            "intervald": [(int(freq), [int(i) for i in intervals])
                          for freq, intervals in self.intervald.items()],
            "interval_multiples": bool(self.interval_multiples),
        }

    def get_locator(self, dmin, dmax):
        """Pick the best locator based on a distance."""
        _refuse_non_dates(dmin, dmax)
        choice = _dates.auto_date_choice(self._spec(), _zoned(dmin), _zoned(dmax))
        if choice['frequency'] is not None:
            self._freq = choice['frequency']
        if choice['fallback_interval'] is not None:
            _api.warn_external(
                f"AutoDateLocator was unable to pick an appropriate "
                f"interval for this date range. It may be necessary "
                f"to add an interval value to the AutoDateLocator's "
                f"intervald dictionary. Defaulting to "
                f"{choice['fallback_interval']}.")
        if choice['kind'] == 'rrule':
            rule = choice['rule']
            rrule = rrulewrapper(
                rule['freq'], interval=rule['interval'], dtstart=dmin,
                until=dmax, **{key: rule[key] for key in (
                    'bymonth', 'bymonthday', 'byhour', 'byminute',
                    'bysecond')})
            locator = RRuleLocator(rrule, tz=self.tz)
        elif choice['kind'] == 'year':
            locator = YearLocator(choice['base'], month=choice['month'],
                                  day=choice['day'], tz=self.tz)
        else:
            locator = MicrosecondLocator(choice['interval'], tz=self.tz)
            if _date2num(dmin) > 70 * 365 and choice['interval'] < 1000:
                from .dates import get_epoch
                _api.warn_external(
                    'Plotting microsecond time intervals for dates far from '
                    f'the epoch (time origin: {get_epoch()}) is not well-'
                    'supported. See matplotlib.dates.set_epoch to change the '
                    'epoch.')
        locator.set_axis(self.axis)
        return locator


class YearLocator(RRuleLocator):
    """
    Make ticks on a given day of each year that is a multiple of base.

    Examples::

      # Tick every year on Jan 1st
      locator = YearLocator()

      # Tick every 5 years on July 4th
      locator = YearLocator(5, month=7, day=4)
    """
    def __init__(self, base=1, month=1, day=1, tz=None):
        """
        Parameters
        ----------
        base : int, default: 1
            Mark ticks every *base* years.
        month : int, default: 1
            The month on which to place the ticks, starting from 1. Default is
            January.
        day : int, default: 1
            The day on which to place the ticks.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        rule = rrulewrapper(YEARLY, interval=base, bymonth=month,
                            bymonthday=day, **self.hms0d)
        super().__init__(rule, tz=tz)
        self.base = ticker._Edge_integer(base, 0)

    def _spec(self):
        c = self.rule._construct
        return {"kind": "date_year", "base": int(self.base.step),
                "month": int(c.get('bymonth', 1)),
                "day": int(c.get('bymonthday', 1)), "tz": _native_tz(self.tz)}

    def _create_rrule(self, vmin, vmax):
        # 'start' needs to be a multiple of the interval to create ticks on
        # interval multiples when the tick frequency is YEARLY
        ymin = max(self.base.le(vmin.year) * self.base.step, 1)
        ymax = min(self.base.ge(vmax.year) * self.base.step, 9999)

        c = self.rule._construct
        replace = {'year': ymin,
                   'month': c.get('bymonth', 1),
                   'day': c.get('bymonthday', 1),
                   'hour': 0, 'minute': 0, 'second': 0}

        start = vmin.replace(**replace)
        stop = start.replace(year=ymax)
        self.rule.set(dtstart=start, until=stop)

        return start, stop


class MonthLocator(RRuleLocator):
    """
    Make ticks on occurrences of each month, e.g., 1, 3, 12.
    """
    def __init__(self, bymonth=None, bymonthday=1, interval=1, tz=None):
        """
        Parameters
        ----------
        bymonth : int or list of int, default: all months
            Ticks will be placed on every month in *bymonth*. Default is
            ``range(1, 13)``, i.e. every month.
        bymonthday : int, default: 1
            The day on which to place the ticks.
        interval : int, default: 1
            The interval between each iteration. For example, if
            ``interval=2``, mark every second occurrence.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        if bymonth is None:
            bymonth = range(1, 13)

        rule = rrulewrapper(MONTHLY, bymonth=bymonth, bymonthday=bymonthday,
                            interval=interval, **self.hms0d)
        super().__init__(rule, tz=tz)


class WeekdayLocator(RRuleLocator):
    """
    Make ticks on occurrences of each weekday.
    """

    def __init__(self, byweekday=1, interval=1, tz=None):
        """
        Parameters
        ----------
        byweekday : int, list of int, constant from :mod:`dateutil.rrule`, or \
            list of constants, default: 1 (Tuesday)
            Ticks will be placed on every weekday in *byweekday*. Default is
            every Tuesday.

            Elements of *byweekday* (if a sequence) must be either integers or
            MO, TU, WE, TH, FR, SA, SU, the constants from
            :mod:`dateutil.rrule`, which have been imported into the
            :mod:`matplotlib.dates` namespace.
        interval : int, default: 1
            The interval between each iteration. For example, if
            ``interval=2``, mark every second occurrence.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        rule = rrulewrapper(DAILY, byweekday=byweekday,
                            interval=interval, **self.hms0d)
        super().__init__(rule, tz=tz)


class DayLocator(RRuleLocator):
    """
    Make ticks on occurrences of each day of the month.  For example,
    1, 15, 30.
    """
    def __init__(self, bymonthday=None, interval=1, tz=None):
        """
        Parameters
        ----------
        bymonthday : int or list of int, default: all days
            Ticks will be placed on every day in *bymonthday*. Default is
            ``bymonthday=range(1, 32)``, i.e., every day of the month.
        interval : int, default: 1
            The interval between each iteration. For example, if
            ``interval=2``, mark every second occurrence.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        if interval != int(interval) or interval < 1:
            raise ValueError("interval must be an integer greater than 0")
        if bymonthday is None:
            bymonthday = range(1, 32)

        rule = rrulewrapper(DAILY, bymonthday=bymonthday,
                            interval=interval, **self.hms0d)
        super().__init__(rule, tz=tz)


class HourLocator(RRuleLocator):
    """
    Make ticks on occurrences of each hour.
    """
    def __init__(self, byhour=None, interval=1, tz=None):
        """
        Parameters
        ----------
        byhour : int or list of int, default: all hours
            Ticks will be placed on every hour in *byhour*. Default is
            ``byhour=range(24)``, i.e., every hour.
        interval : int, default: 1
            The interval between each iteration. For example, if
            ``interval=2``, mark every second occurrence.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        if byhour is None:
            byhour = range(24)

        rule = rrulewrapper(HOURLY, byhour=byhour, interval=interval,
                            byminute=0, bysecond=0)
        super().__init__(rule, tz=tz)


class MinuteLocator(RRuleLocator):
    """
    Make ticks on occurrences of each minute.
    """
    def __init__(self, byminute=None, interval=1, tz=None):
        """
        Parameters
        ----------
        byminute : int or list of int, default: all minutes
            Ticks will be placed on every minute in *byminute*. Default is
            ``byminute=range(60)``, i.e., every minute.
        interval : int, default: 1
            The interval between each iteration. For example, if
            ``interval=2``, mark every second occurrence.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        if byminute is None:
            byminute = range(60)

        rule = rrulewrapper(MINUTELY, byminute=byminute, interval=interval,
                            bysecond=0)
        super().__init__(rule, tz=tz)


class SecondLocator(RRuleLocator):
    """
    Make ticks on occurrences of each second.
    """
    def __init__(self, bysecond=None, interval=1, tz=None):
        """
        Parameters
        ----------
        bysecond : int or list of int, default: all seconds
            Ticks will be placed on every second in *bysecond*. Default is
            ``bysecond = range(60)``, i.e., every second.
        interval : int, default: 1
            The interval between each iteration. For example, if
            ``interval=2``, mark every second occurrence.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        if bysecond is None:
            bysecond = range(60)

        rule = rrulewrapper(SECONDLY, bysecond=bysecond, interval=interval)
        super().__init__(rule, tz=tz)


class MicrosecondLocator(DateLocator):
    """
    Make ticks on regular intervals of one or more microsecond(s).

    .. note::

        By default, Matplotlib uses a floating point representation of time in
        days since the epoch, so plotting data with
        microsecond time resolution does not work well for
        dates that are far (about 70 years) from the epoch (check with
        `~.dates.get_epoch`).

        If you want sub-microsecond resolution time plots, it is strongly
        recommended to use floating point seconds, not datetime-like
        time representation.

        If you really must use datetime.datetime() or similar and still
        need microsecond precision, change the time origin via
        `.dates.set_epoch` to something closer to the dates being plotted.
        See :doc:`/gallery/ticks/date_precision_and_epochs`.

    """
    def __init__(self, interval=1, tz=None):
        """
        Parameters
        ----------
        interval : int, default: 1
            The interval between each iteration. For example, if
            ``interval=2``, mark every second occurrence.
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        """
        super().__init__(tz=tz)
        self._interval = interval
        self._wrapped_locator = ticker.MultipleLocator(interval)

    def set_axis(self, axis):
        self._wrapped_locator.set_axis(axis)
        return super().set_axis(axis)

    def __call__(self):
        # if no data have been set, this will tank with a ValueError
        try:
            dmin, dmax = self.viewlim_to_dt()
        except ValueError:
            return []

        return self.tick_values(dmin, dmax)

    def tick_values(self, vmin, vmax):
        nmin, nmax = _date2num((vmin, vmax))
        t0 = np.floor(nmin)
        nmax = nmax - t0
        nmin = nmin - t0
        nmin *= MUSECONDS_PER_DAY
        nmax *= MUSECONDS_PER_DAY

        ticks = self._wrapped_locator.tick_values(nmin, nmax)

        ticks = ticks / MUSECONDS_PER_DAY + t0
        return ticks

    def _get_unit(self):
        # docstring inherited
        return 1. / MUSECONDS_PER_DAY

    def _get_interval(self):
        # docstring inherited
        return self._interval
