# The type stub of matplotlib.dates: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import datetime
import functools
import logging
import re
from dateutil.rrule import rrule, MO, TU, WE, TH, FR, SA, SU, YEARLY, MONTHLY, WEEKLY, DAILY, HOURLY, MINUTELY, SECONDLY
from dateutil.relativedelta import relativedelta
import dateutil.parser
import dateutil.tz
import numpy as np
import matplotlib as mpl
from matplotlib import _api, cbook, ticker, units
__all__ = ('datestr2num', 'date2num', 'num2date', 'num2timedelta', 'drange', 'set_epoch', 'get_epoch', 'DateFormatter', 'ConciseDateFormatter', 'AutoDateFormatter', 'DateLocator', 'RRuleLocator', 'AutoDateLocator', 'YearLocator', 'MonthLocator', 'WeekdayLocator', 'DayLocator', 'HourLocator', 'MinuteLocator', 'SecondLocator', 'MicrosecondLocator', 'rrule', 'MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU', 'YEARLY', 'MONTHLY', 'WEEKLY', 'DAILY', 'HOURLY', 'MINUTELY', 'SECONDLY', 'MICROSECONDLY', 'relativedelta', 'DateConverter', 'ConciseDateConverter', 'rrulewrapper')
_log = logging.getLogger(__name__)
UTC = datetime.timezone.utc

def _get_tzinfo(tz=None):
    ...
EPOCH_OFFSET = float(datetime.datetime(1970, 1, 1).toordinal())
MICROSECONDLY = SECONDLY + 1
HOURS_PER_DAY = 24.0
MIN_PER_HOUR = 60.0
SEC_PER_MIN = 60.0
MONTHS_PER_YEAR = 12.0
DAYS_PER_WEEK = 7.0
DAYS_PER_MONTH = 30.0
DAYS_PER_YEAR = 365.0
MINUTES_PER_DAY = MIN_PER_HOUR * HOURS_PER_DAY
SEC_PER_HOUR = SEC_PER_MIN * MIN_PER_HOUR
SEC_PER_DAY = SEC_PER_HOUR * HOURS_PER_DAY
SEC_PER_WEEK = SEC_PER_DAY * DAYS_PER_WEEK
MUSECONDS_PER_DAY = 1000000.0 * SEC_PER_DAY
MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY, SATURDAY, SUNDAY = (MO, TU, WE, TH, FR, SA, SU)
WEEKDAYS = (MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY, SATURDAY, SUNDAY)
_epoch = None

def _reset_epoch_test_example():
    ...

def set_epoch(epoch):
    ...

def get_epoch():
    ...

def _dt64_to_ordinalf(d):
    ...

def _from_ordinalf(x, tz=None):
    ...
_from_ordinalf_np_vectorized = np.vectorize(_from_ordinalf, otypes='O')
_dateutil_parser_parse_np_vectorized = np.vectorize(dateutil.parser.parse)

def datestr2num(d, default=None):
    ...

def date2num(d):
    ...

def num2date(x, tz=None):
    ...
_ordinalf_to_timedelta_np_vectorized = np.vectorize(lambda x: datetime.timedelta(days=x), otypes='O')

def num2timedelta(x):
    ...

def drange(dstart, dend, delta):
    ...

def _wrap_in_tex(text):
    ...

class DateFormatter(ticker.Formatter):

    def __init__(self, fmt, tz=None, *, usetex=None):
        self.tz = _get_tzinfo(tz)
        self.fmt = fmt
        self._usetex = mpl._val_or_rc(usetex, 'text.usetex')

    def __call__(self, x, pos=0):
        ...

    def set_tzinfo(self, tz):
        self.tz = _get_tzinfo(tz)

