# The type stub of matplotlib.testing.jpl_units.Epoch: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import operator
import math
import datetime as DT
from matplotlib import _api
from matplotlib.dates import date2num

class Epoch:
    allowed = {'ET': {'UTC': +64.1839}, 'UTC': {'ET': -64.1839}}

    def __init__(self, frame, sec=None, jd=None, daynum=None, dt=None):
        self._frame = frame
        self._jd = math.floor(jd)
        self._seconds = (jd - self._jd) * 86400.0
        self._seconds = float(sec)
        self._jd = float(jd)

    def convert(self, frame):
        ...

    def frame(self):
        ...

    def julianDate(self, frame):
        ...

    def secondsPast(self, frame, jd):
        ...

    def _cmp(self, op, rhs):
        ...
    __eq__ = functools.partialmethod(_cmp, operator.eq)
    __ne__ = functools.partialmethod(_cmp, operator.ne)
    __lt__ = functools.partialmethod(_cmp, operator.lt)
    __le__ = functools.partialmethod(_cmp, operator.le)
    __gt__ = functools.partialmethod(_cmp, operator.gt)
    __ge__ = functools.partialmethod(_cmp, operator.ge)

    def __add__(self, rhs):
        ...

    def __sub__(self, rhs):
        ...

    def __str__(self):
        ...

    def __repr__(self):
        ...

    @staticmethod
    def range(start, stop, step):
        ...
__all__ = ['functools', 'operator', 'math', 'DT', 'date2num', 'Epoch']
