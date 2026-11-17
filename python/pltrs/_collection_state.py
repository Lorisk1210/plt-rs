"""
The state of collections that matplotlib keeps as objects of the
collection's own, which programs change in place (see `._own_state`): the
offsets, the paths, the sizes, the line widths, the antialiasing flags,
the dash patterns, the URLs, the face, edge and hatch colours and the
shape transforms. Each state reads what pltrs (Rust) holds as the
getter of the collection gave it before there were states, and hands
pltrs (Rust) the object's content through the native setter of the
property, nothing else of the collection changing with it (what a setter
of matplotlib's computes besides is not computed again: the object was
changed, not set).
"""

import numpy as np

from . import _unusable, _values
from ._native import artist as _artist, collections as _collections
from ._own_state import State, comparable, content, note_paths
from .path import Path


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from . import rcParams
    return rcParams


def _read_offsets(collection):
    offsets, masked = _collections.get_offsets(collection._native)
    # A collection made with its offsets (barbs) keeps their dtype.
    dtype = collection.__dict__.get("_offsets_dtype")
    if dtype is not None and not masked:
        offsets = offsets.astype(dtype, copy=False)
    return np.ma.masked_invalid(offsets) if masked else offsets


def _take_offsets(collection, offsets):
    masked = isinstance(offsets, np.ma.MaskedArray)
    _collections.set_offsets(
        collection._native,
        np.ma.filled(np.ma.asarray(offsets, dtype=float), np.nan).reshape(-1, 2), masked)


def _flat(collection, array):
    """Whether pltrs (Rust) can compare *array* with its own numbers for
    *collection* (`.State.compares`): not for the collections of a 3-D
    Axes, whose numbers each projection sets."""
    return comparable(array) and not hasattr(collection, "do_3d_projection")


def _flat_or_masked(collection, array):
    """`_flat`, also for a masked array of floats: pltrs (Rust) holds NaN
    where it masks."""
    return _flat(collection, np.ma.getdata(array) if type(array) is np.ma.MaskedArray
                 else array)


def _same_offsets(collection, offsets):
    """Whether pltrs (Rust) holds what *offsets* holds (`.State.same`)."""
    mask = np.ma.getmask(offsets)
    return _collections.holds_offsets(
        collection._native, np.ma.getdata(offsets), None if mask is np.ma.nomask else mask)


#: The offsets of the items (matplotlib's ``_offsets``). Nothing writes
#: them natively but `_take_offsets` and `~.Collection.set_offsets` (and
#: an ``_offsets`` assigned), which forget the array handed out: an array
#: of floats is compared with pltrs (Rust)'s offsets where both are.
OFFSETS = State(
    "offsets", _read_offsets, _take_offsets,
    same=_same_offsets, compares=_flat_or_masked)


def _take_paths(collection, paths):
    if isinstance(paths, Path):
        _collections.set_single_path(collection._native, paths)
    else:
        _collections.set_paths(collection._native, list(paths))


#: The paths of the items (matplotlib's ``_paths``): the sequence, its
#: paths, and their vertices and codes.
PATHS = State("paths", lambda collection: collection._read_paths(), _take_paths,
              own=note_paths)


#: The vertices of a quad mesh (matplotlib's ``_coordinates``), masked or
#: not as the mesh was given them.
COORDINATES = State(
    "coordinates", lambda mesh: mesh._read_coordinates(),
    lambda mesh, coordinates: _collections.set_coordinates(
        mesh._native, mesh._native_coordinates(coordinates)),
    lambda coordinates: content(np.ma.asarray(coordinates)))


def _read_sizes(collection):
    sizes = _collections.get_sizes(collection._native)
    return _unusable.kept(collection, "sizes", _values.given(
        collection, "sizes", np.zeros(0) if sizes is None else sizes))


def _take_sizes(collection, sizes):
    _collections.set_sizes(collection._native, np.asarray(sizes, float))


#: The areas of the items (matplotlib's ``_sizes``), by which each draw
#: scales them. Nothing writes them natively but `_take_sizes` and
#: `~.Collection.set_sizes`, which forgets the array kept: an array of
#: floats is compared with pltrs (Rust)'s sizes where both are.
SIZES = State(
    "sizes", _read_sizes, _take_sizes,
    same=lambda collection, sizes: _collections.holds_sizes(collection._native, sizes),
    compares=_flat)


#: The line widths of the items (matplotlib's ``_linewidths``).
LINEWIDTHS = State("linewidths", lambda collection: collection._read_linewidth(),
                   lambda collection, widths: collection._take_linewidth(widths))


def _take_antialiaseds(collection, flags):
    _collections.set_antialiaseds(collection._native, [bool(flag) for flag in flags])


#: Whether each item is antialiased (matplotlib's ``_antialiaseds``).
ANTIALIASEDS = State("antialiaseds", lambda collection: collection._read_antialiased(),
                     _take_antialiaseds)


def _take_mesh_antialiaseds(mesh, flags):
    mesh._antialiaseds = [bool(flag) for flag in flags]


#: The flags a mesh reads back (matplotlib's ``_antialiaseds``, which its
#: mesh does not draw with: it draws with the antialiasing it was made
#: with).
MESH_ANTIALIASEDS = State("antialiaseds", lambda mesh: mesh._read_antialiased(),
                          _take_mesh_antialiaseds)


def _read_urls(collection):
    if _values.is_kept(collection, "urls"):
        return _values.given(collection, "urls", None)
    urls = _collections.get_urls(collection._native)
    # A collection starts with `[None]` (matplotlib's `set_urls(None)`).
    if not urls and "urls" not in collection.__dict__.get("_given_values", {}):
        return [None]
    return urls


