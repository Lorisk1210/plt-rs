# The type stub of matplotlib.testing.jpl_units: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from .Duration import Duration
from .Epoch import Epoch
from .UnitDbl import UnitDbl
from .StrConverter import StrConverter
from .EpochConverter import EpochConverter
from .UnitDblConverter import UnitDblConverter
from .UnitDblFormatter import UnitDblFormatter
__version__ = '1.0'
__all__ = ['register', 'Duration', 'Epoch', 'UnitDbl', 'UnitDblFormatter']

def register():
    ...
m = UnitDbl(1.0, 'm')
km = UnitDbl(1.0, 'km')
mile = UnitDbl(1.0, 'mile')
deg = UnitDbl(1.0, 'deg')
rad = UnitDbl(1.0, 'rad')
sec = UnitDbl(1.0, 'sec')
min = UnitDbl(1.0, 'min')
hr = UnitDbl(1.0, 'hour')
day = UnitDbl(24.0, 'hour')
sec = UnitDbl(1.0, 'sec')
