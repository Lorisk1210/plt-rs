# The type stub of matplotlib.testing.jpl_units.Duration: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import operator
from matplotlib import _api

class Duration:
    allowed = ['ET', 'UTC']

    def __init__(self, frame, seconds):
        self._frame = frame
        self._seconds = seconds

    def frame(self):
        ...

    def __abs__(self):
        ...

    def __neg__(self):
        ...

    def seconds(self):
        ...

    def __bool__(self):
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

    def __mul__(self, rhs):
        ...
    __rmul__ = __mul__

    def __str__(self):
        ...

    def __repr__(self):
        ...

    def checkSameFrame(self, rhs, func):
        ...
__all__ = ['functools', 'operator', 'Duration']
