"""
Runs the rows and compares their outcomes with matplotlib's.

``reference.json`` holds what matplotlib does: the names of the surface's
rows (`surface.discover`) and the outcome of every row. `all_rows` makes
the rows from it, `outcomes` measures them on the library at hand, and
`differences` says by kind where two sets of outcomes part.
"""

import json
import multiprocessing
import pathlib

import pltrs
from pltrs import legend, patches
from pltrs.path import Path

from . import given, handed, scenes, setters, surface, threed
from .measure import outcome

#: matplotlib's outcomes, recorded by ``python -m mutability --record``.
RECORDED = pathlib.Path(__file__).with_name("reference.json")

#: The kinds of difference, in the order a row is counted under the first
#: that applies, with the keys of an outcome that decide each.
KINDS = (
    ("error", ("read", "mutate", "error")),
    ("state", ("made", "drawn", "unpickled", "made_fields", "drawn_fields")),
    ("program", ("projections", "draws", "seen_first_projection", "seen_first_draw",
                 "seen_last_draw", "class", "seen", "path2d", "shaft_drawn", "head_drawn")),
    ("lost", ("reread",)),
    ("undrawn", ("draw",)),
    ("identity", ("same", "kept", "after")),
    ("limits", ("datalim", "relim")),
    ("measure", ("measure",)),
    ("pickle", ("pickle",)),
    ("first draw", ("first_draw", "first_kept", "first_value")),
    ("value", ("value",)),
)


#: The rows on which pltrs is known to part from matplotlib, with the keys
#: of their outcomes that differ: what is left to do, written by ``python
#: -m mutability --expect`` and held by ``python/tests/test_mut1.py``, which
#: fails on a row that differs otherwise and on one that no longer does.
EXPECTED = pathlib.Path(__file__).with_name("expected.json")


def recorded():
    """matplotlib's recorded surface and outcomes."""
    return json.loads(RECORDED.read_text("utf-8"))


def found_in(reference):
    """What matplotlib's subjects have, as the reference *reference*
    recorded it: the members of the surface's subjects (``surface``) and
    the fields of the 3-D artists (``art3d``), which `all_rows` makes the
    rows of."""
    return {"surface": reference["surface"], "art3d": reference.get("art3d", {})}


def expected():
    """The rows pltrs is known to differ on: ``{name: [keys]}``."""
    return json.loads(EXPECTED.read_text("utf-8"))


def groups(found):
    """The names of every row by group, in order: the surface's rows by
    subject, then ``handed``, ``given`` and ``setters``, then the 3-D
    artists' rows by subject (``3d/<subject>``) and those of the
    program's subclasses (``3d/programs``)."""
    by_group = {subject: [row.name for row in surface.rows({subject: kinds})]
                for subject, kinds in found["surface"].items()}
    by_group["handed"] = [row.name for row in handed.ROWS]
    by_group["given"] = [row.name for row in given.ROWS]
    by_group["setters"] = [row.name for row in setters.rows(found["surface"])]
    for row in threed.rows(found["art3d"]):
        subject, dot, _ = row.name.partition(".")
        by_group.setdefault(subject if dot else "3d/programs", []).append(row.name)
    return by_group


def measure_here(found, names):
    """The outcomes of the rows *names*, measured in this process:
    ``{name: outcome}``."""
    if not _rows:
        _start(found)
    return dict(map(_measure, names))


def differing(ours, theirs):
    """The rows of *ours* whose outcome parts from matplotlib's in
    *theirs*, with the keys it parts on: ``{name: [keys]}``."""
    found = {name: differing_keys(outcome, theirs[name]) for name, outcome in ours.items()}
    return {name: keys for name, keys in found.items() if keys}


def all_rows(found):
    """Every row: the surface's rows of the names *found* (`found_in`),
    then the idioms and helper objects, the objects given to artists, the
    surface's rows of some subjects with a setter called after the
    change, and the rows of the 3-D artists' state."""
    return [*surface.rows(found["surface"]), *handed.ROWS, *given.ROWS,
            *setters.rows(found["surface"]), *threed.rows(found["art3d"])]


