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
comparison of each of its objects per native call that reads it and per
draw, with three things keeping that small:

* Native functions that read none of this state tell nobody (the
  extension lists them, ``_native.artist.passing_by()``): a program that
  holds a scatter's offsets and sets its line width a thousand times
  pays for no comparison until something reads the scatter.
* An array nothing but its record holds is not compared (`_Record.settled`):
  no code has it to change it. ``x = scatter.get_offsets()`` used once
  costs one comparison in all.
* A state whose array pltrs (Rust) holds as it is compares the two where
  they are (`State.same`): no copy of the array is made or kept.

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
#: The attribute of a root figure holding what it asks besides (`follow`).
FOLLOWED = "_own_state_followed"


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


def comparable(obj):
    """Whether *obj* is an array pltrs (Rust) can compare with its own
    numbers where it is (`State.same`): exactly numpy's, of float64, in
    row-major order."""
    return type(obj) is np.ndarray and obj.dtype.char == "d" and obj.flags.c_contiguous


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
    changed in it. *same*, if given, tells for an artist and an object
    whether pltrs (Rust) holds exactly what the object holds, comparing
    the two where they are (None for an object it cannot compare so);
    *compares* then says for which artists and objects it does
    (`comparable` by default). Such an object has no key kept, and is
    compared with pltrs (Rust)'s own copy instead of with what it held
    before. That takes a state nothing writes natively but its *take* and
    setters that forget the record (`drop`): pltrs (Rust) holding
    something else then means that the program changed the object. A
    class defines its states once, beside the getters handing them out.
    """

    __slots__ = ("name", "read", "take", "key", "version", "own", "same", "compares")

    def __init__(self, name, read, take, key=content, version=None, own=None, same=None,
                 compares=None):
        self.name = name
        self.read = read
        self.take = take
        self.key = key
        self.version = version
        self.own = own
        self.same = same
        self.compares = (lambda artist, obj: comparable(obj)) if compares is None else compares

    def seen(self, artist, obj):
        """What a new record of *obj*, which pltrs (Rust) holds now, has
        seen of it: `NATIVELY` where the two are compared as they are (the
        next hand-over finds out whether pltrs (Rust) holds it bit for
        bit, and hands it over again otherwise), else its key."""
        if self.same is not None and self.compares(artist, obj):
            return NATIVELY
        return self.key(obj)

    def __repr__(self):
        return f"State({self.name!r})"


class _Natively:
    """What a record has seen of an object that is compared with pltrs
    (Rust)'s own copy (`State.same`): nothing kept."""

    def __repr__(self):
        return "NATIVELY"


#: See `_Natively`.
NATIVELY = _Natively()


class _Record:
    """
    An object that is an artist's state: the *state* it is, what the
    object held when pltrs (Rust) last took it (*seen*; `NATIVELY` for an
    object compared with pltrs (Rust)'s own copy), what pltrs (Rust) held
    then, as `State.read` gave it (*native*: None until it is read), and
    the state's version then (`State.version`).

    *settled* says that the object was compared at a moment when nothing
    but this record held it and that nobody was given it since (`obj`
    says so no more: whoever reads it may keep it), and *verified* that
    the last hand-over found pltrs (Rust) holding exactly what the object
    holds, for the getter that asked for it.
    """

    __slots__ = ("state", "_obj", "seen", "native", "version", "settled", "verified")

    def __init__(self, state, obj, seen, native, version):
        self.state = state
        self._obj = obj
        self.seen = seen
        self.native = native
        self.version = version
        self.settled = False
        self.verified = False

    @property
    def obj(self):
        """The object, about to be given to somebody."""
        self.settled = False
        return self._obj


def _counts_tell():
    """Whether reference counts tell that nothing holds an object: not
    where the interpreter runs without its GIL, which splits them between
    threads and defers some."""
    return getattr(sys, "_is_gil_enabled", lambda: True)()


def _references(array):
    """How many references `sys.getrefcount` counts to *array*, given as
    an attribute read off what holds it: counted in a function of one
    argument, as `_alone` counts."""
    return sys.getrefcount(array)


def _count_unheld():
    """The count `_references` gives for an object that only one object
    holds, measured with an array of this module's own that a record
    holds (what the interpreter itself holds during the call differs by
    version); 0, which no count goes down to, where counts tell nothing
    (`_counts_tell`)."""
    if not _counts_tell():
        return 0
    probe = _Record(None, np.zeros(1), None, None, None)
    return _references(probe._obj)


#: See `_count_unheld`.
_UNHELD = _count_unheld()


def _bare(array):
    """Whether *array* is exactly numpy's array of numbers that owns them:
    whatever shares its numbers holds it (a view by its base, a buffer by
    its exporter)."""
    return type(array) is np.ndarray and array.base is None and not array.dtype.hasobject


def _alone(array):
    """Whether the one object that holds *array* (given as an attribute
    read off it) is all that can reach it: no other reference, and no
    weak one."""
    return sys.getrefcount(array) <= _UNHELD and not weakref.getweakrefcount(array)


