"""Axes: the plotting area of a figure (matplotlib's ``axes`` package)."""

from . import _base
from ._axes import Axes
from . import _secondary_axes  # noqa: F401  (matplotlib's axes import it)

#: matplotlib's former name for an Axes placed on a subplot grid.
Subplot = Axes


class _SubplotBaseMeta(type):
    def __instancecheck__(self, obj):
        return (isinstance(obj, _base._AxesBase)
                and obj.get_subplotspec() is not None)


class SubplotBase(metaclass=_SubplotBaseMeta):
    pass


def subplot_class_factory(cls):
    return cls


__all__ = ["Axes", "Subplot", "SubplotBase", "subplot_class_factory"]
