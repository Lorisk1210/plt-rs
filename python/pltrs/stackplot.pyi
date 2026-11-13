# The type stub of matplotlib.stackplot: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from pltrs.axes import Axes
from pltrs.collections import PolyCollection

from collections.abc import Iterable
from typing import Literal
from numpy.typing import ArrayLike
from pltrs.typing import ColorType

def stackplot(
    axes: Axes,
    x: ArrayLike,
    *args: ArrayLike,
    labels: Iterable[str] = ...,
    colors: Iterable[ColorType] | None = ...,
    hatch: Iterable[str] | str | None = ...,
    baseline: Literal["zero", "sym", "wiggle", "weighted_wiggle"] = ...,
    **kwargs
) -> list[PolyCollection]: ...

__all__ = ['stackplot']
