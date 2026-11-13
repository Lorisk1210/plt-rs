# The type stub of matplotlib.axes: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from typing import TypeVar

from ._axes import Axes as Axes


_T = TypeVar("_T")

# Backcompat.
Subplot = Axes

class _SubplotBaseMeta(type):
    def __instancecheck__(self, obj) -> bool: ...

class SubplotBase(metaclass=_SubplotBaseMeta): ...

def subplot_class_factory(cls: type[_T]) -> type[_T]: ...
