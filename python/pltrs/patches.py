"""
Shapes with a face and an edge (matplotlib's ``patches``).
"""

from ._native import patches as _patches
from .artist import Artist


class Patch(Artist):
    """A patch is a 2D artist with a face color and an edge color."""


class Rectangle(Patch):
    """
    A rectangle defined via an anchor point *xy* and its *width* and
    *height*.
    """

    def get_x(self):
        """Return the left coordinate of the rectangle."""
        return _patches.get_bounds(self._native)[0]

    def get_y(self):
        """Return the bottom coordinate of the rectangle."""
        return _patches.get_bounds(self._native)[1]

    def get_xy(self):
        """Return the left and bottom coords of the rectangle as a tuple."""
        return _patches.get_bounds(self._native)[:2]

    def get_width(self):
        """Return the width of the rectangle."""
        return _patches.get_bounds(self._native)[2]

    def get_height(self):
        """Return the height of the rectangle."""
        return _patches.get_bounds(self._native)[3]

    def get_facecolor(self):
        """Return the face color."""
        return _patches.get_facecolor(self._native)
