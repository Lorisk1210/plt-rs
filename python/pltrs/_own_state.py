"""
State of an artist that is a Python object of the artist's own.

matplotlib's artists keep their state in Python objects, and many getters
return the object itself: ``collection.get_paths()`` is the list of paths
the draw reads, ``scatter.get_offsets()`` the array it places the markers
by, ``line.get_xydata()`` the array of the line's path. Programs change
those objects in place and expect the artist to follow (the half violin:
``body.get_paths()[0].vertices[:, 0] = ...``). The same goes for some
objects a program gives an artist, which matplotlib keeps as they are (a
polygon's ``xy``, the paths of a `.PathCollection`).

pltrs (Rust) keeps that state itself. So an artist keeps a *record* of
each such object, made when the object is first handed out (`hand_out`)
or taken in (`keep`), under the name of the `State` it holds: the object,
what it held when pltrs (Rust) last took it (`State.key`), and what pltrs
(Rust) then held. From then on:

* **The getter gives the same object** while pltrs (Rust) holds what it
  held (`hand_out` reads it and compares): a setter that replaces the
  object in matplotlib drops the record (`drop`) or keeps the new object
  (`keep`), and state pltrs (Rust) changed by another way gives a new
  object, as matplotlib's code binds a new one.
* **pltrs (Rust) takes what the program changed** (`sync`: `State.take`
  for each record whose object no longer holds what it held) wherever it
  reads the artist: the artist's native object asks to be told before
  any native call taking it (``_native.artist.tell_before_calls``, see
  `tell`), which covers every getter, measure and hit test of the artist
  itself; and what reads the artists of a figure together tells them
  first (`tell_figure`: its layouts and draws through the figure's layout
  hook, its measures, the data limits recomputed, its pickles). Nothing
  is taken where matplotlib would not read the attribute: the data
  limits stay until ``relim``. What is taken makes nothing stale, as a
  change in place makes no artist of matplotlib's stale.

An artist without a record pays nothing: its native object asks for
nothing, and its figure has no hook for it. One with a record pays the
comparison of each of its objects per native call on it and per draw.
Records hold their objects as the artist would (they are in its
``__dict__``, left out of its pickle), and pltrs (Rust) holds only a weak
way back to the artist.
"""

import os
import sys
import weakref

import numpy as np

from ._native import artist as _artist
from .path import Path

#: The attribute of an artist holding its records, by state name.
RECORDS = "_own_state"

#: The attribute of a root figure holding its artists with records,
#: weakly.
OWNERS = "_own_state_artists"


def content(obj):
    """
    What *obj* holds, as a value comparing equal while *obj* is unchanged:
    the bytes and shape of an array (and its mask), the contents of a
    list, tuple or `.Path`; any other object (a number, a string, None)
    is its own.
    """
    if isinstance(obj, np.ndarray):
        if obj.dtype.hasobject:
            return (obj.shape, tuple(map(content, obj.ravel().tolist())))
        if isinstance(obj, np.ma.MaskedArray):
            return (obj.shape, obj.dtype.str, obj.data.tobytes(),
                    np.ma.getmaskarray(obj).tobytes())
        return (obj.shape, obj.dtype.str, obj.tobytes())
    if isinstance(obj, (list, tuple)):
        return (type(obj), *map(content, obj))
    if isinstance(obj, Path):
        return (content(obj._vertices), content(obj._codes), obj._interpolation_steps)
    return obj


class State:
    """
    One piece of an artist's state that matplotlib keeps as an object.

    *read* gives what pltrs (Rust) holds of it for an artist, as the
    object matplotlib's attribute would be: a new one, or the object the
    artist was given while pltrs (Rust) holds what it holds (`._values`).
    *take* hands pltrs (Rust) what such an object of the artist holds now;
    *key* gives what an object holds (`content` by default). *version*,
    if given, answers for an artist a value that changes whenever
    matplotlib's code binds a new object without the state changing (a
    draw mapping colours anew, say): the object handed out before is the
    artist's no more. *own*, if given, is told the artist and each object
    that becomes its own, and again once pltrs (Rust) took what a program
    changed in it. A class defines its states once, beside the getters
    handing them out.
    """

    __slots__ = ("name", "read", "take", "key", "version", "own")

    def __init__(self, name, read, take, key=content, version=None, own=None):
        self.name = name
        self.read = read
        self.take = take
        self.key = key
        self.version = version
        self.own = own

    def __repr__(self):
        return f"State({self.name!r})"


class _Record:
    """An object that is an artist's state: the *state* it is, what the
    object held when pltrs (Rust) last took it (*seen*), what pltrs (Rust)
    held then, as `State.read` gave it (*native*: None until it is read),
    and the state's version then (`State.version`)."""

    __slots__ = ("state", "obj", "seen", "native", "version")

    def __init__(self, state, obj, seen, native, version):
        self.state = state
        self.obj = obj
        self.seen = seen
        self.native = native
        self.version = version