def _take_urls(collection, urls):
    _collections.set_urls(collection._native,
                          [None if url is None else str(url) for url in urls])
    # (As `Collection.set_urls` keeps them: no URLs are not the `[None]` of
    # a collection never given any.)
    _values.keep(collection, "urls", urls)


#: The URL of each item (matplotlib's ``_urls``).
URLS = State("urls", _read_urls, _take_urls)


def _mapping_drawn(collection):
    """What tells the colours of *collection* a draw bound anew: matplotlib's
    draw maps the values of a collection that has some onto new arrays, and
    sets the colours it does not map again, so the arrays handed out before
    are the collection's no more. Without values only its first draw sets
    them again (the first ``update_scalarmappable`` finds its flags
    changed); later ones leave them alone. (Until that draw the colours
    read back as the program left them, `shown_until_drawn`.)"""
    draws = _artist.draw_count(collection._native)
    return draws if collection._mapped is not None else min(draws, 1)


def _set_again_by_next_draw(collection):
    """Whether the next draw of matplotlib's collection sets its colours
    again from what the setters were given, so that what a program changed
    in the arrays handed out is never drawn (see `_mapping_drawn`)."""
    return collection._mapped is not None or not _artist.draw_count(collection._native)


def _colors(colors):
    """*colors*, an array handed out, as the RGBA rows pltrs (Rust) takes."""
    return np.asarray(colors, dtype=float).reshape(-1, 4)


def _take_facecolors(collection, colors):
    """Hand pltrs (Rust) the face colours changed in the array handed out,
    unless the first draw is still to come: matplotlib's sets them again
    then, and the change is only read back until it does
    (`shown_until_drawn`). (The faces of a collection mapping values are
    mapped anew at every draw in pltrs (Rust) too.)"""
    if collection._mapped is None and not _artist.draw_count(collection._native):
        collection.__dict__.setdefault("_shown_until_drawn", {})["facecolors"] = (
            0, np.array(colors))
    else:
        _collections.set_facecolors(collection._native, _colors(colors))


#: The face colours as drawn (matplotlib's ``_facecolors``).
FACECOLORS = State(
    "facecolors", lambda collection: collection._read_facecolor(), _take_facecolors,
    version=_mapping_drawn)


def shown_until_drawn(collection, name, native):
    """The colours *name* of *collection* that the program changed in the
    array handed out before a draw that sets them again (`_taking_unmapped`,
    `_take_facecolors`), until that draw; *native*, what pltrs (Rust)
    holds, otherwise."""
    shown = collection.__dict__.get("_shown_until_drawn", {}).get(name)
    if shown is not None and shown[0] == _artist.draw_count(collection._native):
        return shown[1]
    return native


def _taking_unmapped(name, take):
    """
    *take* for the colours *name* of a collection whose next draw leaves
    them alone. matplotlib's draw of a collection mapping values, and the
    first draw of any collection, set the colours they do not map again
    from what `~.Collection.set_edgecolor` was given, so what a program
    changed in the array is never drawn: the collection only reads it back
    until then (`shown_until_drawn`), in its pickle too.
    """
    def take_unmapped(collection, colors):
        if not _set_again_by_next_draw(collection):
            take(collection._native, _colors(colors))
        else:
            collection.__dict__.setdefault("_shown_until_drawn", {})[name] = (
                _artist.draw_count(collection._native), np.array(colors))
    return take_unmapped


#: The edge colours as drawn (matplotlib's ``_edgecolors``, where they are
#: colours of their own).
EDGECOLORS = State(
    "edgecolors", lambda collection: collection._read_edgecolor(),
    _taking_unmapped("edgecolors", _collections.set_edgecolors), version=_mapping_drawn)

#: The hatch colours as drawn (matplotlib's ``_hatchcolors``, where they
#: are colours of their own).
HATCHCOLORS = State(
    "hatchcolors",
    lambda collection: shown_until_drawn(collection, "hatchcolors",
                                         collection._get("hatchcolor")),
    _taking_unmapped("hatchcolors", _collections.set_hatchcolors), version=_mapping_drawn)


def _take_linestyles(collection, dashes):
    # The patterns handed out are scaled by the line widths, each by its
    # own, cycled (matplotlib's ``_bcast_lwls``); pltrs (Rust) takes them
    # unscaled.
    widths = np.asarray(collection._get("linewidth"), dtype=float)
    scaled = len(widths) > 0 and _rcParams()["lines.scale_dashes"] and (
        not _rcParams()["_internal.classic_mode"])
    styles = []
    for index, (offset, pattern) in enumerate(dashes):
        width = widths[index % len(widths)] if scaled else 1.0
        if pattern is not None and width:
            offset, pattern = offset / width, tuple(length / width for length in pattern)
        styles.append((offset, None if pattern is None else tuple(pattern)))
    _collections.set_linestyles(collection._native, styles)


#: The dash patterns of the items, scaled by their line widths
#: (matplotlib's ``_linestyles``).
LINESTYLES = State("linestyles", lambda collection: collection._lwls()[1], _take_linestyles)


def _sized(collection):
    """Whether the items of *collection* are scaled by sizes."""
    return _collections.get_sizes(collection._native) is not None



#: The shape transform of each item (matplotlib's ``_transforms``), which
#: the draw of a collection with sizes makes anew from them (until then
#: the array reads back as the program left it).
TRANSFORMS = State(
    "transforms", lambda collection: collection._read_transforms(),
    lambda collection, matrices: collection._take_transforms(
        np.asarray(matrices, dtype=float)),
    version=lambda collection: (
        _artist.draw_count(collection._native) if _sized(collection) else None))
