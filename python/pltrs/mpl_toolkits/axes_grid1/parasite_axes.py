"""
Host axes, which carry parasite axes (matplotlib's
``mpl_toolkits.axes_grid1.parasite_axes``).

pltrs has the host axes so far: `host_axes` and `host_subplot`, and the
classes `host_axes_class_factory` makes (`HostAxes`), which pickle as
matplotlib's do.
"""

from pltrs import cbook
from pltrs.axes import Axes


class HostAxesBase:
    """The mixin of host axes: their list of parasites."""

    @classmethod
    def _from_native(cls, native, figure):
        ax = super()._from_native(native, figure)
        ax.parasites = []
        return ax


host_axes_class_factory = host_subplot_class_factory = \
    cbook._make_class_factory(HostAxesBase, "{}HostAxes", "_base_axes_class")
HostAxes = SubplotHost = host_axes_class_factory(Axes)


def host_axes(*args, axes_class=Axes, figure=None, **kwargs):
    """
    Create axes that can act as a hosts to parasitic axes.

    Parameters
    ----------
    figure : `~matplotlib.figure.Figure`
        Figure to which the axes will be added. Defaults to the current figure
        `.pyplot.gcf()`.

    *args, **kwargs
        Will be passed on to the underlying `~.axes.Axes` object creation.
    """
    import pltrs.pyplot as plt
    host_axes_class = host_axes_class_factory(axes_class)
    if figure is None:
        figure = plt.gcf()
    ax = figure.add_axes(*args, axes_class=host_axes_class, **kwargs)
    # As made by its class and added (matplotlib's `add_axes(ax)`), the
    # axes keeps no projection key for pyplot.subplot to reuse it by.
    ax._projection_init = None
    return ax


host_subplot = host_axes