def _only_recorded(record):
    """
    Whether nothing but *record* holds its object, so that no code can
    change it: an array of numbers that owns them (`_bare`), or a masked
    array of such numbers and such a mask that nothing else holds, with
    no reference to it but the record's (`_alone`). Anything else may be
    reached another way and is compared: a list by its items, a path by
    its vertices, a view by its base. (Code that kept only the address of
    the numbers could still write there, as under matplotlib, whose artist
    keeps the array alive as the record does; that write is not seen.)
    """
    if not _alone(record._obj):
        return False
    kind = type(record._obj)
    if kind is np.ndarray:
        return _bare(record._obj)
    if kind is not np.ma.MaskedArray:
        return False
    # (A masked array is a view of its numbers; its mask is another array,
    # or the one constant for no mask.)
    return (_bare(record._obj.base) and _alone(record._obj.base)
            and (record._obj._mask is np.ma.nomask
                 or (_bare(record._obj._mask) and _alone(record._obj._mask))))


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
    if not records:
        records = artist.__dict__.setdefault(RECORDS, {})
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
    record = None
    if records:
        record = records.get(state.name)
        if record is not None:
            if (record.settled and record.seen is NATIVELY and record.state is state
                    and _only_recorded(record)):
                # Nothing held the object since pltrs (Rust) was found to
                # hold what it holds: it is the artist's still, unread.
                return record.obj
            # The object is about to be given out: compared now, whoever
            # held it before.
            record.settled = record.verified = False
        # What the program changed is pltrs (Rust)'s before it is read
        # (also where the read takes another native object than the
        # artist's: a contour set's own).
        _artist.touch(artist._native)
        record = records.get(state.name)
    natively = record is not None and record.seen is NATIVELY and record.state is state
    if natively and (record.verified or state.same(artist, record._obj)):
        # pltrs (Rust) holds what the object holds (the hand-over just
        # found so, or this call is made inside a native call, where
        # nothing is handed over): nothing to read.
        return record.obj
    native = state.read(artist)
    if native is None:
        if record is not None:
            del records[state.name]
        return None
    if record is not None and native is record._obj:
        return record.obj
    if record is None and _read_by_package(artist):
        return native
    seen = state.seen(artist, native)
    if seen is NATIVELY:
        _record(artist, state, native, seen, None)
        return native
    if (record is not None and not natively and record.state is state and (
            state.version is None or record.version == state.version(artist))):
        if record.native is None:
            record.native = seen
        if record.native == seen:
            return record.obj
    _record(artist, state, native, seen, seen)
    return native


def keep(artist, state, obj):
    """Make *obj*, which *artist* was just given and pltrs (Rust) just
    took, the object of *state* of *artist*, as matplotlib keeps what it
    is given there."""
    _record(artist, state, obj, state.seen(artist, obj), None)


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


def follow(obj, figure):
    """
    Have the root *figure* ask *obj* for what the program changed in place
    in it (``obj._take_changed_in_place()``) where it tells its artists
    (`sync_figure`, `tell_figure`). So are followed what is no artist with
    records: an Axes whose transform trees are built, for the boxes they
    rest on (its view and data limits: ``ax.viewLim.get_points()[1, 1] =
    top``; it also looks itself before its limits are read, `take_leaves`),
    and a locator that keeps an array the program has (`.FixedLocator`).
    No native call asks for them.
    """
    followed = figure.__dict__.get(FOLLOWED)
    if followed is None:
        followed = figure.__dict__[FOLLOWED] = weakref.WeakSet()
        from .axes import _program_locators
        _program_locators.follow_own_state(figure)
    followed.add(obj)


def take_leaves(artist):
    """Hand pltrs (Rust) the boxes of *artist* whose points the program
    changed in the arrays themselves (`follow`)."""
    for watcher in artist.__dict__.get("_leaf_watchers", ()):
        watcher.take_changed_in_place()


def _changed(artist, record):
    """What *record* is to see of its object from now on if the object
    no longer holds what pltrs (Rust) last took, else None."""
    state = record.state
    if record.seen is NATIVELY:
        record.verified = bool(state.same(artist, record._obj))
        return None if record.verified else NATIVELY
    seen = state.key(record._obj)
    return None if seen == record.seen else seen


def sync(artist):
    """Hand pltrs (Rust) each object of *artist* whose content changed
    since pltrs (Rust) last took it, and the limits of an Axes changed in
    the arrays of their boxes (`follow`). An object nothing but its
    record holds since it was last compared is left alone
    (`_only_recorded`)."""
    for watcher in artist.__dict__.get("_leaf_watchers", ()):
        watcher.take_changed_in_place()
    taken = False
    for record in list(artist.__dict__.get(RECORDS, {}).values()):
        if record.settled and _only_recorded(record):
            continue
        seen = _changed(artist, record)
        if seen is not None:
            state = record.state
            state.take(artist, record._obj)
            if seen is NATIVELY:
                # (An object pltrs (Rust) did not take as it is has a key
                # from now on, as one of a state without `State.same`.)
                record.verified = bool(state.same(artist, record._obj))
                if not record.verified:
                    seen = state.key(record._obj)
            record.seen = seen
            record.native = None
            taken = True
            if state.own is not None:
                state.own(artist, record._obj)
        record.settled = _only_recorded(record)
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
    for followed in list(figure.__dict__.get(FOLLOWED, ())):
        followed._take_changed_in_place()


