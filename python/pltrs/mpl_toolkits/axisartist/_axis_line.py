"""
The line of an axis artist (``AxisArtist.line``): matplotlib's artist, a
`.PathPatch` along the axis or the arrow its axis line style makes
(`.AxislineStyle`), mirroring the line pltrs (Rust) draws for the axis
artist: what the program sets on it (colour, width, visibility) is told
to that line.
"""

from pltrs import _mirrors
from pltrs._native import axisartist as _axisartist


class _LineMirroring(_mirrors.Mirroring):
    """The line pltrs (Rust) draws for an axis artist."""

    def __init__(self, axis_artist):
        self._axis_artist = axis_artist

    def push(self, artist):
        native = self._axis_artist._native
        _axisartist.set_line(native, "color", artist.get_edgecolor())
        _axisartist.set_line(native, "linewidth", artist.get_linewidth())
        _axisartist.set_line(native, "visible", bool(artist.get_visible()))


def follow(line, axis_artist):
    """Make *line*, an artist of matplotlib's classes made for
    *axis_artist*, mirror the line pltrs (Rust) draws for it."""
    if not getattr(type(line).set_edgecolor, "_pushes", False):
        _mirrors.mirrorable(type(line))
    return _mirrors.attach(line, _LineMirroring(axis_artist))
