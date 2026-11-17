"""
The boxes matplotlib's `.Axes3D` keeps its limits in: ``xy_viewLim`` and
``zz_viewLim`` (the view limits of x and y, and of z in x), ``xy_dataLim``
and ``zz_dataLim`` (the data limits likewise).

pltrs (Rust) keeps the limits of each axis; each box is a `.Bbox` made
the first time it is read and given the limits pltrs (Rust) holds each
time it is read again, in the array of its points, so that the box and
the array stay the same objects, as matplotlib's do. What a program
changes in that array (``ax.xy_viewLim.intervalx[1] = right``) is pltrs
(Rust)'s limits before the box is read again and before the figure reads
its artists (`pltrs._own_state.follow`), as matplotlib's draw reads the
boxes.
"""

import numpy as np

from pltrs import _own_state
from pltrs._native import mplot3d as _mplot3d
from pltrs.transforms import Bbox

#: Each box, by the attribute that keeps it: the native setter of the
#: limits it holds, and the axes whose limits its x and y intervals are
#: (None: the unit interval).
_BOXES = {
    "_xy_viewLim": (_mplot3d.set_view_interval, "x", "y"),
    "_zz_viewLim": (_mplot3d.set_view_interval, "z", None),
    "_xy_dataLim": (_mplot3d.set_data_interval, "x", "y"),
    "_zz_dataLim": (_mplot3d.set_data_interval, "z", None),
}

#: The attribute of an Axes holding the points each box was last given,
#: as bytes, by the box's attribute.
_SEEN = "_limit_boxes_seen"


def _take(axes, name, box):
    """Give pltrs (Rust) the limits of the box *name* of *axes* if the
    program changed them in the array of its points."""
    seen = axes.__dict__[_SEEN]
    now = box._points.tobytes()
    if now != seen[name]:
        setter, first, second = _BOXES[name]
        (x0, y0), (x1, y1) = box._points
        setter(axes._native, first, (float(x0), float(x1)))
        if second is not None:
            setter(axes._native, second, (float(y0), float(y1)))
        seen[name] = now


def _kept_box(axes, name, limits):
    """The box *name* of *axes*, holding what *limits* gives (the corners
    of the limits pltrs (Rust) holds), read once the program's changes in
    place are taken."""
    box = axes.__dict__.get(name)
    if box is None:
        box = axes.__dict__[name] = Bbox(limits())
        axes.__dict__.setdefault(_SEEN, {})[name] = box._points.tobytes()
        figure = axes.get_figure(root=True)
        if figure is not None:
            _own_state.follow(axes, figure)
        return box
    _take(axes, name, box)
    # (A box of z keeps the y interval the program gave it: no limits.)
    columns = slice(None) if _BOXES[name][2] is not None else slice(0, 1)
    points = np.asarray(limits(), dtype=float)[:, columns]
    if np.any(box._points[:, columns] != points):
        box._points[:, columns] = points
        box.invalidate()
    axes.__dict__[_SEEN][name] = box._points.tobytes()
    return box


class LimitBoxesMixin:
    """The limit boxes of `.Axes3D` (see the module docstring)."""

    def _take_changed_in_place(self):
        # docstring inherited: also the limit boxes read before.
        super()._take_changed_in_place()
        for name in _BOXES:
            box = self.__dict__.get(name)
            if box is not None:
                _take(self, name, box)

    def _xy_view_points(self):
        """The corners of the view limits of x and y."""
        (x0, x1), (y0, y1) = self.get_xlim3d(), self.get_ylim3d()
        return [[x0, y0], [x1, y1]]

    def _zz_view_points(self):
        """The corners of the view limits of z, over the unit interval."""
        z0, z1 = self.get_zlim3d()
        return [[z0, 0], [z1, 1]]

    def _xy_data_points(self):
        """The corners of the data limits of x and y."""
        x0, x1 = _mplot3d.get_data_interval(self._native, 'x')
        y0, y1 = _mplot3d.get_data_interval(self._native, 'y')
        return [[x0, y0], [x1, y1]]

    def _zz_data_points(self):
        """The corners of the data limits of z, over the unit interval."""
        z0, z1 = _mplot3d.get_data_interval(self._native, 'z')
        return [[z0, 0], [z1, 1]]

    @property
    def xy_viewLim(self):
        """The view limits of x and y, as a box."""
        return _kept_box(self, "_xy_viewLim", self._xy_view_points)

    @property
    def zz_viewLim(self):
        """The view limits of z, as the x interval of a box."""
        return _kept_box(self, "_zz_viewLim", self._zz_view_points)

    @property
    def xy_dataLim(self):
        """The data limits of x and y, as a box."""
        return _kept_box(self, "_xy_dataLim", self._xy_data_points)

    @property
    def zz_dataLim(self):
        """The data limits of z, as the x interval of a box."""
        return _kept_box(self, "_zz_dataLim", self._zz_data_points)
