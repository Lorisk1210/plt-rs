"""
Contours on triangular grids (matplotlib's ``tri._tricontour``).
"""

from ..contour import ContourSet


class TriContourSet(ContourSet):
    """
    Create and store a set of contour lines or filled regions for
    a triangular grid.

    This class is typically not instantiated directly by the user but by
    `~.Axes.tricontour` and `~.Axes.tricontourf`.
    """


def tricontour(ax, *args, **kwargs):
    """
    Draw contour lines on an unstructured triangular grid in *ax*
    (`~.Axes.tricontour`).

    Returns
    -------
    `~matplotlib.tri.TriContourSet`
    """
    return ax.tricontour(*args, **kwargs)


def tricontourf(ax, *args, **kwargs):
    """
    Draw contour regions on an unstructured triangular grid in *ax*
    (`~.Axes.tricontourf`).

    Returns
    -------
    `~matplotlib.tri.TriContourSet`
    """
    return ax.tricontourf(*args, **kwargs)