class _Teller:
    """What an artist's native object calls before a native call takes it:
    the artist, held weakly, hands over what changed."""

    __slots__ = ("artist",)

    def __init__(self, artist):
        self.artist = weakref.ref(artist)

    def __call__(self):
        artist = self.artist()
        if artist is not None:
            try:
                sync(artist)
            except Exception:
                # State pltrs (Rust) cannot take stays the object's: the
                # draw fails on it, as matplotlib's does (`sync_figure`).
                pass


def _record(artist, state, obj, seen, native):
    """Make *obj* the object of *state* of *artist*."""
    records = artist.__dict__.get(RECORDS)
    if records is None:
        records = artist.__dict__[RECORDS] = {}
        tell(artist)
    version = None if state.version is None else state.version(artist)
    records[state.name] = _Record(state, obj, seen, native, version)
    if state.own is not None:
        state.own(artist, obj)


def tell(artist):
    """Have *artist*, which has records, told before every native call
    taking its native object (again once it has another: `._handles.adopt`),
    and its figure, if it is in one, tell it before it reads its artists."""
    _artist.tell_before_calls(artist._native, _Teller(artist))
    figure = getattr(artist._figure, "_figure", None)
    if figure is not None:
        owners = figure.__dict__.get(OWNERS)
        if owners is None:
            owners = figure.__dict__[OWNERS] = weakref.WeakSet()
            from .axes import _program_locators
            _program_locators.follow_own_state(figure)
        owners.add(artist)


#: Where the package's modules are, and those of them that only wrap
#: the functions they call (deprecations, aliases, keyword renaming).
_PACKAGE = os.path.dirname(__file__) + os.sep
_WRAPPERS = (os.path.join("_api", "__init__.py"), os.path.join("_api", "deprecation.py"),
             "_preprocess.py")


def _read_by_package(artist):
    """
    Whether the getter now handing out state of *artist* runs for the
    package's own work: a plotting method reading the lines it made
    (`~.Axes.boxplot`), a legend handler reading the artist it stands
    for, a draw. Nobody changes what the package reads, so it is given a
    new object and no record is made: only state a program (or a library)
    asked for is compared afterwards.

    What the program called tells: the outermost function of the package
    in the call, up to the program's own code. A member of the artist
    itself (``scatter.get_offsets()``, ``line._x``) or a getter of
    another object handing the artist's state on
    (``shadow.get_path()``, ``artist.get_transformed_clip_path_and_affine()``)
    is the program asking; anything else is the package working. Asked
    before an artist's first record of a state only.
    """
    frame = sys._getframe(2)
    outer = None
    while frame is not None and frame.f_code.co_filename.startswith(_PACKAGE):
        if not frame.f_code.co_filename.endswith(_WRAPPERS):
            outer = frame
        frame = frame.f_back
    if outer is None or outer.f_code.co_name.startswith("get_"):
        return False
    return outer.f_locals.get("self") is not artist


def hand_out(artist, state):
    """
    The object of *state* of *artist*: the one handed out or kept before
    while pltrs (Rust) holds what it held then, else a new one, which the
    artist keeps from now on. An answer that is no object a program could
    change (None) is kept by nobody, and so is what the package's own code
    reads (`_read_by_package`).
    """
    records = artist.__dict__.get(RECORDS)
    if records:
        # What the program changed is pltrs (Rust)'s before it is read
        # (also where the read takes another native object than the
        # artist's: a contour set's own).
        _artist.touch(artist._native)
    native = state.read(artist)
    record = None if records is None else records.get(state.name)
    if native is None:
        if record is not None:
            del records[state.name]
        return None
    if record is not None and native is record.obj:
        return native
    if record is None and _read_by_package(artist):
        return native
    key = state.key(native)
    if (record is not None and record.state is state and (
            state.version is None or record.version == state.version(artist))):
        if record.native is None:
            record.native = key
        if record.native == key:
            return record.obj
    _record(artist, state, native, key, key)
    return native


def keep(artist, state, obj):
    """Make *obj*, which *artist* was just given and pltrs (Rust) just
    took, the object of *state* of *artist*, as matplotlib keeps what it
    is given there."""
    _record(artist, state, obj, state.key(obj), None)


def hand_out_kept(artist, state, obj):
    """*obj*, the object of *state* that *artist* keeps in Python itself
    (what pltrs (Rust) draws is made of it), handed to the program: the
    artist's own from now on, while it keeps it."""
    record = artist.__dict__.get(RECORDS, {}).get(state.name)
    if record is None or record.obj is not obj:
        keep(artist, state, obj)
    return obj


def drop(artist, name):
    """Forget the object of the state *name* of *artist*, if it keeps one:
    the state was set anew, and matplotlib's artist has another object."""
    records = artist.__dict__.get(RECORDS)
    if records is not None:
        records.pop(name, None)


