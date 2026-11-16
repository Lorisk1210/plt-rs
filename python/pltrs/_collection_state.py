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
from ._own_state import State, note_paths
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


#: The offsets of the items (matplotlib's ``_offsets``).
OFFSETS = State("offsets", _read_offsets, _take_offsets)


def _take_paths(collection, paths):
    if isinstance(paths, Path):
        _collections.set_single_path(collection._native, paths)
    else:
        _collections.set_paths(collection._native, list(paths))


#: The paths of the items (matplotlib's ``_paths``): the sequence, its
#: paths, and their vertices and codes.
PATHS = State("paths", lambda collection: collection._read_paths(), _take_paths,
              own=note_paths)


def _read_sizes(collection):
    sizes = _collections.get_sizes(collection._native)
    return _unusable.kept(collection, "sizes", _values.given(
        collection, "sizes", np.zeros(0) if sizes is None else sizes))


def _take_sizes(collection, sizes):
    _collections.set_sizes(collection._native, np.asarray(sizes, float))


#: The areas of the items (matplotlib's ``_sizes``), by which each draw
#: scales them.
SIZES = State("sizes", _read_sizes, _take_sizes)


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
    are the collection's no more; without values it leaves them alone.
    (Until that draw the colours read back as the program left them, which
    pltrs (Rust) does with the colours it is handed too.)"""
    if collection._A is None:
        return None
    return _artist.draw_count(collection._native)


def _colors(colors):
    """*colors*, an array handed out, as the RGBA rows pltrs (Rust) takes."""
    return np.asarray(colors, dtype=float).reshape(-1, 4)


#: The face colours as drawn (matplotlib's ``_facecolors``).
FACECOLORS = State(
    "facecolors", lambda collection: collection._read_facecolor(),
    lambda collection, colors: _collections.set_facecolors(collection._native, _colors(colors)),
    version=_mapping_drawn)


def shown_until_drawn(collection, name, native):
    """The colours *name* of *collection* that the program changed in the
    array handed out while the collection maps values, until its next draw
    (`_taking_unmapped`); *native*, what pltrs (Rust) holds, otherwise."""
    shown = collection.__dict__.get("_shown_until_drawn", {}).get(name)
    if shown is not None and shown[0] == _artist.draw_count(collection._native):
        return shown[1]
    return native


def _taking_unmapped(name, take):
    """
    *take* for the colours *name* of a collection without values to map.
    matplotlib's draw of a collection mapping values sets the colours it
    does not map again from what `~.Collection.set_edgecolor` was given,
    so what a program changed in the array is never drawn: the collection
    only reads it back until then (`shown_until_drawn`), in its pickle too.
    """
    def take_unmapped(collection, colors):
        if collection._A is None:
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
