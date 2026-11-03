"""
Contours on triangular grids (matplotlib's ``tri._tricontour``).
"""

from .. import _tri
from ..contour import ContourSet, _ContourGenerator


class TriContourSet(ContourSet):
    """
    Create and store a set of contour lines or filled regions for
    a triangular grid.

    This class is typically not instantiated directly by the user but by
    `~.Axes.tricontour` and `~.Axes.tricontourf`.
    """

    # matplotlib's triangular contours keep the extremes as Python floats.
    @property
    def zmin(self):
        """The smallest value of the data."""
        return float(super().zmin)

    @property
    def zmax(self):
        """The largest value of the data."""
        return float(super().zmax)


class _TriContourGenerator(_ContourGenerator):
    """
    What a `TriContourSet` was generated from: the triangulation and z,
    with matplotlib's generator of the ``_tri`` extension for them.
    """

    def _make_matplotlib(self):
        tri, z = self.field
        return _tri.TriContourGenerator(tri.get_cpp_triangulation(), z)


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
