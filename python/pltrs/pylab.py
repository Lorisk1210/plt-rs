"""
`pylab` is a historic interface: pyplot and numpy in one namespace.

It imports pltrs by name, as it is loaded as the top-level ``pylab`` when
matplotlib's suite runs.
"""

from numpy import *  # noqa: F401, F403
from pltrs.pyplot import *  # noqa: F401, F403
