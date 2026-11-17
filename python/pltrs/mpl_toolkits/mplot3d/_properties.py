"""
The properties of the 3-D artists.

**Those the plotting methods give the artists they make.** matplotlib
makes an artist with the properties given, and only then adds it to the
axes: a property no setter of its class takes fails the call before
anything is added. pltrs (Rust) makes the artist in the axes, so the
methods check the properties first (`check_known`).

**The attributes matplotlib's 3-D artists keep their state in.**
matplotlib's `~mpl_toolkits.mplot3d.art3d` classes keep the geometry of
the third dimension in plain attributes (``_segments3d``, ``_faces``,
``_3dverts_codes``, a text's ``_dir_vec``), which programs and libraries
read, change in place and assign, and the values of their last projection
in others (``_vzs``, ``_facecolors2d``, ``_offset_zordered``), which
they read. pltrs (Rust) keeps the geometry and projects it. Two
descriptors give the classes of `.art3d` the attributes all the same,
whichever way an artist was made:

* `Field`: an attribute of the geometry. An artist made by the program
  keeps the object it was given, as matplotlib does; one a 3-D Axes made
  (``quiver``, ``plot_wireframe``, ``plot_surface``) answers with what
  pltrs (Rust) holds, in the container matplotlib's method builds
  (`._fields3d`). Either is the artist's own object from the first read
  on (`pltrs._own_state`): the same at every read, and handed back to
  pltrs (Rust) when the program changed it in place. Assigned, the
  attribute keeps the object assigned and hands it over at once.
* `Left`: an attribute a projection leaves behind. It is computed from
  the geometry and the projection matrix of the last draw as matplotlib's
  ``do_3d_projection`` computes it, when it is first read after a
  projection, and is the same object until the next one. Assigned, it
  reads back what was assigned until the next projection, which is when
  matplotlib overwrites it; nothing is drawn from it.

An artist nobody asks for these attributes pays nothing: no record is
made and nothing is computed.
"""

from pltrs import _own_state
from pltrs._artist_kwargs import check_known  # noqa: F401  (the methods' name for it)
from pltrs._native import artist as _artist


class _Missing:
    """What a `Field` reads of an artist that has no such attribute."""


#: See `_Missing`.
MISSING = _Missing()


def stamp(artist):
    """What tells the projections of *artist* (a collection or a patch)
    apart: how often its Axes was drawn and how often the program
    projected it (``do_3d_projection``). What a projection leaves in
    matplotlib's artist is another object after each. matplotlib's 3-D
    Axes projects its visible collections and patches when it is drawn,
    before it draws any: also an animated one, which is not drawn then,
    and one the draw fails before. An artist without an Axes, and a
    hidden one, count their own draws."""
    if "_native" not in artist.__dict__:
        return (0, 0)
    axes = artist.axes
    drawn = axes if axes is not None and artist.get_visible() else artist
    return (_artist.draw_count(drawn._native), artist.__dict__.get("_projections", 0))


def _no_attribute(artist, name):
    return AttributeError(f"'{type(artist).__name__}' object has no attribute '{name}'",
                          name=name, obj=artist)


class Field:
    """
    An attribute of the 3-D geometry of an artist (see the module).

    *read* gives, for an artist that keeps no object of its own for the
    attribute, what pltrs (Rust) holds as matplotlib's attribute would be
    (`MISSING` for an artist without it); *push* hands pltrs (Rust) what
    such an object holds (the artist and the object). *follows*, if given,
    says for an artist whether pltrs (Rust) follows the attribute at all:
    not for a patch another class of the program's draws.
    """

    def __init__(self, read, push, follows=None):
        self._read = read
        self._push = push
        self._follows = follows

    def __set_name__(self, owner, name):
        self.name = name
        self.state = _own_state.State(
            name.strip("_"), self._native, self._take)

    def _native(self, artist):
        value = self._read(artist)
        return None if value is MISSING else value

    def _take(self, artist, value):
        artist.__dict__[self.name] = value
        self._push(artist, value)

    def peek(self, artist, default=None):
        """The object *artist* keeps for the attribute, if it keeps one
        (else *default*), for the package's own code: nothing is read from
        pltrs (Rust) and no record is made."""
        return artist.__dict__.get(self.name, default)

    # (The artist is named `self`: `pltrs._own_state` tells a read by the
    # program from one by the package by the `self` of the outermost
    # package function, which for a read of the attribute is this one.)
    def __get__(field, self, owner=None):
        if self is None:
            return field
        values = self.__dict__
        if field.name in values:
            value = values[field.name]
            if value is None or "_native" not in values or (
                    field._follows is not None and not field._follows(self)):
                return value
            return _own_state.hand_out_kept(self, field.state, value)
        if "_native" in values:
            value = _own_state.hand_out(self, field.state)
            if value is not None or field._read(self) is not MISSING:
                return value
        raise _no_attribute(self, field.name)

    def __set__(field, self, value):
        self.__dict__[field.name] = value
        if _own_state.RECORDS in self.__dict__:
            _own_state.drop(self, field.state.name)
        if "_native" in self.__dict__:
            field._push(self, value)
            self.stale = True

    def __delete__(field, self):
        if field.name not in self.__dict__:
            raise _no_attribute(self, field.name)
        del self.__dict__[field.name]


#: The attribute of an artist holding what `Left` attributes last read or
#: were assigned, by name: the projection it is of (`stamp`) and the value.
LEFT = "_left3d"


class Left:
    """
    An attribute the projection of an artist leaves behind (see the
    module): *compute* gives it for an artist projected at least once;
    *before* is what matplotlib's artist has before its first projection
    (`MISSING`: no such attribute yet).
    """

    def __init__(self, compute, before=MISSING):
        self._compute = compute
        self._before = before

    def __set_name__(self, owner, name):
        self.name = name

    def __get__(left, self, owner=None):
        if self is None:
            return left
        now = stamp(self)
        made = self.__dict__.get(LEFT, {}).get(left.name)
        if made is not None and made[0] == now:
            return made[1]
        value = left._before if now == (0, 0) else left._compute(self)
        if value is MISSING:
            raise _no_attribute(self, left.name)
        if now != (0, 0):
            self.__dict__.setdefault(LEFT, {})[left.name] = (now, value)
        return value

    def __set__(left, self, value):
        self.__dict__.setdefault(LEFT, {})[left.name] = (stamp(self), value)
