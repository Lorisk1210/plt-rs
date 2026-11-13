# The type stub of matplotlib.testing.jpl_units.StrConverter: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import numpy as np
import matplotlib.units as units
__all__ = ['StrConverter']

class StrConverter(units.ConversionInterface):

    @staticmethod
    def axisinfo(unit, axis):
        ...

    @staticmethod
    def convert(value, unit, axis):
        ...

    @staticmethod
    def default_units(value, axis):
        ...
