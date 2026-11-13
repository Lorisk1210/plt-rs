# The type stub of matplotlib.tri._trifinder: matplotlib 3.11.2's, under the
# Matplotlib License, as matplotlib installs it
# (python/tools/matplotlib_stubs.py).

from matplotlib.tri import Triangulation
from numpy.typing import ArrayLike

class TriFinder:
    def __init__(self, triangulation: Triangulation) -> None: ...
    def __call__(self, x: ArrayLike, y: ArrayLike) -> ArrayLike: ...

class TrapezoidMapTriFinder(TriFinder):
    def __init__(self, triangulation: Triangulation) -> None: ...
    def __call__(self, x: ArrayLike, y: ArrayLike) -> ArrayLike: ...
