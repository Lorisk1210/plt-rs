"""Programs subclassing matplotlib's classes and overriding one member, as
libraries do: is the override called where matplotlib calls it, as often
and with the same arguments, and is its result used?

The members come from the census of the libraries' overrides
(``spec/diff/overrides_census.json``, ``overrides_census.py``), most
libraries first, then from the members matplotlib's own code calls on
``self`` in the classes of the scenes (``SELF_CALLED`` per class, the most
called first), which a library may override as well.

Each program defines ``Sub``, a subclass of the class (or of a working
``Impl`` over an abstract base, ``overrides_scenes``), whose member notes
each call (the member, its arguments in a notation of their own, ``note``,
and how deep in other recorded calls it came) and returns what
matplotlib's member returns. Then the object lives as the scene says:
made, added, drawn, drawn again unchanged, a property set and drawn,
measured (``get_window_extent``, ``get_tightbbox``), saved tight to PNG and
SVG, hit (``contains``, ``pick``), pickled with its figure, removed and its
Axes cleared. After each step ``take()`` returns the calls noted since the
last: the value the tester compares. The classes are defined through
``exec`` so that their returns are not recorded one by one.

Programs named ``<Class>-<member>`` record; ``<Class>-<member>-changed``
(``overrides_changes``) change the result and read what the figure shows,
the figure left open for its image.
"""

from __future__ import annotations

import ast
import functools
import importlib
import inspect
import json
import pathlib
import re
import textwrap
from typing import Any, Iterator

from . import common
from .overrides_changes import ATTRIBUTES, CHANGES, LATER, SHOWS
from .overrides_scenes import ADDING, IMPORTS, SCENES, member_steps

CENSUS = pathlib.Path(__file__).resolve().parent.parent / "overrides_census.json"

#: How many members matplotlib calls on ``self`` each scene's class gets
#: programs for, beyond the census' (the most called first).
SELF_CALLED = 6

#: Members whose number of calls is an accident of matplotlib's
#: implementation (how often its code reads them while it draws, makes
#: ticks or measures), which no override of the census depends on: their
#: programs compare the distinct calls of each step, in the order of their
#: first call (``distinct(take())``), with the reason.
DISTINCT: dict[str, str] = {
    **dict.fromkeys(
        ("get_figure", "get_transform", "get_patch_transform", "get_path", "get_paths", "get_transforms",
         "get_spine_transform", "get_clip_box", "get_clip_on", "get_clip_path", "get_visible", "get_text",
         "get_usetex", "get_rotation", "get_children", "get_child", "get_bbox", "get_offset", "get_extent",
         "get_edgecolor", "get_facecolor", "get_hatchcolor", "_get_default_edgecolor", "pchanged"),
        "matplotlib's code reads the artist through it wherever it needs it while it draws and measures; "
        "pltrs (Rust) holds what it reads"),
    **dict.fromkeys(
        ("get_matrix", "get_affine", "get_xaxis_transform", "get_yaxis_transform", "get_view_interval",
         "get_window_extent", "_adjust_location"),
        "matplotlib evaluates it again for every tick, label and box it places; pltrs evaluates it once "
        "per layout"),
    **dict.fromkeys(
        ("__init__", "update_position", "_get_tick", "_get_text1_transform", "_get_text2_transform",
         "_set_labelrotation", "_apply_params", "_apply_tickdir", "_copy_tick_props"),
        "matplotlib makes and updates tick objects lazily, as many as its last ticking needed; pltrs keeps "
        "one per level and updates it per tick drawn"),
    **dict.fromkeys(
        ("inverse", "autoscale_None", "process_value", "vmin", "vmax", "scaled", "_changed", "_ensure_inited"),
        "matplotlib's colour mapping and colorbar ask the norm through cascades of callbacks"),
    **dict.fromkeys(
        ("transform", "transform_affine"),
        "matplotlib maps a text's point through it at every draw and measure, and its prints draw twice"),
    **dict.fromkeys(
        ("_get_unit", "_get_interval", "viewlim_to_dt", "_create_rrule", "convert", "get_geometry",
         "get_grid_positions", "get_subplot_params", "get_size"),
        "matplotlib's code recomputes it wherever it needs it"),
}

