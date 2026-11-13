# The type stub of matplotlib.testing.jpl_units.UnitDbl: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import operator
from matplotlib import _api

class UnitDbl:
    allowed = {'m': (0.001, 'km'), 'km': (1, 'km'), 'mile': (1.609344, 'km'), 'rad': (1, 'rad'), 'deg': (0.0174532925199433, 'rad'), 'sec': (1, 'sec'), 'min': (60.0, 'sec'), 'hour': (3600, 'sec')}
    _types = {'km': 'distance', 'rad': 'angle', 'sec': 'time'}

    def __init__(self, value, units):
        self._value = float(value * data[0])
        self._units = data[1]

    def convert(self, units):
        ...

    def __abs__(self):
        ...

    def __neg__(self):
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

    def _binop_unit_unit(self, op, rhs):
        ...
    __add__ = functools.partialmethod(_binop_unit_unit, operator.add)
    __sub__ = functools.partialmethod(_binop_unit_unit, operator.sub)

    def _binop_unit_scalar(self, op, scalar):
        ...
    __mul__ = functools.partialmethod(_binop_unit_scalar, operator.mul)
    __rmul__ = functools.partialmethod(_binop_unit_scalar, operator.mul)

    def __str__(self):
        ...

    def __repr__(self):
        ...

    def type(self):
        ...

    @staticmethod
    def range(start, stop, step=None):
        ...

    def checkSameUnits(self, rhs, func):
        ...
__all__ = ['functools', 'operator', 'UnitDbl']
