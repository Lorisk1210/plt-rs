"""The status bar message: coordinates and the data under the pointer
(``NavigationToolbar2._mouse_event_to_message``, ``get_cursor_data`` and
``format_cursor_data``).

Area ``cursor``. Every op builds ``scene`` in ``Figure(figsize=(4, 3),
dpi=100)`` with ``ax = fig.add_subplot()``, draws it once and records
the message for a pointer motion at each display pixel of ``pixels``
(whole pixels from the bottom left).

Ops
---
``cursor.message``  {scene, pixels} -> [str, ...]

Scenes
------
``FIELD`` is the 6 × 8 array ``sin(0.7 r) cos(0.4 c) + 0.1 r``.

``scalar``       ``imshow(FIELD)``
``lognorm``      ``imshow(10 ** ((8 r + c) / 12 - 1), norm=LogNorm())``
``boundary``     ``imshow(FIELD, norm=BoundaryNorm([-1, -0.5, 0, 0.25, 0.5, 1, 1.5], 256))``
``integers``     ``imshow(7 * arange(48).reshape(6, 8) - 100)`` (integers)
``masked``       ``imshow(FIELD)`` with NaN where ``(r + c) % 5 == 0``
``rgb``          ``imshow`` of float colours ``((0.13 r + 0.07 c) % 1, (0.29 r + 0.3) % 1,
                 (0.21 c + 0.6) % 1)``
``rgba_bytes``   ``imshow`` of ``uint8`` colours ``(37 r + 11 c, 255 - 23 c, 40 r, 255 - 30 r)``
``lower``        ``imshow(FIELD, origin="lower", extent=(0, 4, 0, 3))``
``singular``     ``imshow(full((3, 4), 2.5))`` (``vmin == vmax``)
``overlap``      ``imshow(FIELD)``, a gray ``imshow(FIELD[:3, :4], extent=(1, 4, 3, 1))``
                 on top and a line ``plot([0, 7], [0, 5])``
``hidden``       ``imshow(FIELD)`` under a hidden ``imshow(full((2, 2), 9.0),
                 extent=(2, 6, 4, 1))``
``nomouseover``  ``imshow(FIELD)`` with ``set_mouseover(False)``
``mesh``         ``pcolormesh(X, Y, FIELD)`` with ``X = [0, 1, 3, 4, 5, 7, 8, 9, 12]``,
                 ``Y = [0, 2, 3, 5, 6, 8, 9]`` and ``set_mouseover(True)``
``pcolorfast``   ``pcolorfast(X, Y, FIELD)`` (a ``PcolorImage``)
``nonuniform``   a ``NonUniformImage`` of ``FIELD`` with centres ``X[:-1]``, ``Y[:-1]``,
                 viewed over ``xlim=(-1, 10)``, ``ylim=(-1, 9)``
"""

import numpy as np
from matplotlib.backend_bases import MouseEvent, NavigationToolbar2
from matplotlib.colors import BoundaryNorm, LogNorm
from matplotlib.figure import Figure
from matplotlib.image import NonUniformImage

from spec_support import ProbeSet

probes = ProbeSet("cursor")

ROWS, COLS = np.mgrid[0:6, 0:8]
FIELD = np.sin(0.7 * ROWS) * np.cos(0.4 * COLS) + 0.1 * ROWS
X_EDGES = [0, 1, 3, 4, 5, 7, 8, 9, 12]
Y_EDGES = [0, 2, 3, 5, 6, 8, 9]


def _masked():
    field = FIELD.copy()
    field[(ROWS + COLS) % 5 == 0] = np.nan
    return field


def _rgb():
    return np.stack([(0.13 * ROWS + 0.07 * COLS) % 1, (0.29 * ROWS + 0.3) % 1,
                     (0.21 * COLS + 0.6) % 1], axis=-1)


def _rgba_bytes():
    return np.stack([37 * ROWS + 11 * COLS, 255 - 23 * COLS, 40 * ROWS, 255 - 30 * ROWS],
                    axis=-1).astype(np.uint8)


def _overlap(ax):
    ax.imshow(FIELD)
    ax.imshow(FIELD[:3, :4], extent=(1, 4, 3, 1), cmap="gray")
    ax.plot([0, 7], [0, 5])


def _hidden(ax):
    ax.imshow(FIELD)
    ax.imshow(np.full((2, 2), 9.0), extent=(2, 6, 4, 1)).set_visible(False)


def _nonuniform(ax):
    im = NonUniformImage(ax)
    im.set_data(X_EDGES[:-1], Y_EDGES[:-1], FIELD)
    ax.add_image(im)
    ax.set(xlim=(-1, 10), ylim=(-1, 9))


SCENES = {
    "scalar": lambda ax: ax.imshow(FIELD),
    "lognorm": lambda ax: ax.imshow(10 ** ((8 * ROWS + COLS) / 12 - 1), norm=LogNorm()),
    "boundary": lambda ax: ax.imshow(
        FIELD, norm=BoundaryNorm([-1, -0.5, 0, 0.25, 0.5, 1, 1.5], 256)),
    "integers": lambda ax: ax.imshow(7 * np.arange(48).reshape(6, 8) - 100),
    "masked": lambda ax: ax.imshow(_masked()),
    "rgb": lambda ax: ax.imshow(_rgb()),
    "rgba_bytes": lambda ax: ax.imshow(_rgba_bytes()),
    "lower": lambda ax: ax.imshow(FIELD, origin="lower", extent=(0, 4, 0, 3)),
    "singular": lambda ax: ax.imshow(np.full((3, 4), 2.5)),
    "overlap": _overlap,
    "hidden": _hidden,
    "nomouseover": lambda ax: ax.imshow(FIELD).set_mouseover(False),
    "mesh": lambda ax: ax.pcolormesh(X_EDGES, Y_EDGES, FIELD).set_mouseover(True),
    "pcolorfast": lambda ax: ax.pcolorfast(X_EDGES, Y_EDGES, FIELD),
    "nonuniform": _nonuniform,
}


def _messages(scene, pixels):
    fig = Figure(figsize=(4, 3), dpi=100)
    SCENES[scene](fig.add_subplot())
    fig.draw_without_rendering()
    return [NavigationToolbar2._mouse_event_to_message(
        MouseEvent("motion_notify_event", fig.canvas, x, y)) for x, y in pixels]


def test_messages():
    for scene in SCENES:
        for y in range(6, 300, 17):
            probes.probe("cursor.message", _messages, scene=scene,
                         pixels=[[x, y] for x in range(5, 400, 17)])