#: The classes whose objects matplotlib's ``pick`` runs (it asks every axis
#: for its children, which makes the ticks: the locator, the formatter,
#: the scale's transform): the pick step of their programs is not compared.
PICK_RUNS = {
    "matplotlib.ticker.Locator", "matplotlib.ticker.MaxNLocator", "matplotlib.ticker.AutoMinorLocator",
    "matplotlib.dates.DateLocator", "matplotlib.dates.AutoDateLocator", "matplotlib.ticker.Formatter",
    "matplotlib.ticker.ScalarFormatter", "matplotlib.scale.ScaleBase", "matplotlib.scale.LinearScale",
    "matplotlib.scale.LogScale", "matplotlib.scale.SymmetricalLogScale", "matplotlib.scale.LogitScale",
    "matplotlib.dates.DateConverter", "matplotlib.units.ConversionInterface",
}

#: The recorder every program defines (run with ``exec``).
HELPERS = '''
CALLS = []
DEPTH = [0]
ignore = lambda *values: None
def note(value, depth=0):
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, (int, np.integer)):
        return int(value)
    if isinstance(value, (float, np.floating)):
        return float(f'{float(value):.5g}')
    if isinstance(value, np.ndarray):
        if value.dtype.kind in 'biuf':
            return ('array', value.shape, [note(v) for v in value.ravel()[:4].tolist()])
        return ('array', value.shape, value.dtype.kind)
    if isinstance(value, (list, tuple)):
        return tuple(note(v, depth + 1) for v in value[:6]) if depth < 3 else ('items', len(value))
    if isinstance(value, dict):
        return {str(k): note(v, depth + 1) for k, v in sorted(value.items(), key=lambda kv: str(kv[0]))[:24]}
    if isinstance(value, transforms.BboxBase):
        return ('Bbox', tuple(round(float(v), 3) for v in value.bounds))
    if isinstance(value, path.Path):
        return ('Path', len(value.vertices))
    return type(value).__name__
def bound(method, args, kwargs):
    try:
        given = inspect.signature(method).bind(*args, **kwargs)
    except (TypeError, ValueError):
        return note(args), note(kwargs)
    given.apply_defaults()
    return note(dict(given.arguments)), None
def call(name, method, args=(), kwargs=None):
    kwargs = kwargs or {}
    CALLS.append((DEPTH[0], name, *bound(method, args, kwargs)))
    DEPTH[0] += 1
    try:
        return method(*args, **kwargs)
    finally:
        DEPTH[0] -= 1
def take():
    taken = list(CALLS)
    CALLS.clear()
    return taken
def distinct(calls):
    kept = []
    for made in calls:
        if made not in kept:
            kept.append(made)
    return kept
def base_attribute(cls, name):
    for klass in cls.__mro__[1:]:
        if name in vars(klass):
            return vars(klass)[name]
    return None
def base_get(obj, cls, name):
    found = base_attribute(cls, name)
    if hasattr(found, '__get__'):
        return found.__get__(obj, type(obj))
    return obj.__dict__.get(name, found)
def base_set(obj, cls, name, value):
    found = base_attribute(cls, name)
    if hasattr(found, '__set__'):
        found.__set__(obj, value)
    else:
        obj.__dict__[name] = value
classes = types.ModuleType('overridden')
sys.modules['overridden'] = classes
def keep(cls):
    cls.__module__ = 'overridden'
    setattr(classes, cls.__name__, cls)
    return cls
'''

#: The statements of every program before its classes.
SETUP = [*common.HEADER, *common.DATA, IMPORTS, "import inspect, io, pickle, sys, types",
         "from matplotlib.backend_bases import MouseEvent", f'exec("""{HELPERS}""")']

#: The measuring, hitting and printing steps of every life (hit while
#: the figure is as the screen shows it: a print at another resolution
#: leaves matplotlib's patches where that print drew them).
MEASURE = (
    "ignore(art.get_window_extent(fig.canvas.get_renderer()))",
    "ignore(art.get_tightbbox(fig.canvas.get_renderer()))",
    "art.contains(event)[0]",
    "art.set_picker(True)",
    "fig.pick(event)",
    "fig.savefig(io.BytesIO(), format='png', bbox_inches='tight')",
    "fig.savefig(io.BytesIO(), format='svg', bbox_inches='tight')",
    "ignore(pickle.loads(pickle.dumps(fig)))",
)

