"""
How the GUI-events harness finds a program's canvas on the screen
(`gui_events.canvas`): by the pixels the program wrote, which must be
whole, in a picture of the window, at the place the program gave or
anywhere; and what it keeps when the canvas does not show.
"""

import pytest

from gui_events.canvas import Canvas, NotFound, wait_shown
from gui_events.errors import ProgramFailed

WHITE, GREY = b"\xff\xff\xff", b"\x2e\x34\x40"


def _canvas():
    """A 6 × 4 canvas: white, with one striped row (the most distinct)
    and one with a single dot."""
    rows = [WHITE * 6,
            (b"\x00\x00\x00" + WHITE) * 3,
            WHITE * 2 + b"\x10\x20\x30" + WHITE * 3,
            WHITE * 6]
    return Canvas((6, 4), b"".join(rows))


def _screen(canvas, size, at):
    """An image of *size* with a grey background and *canvas* at *at*."""
    width, height = size
    pixels = bytearray(GREY * width * height)
    x, y = at
    for row in range(canvas.height):
        start = 3 * (width * (y + row) + x)
        line = canvas.pixels[3 * canvas.width * row:
                             3 * canvas.width * (row + 1)]
        pixels[start:start + len(line)] = line
    return bytes(pixels)


def test_the_most_distinct_rows_identify_a_canvas():
    assert _canvas().rows == [1, 2]


def test_a_canvas_is_found_anywhere_or_at_its_place():
    canvas = _canvas()
    screen = _screen(canvas, (20, 10), (7, 3))
    assert canvas.find(screen, (20, 10)) == (7, 3)
    assert canvas.find(screen, (20, 10), at=(7, 3)) == (7, 3)
    assert canvas.find(screen, (20, 10), at=(6, 3)) is None


def test_a_canvas_partly_off_the_image_is_not_found_there():
    canvas = _canvas()
    screen = _screen(canvas, (20, 10), (0, 0))
    assert canvas.find(screen, (20, 10), at=(16, 0)) is None
    assert canvas.find(screen, (20, 10), at=(0, 8)) is None


def test_a_canvas_file_cut_short_is_refused(tmp_path):
    path = tmp_path / "scenario.pltrs.canvas"
    path.write_bytes(_canvas().pixels[:-1])
    with pytest.raises(ProgramFailed, match="holds 71 bytes, not the 72"):
        Canvas.read(path, (6, 4))
    path.write_bytes(b"")
    with pytest.raises(ProgramFailed, match="holds 0 bytes"):
        Canvas.read(path, (6, 4))


class _Window:
    """An X connection showing one window of *size* with *pixels*."""

    def __init__(self, size, pixels):
        self.size, self.pixels = size, pixels

    def geometry(self, window):
        return (0, 0, *self.size)

    def image(self, window, x, y, width, height):
        assert (x, y, width, height) == (0, 0, *self.size)
        return self.pixels


def test_a_shown_canvas_is_found_without_waiting(tmp_path):
    canvas = _canvas()
    conn = _Window((20, 10), _screen(canvas, (20, 10), (4, 5)))
    assert wait_shown(conn, 1, canvas, keep=tmp_path / "s") == (4, 5, 6, 4)
    assert list(tmp_path.iterdir()) == []


def test_a_canvas_that_never_shows_leaves_both_pictures(tmp_path, monkeypatch):
    from PIL import Image
    monkeypatch.setattr("gui_events.canvas.SHOWN_WITHIN", 0)
    canvas = _canvas()
    conn = _Window((20, 10), GREY * 200)
    with pytest.raises(NotFound) as caught:
        wait_shown(conn, 1, canvas, keep=tmp_path / "polygon.pltrs")
    shown = tmp_path / "polygon.pltrs.screen.png"
    wanted = tmp_path / "polygon.pltrs.canvas.png"
    assert str(shown) in str(caught.value) and str(wanted) in str(caught.value)
    with Image.open(shown) as image:
        assert (image.size, image.convert("RGB").tobytes()) == (
            (20, 10), GREY * 200)
    with Image.open(wanted) as image:
        assert (image.size, image.convert("RGB").tobytes()) == (
            (6, 4), canvas.pixels)
