# The type stub of matplotlib.tri._tripcolor: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from pltrs.axes import Axes
from pltrs.collections import PolyCollection, TriMesh
from pltrs.colors import Normalize, Colormap
from pltrs.tri._triangulation import Triangulation

from numpy.typing import ArrayLike

from typing import overload, Literal

@overload
def tripcolor(
    ax: Axes,
    triangulation: Triangulation,
    c: ArrayLike = ...,
    *,
    alpha: float = ...,
    norm: str | Normalize | None = ...,
    cmap: str | Colormap | None = ...,
    vmin: float | None = ...,
    vmax: float | None = ...,
    shading: Literal["flat"] = ...,
    facecolors: ArrayLike | None = ...,
    **kwargs
) -> PolyCollection: ...
@overload
def tripcolor(
    ax: Axes,
    x: ArrayLike,
    y: ArrayLike,
    c: ArrayLike = ...,
    *,
    alpha: float = ...,
    norm: str | Normalize | None = ...,
    cmap: str | Colormap | None = ...,
    vmin: float | None = ...,
    vmax: float | None = ...,
    shading: Literal["flat"] = ...,
    facecolors: ArrayLike | None = ...,
    **kwargs
) -> PolyCollection: ...
@overload
def tripcolor(
    ax: Axes,
    triangulation: Triangulation,
    c: ArrayLike = ...,
    *,
    alpha: float = ...,
    norm: str | Normalize | None = ...,
    cmap: str | Colormap | None = ...,
    vmin: float | None = ...,
    vmax: float | None = ...,
    shading: Literal["gouraud"],
    facecolors: ArrayLike | None = ...,
    **kwargs
) -> TriMesh: ...
@overload
def tripcolor(
    ax: Axes,
    x: ArrayLike,
    y: ArrayLike,
    c: ArrayLike = ...,
    *,
    alpha: float = ...,
    norm: str | Normalize | None = ...,
    cmap: str | Colormap | None = ...,
    vmin: float | None = ...,
    vmax: float | None = ...,
    shading: Literal["gouraud"],
    facecolors: ArrayLike | None = ...,
    **kwargs
) -> TriMesh: ...
