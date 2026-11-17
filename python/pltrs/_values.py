"""
Values with the types matplotlib gives them.

pltrs (Rust) holds numbers as its own types (``f64``), while matplotlib
answers with the Python objects it happens to keep: the numpy floats of
the arrays its boxes hold (``ax.get_xlim()``), and very often the very
object the program gave (``line.set_zorder(5)`` reads back as the int
``5``). The helpers here turn what pltrs (Rust) answers into what
matplotlib answers:

* `float64s` gives the numpy floats of a box's array;
* `remember` and `given` keep a property as the program gave it: a setter
  remembers the value matplotlib would keep, and its getter returns that
  value as long as pltrs (Rust) still holds what it was converted to, else
  what pltrs (Rust) holds now (a value pltrs (Rust) changed itself);
* `keep` keeps a value matplotlib stores unchecked that pltrs (Rust)
  cannot hold (a string zorder, a hatch line width of None): the getter
  returns it until the property is set again;
* `assign`, `assigned`, `take` and `assignable` keep what a program
  assigns to an instance attribute of matplotlib's that pltrs (Rust)
  holds and matplotlib's code reads again only later, or never: a
  `Wedge`'s fields, read by its next setter (`take`), a `StepPatch`'s
  orientation, read by ``set_data``, a contour set's extremes, read only
  while it is made. An attribute whose assignment has an effect
  matplotlib's later code sees (its draws, its measures) takes a setter:
  ``assignable(getter, setter)`` assigns through it, as the private
  plain fields of `_artist_private` do with their public setters
  (``Collection._facecolors``) and one matplotlib's draws read does by
  the property's own setter (`RegularPolygon.xy`).
"""

from typing import NamedTuple

import numpy as np

#: No value given (`given`'s default).
NONE_GIVEN = object()


class InUnits(NamedTuple):
    """A value in units the program gave (a date, a quantity), which pltrs
    (Rust) was handed converted, as *number*: `given` answers *value*
    while pltrs (Rust) holds *number*, and whatever it holds while
    *number* is None, no Axes having converted the value yet (see
    `._unit_values`)."""

    value: object
    number: object


def float64s(values):
    """*values* as a tuple of numpy floats, as matplotlib reads its limits
    off a box's array (``tuple(self.viewLim.intervalx)``)."""
    return tuple(np.asarray(values, dtype=float))


def remember(artist, name, value):
    """Keep *value* as what *artist*'s property *name* reads back as."""
    artist.__dict__.setdefault("_given_values", {})[name] = value
    artist.__dict__.get("_unheld_values", set()).discard(name)


def keep(artist, name, value):
    """Keep *value*, which pltrs (Rust) cannot hold, as what *artist*'s
    property *name* reads back as, whatever pltrs (Rust) holds."""
    remember(artist, name, value)
    artist.__dict__.setdefault("_unheld_values", set()).add(name)


def is_kept(artist, name):
    """Whether *artist*'s property *name* holds a value pltrs (Rust) could
    not (`keep`)."""
    return name in getattr(artist, "__dict__", {}).get("_unheld_values", ())


def remember_all(artist, values, names):
    """`remember` those of the properties *values* (a dict) has among
    *names*."""
    for name in names:
        if name in values:
            remember(artist, name, values[name])


def inherit(artist, source):
    """Let *artist* read back the properties *source* was given
    (`remember`) that it was not given itself, as the artists matplotlib
    makes from others keep their very objects (``update_from``)."""
    given = getattr(source, "__dict__", {}).get("_given_values")
    if given:
        own = artist.__dict__.setdefault("_given_values", {})
        for name, value in given.items():
            own.setdefault(name, value)


def given_fields(artist, names, native):
    """The fields *native* (a tuple) of *artist*, named *names*, each as
    `given` reads it."""
    return tuple(given(artist, name, value) for name, value in zip(names, native))


def given(artist, name, native, default=NONE_GIVEN, same=None):
    """
    *artist*'s property *name* as the program gave it (`remember`), or as
    *default* (matplotlib's default, typed as matplotlib types it) when
    none was given, while it still equals *native*, what pltrs (Rust)
    holds (as *same* compares, by default as numbers); else *native*.
    """
    value = artist.__dict__.get("_given_values", {}).get(name, default)
    if is_kept(artist, name):
        return value
    if isinstance(value, InUnits):
        return (value.value if value.number is None or _same(value.number, native)
                else native)
    if value is NONE_GIVEN or not (same or _same)(value, native):
        return native
    return value


def given_flag(artist, name, native):
    """*artist*'s flag *name* (``visible``, ``clip_on``, ...) as the
    program gave it (`remember`), matplotlib keeping the object (``1``,
    ``'False'``, None), while pltrs (Rust) still holds its truth value
    *native*; else *native*. ``snap`` and ``annotation_clip`` have a
    third state, None (auto), which only None stands for."""
    return given(artist, name, native,
                 same=_same_snap if name in _THREE_STATES else _same_truth)


#: The flags with a third state, None (auto).
_THREE_STATES = frozenset({"snap", "annotation_clip"})


def given_text(artist, name, native):
    """*artist*'s text property *name* (``gid``, ``url``) as the program
    gave it (`remember`), matplotlib keeping the object (an int), while
    pltrs (Rust) still holds its `str` *native*; else *native*."""
    return given(artist, name, native, same=_same_text)


