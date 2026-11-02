"""
`pylab` is a historic interface: pyplot and numpy in one namespace
(matplotlib's ``pylab``, from matplotlib 3.11.2 under the Matplotlib
License, without dateutil's ``rrule`` and ``relativedelta``, which pltrs'
``dates`` does not re-export).

It imports pltrs by name, as it is loaded as the top-level ``pylab`` when
matplotlib's suite runs.
"""

from __future__ import annotations

from typing import TYPE_CHECKING  # noqa: F401  (matplotlib's pyplot has it)

from pltrs.dates import (  # noqa: F401
    date2num, num2date, datestr2num, drange, DateFormatter, DateLocator,
    RRuleLocator, YearLocator, MonthLocator, WeekdayLocator, DayLocator,
    HourLocator, MinuteLocator, SecondLocator, MO, TU, WE, TH, FR,
    SA, SU, YEARLY, MONTHLY, WEEKLY, DAILY, HOURLY, MINUTELY, SECONDLY)

# bring all the symbols in so folks can import them from
# pylab in one fell swoop

from pltrs.mlab import (  # noqa: F401
    detrend, detrend_linear, detrend_mean, detrend_none, window_hanning,
    window_none)

from pltrs import cbook, mlab, pyplot as plt  # noqa: F401
from pltrs.pyplot import *  # noqa: F401, F403
# What matplotlib's pyplot, which has no __all__, brings in besides.
from pltrs import (  # noqa: F401
    color_sequences, colormaps, rcParamsDefault, rcParamsOrig)
from pltrs.backends import backend_registry  # noqa: F401

from numpy import *  # noqa: F401, F403
from numpy.fft import *  # noqa: F401, F403
from numpy.random import *  # noqa: F401, F403
from numpy.linalg import *  # noqa: F401, F403

import numpy as np  # noqa: F401
import numpy.ma as ma  # noqa: F401

# don't let numpy's datetime hide stdlib
import datetime  # noqa: F401

# This is needed, or bytes will be numpy.random.bytes from
# "from numpy.random import *" above
bytes = __import__("builtins").bytes
# We also don't want the numpy version of these functions
abs = __import__("builtins").abs
bool = __import__("builtins").bool
max = __import__("builtins").max
min = __import__("builtins").min
pow = __import__("builtins").pow
round = __import__("builtins").round
