"""
Driving Wayland windows through a weston nested in the X server
(`xdisplay.private_wayland`): the input faked on the X server reaches
weston's window, whose seat hands it to the Wayland clients. A client
cannot tell where its window is on the output, so the driver finds the
canvas by its pixels, which the program writes when it is ready.
"""

import time

from .errors import ProgramFailed

#: The title of the X window of weston's output.
OUTPUT_TITLE = "Weston Compositor - X1"
#: Where the close button of a window decoration is from the canvas'
#: top-right corner, for Qt's and pltrs' decorations alike: in the title
#: bar right above the canvas, at its right end.
CLOSE_BUTTON = (-14, -15)
#: How long a window may take to show (weston's on the X server, the
#: canvas on weston's output): seconds, on a machine busy with other work.
SHOWN_WITHIN = 30


class NotFound(ProgramFailed):
    """The canvas is not on weston's output."""


def _distinct_rows(pixels, width, height, count=2):
    """The *count* rows of an RGB image with the most colour changes,
    which identify it best."""
    def changes(row):
        start = 3 * width * row
        line = pixels[start:start + 3 * width]
        return sum(line[i:i + 3] != line[i + 3:i + 6]
                   for i in range(0, len(line) - 3, 3))
    return sorted(range(height), key=changes, reverse=True)[:count]


def locate(conn, output, canvas_path, size):
    """
    ``(x, y, width, height)`` of the canvas in weston's output window
    *output*: where the canvas' pixels (*canvas_path*, raw RGB rows of a
    canvas of *size*) show, found by their most distinctive rows. Waits
    a while for the window to be shown.
    """
    width, height = size
    canvas = canvas_path.read_bytes()
    rows = _distinct_rows(canvas, width, height)
    patterns = [canvas[3 * width * row:3 * width * (row + 1)] for row in rows]
    deadline = time.monotonic() + SHOWN_WITHIN
    while time.monotonic() < deadline:
        _, _, out_width, out_height = conn.geometry(output)
        screen = conn.image(output, 0, 0, out_width, out_height)
        stride = 3 * out_width
        for top in range(out_height - height + 1):
            line = screen[stride * (top + rows[0]):stride * (top + rows[0] + 1)]
            at = line.find(patterns[0])
            while at >= 0:
                if at % 3 == 0 and all(
                        screen[stride * (top + row) + at:
                               stride * (top + row) + at + 3 * width] == pattern
                        for row, pattern in zip(rows[1:], patterns[1:])):
                    return at // 3, top, width, height
                at = line.find(patterns[0], at + 1)
        time.sleep(0.1)
    raise NotFound("the canvas is not on weston's output")


def close(driver):
    """Click the close button of the window's decoration."""
    x, y = CLOSE_BUTTON
    driver.move(driver.width + x, driver.height - y)
    driver.click(1)