#: The end of each kind of life.
ENDS = {
    "artist": ("art.remove()", "fig.canvas.draw()", "ax.cla()", "fig.canvas.draw()"),
    "axes": ("ax.cla()", "fig.canvas.draw()", "ax.remove()", "fig.canvas.draw()"),
    "figure": ("fig.clear()", "fig.canvas.draw()"),
    "value": ("ax.cla()", "fig.canvas.draw()"),
}


def _class(dotted: str) -> type:
    module, _, name = dotted.rpartition(".")
    value: Any = importlib.import_module(module)
    for part in name.split("."):
        value = getattr(value, part)
    return value


def _program_base(cls: str) -> tuple[str, type]:
    """The base ``Sub`` derives from (``Impl`` where the scene has one), and
    the class it stands for under matplotlib."""
    scene = SCENES[cls]
    if not scene.impl:
        module, _, name = cls.rpartition(".")
        if module.startswith("mpl_toolkits"):
            return cls, _class(cls)
        return f"{module.split('.', 1)[1]}.{name}" if module != "matplotlib" else name, _class(cls)
    scope: dict[str, Any] = {}
    exec("\n".join([*common.HEADER, IMPORTS, *scene.imports, scene.impl]), scope)  # noqa: S102 - the generator's own source
    return "Impl", scope["Impl"]


def _override(member: str, kind: str, base: type) -> str | None:
    """The source of a recording override of ``member`` (None if the class
    has no such member to override)."""
    if kind == "attribute":
        value = common.literal(getattr(base, member, None)) if hasattr(base, member) else None
        return None if value is None else f"{member} = {value}\n"
    if not hasattr(base, member):
        return None
    if kind == "property":
        held = inspect.getattr_static(base, member)
        source = (f"@property\ndef {member}(self):\n"
                  f"    return call({member!r}, lambda: base_get(self, Sub, {member!r}))\n")
        if getattr(held, "fset", None) is not None:
            source += (f"@{member}.setter\ndef {member}(self, value):\n"
                       f"    call({member + '='!r}, lambda: base_set(self, Sub, {member!r}, value))\n")
        return source
    if kind == "staticmethod":
        return (f"@staticmethod\ndef {member}(*args, **kwargs):\n"
                f"    return call({member!r}, base_attribute(Sub, {member!r}), args, kwargs)\n")
    if kind == "classmethod":
        return (f"@classmethod\ndef {member}(cls, *args, **kwargs):\n"
                f"    return call({member!r}, super().{member}, args, kwargs)\n")
    named = _leading_parameters(getattr(base, member))
    head = "".join(f"{name}, " for name in named)
    passed = "(" + "".join(f"{name}, " for name in named) + "*more)" if named else "more"
    return (f"def {member}(self, {head}*more, **keywords):\n"
            f"    return call({member!r}, super().{member}, {passed}, keywords)\n")


def _leading_parameters(method: Any) -> list[str]:
    """The parameters of ``method`` after ``self`` that callers must pass
    by position, which an override names too (matplotlib reads some
    signatures: ``scale_factory`` passes ``axis`` only to a class naming
    it)."""
    try:
        parameters = list(inspect.signature(method).parameters.values())[1:]
    except (TypeError, ValueError):
        return []
    named = []
    for parameter in parameters:
        if parameter.kind is not inspect.Parameter.POSITIONAL_OR_KEYWORD or parameter.default is not parameter.empty:
            break
        named.append(parameter.name)
    return named


def _classes(cls: str, base: str, body: str) -> str:
    """The source of ``Impl``, ``Sub`` and the scene's hosts."""
    scene = SCENES[cls]
    extra = "name = 'overridden'\n" if cls.startswith("matplotlib.scale.") and "\nname =" not in "\n" + body else ""
    sub = f"@keep\nclass Sub({base}):\n{textwrap.indent(extra + body, '    ')}"
    # Classes the program defines are pickled by name, as a library's are.
    kept = [f"keep({name})" for name in re.findall(r"^class (\w+)\(", scene.impl + scene.hosts, re.M)]
    return "\n".join(part.strip("\n") for part in (scene.impl, sub, scene.hosts, *kept) if part) + "\n"


def _with_takes(steps: tuple[str, ...] | list[str], cls: str, member: str) -> list[str]:
    """Each step followed by the calls it made: all of them, the distinct
    ones of a member whose count is an accident (`DISTINCT`), none of a
    pick that runs the object (`PICK_RUNS`)."""
    taken = "distinct(take())" if member in DISTINCT else "take()"
    statements = []
    for step in steps:
        statements.append(step)
        if step == "fig.pick(event)" and cls in PICK_RUNS:
            statements.append("ignore(take(), 'matplotlib picks the ticks of every axis, which it makes anew')")
        else:
            statements.append(taken)
    return statements


