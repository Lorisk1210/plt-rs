# The type stub of matplotlib.category: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from collections import OrderedDict
import dateutil.parser
import itertools
import logging
import numpy as np
from matplotlib import _api, cbook, ticker, units
_log = logging.getLogger(__name__)

class StrCategoryConverter(units.ConversionInterface):

    @staticmethod
    def convert(value, unit, axis):
        ...

    @staticmethod
    def axisinfo(unit, axis):
        ...

    @staticmethod
    def default_units(data, axis):
        ...

    @staticmethod
    def _validate_unit(unit):
        ...

class StrCategoryLocator(ticker.Locator):

    def __init__(self, units_mapping):
        self._units = units_mapping

    def __call__(self):
        ...

    def tick_values(self, vmin, vmax):
        ...

class StrCategoryFormatter(ticker.Formatter):

    def __init__(self, units_mapping):
        self._units = units_mapping

    def __call__(self, x, pos=None):
        ...

    def format_ticks(self, values):
        ...

    @staticmethod
    def _text(value):
        ...

class UnitData:

    def __init__(self, data=None):
        self._mapping = OrderedDict()
        self._counter = itertools.count()

    @staticmethod
    def _str_is_convertible(val):
        ...

    def update(self, data):
        self._mapping[val] = next(self._counter)
units.registry[str] = units.registry[np.str_] = units.registry[bytes] = units.registry[np.bytes_] = StrCategoryConverter()
__all__ = ['OrderedDict', 'dateutil', 'itertools', 'logging', 'np', 'cbook', 'ticker', 'units', 'StrCategoryConverter', 'StrCategoryLocator', 'StrCategoryFormatter', 'UnitData']
