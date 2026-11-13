# The type stub of matplotlib.testing.jpl_units.EpochConverter: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from matplotlib import cbook, units
import matplotlib.dates as date_ticker
__all__ = ['EpochConverter']

class EpochConverter(units.ConversionInterface):
    jdRef = 1721425.5

    @staticmethod
    def axisinfo(unit, axis):
        ...

    @staticmethod
    def float2epoch(value, unit):
        ...

    @staticmethod
    def epoch2float(value, unit):
        ...

    @staticmethod
    def duration2float(value):
        ...

    @staticmethod
    def convert(value, unit, axis):
        ...

    @staticmethod
    def default_units(value, axis):
        ...
