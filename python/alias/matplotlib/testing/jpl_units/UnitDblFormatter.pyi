# The type stub of matplotlib.testing.jpl_units.UnitDblFormatter: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import matplotlib.ticker as ticker
__all__ = ['UnitDblFormatter']

class UnitDblFormatter(ticker.ScalarFormatter):

    def __call__(self, x, pos=None):
        ...

    def format_data_short(self, value):
        ...

    def format_data(self, value):
        ...
