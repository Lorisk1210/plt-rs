"""
Values in units that artists keep as given.

matplotlib's patches, texts and annotations keep the dates, quantities
and category names a program gives them, and convert them with the units
of their Axes each time they draw or measure (``Rectangle._convert_units``,
``Ellipse._recompute_transform``, ``Text.draw``, ``_get_xy``): after an
axis changes units (``set_units``, a new converter), they are drawn,
measured and counted in the data limits in the new ones. pltrs (Rust)
holds numbers. Such an artist keeps the values as given (in the class'
own fields, see `geometry`) and hands pltrs (Rust) their conversion when
it joins an Axes (its ``_convert_units_for``); it then `follow`s the units
of the Axes' axes, and an axis whose units change has it convert them
again (`units_changed`) before it tells its callbacks, so that the Axes'
data limits are recomputed from the new numbers, as matplotlib's
``relim`` measures the patch converted anew.

Only artists given values in units take part: nothing is kept, and no
axis holds a set, for any other. The module is imported where it is first
needed: the callers on the plain path tell plain values first by their
types (`._kept_geometry.PLAIN_TYPES`), without a call.
"""

import weakref

from . import _kept_geometry, _unusable, _values


def any_in_units(values):
    """Whether any of *values* is in units (`._kept_geometry.in_units`)."""
    return any(map(_kept_geometry.in_units, values))


def keep_geometry(artist, fields):
    """Keep *fields*, the fields of *artist*'s geometry (a dict), as given:
    some of them are in units. matplotlib keeps every field as given and
    converts them together (a rectangle's far edge as ``x + width``), so
    those in numbers are kept beside those in units. They read back as
    given meanwhile, before any Axes converts them."""
    artist._unit_geometry = dict(fields)
    for name, value in fields.items():
        converted(artist, name, value, None)


def forget_geometry(artist):
    """Have *artist* keep no geometry in units (`keep_geometry`): its
    fields are numbers again."""
    artist.__dict__.pop("_unit_geometry", None)


def kept_in_units(artist, given, current, scalars=dict.values):
    """
    Whether *artist*'s geometry, with the fields *given* (a dict of those
    that may be in units, set now), is in units: then kept as given
    (`keep_geometry`) and converted (`convert`), else no longer kept.
    *current()* gives those fields as they are, for an artist that keeps
    none yet; *scalars(fields)* gives the values of *fields* that are
    tested (a point's coordinates).
    """
    kept = kept_geometry(artist)
    if not given or (kept is None and not any_in_units(scalars(given))):
        return False
    fields = {**(current() if kept is None else kept), **given}
    if not any_in_units(scalars(fields)):
        forget_geometry(artist)
        return False
    keep_geometry(artist, fields)
    convert(artist)
    return True


def point_scalars(fields, name="center"):
    """The values of *fields* to test for units (`kept_in_units`), the
    point *name* by its two coordinates."""
    values = [value for key, value in fields.items() if key != name]
    point = fields.get(name)
    try:
        values += [point[0], point[1]]
    except (TypeError, LookupError):
        pass
    return values


def kept_geometry(artist):
    """The fields of *artist*'s geometry it keeps (`keep_geometry`), or
    None."""
    return artist.__dict__.get("_unit_geometry")


def converted(artist, name, value, number):
    """Have *artist*'s field *name*, given as *value* (in units), read back
    as given while pltrs (Rust) holds *number*, its conversion, or None
    before any (`._values.given`)."""
    _values.remember(artist, name, _values.InUnits(value, number))


def convert(artist):
    """
    Have *artist*, given values in units by a setter, convert them now,
    if it has an Axes. matplotlib converts them only where it draws or
    measures the artist, so a conversion that fails now (the width of a
    rectangle whose new left edge is a date is still a number) is tried
    there again, and fails there as matplotlib's does (see `._unusable`).
    """
    axes = artist.axes
    if axes is None:
        return
    try:
        artist._convert_units_for(axes)
    except Exception:
        _unusable.keep(artist, "unit_values", None,
                       lambda _: artist._convert_units_for(artist.axes))


def follow(artist, axes):
    """Have *artist*, which converted its values in units for *axes*,
    convert them again when an axis of *axes* changes units
    (`units_changed`)."""
    _unusable.drop(artist, "unit_values")
    for axis in axes._axis_map.values():
        if axis is None:
            continue
        artists = axis.__dict__.get("_unit_artists")
        if artists is None:
            artists = axis._unit_artists = weakref.WeakSet()
        artists.add(artist)


def units_changed(axis):
    """Have the artists following the units of *axis* (`follow`) that are
    still on its Axes convert their values in units again.

    matplotlib converts those values only where it draws, so converting
    them now leaves the other axes of the Axes as they are: one without a
    converter and without units keeps none (`.axis.Axis.convert_units`),
    and so still takes its units from the next data or limits given in
    units, as an axis of matplotlib's does before the first draw (the
    gallery's ``units/annotate_with_units``: ``set_xlim`` and then
    ``set_ylim`` in centimetres, after annotations placed in them)."""
    artists = axis.__dict__.get("_unit_artists")
    if not artists:
        return
    ahead = [other for other in axis.axes._axis_map.values()
             if other is not None and not other.have_units()
             and not other.__dict__.get("_converts_ahead")]
    for other in ahead:
        other._converts_ahead = True
    try:
        for artist in list(artists):
            if artist.axes is axis.axes:
                artist._convert_units_for(axis.axes)
            else:
                artists.discard(artist)
    finally:
        for other in ahead:
            del other._converts_ahead
