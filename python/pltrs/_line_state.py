"""
The state of lines that matplotlib keeps as objects of the line's own,
which programs change in place (see `._own_state`): the numbers drawn
(``_xy``, of which ``_x`` and ``_y`` are views), the path made of them,
and the list of markers to draw a line was given. matplotlib's ``recache`` makes the path of a line without steps of
the array of its numbers itself, so a change of either is a change of
both, drawn and measured; `XY` and `PATH` share the array the same way.
"""

import numpy as np

from ._native import lines as _lines
from ._own_state import State, hand_out
from .path import Path


def _read_xy(line):
    x, y = np.broadcast_arrays(_lines.get_xdata(line._native), _lines.get_ydata(line._native))
    return np.column_stack([x, y]).astype(float)


def _stepped(line):
    """Whether *line* is drawn in steps: matplotlib then makes its path of
    new arrays, and its draws and data limits read the path alone."""
    return line.get_drawstyle() != "default"


def _take_xy(line, xy):
    if not _stepped(line):
        _lines.set_data(line._native, np.array(xy[:, 0], dtype=float),
                        np.array(xy[:, 1], dtype=float))


#: The numbers drawn, as an ``(N, 2)`` array (matplotlib's ``_xy``).
XY = State("xy", _read_xy, _take_xy)


def columns(line):
    """The x and the y numbers drawn, as the two views of the line's own
    array that matplotlib's ``_x`` and ``_y`` are: the same views while the
    array is the same."""
    xy = hand_out(line, XY)
    views = line.__dict__.get("_xy_views")
    if views is None or views[0] is not xy:
        views = line._xy_views = (xy, xy[:, 0], xy[:, 1])
    return views[1], views[2]


def _steps_of(line):
    """The getter and the setter of the interpolation steps of *line*'s
    path, which are pltrs (Rust)'s (`.Path._steps_of`)."""
    return (lambda: _lines.get_interpolation_steps(line._native),
            lambda steps: _lines.set_interpolation_steps(line._native, steps))


def _read_path(line):
    vertices, codes = _lines.get_path(line._native)
    xy = hand_out(line, XY)
    if codes is None and vertices.shape == xy.shape and np.array_equal(
            vertices, xy, equal_nan=True):
        # No steps: the path is made of the line's own array.
        vertices = xy
    path = Path(vertices, codes)
    path._steps_of = _steps_of(line)
    return path


def _take_path(line, path):
    vertices = np.asarray(path.vertices, dtype=float)
    if len(vertices) == len(_lines.get_xdata(line._native)):
        _lines.set_data(line._native, np.array(vertices[:, 0]), np.array(vertices[:, 1]))


def _path_key(path):
    """The vertices and codes of *path* (its interpolation steps are pltrs
    (Rust)'s already)."""
    vertices, codes = path._vertices, path._codes
    return (np.shape(vertices), np.asarray(vertices).tobytes(),
            None if codes is None else np.asarray(codes).tobytes())


#: The path drawn (matplotlib's ``_path``). The path of a line drawn in
#: steps is made of its numbers by pltrs (Rust) at every draw: its vertices
#: can be read, and what a program changes in them is not drawn.
PATH = State("path", _read_path, _take_path, _path_key)


#: The markers drawn, where the line was given a list or an array of them
#: (matplotlib's ``_markevery``, the object given).
MARKEVERY = State("markevery", lambda line: line._markevery,
                  lambda line, every: line._take_markevery(every))
