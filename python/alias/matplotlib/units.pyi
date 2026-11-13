# The type stub of matplotlib.units: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from decimal import Decimal
from numbers import Number
import numpy as np
from numpy import ma
from matplotlib import cbook

class ConversionError(TypeError):
    pass

def _is_natively_supported(x):
    ...

class AxisInfo:

    def __init__(self, majloc=None, minloc=None, majfmt=None, minfmt=None, label=None, default_limits=None):
        self.majloc = majloc
        self.minloc = minloc
        self.majfmt = majfmt
        self.minfmt = minfmt
        self.label = label
        self.default_limits = default_limits

class ConversionInterface:

    @staticmethod
    def axisinfo(unit, axis):
        ...

    @staticmethod
    def default_units(x, axis):
        ...

    @staticmethod
    def convert(obj, unit, axis):
        ...

class DecimalConverter(ConversionInterface):

    @staticmethod
    def convert(value, unit, axis):
        ...

class Registry(dict):

    def get_converter(self, x):
        ...
registry = Registry()
registry[Decimal] = DecimalConverter()
__all__ = ['Decimal', 'Number', 'np', 'ma', 'cbook', 'ConversionError', 'AxisInfo', 'ConversionInterface', 'DecimalConverter', 'Registry', 'registry']
