# The type stub of matplotlib.tri._trifinder: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from pltrs.tri import Triangulation
from numpy.typing import ArrayLike

class TriFinder:
    def __init__(self, triangulation: Triangulation) -> None: ...
    def __call__(self, x: ArrayLike, y: ArrayLike) -> ArrayLike: ...

class TrapezoidMapTriFinder(TriFinder):
    def __init__(self, triangulation: Triangulation) -> None: ...
    def __call__(self, x: ArrayLike, y: ArrayLike) -> ArrayLike: ...
