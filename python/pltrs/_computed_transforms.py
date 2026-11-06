"""
Transforms artists compute when they are drawn.

matplotlib reads an artist's ``get_transform()`` in its ``draw``, so a
subclass of the program's overriding it (a collection scaling its shapes
with the axes, say) has its transform evaluated at every draw, from the
laid-out figure; an annotation evaluates there the coordinate systems the
program computes (`._text_coords`). pltrs (Rust) draws artists from their
native transforms: an artist whose class overrides ``get_transform``
outside this package, and an annotation with such coordinate systems
(``_evaluates_coords``), are kept for their figure (`track`), whose draw
hook sets their native transforms from the program's once the figure is
laid out and before its artists are drawn
(``_native.transforms.set_draw_hook``).
"""

import weakref

from . import _live_transforms
from ._program_classes import takes_from_program
from ._native import transforms as _transforms

#: The tracked artists of each root figure.
_tracked = weakref.WeakKeyDictionary()


def track(artist, figure):
    """Have the root *figure* evaluate the transforms of *artist* at every
    draw when its class overrides ``get_transform`` or it evaluates
    coordinate systems of the program's, which it then evaluates now."""
    evaluates_coords = getattr(artist, "_evaluates_coords", False)
    if not (evaluates_coords or _computes_transform(artist)):
        return
    if evaluates_coords:
        artist._evaluate_coords()
    artists = _tracked.get(figure)
    if artists is None:
        artists = _tracked[figure] = weakref.WeakSet()
        figure_ref = weakref.ref(figure)

        def hook():
            alive = figure_ref()
            if alive is not None:
                _evaluate(alive)

        _transforms.set_draw_hook(figure._native, hook)
    artists.add(artist)


def tracked(figure):
    """The artists whose transforms the root *figure* evaluates."""
    return list(_tracked.get(figure, ()))


def _computes_transform(artist):
    """Whether the class of *artist* overrides ``get_transform``, but for
    an Axes, which matplotlib's draw never asks for its own transform
    (astropy's ``WCSAxes.get_transform(frame)`` is another method)."""
    from .axes._base import _AxesBase
    return (takes_from_program(type(artist), "get_transform")
            and not isinstance(artist, _AxesBase))


def _evaluate(figure):
    """Set the native transforms of every tracked artist still in the root
    *figure* from the program's."""
    for artist in list(_tracked.get(figure, ())):
        if artist._figure is not figure:
            continue
        if _computes_transform(artist):
            _transforms.set_transform(
                artist._native, _live_transforms.to_native(artist.get_transform()))
        if getattr(artist, "_evaluates_coords", False):
            artist._evaluate_coords()
