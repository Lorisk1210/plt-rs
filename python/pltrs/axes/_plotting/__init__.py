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
from .bins2d import Bins2dMixin
from .contours import ContoursMixin
from .ecdf import EcdfMixin
from .errorbar import ErrorbarMixin
from .events import EventsMixin
from .fields import FieldsMixin
from .fills import FillsMixin
from .hist import HistMixin
from .images import ImagesMixin
from .labels import LabelsMixin
from .lines import LinesMixin
from .mesh import MeshMixin
from .scatter import ScatterMixin
from .spectral import SpectralMixin
from .stats import StatsMixin
from .tables import TablesMixin


class PlottingMixin(ChildrenMixin, LinesMixin, ScatterMixin, BarsMixin,
                    ErrorbarMixin, HistMixin, FillsMixin, EventsMixin, StatsMixin,
                    ContoursMixin, MeshMixin, FieldsMixin, LabelsMixin,
                    Bins2dMixin, ImagesMixin, SpectralMixin, EcdfMixin,
                    TablesMixin):
    """The plotting methods of `~.axes.Axes`."""


__all__ = ["PlottingMixin"]
