"""
Values matplotlib keeps unchecked and fails on only when it uses them.

matplotlib's setters keep many values as they are given: a
``mutation_aspect`` of ``'equal'``, an ``orientation`` of ``'vertical'``,
a tick location of ``'nowhere'``. They fail later, when the artist
computes with the value as it is measured or drawn, with the error that
computation raises. pltrs (Rust) holds the values it computes with, so a
value it cannot hold never reaches it: the artist keeps it (`keep`), and
reads it back (`kept`), while pltrs (Rust) keeps the value it had. Until
a usable value replaces it (`drop`), the artist fails as matplotlib's
does wherever matplotlib's would compute with it (`check`): in its own
methods, and when its figure is drawn or (for values matplotlib measures
with) measured, which pltrs (Rust) then does through the artist's Python
``draw`` and ``get_tightbbox``.
"""

from . import _python_drawing


def keep(artist, name, value, use, *, measured=True, drawn=True):
    """
    Keep *value*, which pltrs (Rust) cannot hold, as *artist*'s property
    *name*. *use(value)* raises what matplotlib's first computation with
    it raises; *measured* tells whether matplotlib computes with it when
    it measures the artist (or only when it draws it), *drawn* whether
    drawing the artist's figure does (not for a value its parent replaces
    before it draws the artist, as an axis places its ticks anew).
    """
    artist.__dict__.setdefault("_unusable", {})[name] = (value, use, measured, drawn)
    _watch(artist)


def drop(artist, name):
    """Forget the value *artist* kept for *name* (`keep`): a usable one,
    which pltrs (Rust) holds, replaced it."""
    values = artist.__dict__.get("_unusable")
    if values and values.pop(name, None) is not None:
        _watch(artist)


def kept(artist, name, default):
    """The value *artist* keeps for *name* (`keep`), else *default*."""
    entry = artist.__dict__.get("_unusable", {}).get(name)
    return default if entry is None else entry[0]


def keeps(artist):
    """Whether *artist* keeps a value matplotlib fails on when it draws the
    artist (`keep`)."""
    return any(entry[3] for entry in artist.__dict__.get("_unusable", {}).values())


def check(artist, *, measuring=False):
    """Fail as matplotlib's *artist* fails to compute with the values it
    keeps (`keep`): with all of them when it draws, with those it
    measures with when *measuring*, in the order matplotlib computes with
    them (the names an artist's ``_kept_order()`` gives first, in its
    order, then the others as they were kept)."""
    kept = artist.__dict__.get("_unusable", {})
    names = list(kept)
    order = getattr(artist, "_kept_order", None)
    if order is not None:
        first = order()
        names.sort(key=lambda name: first.index(name) if name in first else len(first))
    for name in names:
        value, use, measured, _ = kept[name]
        if measured or not measuring:
            use(value)


def _watch(artist):
    """Have pltrs (Rust) draw (and measure) *artist* through Python while
    it keeps a value, so that drawing its figure fails as matplotlib's
    does; as before otherwise."""
    drawn = [entry for entry in artist.__dict__.get("_unusable", {}).values()
             if entry[3]]
    if drawn:
        _python_drawing.draw_through_python(artist, any(entry[2] for entry in drawn))
    else:
        _python_drawing.attach(artist, again=True)
    # An artist another draws as a part of it (a table's cell) has that one
    # check what it keeps.
    changed = getattr(artist, "_kept_changed", None)
    if changed is not None:
        changed()
