"""
The plotting methods of `~.axes.Axes` (matplotlib's ``axes._axes``), in
one mixin per family of methods; `PlottingMixin` gathers them for the
`~.axes.Axes` class.

Each method turns matplotlib's signature (positional forms, aliases,
defaults, the checks raising matplotlib's messages) into the explicit
arguments of a native function of ``_native.plotting``, and wraps the
artists it returns (see `.._handles`).
"""

from ._children import ChildrenMixin
from .bars import BarsMixin
from .contours import ContoursMixin
from .errorbar import ErrorbarMixin
from .events import EventsMixin
from .fills import FillsMixin
from .hist import HistMixin
from .lines import LinesMixin
from .mesh import MeshMixin
from .scatter import ScatterMixin
from .stats import StatsMixin


class PlottingMixin(ChildrenMixin, LinesMixin, ScatterMixin, BarsMixin,
                    ErrorbarMixin, HistMixin, FillsMixin, EventsMixin, StatsMixin,
                    ContoursMixin, MeshMixin):
    """The plotting methods of `~.axes.Axes`."""


__all__ = ["PlottingMixin"]
