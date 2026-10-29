"""
Matplotlib provides sophisticated date plotting capabilities, standing on the
shoulders of python :mod:`datetime` (matplotlib's ``dates``).

Matplotlib represents dates using floating point numbers specifying the number
of days since a default epoch of 1970-01-01 UTC; for example,
1970-01-01, 06:00 is the floating point number 0.25. The epoch can be changed
with `set_epoch` or :rc:`date.epoch`.

pltrs (Rust) numbers the dates and converts the numbers back (``pltrs::dates``);
Python's `datetime` objects and numpy's ``datetime64`` arrays come in and
out here, with their time zones.
"""

import datetime

import numpy as np

from . import cbook, ticker, units
from ._native import dates as _dates

__all__ = ('datestr2num', 'date2num', 'num2date', 'num2timedelta', 'drange',
           'set_epoch', 'get_epoch', 'DateFormatter', 'DateConverter', 'UTC')

UTC = datetime.timezone.utc

MICROSECONDLY = 7
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


def _rc(key):
    """The rcParam *key* (imported when first needed)."""
    from . import rcParams
    return rcParams[key]


def _get_tzinfo(tz=None):
    """
    Generate `~datetime.tzinfo` from a string or return `~datetime.tzinfo`.
    If None, retrieve the preferred timezone from the rcParams dictionary.
    """
    tz = _rc('timezone') if tz is None else tz
    if isinstance(tz, str):
        if tz == 'UTC':
            return UTC
        import zoneinfo
        try:
            return zoneinfo.ZoneInfo(tz)
        except (zoneinfo.ZoneInfoNotFoundError, ValueError) as err:
            raise ValueError(f"{tz} is not a valid timezone as parsed by"
                             " zoneinfo.ZoneInfo") from err
    if isinstance(tz, datetime.tzinfo):
        return tz
    raise TypeError(f"tz must be string or tzinfo subclass, not {tz!r}.")


def _reset_epoch_test_example():
    """
    Reset the Matplotlib date epoch so it can be set again.

    Only for use in tests and examples.
    """
    _dates.reset_epoch()


def set_epoch(epoch):
    """
    Set the epoch (origin for dates) for datetime calculations.

    The default epoch is :rc:`date.epoch`.

    If microsecond accuracy is desired, the date being plotted needs to be
    within approximately 70 years of the epoch. Matplotlib internally
    represents dates as days since the epoch, so floating point dynamic
    range needs to be within a factor of 2^52.

    `~.dates.set_epoch` must be called before any dates are converted
    (i.e. near the import section) or a RuntimeError will be raised.

    Parameters
    ----------
    epoch : str
        valid UTC date parsable by `numpy.datetime64` (do not include
        timezone).
    """
    _dates.set_epoch(str(epoch))


def get_epoch():
    """
    Get the epoch used by `.dates`.

    Returns
    -------
    epoch : str
        String for the epoch (parsable by `numpy.datetime64`).
    """
    return _dates.get_epoch()


def _to_datetime64(d):
    """The dates *d* (datetimes, dates, ``datetime64``) as an array of
    ``datetime64[us]`` in UTC."""
    d = np.asarray(d)
    if not np.issubdtype(d.dtype, np.datetime64):
        flat = [x.astimezone(UTC).replace(tzinfo=None)
                if isinstance(x, datetime.datetime) and x.tzinfo is not None
                else x for x in d.ravel()]
        d = np.array(flat, dtype='datetime64[us]').reshape(d.shape)
    return d.astype('datetime64[us]')


def date2num(d):
    """
    Convert datetime objects to Matplotlib dates.

    Parameters
    ----------
    d : `datetime.datetime` or `numpy.datetime64` or sequences of these

    Returns
    -------
    float or sequence of floats
        Number of days since the epoch.  See `.get_epoch` for the
        epoch, which can be changed by :rc:`date.epoch` or `.set_epoch`.  If
        the epoch is "1970-01-01T00:00:00" (default) then noon Jan 1 1970
        ("1970-01-01T12:00:00") returns 0.5.

    Notes
    -----
    The Gregorian calendar is assumed; this is not universal practice.
    For details see the module docstring.
    """
    # Unpack in case of e.g. Pandas or xarray object
    d = cbook._unpack_to_numpy(d)

    # make an iterable, but save state to unpack later:
    iterable = np.iterable(d)
    if not iterable:
        d = [d]

    masked = np.ma.is_masked(d)
    mask = np.ma.getmask(d)
    d = np.asarray(d)
    if not d.size:
        # deals with an empty array...
        return d.astype('float64')
    d64 = _to_datetime64(d)
    nat = np.isnat(d64)
    d = _dates.micros_to_num(d64.view(np.int64), nat)
    if masked:
        d = np.ma.masked_array(d, mask=mask)
    return d if iterable else d[0]


def num2date(x, tz=None):
    """
    Convert Matplotlib dates to `~datetime.datetime` objects.

    Parameters
    ----------
    x : float or sequence of floats
        Number of days (fraction part represents hours, minutes, seconds)
        since the epoch.  See `.get_epoch` for the
        epoch, which can be changed by :rc:`date.epoch` or `.set_epoch`.
    tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
        Timezone of *x*. If a string, *tz* is passed to `dateutil.tz`.

    Returns
    -------
    `~datetime.datetime` or sequence of `~datetime.datetime`
        Dates are returned in timezone *tz*.

        If *x* is a sequence, a sequence of `~datetime.datetime` objects will
        be returned.
    """
    tz = _get_tzinfo(tz)
    values = np.asarray(x, dtype=float)
    dates = [datetime.datetime(*date, *time, tzinfo=UTC).astimezone(tz)
             for date, time in _dates.num_to_utc(values.ravel())]
    if np.iterable(x):
        return np.array(dates, dtype=object).reshape(values.shape).tolist()
    return dates[0]


