# The type stub of matplotlib.tri._triplot: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from pltrs.tri._triangulation import Triangulation
from pltrs.axes import Axes
from pltrs.lines import Line2D

from typing import overload
from numpy.typing import ArrayLike

@overload
def triplot(
    ax: Axes, triangulation: Triangulation, *args, **kwargs
) -> tuple[Line2D, Line2D]: ...
@overload
def triplot(
    ax: Axes, x: ArrayLike, y: ArrayLike, triangles: ArrayLike = ..., *args, **kwargs
) -> tuple[Line2D, Line2D]: ...
