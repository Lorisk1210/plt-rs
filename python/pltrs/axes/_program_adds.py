"""
The ``add_*`` methods of an Axes class of the program's, called where
matplotlib's plotting methods add what they make.

matplotlib's ``plot`` adds each line it makes with ``self.add_line``,
``scatter`` its collection with ``self.add_collection``, ``imshow`` its
image with ``self.add_image``, ``bar`` each rectangle with
``self.add_patch``, ``text`` its text with ``self._add_text``: a class
overriding one of them (to register what is plotted) sees every artist
the plotting methods make. pltrs (Rust) makes and adds those artists
itself; for a class of the program's taking one of these methods,
`announce` calls it with each artist the plotting method made, and the
package's own method it reaches (``super().add_line(line)``) adds nothing
more, the artist being in the Axes already (`taken`). An override that
does not reach it leaves the artist out of the Axes, as matplotlib's
plotting method then leaves it; what an override returns is not used, as
matplotlib's plotting methods use nothing it returns. Objects of the
package's classes never get here.
"""

import functools
import threading

from .._program_classes import takes_from_program


class _Announced(threading.local):
    """The artists announced to the program's ``add_*`` method running,
    which the package's method then takes as added (`taken`)."""

    def __init__(self):
        self.ids = set()


_announced = _Announced()


@functools.cache
def _add_method(axes_class, artist_class):
    """The name of the method matplotlib adds an artist of *artist_class*
    with, if *axes_class* takes it from the program; else None."""
    from ..collections import Collection
    from ..image import AxesImage
    from ..lines import Line2D
    from ..patches import Patch
    from ..table import Table
    from ..text import Text
    for kind, name in ((Line2D, "add_line"), (Collection, "add_collection"),
                       (AxesImage, "add_image"), (Patch, "add_patch"),
                       (Text, "_add_text"), (Table, "add_table")):
        if issubclass(artist_class, kind):
            return name if takes_from_program(axes_class, name) else None
    return None


def announce(axes, artist):
    """Call the ``add_*`` method of *axes*' class of the program's that
    matplotlib's plotting method adds *artist* (just made and added) with,
    if the class overrides it; take *artist* out of the Axes again if the
    method does not reach the package's."""
    name = _add_method(type(axes), type(artist))
    if name is None or vars(artist).get("_announced_to") is axes:
        return
    artist.__dict__["_announced_to"] = axes
    _announced.ids.add(id(artist))
    try:
        getattr(axes, name)(artist, **_keywords(name, artist))
    finally:
        if id(artist) in _announced.ids:
            _announced.ids.discard(id(artist))
            artist.remove()


def _keywords(name, artist):
    """The keywords matplotlib's plotting method adds *artist* with: the
    collections of lines and of meshes without autoscaling (``hlines``,
    ``vlines``, ``eventplot``, ``pcolor``, ``pcolormesh``, which set the
    limits themselves)."""
    from ..collections import EventCollection, LineCollection, PolyCollection, QuadMesh
    if name == "add_collection" and type(artist) in (
            LineCollection, EventCollection, PolyCollection, QuadMesh):
        return {"autolim": False}
    return {}


def taken(artist):
    """Whether *artist*, being handed to the package's ``add_*`` method,
    is one `announce` handed the program's method: in the Axes already."""
    if id(artist) not in _announced.ids:
        return False
    _announced.ids.discard(id(artist))
    return True
