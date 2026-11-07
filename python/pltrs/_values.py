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
  returns it until the property is set again.
"""

import numpy as np

#: No value given (`given`'s default).
NONE_GIVEN = object()


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
    if value is NONE_GIVEN or not (same or _same)(value, native):
        return native
    return value


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
