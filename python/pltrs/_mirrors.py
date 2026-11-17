"""
Artists standing for parts that pltrs (Rust) draws as parts of another
artist.

matplotlib keeps some parts of an artist as artists of their own: the
ticks, label and offset text of an axis, the box behind a text, the arrow
of an annotation, the text of a table cell. pltrs (Rust) draws them as
parts of their artist. pltrs (Python) hands them out as *mirrors*: plain
objects of matplotlib's very classes (`.Text`, `.Line2D`,
`.FancyBboxPatch`, `.FancyArrowPatch`; the `.FontProperties` of a text),
filled from the part, whose ``_mirror`` is the `Mirroring` of the part
they stand for. Every setter of
a class made `mirrorable` gives the part its mirror's state once it ran
(`Mirroring.push`), and the mirror asks the part where it is drawn, in
which figure and with which transform.
"""

import contextlib
import functools
import inspect
import weakref

from . import _artist_staleness
from ._native import artist as _artist

#: What a `Mirroring` answers to leave a question to the mirror's class.
OWN = object()


class Mirroring:
    """
    The part a mirror stands for.

    A subclass gives the part the mirror's state in ``push(artist)``, and
    may answer where the part is drawn (`window_extent`), which figure it
    is in (`figure`) and the transform matplotlib places it with
    (`transform`); `OWN` leaves the answer to the mirror's class.
    """

    #: Whether matplotlib's part has a stale callback, which stales what
    #: holds it (a figure's patch): pushing then runs the program's
    #: callbacks as the owner's change (see `push`).
    stales_owner = False

    def window_extent(self, artist, renderer=None, dpi=None):
        """The display box of the part *artist* mirrors, or `OWN`."""
        return OWN

    def tight_box(self, artist, renderer=None):
        """The tight display box of the part, by default its extent."""
        return self.window_extent(artist, renderer)

    def figure(self, artist, root=False):
        """The figure the part is in, or `OWN`."""
        return OWN

    def transform(self, artist):
        """The transform of the part *artist* mirrors, or `OWN`."""
        return OWN

    def transform_set(self, artist):
        """Whether the part was given its transform, or `OWN`."""
        return OWN

    def contains(self, artist, *args, **kwargs):
        """Whether the mouse event is over the part, or `OWN`."""
        return OWN

    def contains_point(self, artist, *args, **kwargs):
        """Whether the display point is in the part, or `OWN`."""
        return OWN

    def path(self, artist):
        """The path of the part *artist* mirrors (``get_path``), or
        `OWN`."""
        return OWN

    def display_path(self, artist):
        """The ``(vertices, codes)`` of the outline of the part as drawn,
        in display pixels, or `OWN`."""
        return OWN

    def clip_box(self, artist):
        """The clip box of the part (``get_clip_box``), or `OWN`."""
        return OWN

    def clip_path(self, artist):
        """The clip path of the part (``get_clip_path``), or `OWN`."""
        return OWN

    def clip_spec(self, artist):
        """The native clip region of artists clipped to the part (see
        `._artist_clipping`), or `OWN`."""
        return OWN

    def alone(self, artist):
        """An artist of its own that draws the part as matplotlib's artist
        draws itself where something else than its owner draws it (an Axes
        listing it among its children), or `OWN`: the mirror's own draw."""
        return OWN

    #: The artist whose draws draw the part, after which the mirror reads
    #: as fresh as matplotlib's draw of it leaves it (see `.Artist.stale`),
    #: or None: the mirror's own draws.
    drawer = None

    def drawn_by_drawer(self, artist):
        """Whether the draws of the `drawer` draw the part: unless the
        mirror *artist* is hidden."""
        return artist.get_visible()

    def refresh(self, artist):
        """Give the mirror *artist* what drawing the part changed since
        it was filled, before one of its ``_refreshed_getters`` reads it
        or one of its setters changes it."""


#: The mirrorings of objects whose attributes are their value (font
#: properties hash and compare by them), kept apart, by the object's id.
_apart = {}


def attach_apart(obj, mirroring):
    """Make *obj* the mirror of what *mirroring* stands for, without an
    attribute of its own."""
    _apart[id(obj)] = mirroring
    weakref.finalize(obj, _apart.pop, id(obj), None)
    return obj


def is_attached(obj):
    """Whether *obj* mirrors a part."""
    return mirroring(obj) is not None


def mirroring(obj):
    """The `Mirroring` of the part *obj* mirrors, or None."""
    mirror = getattr(obj, "__dict__", _NO_ATTRIBUTES).get("_mirror")
    if mirror is None and _apart:
        return _apart.get(id(obj))
    return mirror


#: The attributes of an object without a ``__dict__``.
_NO_ATTRIBUTES = {}


def ask(obj, question, *args):
    """What the part *obj* mirrors answers to *question* (a method of
    `Mirroring`), or `OWN` when *obj* mirrors nothing or the part leaves
    the answer to it."""
    mirror = mirroring(obj)
    return OWN if mirror is None else getattr(mirror, question)(obj, *args)