def tell_figure(figure):
    """Tell every artist with records of the figure *figure* is of, as a
    native call taking the artist tells it: before pltrs (Rust) reads the
    figure's artists (to measure them, to recompute data limits, to
    pickle them)."""
    figure = getattr(figure, "_figure", figure)
    owners = figure.__dict__.get(OWNERS)
    if owners:
        for artist in list(owners):
            _artist.touch(artist._native)
    for followed in list(figure.__dict__.get(FOLLOWED, ())):
        followed._take_changed_in_place()


def before_measure(figure):
    """`tell_figure` if *figure* (a root figure, or None) tells any artist:
    before pltrs (Rust) measures it or an Axes of it with no draw, as
    matplotlib's measures read the artists' attributes."""
    if figure is not None and (OWNERS in figure.__dict__ or FOLLOWED in figure.__dict__):
        tell_figure(figure)


#: The values a mappable maps to colours (matplotlib's ``_A``), which
#: every draw maps anew: the array the artist keeps in Python, handed to
#: pltrs (Rust) again as `~.ScalarMappable.set_array` hands it.
ARRAY = State("array", lambda mappable: mappable._mapped,
              lambda mappable, array: mappable._array_set(),
              lambda array: content(np.ma.asarray(array)))


#: The clip box an artist was given, a `.Bbox` of the program's that it
#: may move (matplotlib's ``clipbox``): handed to pltrs (Rust) again, as
#: `~.Artist.set_clip_box` hands it, when its points changed.
CLIP_BOX = State("clip_box", lambda artist: artist._clipbox,
                 lambda artist, box: artist._take_clip_box(box),
                 lambda box: box.get_points().tobytes())


def pickling(artist, state, figure):
    """Before *artist* is pickled as *state* (its ``__dict__``, copied):
    tell it and the artists of its root *figure* (None outside any), so
    that the pickle holds what the program changed, and leave the records
    out of *state* (the artist read back hands out objects of its own)."""
    if state.pop(RECORDS, None) is not None:
        _artist.touch(artist._native)
    if figure is not None:
        state.pop(OWNERS, None)
        state.pop(FOLLOWED, None)
        tell_figure(figure)


#: The attribute of an artist holding the lists of artists it handed out
#: (`same_list`), by name.
LISTS = "_own_lists"


def same_list(owner, name, listed, make):
    """
    The list of artists *owner* hands out as *name*: the list handed out
    before, as matplotlib's attribute is one list for the life of its
    artist, else the one *make* makes of what *listed* gives (the artists
    pltrs (Rust) lists now). The list is of a class whose changes in place
    add and remove artists and which brings itself up to what *listed*
    gives whenever it is used (`._figure_artists.DrawnList`), so nothing
    is compared at a draw and nothing happens while no program uses it.
    """
    lists = owner.__dict__.setdefault(LISTS, {})
    found = lists.get(name)
    if found is None:
        found = lists[name] = make(listed())
        found._listed = listed
    return found


#: The attribute of an object holding what `kept_once_read` attributes
#: read, by name: the object and what told it apart then.
READ = "_kept_once_read"


def kept_once_read(getter=None, *, told_by=None):
    """
    A property for an attribute matplotlib makes once and keeps as an
    object it draws nothing from (a contour set's ``levels``, its lists of
    labels): what *getter* computes when the attribute is first read is
    the object's own from then on, so that a program changing it in place
    reads the change back, in a pickle too, and assigning to it keeps what
    is assigned. No record is kept and nothing is compared: pltrs (Rust)
    reads none of it. *told_by* gives, for an object, a value that changes
    when matplotlib's code changes the attribute in place (a contour set
    labelled again gets more labels): a list read before is then filled
    anew, the same list still.
    """
    if getter is None:
        return lambda getter: kept_once_read(getter, told_by=told_by)
    name = getter.__name__

    def get(obj):
        read = obj.__dict__.setdefault(READ, {})
        token = None if told_by is None else told_by(obj)
        found = read.get(name)
        if found is None:
            found = read[name] = [getter(obj), token]
        elif found[1] != token:
            found[1] = token
            if isinstance(found[0], list):
                found[0][:] = getter(obj)
            else:
                found[0] = getter(obj)
        return found[0]

    def set_(obj, value):
        token = None if told_by is None else told_by(obj)
        obj.__dict__.setdefault(READ, {})[name] = [value, token]

    return property(get, set_, doc=getter.__doc__)


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