class ConciseDateFormatter(ticker.Formatter):
    offset_string = _api.deprecate_privatize_attribute('3.11', alternative='get_offset()')

    def __init__(self, locator, tz=None, formats=None, offset_formats=None, zero_formats=None, show_offset=True, *, usetex=None):
        self._locator = locator
        self._tz = tz
        self.defaultfmt = '%Y'
        self._offset_string = ''
        self.show_offset = show_offset
        self._usetex = mpl._val_or_rc(usetex, 'text.usetex')
        self.formats = formats
        self.formats = ['%Y', '%b', '%d', '%H:%M', '%H:%M', '%S.%f']
        self.zero_formats = zero_formats
        self.offset_formats = offset_formats
        self.offset_formats = ['', '%Y', '%Y-%b', '%Y-%b-%d', '%Y-%b-%d', '%Y-%b-%d %H:%M']
        self.zero_formats = [''] + self.formats[:-1]
        self.zero_formats = [''] + self.formats[:-1]
        self.zero_formats[3] = '%b-%d'

    def __call__(self, x, pos=None):
        ...

    def format_ticks(self, values):
        self._offset_string = ''
        self._offset_string = tickdatetime[0].strftime(offsetfmts[level])
        self._offset_string = tickdatetime[-1].strftime(offsetfmts[level])
        self._offset_string = _wrap_in_tex(self._offset_string)

    def get_offset(self):
        ...

    def format_data_short(self, value):
        ...

class AutoDateFormatter(ticker.Formatter):

    def __init__(self, locator, tz=None, defaultfmt='%Y-%m-%d', *, usetex=None):
        self._locator = locator
        self._tz = tz
        self.defaultfmt = defaultfmt
        self._formatter = DateFormatter(self.defaultfmt, tz)
        self._usetex = mpl._val_or_rc(usetex, 'text.usetex')
        self.scaled = {DAYS_PER_YEAR: rcParams['date.autoformatter.year'], DAYS_PER_MONTH: rcParams['date.autoformatter.month'], 1: rcParams['date.autoformatter.day'], 1 / HOURS_PER_DAY: rcParams['date.autoformatter.hour'], 1 / MINUTES_PER_DAY: rcParams['date.autoformatter.minute'], 1 / SEC_PER_DAY: rcParams['date.autoformatter.second'], 1 / MUSECONDS_PER_DAY: rcParams['date.autoformatter.microsecond']}

    def _set_locator(self, locator):
        self._locator = locator

    def __call__(self, x, pos=None):
        self._formatter = DateFormatter(fmt, self._tz, usetex=self._usetex)

class rrulewrapper:

    def __init__(self, freq, tzinfo=None, **kwargs):
        self._base_tzinfo = tzinfo

    def set(self, **kwargs):
        ...

    def _update_rrule(self, **kwargs):
        self._construct = kwargs.copy()
        self._tzinfo = tzinfo
        self._rrule = rrule(**self._construct)

    def _attach_tzinfo(self, dt, tzinfo):
        ...

    def _aware_return_wrapper(self, f, returns_list=False):
        ...

    def __getattr__(self, name):
        ...

    def __setstate__(self, state):
        ...

class DateLocator(ticker.Locator):
    hms0d = {'byhour': 0, 'byminute': 0, 'bysecond': 0}

    def __init__(self, tz=None):
        self.tz = _get_tzinfo(tz)

    def set_tzinfo(self, tz):
        self.tz = _get_tzinfo(tz)

    def datalim_to_dt(self):
        ...

    def viewlim_to_dt(self):
        ...

    def _get_unit(self):
        ...

    def _get_interval(self):
        ...

    def nonsingular(self, vmin, vmax):
        ...

class RRuleLocator(DateLocator):

    def __init__(self, o, tz=None):
        self.rule = o

    def __call__(self):
        ...

    def tick_values(self, vmin, vmax):
        ...

    def _create_rrule(self, vmin, vmax):
        ...

    def _get_unit(self):
        ...

    @staticmethod
    def get_unit_generic(freq):
        ...

    def _get_interval(self):
        ...

