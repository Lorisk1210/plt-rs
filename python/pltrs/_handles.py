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

import importlib

from ._native import artist as _artist


def wrap(native, cls, figure):
    """
    The object of *figure* for the artist the handle *native* reaches:
    the one made before, or a new *cls* around it.

    ``None`` stays ``None`` (an optional artist that is absent).
    """
    if native is None:
        return None
    figure = figure._figure  # The root keeps the registry.
    key = _artist.id(native)
    obj = figure._artists.get(key)
    if obj is None:
        obj = cls._from_native(native, figure)
        figure._artists[key] = obj
    return obj


def wrap_all(natives, cls, figure):
    """`wrap` for each handle of *natives*, as a list."""
    return [wrap(native, cls, figure) for native in natives]


#: The module of the package defining each class `_native.artist.class_name`
#: names (`artist` for the others).
_CLASS_MODULES = {
    "Line2D": "lines",
    "Text": "text", "Annotation": "text",
    "AxesImage": "image", "PcolorImage": "image", "BboxImage": "image",
    "Collection": "collections", "PatchCollection": "collections",
    "PathCollection": "collections", "PolyCollection": "collections",
    "LineCollection": "collections", "QuadMesh": "collections",
    "EventCollection": "collections", "EllipseCollection": "collections",
    "TriMesh": "collections",
    "Quiver": "quiver", "QuiverKey": "quiver",
    "QuadContourSet": "contour",
    "Legend": "legend",
    "Table": "table",
    "Spine": "spines",
    "Axes": "axes",
    "InsetIndicator": "inset",
    **dict.fromkeys(
        ["Rectangle", "Ellipse", "Circle", "Polygon", "Wedge", "PathPatch",
         "RegularPolygon", "CirclePolygon", "StepPatch", "FancyArrow", "Arrow",
         "Arc", "Annulus", "FancyBboxPatch", "FancyArrowPatch",
         "ConnectionPatch"], "patches"),
}


def wrap_any(native, figure):
    """
    `wrap` for a handle of an artist of any kind: the object made before,
    or a new object of matplotlib's class for the artist's kind.
    """
    if native is None:
        return None
    obj = figure._artists.get(_artist.id(native))
    if obj is not None:
        return obj
    name = _artist.class_name(native)
    module = importlib.import_module(
        f".{_CLASS_MODULES.get(name, 'artist')}", __package__)
    return wrap(native, getattr(module, name), figure)


def adopt(artist, native, figure):
    """
    Make *artist*, a loose artist or one removed from *figure*, the object
    of the artist the handle *native* reaches in *figure*, once an axes or
    the figure adopted it (``_native.artist.adopt_into_axes``); return it.
    """
    figure = figure._figure  # The root keeps the registry.
    artist._native = native
    artist._figure = figure
    figure._artists[_artist.id(native)] = artist
    return artist


def forget(artist):
    """Drop *artist* from its figure's registry (once it is removed)."""
    artist._figure._artists.pop(_artist.id(artist._native), None)
