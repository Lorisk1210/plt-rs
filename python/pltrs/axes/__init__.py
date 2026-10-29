"""Axes: the plotting area of a figure (matplotlib's ``axes`` package)."""

from ._axes import Axes

#: matplotlib's former name for an Axes placed on a subplot grid.
Subplot = Axes

__all__ = ["Axes", "Subplot"]