def _pushing(method):
    @functools.wraps(method)
    def setter(self, *args, **kwargs):
        # What drawing the part changed comes first: matplotlib's draw
        # changed the artist before the program sets it.
        mirror = mirroring(self)
        if mirror is not None:
            mirror.refresh(self)
        result = method(self, *args, **kwargs)
        mirror = mirroring(self)
        if mirror is not None:
            if _artist_staleness.reporting:
                push(mirror, self)
            else:
                mirror.push(self)
        return result
    setter._pushes = True
    return setter


def push(mirror, obj):
    """Give the part *mirror* stands for the state of its mirror *obj*
    (`Mirroring.push`). matplotlib's part is the mirror itself, whose
    setters stale it alone (most parts have no stale callback): the owner
    pltrs (Rust) marks stale meanwhile is reported to no stale callback of
    the program's (`._artist_staleness`), unless the part's would stale it
    (`Mirroring.stales_owner`)."""
    if _artist_staleness.reporting and not mirror.stales_owner:
        _artist.unreported(mirror.push, obj)
    else:
        mirror.push(obj)


def pushing(cls):
    """Class decorator: every setter of *cls* pushes a mirror of it back
    into its part once it ran, having refreshed it first (`Mirroring.refresh`:
    what drawing the part changed comes before the setter's change), once:
    the setters it inherits from a class made pushing already push. An
    alias of a setter (``set_name = set_family``) stays the same function
    as the setter; the aliases ``_api.define_aliases`` makes, which call
    their setter by name, stay as they are (matplotlib's classes inherit
    them)."""
    wrappers = {}
    aliases = {f"set_{alias}" for alias in getattr(cls, "_alias_to_prop", ())}
    for name in dir(cls):
        method = getattr(cls, name) if name.startswith("set_") else None
        if callable(method) and name not in aliases and not getattr(method, "_pushes", False):
            if method not in wrappers:
                wrappers[method] = _pushing(method)
            _replace(cls, name, wrappers[method], getattr(method, "__name__", name))
    return cls


def _replace(cls, name, wrapper, own_name=None):
    """Make *wrapper* the method *name* of *cls*. A method *cls* defines
    itself is named as one of *cls*'s own (by *own_name*, the name the
    wrapped function was defined under, for an alias), so that
    ``inspect.getdoc`` finds the docstring its bases give a method without
    one of its own, as for the method it wraps; one it inherits keeps the
    name and module of the method it wraps, as matplotlib's inherited
    method has them (`.ArtistInspector` names a property's setter by
    them)."""
    if name in vars(cls):
        wrapper.__qualname__ = f"{cls.__qualname__}.{own_name or name}"
        wrapper.__module__ = cls.__module__
    setattr(cls, name, wrapper)


def _asking(method, question):
    @functools.wraps(method)
    def asking(self, *args, **kwargs):
        mirror = self.__dict__.get("_mirror")
        if mirror is not None:
            try:
                answer = getattr(mirror, question)(self, *args, **kwargs)
            except TypeError:
                _check_arguments(method, self, *args, **kwargs)
                raise
            if answer is not OWN:
                return answer
        return method(self, *args, **kwargs)
    return asking


def _check_arguments(method, *args, **kwargs):
    """Raise what calling *method* with *args* and *kwargs* raises when
    they do not fit its parameters (the error naming the method, rather
    than the question a mirror's part was asked)."""
    try:
        inspect.signature(method).bind(*args, **kwargs)
    except TypeError:
        method(*args, **kwargs)


def _refreshing(method):
    @functools.wraps(method)
    def refreshed(self, *args, **kwargs):
        mirror = self.__dict__.get("_mirror")
        if mirror is not None:
            mirror.refresh(self)
        return method(self, *args, **kwargs)
    return refreshed


def mirrorable(cls):
    """Class decorator letting the instances of *cls* be mirrors: every
    setter pushes, the getters the class names in ``_refreshed_getters``
    read the mirror refreshed first, and the extent, the figure and
    whether a point is over it are asked of the part."""
    pushing(cls)
    for name in getattr(cls, "_refreshed_getters", ()):
        _replace(cls, name, _refreshing(getattr(cls, name)))
    questions = {"get_window_extent": "window_extent", "get_tightbbox": "tight_box",
                 "_get_figure": "figure", "get_transform": "transform",
                 "is_transform_set": "transform_set", "contains": "contains",
                 "contains_point": "contains_point", "get_path": "path"}
    for name, question in questions.items():
        if hasattr(cls, name):
            _replace(cls, name, _asking(getattr(cls, name), question))
    return cls


def attach(artist, mirroring):
    """Make *artist* the mirror of the part *mirroring* stands for."""
    artist._mirror = mirroring
    return artist


@contextlib.contextmanager
def detached(artist):
    """Change *artist* without giving its part the changes."""
    mirror = artist.__dict__.pop("_mirror", None)
    try:
        yield artist
    finally:
        if mirror is not None:
            artist._mirror = mirror
