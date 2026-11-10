"""
Waiting for a program's canvas to show: the program writes its canvas'
pixels to a file when it is ready (`recorder.report_ready`), and the
driver looks for them on the screen, where it is about to send input:
in the window at the place the program gave (X11), or anywhere on
weston's output (Wayland, where a client cannot tell where its window
is).
"""

import time

from .errors import ProgramFailed
from .images import save_png

#: How long a window may take to show (weston's on the X server, a
#: program's window, the canvas in it): seconds, on a machine busy with
#: other work. A deadline: the waits end as soon as it shows.
SHOWN_WITHIN = 30
#: How often to look again meanwhile, in seconds.
POLL = 0.05


class NotFound(ProgramFailed):
    """The canvas does not show where the driver looks for it."""


class Canvas:
    """The pixels of a canvas of *size* ``(width, height)``: *pixels*,
    RGB rows from the top."""

    def __init__(self, size, pixels):
        self.width, self.height = size
        self.pixels = pixels
        self.rows = self._distinct_rows()
        self._patterns = [self._row(row) for row in self.rows]

    @classmethod
    def read(cls, path, size):
        """The canvas the program wrote to *path*, which must hold a
        canvas of *size*."""
        pixels = path.read_bytes()
        width, height = size
        if len(pixels) != 3 * width * height:
            raise ProgramFailed(f"{path} holds {len(pixels)} bytes, not the "
                                f"{3 * width * height} of a {width}x{height} "
                                "canvas")
        return cls(size, pixels)

    def _row(self, row):
        start = 3 * self.width * row
        return self.pixels[start:start + 3 * self.width]

    def _distinct_rows(self, count=2):
        """The *count* rows with the most colour changes, which identify
        the canvas best (and change least while an animation plays)."""
        def changes(row):
            line = self._row(row)
            return sum(line[i:i + 3] != line[i + 3:i + 6]
                       for i in range(0, len(line) - 3, 3))
        return sorted(range(self.height), key=changes, reverse=True)[:count]

    def find(self, screen, size, at=None):
        """Where the canvas' distinct rows show in *screen* (RGB rows of
        an image of *size*): ``(x, y)`` of its top-left corner, or None.
        Only at *at* when given, else anywhere."""
        width, height = size
        stride = 3 * width

        def shows_at(x, y):
            return all(
                screen[stride * (y + row) + 3 * x:
                       stride * (y + row) + 3 * (x + self.width)] == pattern
                for row, pattern in zip(self.rows, self._patterns))

        if at is not None:
            x, y = at
            inside = (0 <= x <= width - self.width
                      and 0 <= y <= height - self.height)
            return at if inside and shows_at(x, y) else None
        first = self._patterns[0]
        for top in range(height - self.height + 1):
            line = screen[stride * (top + self.rows[0]):
                          stride * (top + self.rows[0] + 1)]
            index = line.find(first)
            while index >= 0:
                if index % 3 == 0 and shows_at(index // 3, top):
                    return index // 3, top
                index = line.find(first, index + 1)
        return None


def wait_shown(conn, window, canvas, at=None, keep=None):
    """
    Wait until *canvas* (a `Canvas`) shows in X window *window*, at *at*
    (``(x, y)`` in the window) when given, else anywhere in it; give its
    ``(x, y, width, height)`` there. `NotFound` after `SHOWN_WITHIN`
    seconds, when what the window showed last and the canvas are written
    to *keep* (a path whose name gets ``.screen.png`` and ``.canvas.png``
    appended), if given, and named in the message.
    """
    deadline = time.monotonic() + SHOWN_WITHIN
    while True:
        _, _, width, height = conn.geometry(window)
        screen = conn.image(window, 0, 0, width, height)
        found = canvas.find(screen, (width, height), at)
        if found is not None:
            return (*found, canvas.width, canvas.height)
        if time.monotonic() >= deadline:
            break
        time.sleep(POLL)
    where = "where the program put it" if at is not None else "anywhere"
    message = (f"the canvas does not show in the window {where} within "
               f"{SHOWN_WITHIN} s")
    if keep is not None:
        shown = keep.with_name(keep.name + ".screen.png")
        wanted = keep.with_name(keep.name + ".canvas.png")
        save_png(shown, (width, height), screen)
        save_png(wanted, (canvas.width, canvas.height), canvas.pixels)
        message += f" (the window: {shown}; the canvas: {wanted})"
    raise NotFound(message)
