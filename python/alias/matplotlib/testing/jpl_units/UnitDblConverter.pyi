# The type stub of matplotlib.testing.jpl_units.UnitDblConverter: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import numpy as np
from matplotlib import cbook, units
import matplotlib.projections.polar as polar
__all__ = ['UnitDblConverter']

def rad_fn(x, pos=None):
    ...

class UnitDblConverter(units.ConversionInterface):
    defaults = {'distance': 'km', 'angle': 'deg', 'time': 'sec'}

    @staticmethod
    def axisinfo(unit, axis):
        ...

    @staticmethod
    def convert(value, unit, axis):
        ...

    @staticmethod
    def default_units(value, axis):
        ...
