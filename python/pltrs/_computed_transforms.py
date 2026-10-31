"""
Transforms Python subclasses compute when they are drawn.

matplotlib reads an artist's ``get_transform()`` in its ``draw``, so a
subclass of the program's overriding it (a collection scaling its shapes
with the axes, say) has its transform evaluated at every draw, from the
laid-out figure. pltrs (Rust) draws artists from their native transform:
an artist whose class overrides ``get_transform`` outside this package is
kept for its figure (`track`), whose draw hook sets the native transform
of each such artist from its ``get_transform()`` once the figure is laid
out and before its artists are drawn (``_native.transforms.set_draw_hook``).
"""

import functools
import os
import sys
import weakref

from . import _live_transforms
from ._native import transforms as _transforms

#: The tracked artists of each root figure.
_tracked = weakref.WeakKeyDictionary()

#: Where the package's modules are (the classes defined there have their
#: transforms drawn by pltrs (Rust)).
_PACKAGE_DIR = os.path.dirname(os.path.realpath(__file__))


@functools.cache
def _overrides_get_transform(cls):
    """Whether the artist class *cls* takes its ``get_transform`` from a
    class of the program's rather than one of this package."""
    for klass in cls.__mro__:
        if "get_transform" in vars(klass):
            path = getattr(sys.modules.get(klass.__module__), "__file__", None)
            return path is None or not os.path.realpath(path).startswith(
                _PACKAGE_DIR + os.sep)
    return False


def track(artist, figure):
    """Have *figure* evaluate the transform of *artist* at every draw when
    its class overrides ``get_transform``."""
    if not _overrides_get_transform(type(artist)):
        return
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


def _evaluate(figure):
    """Set the native transform of every tracked artist of *figure* still
    in it from its ``get_transform()``."""
    for artist in list(_tracked.get(figure, ())):
        if artist.figure is figure:
            _transforms.set_transform(
                artist._native, _live_transforms.to_native(artist.get_transform()))