class AutoDateLocator(DateLocator):

    def __init__(self, tz=None, minticks=5, maxticks=None, interval_multiples=True):
        self._freq = YEARLY
        self._freqs = [YEARLY, MONTHLY, DAILY, HOURLY, MINUTELY, SECONDLY, MICROSECONDLY]
        self.minticks = minticks
        self.maxticks = {YEARLY: 11, MONTHLY: 12, DAILY: 11, HOURLY: 12, MINUTELY: 11, SECONDLY: 11, MICROSECONDLY: 8}
        self.interval_multiples = interval_multiples
        self.intervald = {YEARLY: [1, 2, 4, 5, 10, 20, 40, 50, 100, 200, 400, 500, 1000, 2000, 4000, 5000, 10000], MONTHLY: [1, 2, 3, 4, 6], DAILY: [1, 2, 3, 7, 14, 21], HOURLY: [1, 2, 3, 4, 6, 12], MINUTELY: [1, 5, 10, 15, 30], SECONDLY: [1, 5, 10, 15, 30], MICROSECONDLY: [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000, 500000, 1000000]}
        self._byranges = [None, range(1, 13), range(1, 32), range(0, 24), range(0, 60), range(0, 60), None]
        self.intervald[DAILY] = [1, 2, 4, 7, 14]
        self.maxticks = dict.fromkeys(self._freqs, maxticks)

    def __call__(self):
        ...

    def tick_values(self, vmin, vmax):
        ...

    def nonsingular(self, vmin, vmax):
        ...

    def _get_unit(self):
        ...

    def get_locator(self, dmin, dmax):
        self._freq = freq

class YearLocator(RRuleLocator):

    def __init__(self, base=1, month=1, day=1, tz=None):
        self.base = ticker._Edge_integer(base, 0)

    def _create_rrule(self, vmin, vmax):
        ...

class MonthLocator(RRuleLocator):

    def __init__(self, bymonth=None, bymonthday=1, interval=1, tz=None):
        ...

class WeekdayLocator(RRuleLocator):

    def __init__(self, byweekday=1, interval=1, tz=None):
        ...

class DayLocator(RRuleLocator):

    def __init__(self, bymonthday=None, interval=1, tz=None):
        ...

class HourLocator(RRuleLocator):

    def __init__(self, byhour=None, interval=1, tz=None):
        ...

class MinuteLocator(RRuleLocator):

    def __init__(self, byminute=None, interval=1, tz=None):
        ...

class SecondLocator(RRuleLocator):

    def __init__(self, bysecond=None, interval=1, tz=None):
        ...

class MicrosecondLocator(DateLocator):

    def __init__(self, interval=1, tz=None):
        self._interval = interval
        self._wrapped_locator = ticker.MultipleLocator(interval)

    def set_axis(self, axis):
        ...

    def __call__(self):
        ...

    def tick_values(self, vmin, vmax):
        ...

    def _get_unit(self):
        ...

    def _get_interval(self):
        ...

class DateConverter(units.ConversionInterface):

    def __init__(self, *, interval_multiples=True):
        self._interval_multiples = interval_multiples

    def axisinfo(self, unit, axis):
        ...

    @staticmethod
    def convert(value, unit, axis):
        ...

    @staticmethod
    def default_units(x, axis):
        ...

class ConciseDateConverter(DateConverter):

    def __init__(self, formats=None, zero_formats=None, offset_formats=None, show_offset=True, *, interval_multiples=True):
        self._formats = formats
        self._zero_formats = zero_formats
        self._offset_formats = offset_formats
        self._show_offset = show_offset
        self._interval_multiples = interval_multiples

    def axisinfo(self, unit, axis):
        ...

class _SwitchableDateConverter:

    @staticmethod
    def _get_converter():
        ...

    def axisinfo(self, *args, **kwargs):
        ...

    def default_units(self, *args, **kwargs):
        ...

    def convert(self, *args, **kwargs):
        ...
units.registry[np.datetime64] = units.registry[datetime.date] = units.registry[datetime.datetime] = _SwitchableDateConverter()
