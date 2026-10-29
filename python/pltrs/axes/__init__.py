"""Axes: the plotting area of a figure (matplotlib's ``axes`` package)."""

from . import _base
from ._axes import Axes

#: matplotlib's former name for an Axes placed on a subplot grid.
Subplot = Axes


class _SubplotBaseMeta(type):
    def __instancecheck__(self, obj):
        return (isinstance(obj, _base._AxesBase)
                and obj.get_subplotspec() is not None)


class SubplotBase(metaclass=_SubplotBaseMeta):
    """An Axes placed on a subplot grid (an `isinstance` check only)."""


def subplot_class_factory(cls):
    """Return *cls*: every Axes class can be placed on a subplot grid."""
    return cls


__all__ = ["Axes", "Subplot", "SubplotBase", "subplot_class_factory"]