def _program(cls: str, member: str, body: str, changed: bool, later: tuple[str, ...] = ()) -> list[str]:
    scene = SCENES[cls]
    base, _ = _program_base(cls)
    statements = [*SETUP, *scene.imports, f'exec("""\n{_classes(cls, base, body)}""")',
                  *_with_takes((*scene.make, "event = MouseEvent('button_press_event', fig.canvas, 150, 100)"),
                               cls, member)]
    if changed:
        return statements + _with_takes(("fig.canvas.draw()", *later, *(("fig.canvas.draw()",) if later else ()),
                                          *SHOWS,
                                          "fig.savefig(io.BytesIO(), format='png', bbox_inches='tight')", *SHOWS),
                                         cls, member)
    life = ("fig.canvas.draw()", "fig.canvas.draw()", scene.prop, "fig.canvas.draw()", *scene.steps,
            *member_steps(cls, member), *MEASURE, *ENDS[scene.kind])
    return statements + _with_takes(life, cls, member)


def _short(cls: str) -> str:
    return cls.rpartition(".")[2]


def _census_rows() -> list[dict[str, Any]]:
    """The census' rows of classes with a scene, most libraries first."""
    rows = json.loads(CENSUS.read_text(encoding="utf-8"))["rows"]
    return [row for row in rows if row["class"] in SCENES]


@functools.cache
def _self_calls(klass: type) -> dict[str, int]:
    """How often the methods of ``klass`` (matplotlib's) call each name on
    ``self``."""
    try:
        tree = ast.parse(textwrap.dedent(inspect.getsource(klass)))
    except (OSError, TypeError, SyntaxError):
        return {}
    counts: dict[str, int] = {}
    for node in ast.walk(tree):
        if (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                and isinstance(node.func.value, ast.Name) and node.func.value.id == "self"):
            counts[node.func.attr] = counts.get(node.func.attr, 0) + 1
    return counts


def self_called(cls: str, base: type, taken: set[str]) -> list[str]:
    """The methods matplotlib's classes in ``base``'s MRO call on ``self``,
    most called first, without those in ``taken``."""
    counts: dict[str, int] = {}
    for klass in base.__mro__:
        if klass.__module__.startswith(("matplotlib", "mpl_toolkits")):
            for name, count in _self_calls(klass).items():
                counts[name] = counts.get(name, 0) + count
    methods = [name for name in counts if not name.startswith("__") and name not in taken
               and inspect.isfunction(inspect.getattr_static(base, name, None))]
    methods.sort(key=lambda name: (-counts[name], name))
    return methods


def programs() -> Iterator[tuple[str, list[str]]]:
    """The census' members, then the members matplotlib calls on ``self``."""
    taken: dict[str, set[str]] = {cls: set() for cls in SCENES}
    for row in _census_rows():
        cls, member = row["class"], row["member"]
        base_name, base = _program_base(cls)
        body = _override(member, row["kind"], base)
        if body is None:
            continue
        taken[cls].add(member)
        name = f"{_short(cls)}-{member}"
        yield name, _program(cls, member, body, changed=False)
        if (cls, member) in CHANGES:
            yield f"{name}-changed", _program(cls, member, CHANGES[cls, member], changed=True)
        elif (cls, member) in ATTRIBUTES:
            yield f"{name}-changed", _program(cls, member, f"{member} = {ATTRIBUTES[cls, member]}\n", changed=True)
        if (cls, member) in LATER:
            later_body, steps = LATER[cls, member]
            yield f"{name}-changed-later", _program(cls, member, later_body, changed=True, later=steps)
    for cls in SCENES:
        _, base = _program_base(cls)
        members = self_called(cls, base, taken[cls])[:SELF_CALLED]
        if SCENES[cls].kind == "axes" and cls.endswith(".Axes"):
            # The rest of the ``add_*`` family, which libraries override to
            # register what is plotted.
            members += [name for name in ADDING if name not in taken[cls] and name not in members]
        for member in members:
            yield f"{_short(cls)}-{member}", _program(cls, member, _override(member, "method", base), changed=False)
