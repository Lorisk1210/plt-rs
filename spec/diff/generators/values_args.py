"""The argument lists of the callables the ``api`` generator calls with
arguments written out (``values.ARGS``), gathered from the areas' modules
and from the plotting generators' variants (``plotting``, ``three_d``)."""

from __future__ import annotations

from typing import Iterator

import matplotlib.axes
from mpl_toolkits.mplot3d import Axes3D

from . import args_artists, args_backends, args_data, args_plotting, args_toolkits, args_tools, plotting, three_d


def _plotting_variants() -> Iterator[tuple[str, list[str]]]:
    """The plotting generators' variants, keyed by the class defining each
    method (an ``Axes3D`` variant only for a method ``Axes3D`` defines)."""
    for method, variants in plotting.VARIANTS.items():
        owner = next(c for c in matplotlib.axes.Axes.__mro__ if method in vars(c))
        yield f"matplotlib.axes:{owner.__qualname__}.{method}", variants
    for method, variants in three_d.VARIANTS.items():
        if method in vars(Axes3D):
            yield f"mpl_toolkits.mplot3d:Axes3D.{method}", variants


#: ``api_index`` key -> the argument lists of the callable's calls.
ARGS: dict[str, list[str]] = {
    **dict(_plotting_variants()),
    **args_plotting.ARGS,
    **args_artists.ARGS,
    **args_data.ARGS,
    **args_tools.ARGS,
    **args_backends.ARGS,
    **args_toolkits.ARGS,
}
