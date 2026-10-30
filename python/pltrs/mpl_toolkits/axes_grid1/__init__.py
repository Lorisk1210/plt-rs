"""
Axes placed by dividers (matplotlib's ``mpl_toolkits.axes_grid1``), over
pltrs (Rust)'s ``axes_grid1``: dividers and their sizes, grids of axes,
host axes with parasites, insets, anchored artists and RGB axes.
"""

from . import axes_size as Size
from .axes_divider import Divider, SubplotDivider, make_axes_locatable
from .axes_grid import AxesGrid, Grid, ImageGrid

from .parasite_axes import host_subplot, host_axes

__all__ = ["Size",
           "Divider", "SubplotDivider", "make_axes_locatable",
           "AxesGrid", "Grid", "ImageGrid",
           "host_subplot", "host_axes"]
