# The type stub of mpl_toolkits.axisartist: matplotlib 3.11.2's, under the
# Matplotlib License, its code without docstrings and function bodies
# (python/tools/matplotlib_stubs.py).

from .axislines import Axes
from .axislines import AxesZero, AxisArtistHelper, AxisArtistHelperRectlinear, GridHelperBase, GridHelperRectlinear, Subplot, SubplotZero
from .axis_artist import AxisArtist, GridlinesCollection
from .grid_helper_curvelinear import GridHelperCurveLinear
from .floating_axes import FloatingAxes, FloatingSubplot
from mpl_toolkits.axes_grid1.parasite_axes import host_axes_class_factory, parasite_axes_class_factory
ParasiteAxes = parasite_axes_class_factory(Axes)
HostAxes = host_axes_class_factory(Axes)
SubplotHost = HostAxes
__all__ = ['Axes', 'AxesZero', 'AxisArtistHelper', 'AxisArtistHelperRectlinear', 'GridHelperBase', 'GridHelperRectlinear', 'Subplot', 'SubplotZero', 'AxisArtist', 'GridlinesCollection', 'GridHelperCurveLinear', 'FloatingAxes', 'FloatingSubplot', 'host_axes_class_factory', 'parasite_axes_class_factory', 'ParasiteAxes', 'HostAxes', 'SubplotHost']
