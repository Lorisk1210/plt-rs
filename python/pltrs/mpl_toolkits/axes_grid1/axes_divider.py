"""
Helper classes to adjust the positions of multiple axes at drawing time
(matplotlib's ``mpl_toolkits.axes_grid1.axes_divider``): pltrs (Rust)'s
dividers place the axes anew at every layout pass.

pltrs has `make_axes_area_auto_adjustable` so far.
"""

from collections.abc import Iterable

from pltrs import _api
from pltrs._native import axes_grid1 as _axes_grid1


def make_axes_area_auto_adjustable(
        ax, use_axes=None, pad=0.1, adjust_dirs=None):
    """
    Add auto-adjustable padding around *ax* to take its decorations (title,
    labels, ticks, ticklabels) into account during layout.

    By default, padding is determined from the decorations of *ax*.
    Pass *use_axes* to consider the decorations of other Axes instead.

    Parameters
    ----------
    ax : `~matplotlib.axes.Axes`
        The Axes whose area is padded.
    use_axes : `~matplotlib.axes.Axes` or list of `~matplotlib.axes.Axes`
        The Axes whose decorations are taken into account.
    pad : float, default: 0.1
        Additional padding in inches.
    adjust_dirs : list of {"left", "right", "bottom", "top"}, optional
        The sides where padding is added; defaults to all four sides.
    """
    if adjust_dirs is None:
        adjust_dirs = ["left", "right", "bottom", "top"]
    if use_axes is None:
        use_axes = ax
    if not isinstance(use_axes, Iterable):
        use_axes = [use_axes]
    for d in adjust_dirs:
        _api.check_in_list(["left", "right", "bottom", "top"], d=d)
    _axes_grid1.make_axes_area_auto_adjustable(
        ax._native, [a._native for a in use_axes], float(pad),
        list(adjust_dirs))
