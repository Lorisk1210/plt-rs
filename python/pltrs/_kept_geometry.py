"""
The geometry of patches as matplotlib keeps it: as given.

matplotlib's patch setters (``Ellipse.set_center``, ``Wedge.set_radius``,
``Rectangle.set_width``, ...) keep what they are given, and the patch
computes with it when it is drawn or measured (``_recompute_transform``,
``_recompute_path``, ``get_patch_transform``): a centre of no numbers or
an infinite angle fails then, with the error Python's or numpy's
arithmetic raises, and products that are no numbers warn as numpy's do.
pltrs (Rust) holds numbers. A patch hands it the numbers it is given
(`hold`) and keeps what pltrs (Rust) cannot hold (`._values.keep`, pltrs
(Rust) keeping the number it had); while any field is kept or not finite
(or the class' arithmetic warns on it), the patch recomputes its geometry
as matplotlib does wherever matplotlib's draws and measures do (`watch`,
`._unusable`), failing and warning as it does.
"""

import numpy as np

from . import _unusable, _values, units

#: The types of values that are never in units, which the callers of
#: `._unit_values` on the plain path test as a set of types, without a call.
PLAIN_TYPES = frozenset({int, float, bool, str, np.float64, np.int64})


def real(value):
    """*value* as the number pltrs (Rust) holds: a real number (numpy's
    too), no string (which matplotlib converts as a category) and no
    sequence."""
    if np.ndim(value) or not units._is_natively_supported(value):
        raise TypeError(f"{value!r} is no number")
    return float(value)


def floated(value):
    """
    *value* as the number matplotlib's arithmetic with it ends in, for the
    fields it computes with as they are instead of converting them with an
    Axes' units (a wedge's centre, a regular polygon's, a fancy box's): a
    real number (`real`), or an object of the program's whose class Python's
    `float` takes (a quantity defining ``__float__``), which numpy floats
    where the vertices become an array of floats. numpy's own scalars (a
    ``datetime64``) fail in its arithmetic before, as here.
    """
    try:
        return real(value)
    except TypeError:
        if (isinstance(value, (str, np.generic)) or np.ndim(value)
                or not hasattr(type(value), "__float__")):
            raise
    try:
        return float(value)
    except (TypeError, ValueError):
        raise TypeError(f"{value!r} is no number") from None


def floated_point(value):
    """The point *value* as pltrs (Rust) holds it, its first two items as
    `floated` numbers."""
    return floated(value[0]), floated(value[1])


def in_units(value):
    """Whether *value* is a quantity in units a converter of
    `.units.registry` converts (``basic_units``' ``TaggedValue``), not a
    number, nor a category name."""
    if isinstance(value, str) or units._is_natively_supported(value):
        return False
    return units.registry.get_converter(value) is not None


def point(value):
    """The point *value* as pltrs (Rust) holds it: its first two items as
    numbers (`real`), all matplotlib reads of it."""
    return real(value[0]), real(value[1])


def same_point(value, native):
    """Whether the point *value* is the point pltrs (Rust) holds as
    *native*: its first two items (`point`), whatever follows them."""
    try:
        return len(value) >= 2 and _values._same(value[:2], native)
    except TypeError:
        return False


def hold(artist, name, value, convert, held):
    """
    *value*, given as *artist*'s field *name*, as pltrs (Rust) holds it
    (*convert(value)*), remembered as given; when pltrs (Rust) cannot hold
    it, kept as given and *held*, what pltrs (Rust) holds now, instead.
    """
    try:
        number = convert(value)
    except (TypeError, ValueError, LookupError):
        _values.keep(artist, name, value)
        return held
    _values.remember(artist, name, value)
    return number


def watch(artist, names, numbers, recompute, *, suspect=False, cached=False):
    """
    Have *artist* recompute its geometry as matplotlib does (*recompute()*)
    where matplotlib's draws and measures it, while one of its fields
    *names* is kept (`hold`), a number pltrs (Rust) holds for it
    (*numbers*) is infinite, or the class finds its numbers *suspect*
    (its arithmetic warns on them): so that it fails and warns as
    matplotlib's does. A class whose matplotlib keeps what it computed
    (*cached*: a path made again only after a change) recomputes until it
    succeeds once. As before otherwise. Whether it does.
    """
    watched = (suspect or any(_values.is_kept(artist, name) for name in names)
               or not np.isfinite(np.asarray(numbers, dtype=float)).all())
    if watched:
        _unusable.keep(artist, "geometry", None,
                       lambda _: _recomputed(artist, recompute, cached))
    else:
        _unusable.drop(artist, "geometry")
    return watched


def _recomputed(artist, recompute, cached):
    """*recompute()*, *artist*'s geometry no longer recomputed (`watch`)
    once it succeeded if *cached*."""
    recompute()
    if cached:
        _unusable.drop(artist, "geometry")
