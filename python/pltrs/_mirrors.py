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
import weakref

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

    def clip_spec(self, artist):
        """The native clip region of artists clipped to the part (see
        `._artist_clipping`), or `OWN`."""
        return OWN


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
        result = method(self, *args, **kwargs)
        mirror = mirroring(self)
        if mirror is not None:
            mirror.push(self)
        return result
    setter._pushes = True
    return setter


def pushing(cls):
    """Class decorator: every setter of *cls* pushes a mirror of it back
    into its part once it ran (once: the setters it inherits from a class
    made pushing already push)."""
    for name in dir(cls):
        method = getattr(cls, name) if name.startswith("set_") else None
        if callable(method) and not getattr(method, "_pushes", False):
            setattr(cls, name, _pushing(method))
    return cls


def _asking(method, question):
    @functools.wraps(method)
    def asking(self, *args, **kwargs):
        mirror = self.__dict__.get("_mirror")
        if mirror is not None:
            answer = getattr(mirror, question)(self, *args, **kwargs)
            if answer is not OWN:
                return answer
        return method(self, *args, **kwargs)
    return asking


def mirrorable(cls):
    """Class decorator letting the instances of *cls* be mirrors: every
    setter pushes, and the extent, the figure and whether a point is over
    it are asked of the part."""
    pushing(cls)
    cls.get_window_extent = _asking(cls.get_window_extent, "window_extent")
    cls.get_tightbbox = _asking(cls.get_tightbbox, "tight_box")
    cls.get_figure = _asking(cls.get_figure, "figure")
    cls.get_transform = _asking(cls.get_transform, "transform")
    cls.is_transform_set = _asking(cls.is_transform_set, "transform_set")
    cls.contains = _asking(cls.contains, "contains")
    if hasattr(cls, "contains_point"):
        cls.contains_point = _asking(cls.contains_point, "contains_point")
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
