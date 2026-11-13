"""
Annotation coordinate systems matplotlib evaluates when it draws: an
`.OffsetFrom` of a box or of a transform, and a callable of the renderer
returning a box or a transform (``xycoords=callable``).

pltrs (Rust) cannot call the program while it lays a figure out or
measures it, so an annotation hands it the transform such a coordinate
system gives now (`native`), and its root figure evaluates it again at
every draw, once the figure is laid out and before its artists are drawn
(`._computed_transforms.track`), as matplotlib evaluates it in the
annotation's ``draw``. An annotation outside any figure has no renderer to
evaluate it with: its points are display pixels until a figure adopts it.
"""

from . import _lazy_imports, _live_transforms
from .artist import Artist
from .transforms import BboxBase, BboxTransformTo, Transform

_text_module = _lazy_imports.module(globals(), ".text", "_text_module")


def is_evaluated(coords):
    """Whether *coords*, a coordinate system of an annotation (or a pair
    of them), is one the program evaluates."""
    if isinstance(coords, tuple):
        return any(map(is_evaluated, coords))
    if isinstance(coords, _text_module.OffsetFrom):
        return not isinstance(coords._artist, Artist)
    return callable(coords) and not isinstance(coords, (str, Artist, Transform))


def native(coords, figure):
    """
    The coordinate system *coords* evaluated with the renderer of
    *figure*, as ``("transform", native)`` for `_native.annotation`.

    A box is the transform to it, which is the live transform spanning it
    for a box of the figure's (``ax.bbox``), so that it follows the axes.
    """
    if figure is None:
        return ("transform", _live_transforms.to_native(None))
    result = coords(figure._get_renderer())
    if isinstance(result, BboxBase):
        spanning = getattr(result, "_spanning", None)
        result = BboxTransformTo(result) if spanning is None else spanning
    elif not isinstance(result, Transform):
        raise TypeError(
            f"xycoords callable must return a BboxBase or Transform, not a "
            f"{type(result).__name__}")
    return ("transform", _live_transforms.to_native(result))
