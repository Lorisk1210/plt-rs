"""
A setter between the change and the next read.

A program that holds an array of an artist and changes it in place also
sets the artist's styles (a scatter read once, then recoloured and
restyled in a loop). Under matplotlib the change and the setter are two
writes to one object; in pltrs the array is handed to pltrs (Rust) before
native calls that read the artist, and the native functions that read
none of it are passed by (``binding::PASSING_BY`` in the extension,
``_native.artist.passing_by()``): the hand-over waits for the next read.
Such a function must leave what the program changed as matplotlib's
setter leaves it, and the setter must not make the artist forget an
object matplotlib keeps.

So for every function passed by, `CALLS` names calls of the public API
that reach it, on subjects that keep objects of their own, and `rows`
makes of each call one row per object the surface changes on its
subject: the object is changed, the call is made, and then the artist is
read, drawn, measured and pickled as in every other row.
``python/tests/test_mut1.py`` holds `CALLS` to the extension's list: a
function passed by without a call here fails it, and so does a call that
does not reach its function or that tells the artist.
"""

from . import surface

#: The subjects whose artist every call of a group is made on.
_COLLECTIONS = ("scatter", "line_collection")
_ANY = ("scatter", "line")


def _on(subjects, **calls):
    """The calls *calls* (by name), each on every subject of *subjects*."""
    return [(subject, name, call) for name, call in calls.items() for subject in subjects]


#: For each native function passed by (``module.name``), calls of the
#: public API that reach it and make no native call that is not passed
#: by: the subject, the name of the call in the rows' names, and the call
#: on the subject.
CALLS = {
    "artist.axes": _on(("scatter",), axes=lambda artist: artist.axes),
    "artist.id": _on(("scatter",), axes=lambda artist: artist.axes),
    "artist.draw_count": _on(("scatter",), set_sizes=lambda artist: artist.set_sizes([30.0])),
    # (The setter of `stale` itself: pltrs (Rust) marks the artist stale in
    # `set_zorder`, which no longer comes back through Python's flag.)
    "artist.set_stale": _on(
        _ANY, set_zorder=lambda artist: (artist.set_zorder(4), setattr(artist, "stale", True))),
    "artist.get_visible": _on(("scatter",), get_visible=lambda artist: artist.get_visible()),
    "artist.set_visible": _on(_ANY, set_visible=lambda artist: artist.set_visible(False)),
    "artist.get_zorder": _on(("scatter",), get_zorder=lambda artist: artist.get_zorder()),
    "artist.set_zorder": _on(_ANY, set_zorder=lambda artist: artist.set_zorder(4)),
    "artist.get_alpha": _on(("line",), get_alpha=lambda artist: artist.get_alpha()),
    "artist.set_alpha": _on(("line", "polygon"), set_alpha=lambda artist: artist.set_alpha(0.5)),
    "artist.get_label": _on(("scatter",), get_label=lambda artist: artist.get_label()),
    "artist.set_label": _on(_ANY, set_label=lambda artist: artist.set_label("set")),
    "artist.get_animated": _on(
        ("scatter",), get_animated=lambda artist: artist.get_animated()),
    "artist.set_animated": _on(_ANY, set_animated=lambda artist: artist.set_animated(True)),
    "artist.set_clip_on": _on(_ANY, set_clip_on=lambda artist: artist.set_clip_on(False)),
    "artist.set_rasterized": _on(
        _ANY, set_rasterized=lambda artist: artist.set_rasterized(True)),
    "artist.set_in_layout": _on(
        _ANY, set_in_layout=lambda artist: artist.set_in_layout(False)),
    "artist.set_snap": _on(_ANY, set_snap=lambda artist: artist.set_snap(True)),
    "artist.set_gid": _on(_ANY, set_gid=lambda artist: artist.set_gid("set")),
    "artist.set_url": _on(_ANY, set_url=lambda artist: artist.set_url("set")),
    "artist.set_picker": _on(_ANY, set_picker=lambda artist: artist.set_picker(True)),
    "collections.set_linewidths": _on(
        _COLLECTIONS, set_linewidth=lambda artist: artist.set_linewidth(5.0)),
    "collections.set_linestyles": _on(
        _COLLECTIONS, set_linestyle=lambda artist: artist.set_linestyle(":")),
    "collections.set_antialiaseds": _on(
        _COLLECTIONS, set_antialiased=lambda artist: artist.set_antialiased(False)),
    "collections.set_capstyle": _on(
        _COLLECTIONS, set_capstyle=lambda artist: artist.set_capstyle("round")),
    "collections.set_joinstyle": _on(
        _COLLECTIONS, set_joinstyle=lambda artist: artist.set_joinstyle("bevel")),
    "collections.set_pickradius": _on(
        _COLLECTIONS, set_pickradius=lambda artist: artist.set_pickradius(9.0)),
    "collections.set_urls": _on(
        _COLLECTIONS, set_urls=lambda artist: artist.set_urls(["set"])),
    "collections.set_hatch": _on(_COLLECTIONS, set_hatch=lambda artist: artist.set_hatch("//")),
    "collections.set_hatch_linewidth": _on(
        _COLLECTIONS, set_hatch_linewidth=lambda artist: artist.set_hatch_linewidth(3.0)),
    "collections.set_sizes": _on(
        ("scatter", "scatter_plain"), set_sizes=lambda artist: artist.set_sizes([30.0])),
    "collections.set_offsets": _on(
        ("scatter", "scatter_plain"),
        set_offsets=lambda artist: artist.set_offsets([[0.5, 0.5], [2.5, 3.0]])),
    "lines.get_drawstyle": _on(
        ("line", "step_line"),
        set_drawstyle=lambda artist: artist.set_drawstyle("steps-mid")),
    "lines.update": _on(
        ("line", "step_line"),
        set_linewidth=lambda artist: artist.set_linewidth(5.0),
        set_color=lambda artist: artist.set_color("red"),
        set_linestyle=lambda artist: artist.set_linestyle(":"),
        set_drawstyle=lambda artist: artist.set_drawstyle("steps-mid"),
        set_markevery=lambda artist: artist.set_markevery(2)),
    "patches.update": _on(
        ("polygon", "path_patch"),
        set_facecolor=lambda artist: artist.set_facecolor("red"),
        set_linewidth=lambda artist: artist.set_linewidth(5.0),
        set_hatch=lambda artist: artist.set_hatch("//")),
}


def calls():
    """Every call once: ``(subject, name, call)``, in the order of `CALLS`."""
    seen, found = set(), []
    for listed in CALLS.values():
        for subject, name, call in listed:
            if (subject, name) not in seen:
                seen.add((subject, name))
                found.append((subject, name, call))
    return found


def rows(found):
    """The rows: for every call, one per object the surface changes on
    its subject (the names *found*, `.surface.discover`'s), named
    ``<subject>.<member> then <call>``."""
    return [surface.row(subject, path, then=(name, call))
            for subject, name, call in calls()
            for path in found[subject]["changed"]]