def kept(artist, name):
    """The object of the state *name* of *artist*, if it keeps one, else
    None, pltrs (Rust) not asked."""
    record = artist.__dict__.get(RECORDS, {}).get(name)
    return None if record is None else record.obj


def sync(artist):
    """Hand pltrs (Rust) each object of *artist* whose content changed
    since pltrs (Rust) last took it."""
    taken = False
    for record in list(artist.__dict__.get(RECORDS, {}).values()):
        seen = record.state.key(record.obj)
        if seen != record.seen:
            record.state.take(artist, record.obj)
            record.seen = seen
            record.native = None
            taken = True
            if record.state.own is not None:
                record.state.own(artist, record.obj)
    if taken and "_mirror" in artist.__dict__:
        # A mirror (an Axes' background patch of the program's, say) gives
        # the part it stands for what it took, as its setters do.
        artist._mirror.push(artist)


def sync_figure(figure):
    """`sync` every artist with records of the root *figure* that is in
    it, while pltrs (Rust) lays the figure out to draw it."""
    for artist in list(figure.__dict__.get(OWNERS, ())):
        if artist._figure is figure and not artist.__dict__.get("_decoupled"):
            sync(artist)


def tell_figure(figure):
    """Tell every artist with records of the figure *figure* is of, as a
    native call taking the artist tells it: before pltrs (Rust) reads the
    figure's artists (to measure them, to recompute data limits, to
    pickle them)."""
    owners = getattr(figure, "_figure", figure).__dict__.get(OWNERS)
    if owners:
        for artist in list(owners):
            _artist.touch(artist._native)


#: The values a mappable maps to colours (matplotlib's ``_A``), which
#: every draw maps anew: the array the artist keeps in Python, handed to
#: pltrs (Rust) again as `~.ScalarMappable.set_array` hands it.
ARRAY = State("array", lambda mappable: mappable._A,
              lambda mappable, array: mappable._array_set(),
              lambda array: content(np.ma.asarray(array)))


def pickling(artist, state, figure):
    """Before *artist* is pickled as *state* (its ``__dict__``, copied):
    tell it and the artists of its root *figure* (None outside any), so
    that the pickle holds what the program changed, and leave the records
    out of *state* (the artist read back hands out objects of its own)."""
    if state.pop(RECORDS, None) is not None:
        _artist.touch(artist._native)
    if figure is not None:
        state.pop(OWNERS, None)
        tell_figure(figure)


#: The paths artists keep as their own while they live, by a digest of
#: their content.
_paths_by_content = weakref.WeakValueDictionary()


def _digest(vertices, codes):
    """A digest of the content of a path of *vertices* and *codes*."""
    vertices = np.asarray(vertices, dtype=float)
    return (vertices.shape, hash(vertices.tobytes()),
            None if codes is None else hash(np.asarray(codes).tobytes()))


#: The attribute of a path holding the artists that keep it as their own,
#: weakly; copies and pickles of the path, paths of their own, leave it
#: out (`.Path.__getstate__`).
PATH_OWNERS = "_owned_by"


def note_paths(artist, paths):
    """
    Note *paths* (a sequence of paths, or one path), which *artist* hands
    out or keeps as its own: to be found again by their content
    (`held_path`), and to tell the artist at once when their vertices,
    their codes or their interpolation steps are assigned
    (`path_assigned`).
    """
    for path in ([paths] if isinstance(paths, Path) else paths):
        _paths_by_content[_digest(path.vertices, path.codes)] = path
        owners = path.__dict__.get(PATH_OWNERS)
        if owners is None:
            owners = path.__dict__[PATH_OWNERS] = weakref.WeakSet()
        owners.add(artist)


def path_assigned(path):
    """
    Hand pltrs (Rust) *path*, whose vertices, codes or interpolation steps
    a program just assigned (``path.vertices = vertices``), for each artist
    keeping it as its own. An assignment says that the path changed, so
    nothing waits for the next read of the artist: code of the program's
    that pltrs (Rust) calls while it draws moves paths so (astropy's frames
    move the outline their Axes clips to as they are drawn), where no
    artist is told.
    """
    for artist in list(path.__dict__[PATH_OWNERS]):
        if "_native" in artist.__dict__:
            sync(artist)


def held_path(vertices, codes, steps):
    """
    The path an artist keeps as its own with *vertices*, *codes* and
    *steps* interpolation steps, if one lives, else None. matplotlib's
    collections hand a non-affine transform of the program's their own
    paths when they draw, and programs keep what it computed by path
    (cartopy's ``InterProjectionTransform`` caches the projected path of
    each source path); pltrs (Rust) maps the paths it draws through such a
    transform by their vertices and codes, so the path the artist keeps is
    found again by its content, and is what the transform gets.
    """
    path = _paths_by_content.get(_digest(vertices, codes))
    if (path is None or path._interpolation_steps != steps
            or not np.array_equal(path.vertices, vertices)
            or not np.array_equal(path.codes, codes)):
        return None
    return path