#: The rows of a worker process, by name.
_rows = {}


def _start(found):
    """Make the rows of *found* in a new worker."""
    pltrs.use("agg")
    _note_shared()
    _rows.update((row.name, row) for row in all_rows(found))


#: What matplotlib shares among objects and a row may change in place,
#: as it was when the worker started: the lists of the default rcParams
#: (``rcdefaults`` gives ``rcParams`` the very lists) and the path all
#: `~.patches.Arrow` patches share, the handlers all legends share
#: (``Legend.get_default_handler_map()`` is the class' own dict); and the
#: arrays the scenes are built of, which an artist keeping one of them
#: hands out for the row to change.
#: Each is put back after every row, so that a row's outcome does not
#: depend on the rows its worker ran before.
_shared = {}


def _note_shared():
    """Note what matplotlib shares among objects, as it is now."""
    _shared["rc"] = {key: list(value) for key, value in pltrs.rcParamsDefault.items()
                     if isinstance(value, list)}
    path = vars(patches.Arrow).get("_path")
    if isinstance(path, Path):
        _shared["arrow"] = (path, path.vertices.copy(), path.codes.copy())
    _shared["handlers"] = dict(legend.Legend.get_default_handler_map())
    _shared["data"] = {(module, name): getattr(module, name).copy()
                       for module in (scenes, threed) for name in module.DATA}


def _restore_shared():
    """Put back what `_note_shared` noted."""
    for key, items in _shared["rc"].items():
        default = pltrs.rcParamsDefault[key]
        if default != items:
            default[:] = items
    if "arrow" in _shared:
        path, vertices, codes = _shared["arrow"]
        path.vertices[:] = vertices
        path.codes[:] = codes
    handlers = legend.Legend.get_default_handler_map()
    if handlers != _shared["handlers"]:
        handlers.clear()
        handlers.update(_shared["handlers"])
    for (module, name), values in _shared["data"].items():
        getattr(module, name)[...] = values


def _measure(name):
    """The outcome of the row *name*, under matplotlib's defaults."""
    try:
        with pltrs.rc_context():
            pltrs.rcdefaults()
            row = _rows[name]
            return name, row.measure() if isinstance(row, threed.Probe) else outcome(row)
    finally:
        _restore_shared()


def outcomes(found, patterns=(), jobs=6):
    """The outcome of every row whose name contains one of *patterns*
    (all without one), measured in *jobs* processes: ``{name: outcome}``."""
    names = [row.name for row in all_rows(found)
             if not patterns or any(pattern in row.name for pattern in patterns)]
    with multiprocessing.get_context("spawn").Pool(jobs, _start, (found,)) as pool:
        return dict(pool.imap(_measure, names, chunksize=8))


def _close(ours, theirs):
    """Whether two plain values agree, numbers to one part in 10⁴."""
    if isinstance(ours, dict) and isinstance(theirs, dict):
        return ours.keys() == theirs.keys() and all(_close(ours[key], theirs[key]) for key in ours)
    if isinstance(ours, list) and isinstance(theirs, list):
        return len(ours) == len(theirs) and all(map(_close, ours, theirs))
    if (isinstance(ours, float) and isinstance(theirs, float)):
        return abs(ours - theirs) <= 1e-4 * max(1.0, abs(ours), abs(theirs))
    return ours == theirs


def differing_keys(ours, theirs):
    """The keys on which the outcome *ours* parts from *theirs*."""
    return [key for key in sorted({*ours, *theirs})
            if not _close(ours.get(key), theirs.get(key))]


def kind_of(keys):
    """The kind a row differing on *keys* is counted under."""
    for kind, deciding in KINDS:
        if any(key in keys for key in deciding):
            return kind
    return "value"


def differences(ours, theirs):
    """
    Where the outcomes *ours* part from matplotlib's, *theirs*:
    ``{kind: {name: [keys]}}`` over the rows both hold.
    """
    found = {}
    for name, outcome_theirs in theirs.items():
        if name not in ours:
            continue
        keys = differing_keys(ours[name], outcome_theirs)
        if keys:
            found.setdefault(kind_of(keys), {})[name] = keys
    return found