def num2timedelta(x):
    """
    Convert number of days to a `~datetime.timedelta` object.

    If *x* is a sequence, a sequence of `~datetime.timedelta` objects will
    be returned.

    Parameters
    ----------
    x : float, sequence of floats
        Number of days. The fraction part represents hours, minutes, seconds.

    Returns
    -------
    `datetime.timedelta` or list[`datetime.timedelta`]
    """
    values = np.asarray(x, dtype=float)
    deltas = [datetime.timedelta(microseconds=int(us))
              for us in _dates.num_to_timedelta(values.ravel())]
    if np.iterable(x):
        return np.array(deltas, dtype=object).reshape(values.shape).tolist()
    return deltas[0]


def datestr2num(d, default=None):
    """
    Convert a date string to a datenum.

    Parameters
    ----------
    d : str or sequence of str
        The dates to convert, as ISO 8601 strings.

    default : datetime.datetime, optional
        pltrs reads ISO 8601 dates, which need no default.
    """
    if default is not None:
        raise NotImplementedError("pltrs: datestr2num(default=...)")
    if isinstance(d, str):
        return _dates.datestr2num(d)
    d = [_dates.datestr2num(s) for s in d]
    if not d:
        return d
    return np.asarray(d)


def drange(dstart, dend, delta):
    """
    Return a sequence of equally spaced Matplotlib dates.

    The dates start at *dstart* and reach up to, but not including *dend*.
    They are spaced by *delta*.

    Parameters
    ----------
    dstart, dend : `~datetime.datetime`
        The date limits.
    delta : `datetime.timedelta`
        Spacing of the dates.

    Returns
    -------
    `numpy.array`
        A list floats representing Matplotlib dates.
    """
    f1 = date2num(dstart)
    f2 = date2num(dend)
    step = delta.total_seconds() / SEC_PER_DAY

    # calculate the difference between dend and dstart in times of delta
    num = int(np.ceil((f2 - f1) / step))

    # calculate end of the interval which will be generated
    dinterval_end = dstart + num * delta

    # ensure, that an half open interval will be generated [dstart, dend)
    if dinterval_end >= dend:
        # if the endpoint is greater than or equal to dend,
        # just subtract one delta
        dinterval_end -= delta
        num -= 1

    f2 = date2num(dinterval_end)  # new float-endpoint
    return np.linspace(f1, f2, num + 1)


def _wrap_in_tex(text):
    """*text* set in TeX's math mode, as usetex date labels are."""
    return _dates.wrap_in_tex(text)


class DateFormatter(ticker.Formatter):
    """
    Format a tick (in days since the epoch) with a
    `~datetime.datetime.strftime` format string.
    """

    def __init__(self, fmt, tz=None, *, usetex=None):
        """
        Parameters
        ----------
        fmt : str
            `~datetime.datetime.strftime` format string
        tz : str or `~datetime.tzinfo`, default: :rc:`timezone`
            Ticks timezone. If a string, *tz* is passed to `dateutil.tz`.
        usetex : bool, default: :rc:`text.usetex`
            To enable/disable the use of TeX's math mode for rendering the
            results of the formatter.
        """
        self.tz = _get_tzinfo(tz)
        self.fmt = fmt
        self._usetex = _rc('text.usetex') if usetex is None else usetex

    def __call__(self, x, pos=0):
        result = num2date(x, self.tz).strftime(self.fmt)
        return _wrap_in_tex(result) if self._usetex else result

    def set_tzinfo(self, tz):
        """Set the time zone of the labels."""
        self.tz = _get_tzinfo(tz)


class DateConverter(units.ConversionInterface):
    """
    Converter for `datetime.date` and `datetime.datetime` data, or for
    date/time data represented as it would be converted by `date2num`.

    The 'unit' tag for such data is None or a `~datetime.tzinfo` instance.
    """

    def __init__(self, *, interval_multiples=True):
        self._interval_multiples = interval_multiples
        super().__init__()

    @staticmethod
    def convert(value, unit, axis):
        """
        If *value* is not already a number or sequence of numbers, convert it
        with `date2num`.

        The *unit* and *axis* arguments are not used.
        """
        return date2num(value)

    @staticmethod
    def default_units(x, axis):
        """
        Return the `~datetime.tzinfo` instance of *x* or of its first element,
        or None
        """
        if isinstance(x, np.ndarray):
            x = x.ravel()

        try:
            x = cbook._safe_first_finite(x)
        except (TypeError, StopIteration):
            pass

        try:
            return x.tzinfo
        except AttributeError:
            pass
        return None


class _SwitchableDateConverter:
    """
    Helper converter-like object that generates and dispatches to
    temporary DateConverter instances based on :rc:`date.converter` and
    :rc:`date.interval_multiples`.
    """

    @staticmethod
    def _get_converter():
        return DateConverter(interval_multiples=_rc('date.interval_multiples'))

    def __getattr__(self, name):
        if name in ["convert", "default_units"]:
            return getattr(self._get_converter(), name)
        raise AttributeError(name)


units.registry[np.datetime64] = \
    units.registry[datetime.date] = \
    units.registry[datetime.datetime] = \
    _SwitchableDateConverter()
