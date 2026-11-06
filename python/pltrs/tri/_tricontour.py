"""
Contours on triangular grids (matplotlib's ``tri._tricontour``).
"""

from ..contour import ContourSet, _ContourGenerator


class TriContourSet(ContourSet):
    """
    Create and store a set of contour lines or filled regions for
    a triangular grid.

    This class is typically not instantiated directly by the user but by
    `~.Axes.tricontour` and `~.Axes.tricontourf`.
    """

    def __init__(self, ax, *args, **kwargs):
        """
        Draw triangular grid contour lines or filled regions,
        depending on whether keyword arg *filled* is False
        (default) or True.

        The first argument of the initializer must be an `~.axes.Axes`
        object.  The remaining arguments and keyword arguments
        are described in the docstring of `~.Axes.tricontour`.
        """
        # matplotlib's set contours its arguments as `tricontour` does.
        filled = kwargs.pop("filled", False)
        ax._tricontour(filled, args, kwargs, made=self)

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
        # The extension loads when first used, as matplotlib's does.
        from .. import _tri
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