def same_cycled(value, native):
    """Whether *native* is *value*, an array cycled over the items of
    *native* (one width given for every level, say), its masked or NaN
    items NaN there (as pltrs (Rust) holds them)."""
    value, native = np.ma.atleast_1d(value), np.atleast_1d(native)
    if value.ndim != 1 or native.ndim != 1 or not value.size or not native.size:
        return False
    if np.ma.is_masked(value):
        value = value.astype(float).filled(np.nan)
    return bool(np.array_equal(np.resize(np.asarray(value), native.size), native,
                               equal_nan=value.dtype.kind in "fc"))


def same_color(a, b):
    """Whether *a* and *b* are one colour (NaN components alike: matplotlib
    takes a colour of NaNs, which mne gives a channel without a place)."""
    from . import colors
    try:
        first, second = colors.to_rgba(a), colors.to_rgba(b)
    except (TypeError, ValueError):
        return False
    return first == second or np.array_equal(first, second, equal_nan=True)


def prop_cycle(axes):
    """The property cycle of *axes*: the one set on it, else
    ``axes.prop_cycle``'s."""
    from . import rcParams
    cycle = (None if axes is None else axes.__dict__.get("_given_values", {}).get("prop_cycle"))
    return rcParams['axes.prop_cycle'] if cycle is None else cycle


def cycle_color(axes, native):
    """The colour of the property cycle of *axes* that *native* is, as the
    cycle holds it (matplotlib's ``tab10`` holds triples), or `given`'s
    default for none."""
    for color in prop_cycle(axes).by_key().get("color", []):
        if same_color(color, native):
            return color
    return NONE_GIVEN


def cycle_number(axes, name, native):
    """The value of the property cycle of *axes* for the number property
    *name* (``alpha``) that *native* is, as the cycle holds it (a cycler of
    an array holds numpy floats), or `given`'s default for none."""
    if native is None:
        return NONE_GIVEN
    for value in prop_cycle(axes).by_key().get(name, []):
        if _same(value, native):
            return value
    return NONE_GIVEN


def _same_truth(value, native):
    """Whether *value* has the truth value pltrs (Rust) holds as
    *native*."""
    try:
        return bool(value) == bool(native)
    except ValueError:  # an array of several items has no truth value
        return False


def _same_snap(value, native):
    """Whether *value* is the snapping pltrs (Rust) holds as *native*:
    None (auto) only for None, else a truth value."""
    if value is None or native is None:
        return value is native
    return _same_truth(value, native)


def _same_text(value, native):
    """Whether *value* is the text pltrs (Rust) holds as *native* (None
    for none)."""
    return (None if value is None else str(value)) == native


def _same(value, native):
    """Whether *value* is what pltrs (Rust) holds as *native*."""
    if value is None or native is None:
        return value is native
    if isinstance(value, str) or isinstance(native, str):
        return value == native
    try:
        a = np.asarray(value, dtype=float)
        b = np.asarray(native, dtype=float)
    except (TypeError, ValueError):
        return False
    return a.shape == b.shape and bool(np.array_equal(a, b, equal_nan=True))


#: The attribute of an object holding what was assigned, by name.
_ASSIGNED = "_assigned_attributes"


def assign(obj, name, value):
    """Keep *value* as what was assigned to *obj*'s attribute *name*."""
    obj.__dict__.setdefault(_ASSIGNED, {})[name] = value


def assigned(obj, name, held):
    """What was assigned to *obj*'s attribute *name*, else *held()*."""
    found = obj.__dict__.get(_ASSIGNED)
    if found is not None and name in found:
        return found[name]
    return held()


def take(obj, *names):
    """What was assigned to *obj*'s attributes *names*, by name, forgotten
    by *obj* (the method reading them has)."""
    found = obj.__dict__.get(_ASSIGNED)
    if not found:
        return {}
    return {name: found.pop(name) for name in names if name in found}


def reassignable(getter):
    """An attribute matplotlib keeps as a plain field that pltrs answers
    from its own state (`assignable`'s decorator): the object *getter*
    gives assigned back changes nothing (and forgets an earlier
    assignment), another value reads back as assigned, which only
    matplotlib's later reads of the attribute see."""
    name = getter.__name__

    def get(obj):
        # `assigned`, in line: reading the attribute while a figure is
        # built costs this one call beside its getter's.
        found = obj.__dict__.get(_ASSIGNED)
        if found is not None and name in found:
            return found[name]
        return getter(obj)

    def set_(obj, value):
        if value is getter(obj):
            take(obj, name)
        else:
            assign(obj, name, value)

    return property(get, set_, doc=getter.__doc__)


def assignable(getter, setter=None):
    """An attribute a program assigns: read through *getter*, assigned
    through *setter* (its effect where matplotlib reads the attribute
    later), or, without one, as a property decorator, *getter*'s attribute
    (named as it is) reads back what was assigned to it, which only
    matplotlib's later reads of the attribute see, else what *getter*
    gives."""
    if setter is not None:
        return property(getter, setter, doc=getter.__doc__)
    name = getter.__name__

    def get(obj):
        # `assigned`, in line: reading the attribute while a figure is
        # built costs this one call beside its getter's.
        found = obj.__dict__.get(_ASSIGNED)
        if found is not None and name in found:
            return found[name]
        return getter(obj)

    def set_(obj, value):
        assign(obj, name, value)

    return property(get, set_, doc=getter.__doc__)
