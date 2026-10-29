"""
One Python object per pltrs (Rust) artist.

A native function returning an artist returns a new native handle each
time (``_native.Native``), but matplotlib code expects the same object
back: ``ax.lines[0] is line``, attributes set on an artist stay, and a
subclass stays a subclass. So each figure keeps a registry of the objects
of its artists, by artist id, and native handles become objects through
`wrap`.

The registry lives on the root `.Figure` (``figure._artists``), and every
artist object refers to its figure, so an artist's object lives as long as
its figure and the figure as long as any of its artists' objects.
"""

from ._native import artist as _artist


def wrap(native, cls, figure):
    """
    The object of *figure* for the artist the handle *native* reaches:
    the one made before, or a new *cls* around it.

    ``None`` stays ``None`` (an optional artist that is absent).
    """
    if native is None:
        return None
    key = _artist.id(native)
    obj = figure._artists.get(key)
    if obj is None:
        obj = cls._from_native(native, figure)
        figure._artists[key] = obj
    return obj


def wrap_all(natives, cls, figure):
    """`wrap` for each handle of *natives*, as a list."""
    return [wrap(native, cls, figure) for native in natives]


def forget(artist):
    """Drop *artist* from its figure's registry (once it is removed)."""
    artist.figure._artists.pop(_artist.id(artist._native), None)
