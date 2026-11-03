"""
The boxes matplotlib's `.Axes3D` keeps its limits in: ``xy_viewLim`` and
``zz_viewLim`` (the view limits of x and y, and of z in x), ``xy_dataLim``
and ``zz_dataLim`` (the data limits likewise).

pltrs (Rust) keeps the limits of each axis; each box is a `.Bbox` made
the first time it is read and given the limits pltrs (Rust) holds each
time it is read again, so that it stays the same object, as matplotlib's
does.
"""

import numpy as np

from pltrs._native import mplot3d as _mplot3d
from pltrs.transforms import Bbox


def _kept_box(axes, name, points):
    """The box *name* of *axes*, with the corners *points*."""
    box = axes.__dict__.get(name)
    if box is None:
        box = axes.__dict__[name] = Bbox(points)
    else:
        box.set_points(np.asarray(points, dtype=float))
    return box


class LimitBoxesMixin:
    """The limit boxes of `.Axes3D` (see the module docstring)."""

    @property
    def xy_viewLim(self):
        """The view limits of x and y, as a box."""
        (x0, x1), (y0, y1) = self.get_xlim3d(), self.get_ylim3d()
        return _kept_box(self, "_xy_viewLim", [[x0, y0], [x1, y1]])

    @property
    def zz_viewLim(self):
        """The view limits of z, as the x interval of a box."""
        z0, z1 = self.get_zlim3d()
        return _kept_box(self, "_zz_viewLim", [[z0, 0], [z1, 1]])

    @property
    def xy_dataLim(self):
        """The data limits of x and y, as a box."""
        x0, x1 = _mplot3d.get_data_interval(self._native, 'x')
        y0, y1 = _mplot3d.get_data_interval(self._native, 'y')
        return _kept_box(self, "_xy_dataLim", [[x0, y0], [x1, y1]])

    @property
    def zz_dataLim(self):
        """The data limits of z, as the x interval of a box."""
        z0, z1 = _mplot3d.get_data_interval(self._native, 'z')
        return _kept_box(self, "_zz_dataLim", [[z0, 0], [z1, 1]])
