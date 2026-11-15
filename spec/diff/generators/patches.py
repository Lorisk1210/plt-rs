"""Programs replacing one of matplotlib's methods or functions by
assignment, as libraries do (``spec/diff/patches_census.json``,
``patches_census.py``): on a class (``Axes.scatter = f``), on one object
(``axis._update_ticks = f``, ``label.set_y = types.MethodType(f,
label)``) or on a module (``scale.scale_factory = f``).

Each program makes the assignment in a scene of ``overrides_scenes`` (the
scene's class itself standing for ``Sub``): before the scene is made for
a class or a module, after it for an object. The replacement records its
calls through the ``overrides`` recorder (``call``) and calls the
original, which it kept, as the census' libraries do; then the scene
lives as the ``overrides`` programs' do, each step followed by the calls
it made (``take()``). A ``-changed`` variant replaces the result as the
library does (another tick list, a label moved, nothing drawn) and reads
what the figure shows, the figure left open for its image.

Programs are named ``<target>-<owner>-<name>``.
"""

from __future__ import annotations

import json
import pathlib
from dataclasses import dataclass
from typing import Iterator

from . import overrides
from .overrides_changes import SHOWS
from .overrides_scenes import SCENES

CENSUS = pathlib.Path(__file__).resolve().parent.parent / "patches_census.json"


@dataclass(frozen=True)
class Placement:
    """Where a program makes a replacement: the scene (a class of
    ``overrides_scenes``), the expression of what is assigned to (its
    original kept from it), whether before the scene is made, the steps
    reaching the replacement, and a changing replacement's body (a
    function of ``original`` and the arguments)."""

    scene: str
    receiver: str
    before: bool
    steps: tuple[str, ...] = ()
    change: str = ""


#: (target, the owner of the census (or its first candidate), name) ->
#: where the program makes the replacement.
PLACEMENTS: dict[tuple[str, str, str], Placement] = {
    ("instance", "matplotlib.axes._base._AxesBase", "format_coord"): Placement(
        "matplotlib.axes.Axes", "ax", False,
        ("NavigationToolbar2(fig.canvas).mouse_move(MouseEvent('motion_notify_event', fig.canvas, 150, 100))",),
        "'at ' + original(*args, **kwargs)"),
    ("module", "matplotlib.scale", "scale_factory"): Placement(
        "matplotlib.axes.Axes", "scale", True, ("ax.set_xscale('log')", "ax.set_yscale('symlog')")),
    ("class", "matplotlib.collections.PathCollection", "__init__"): Placement(
        "matplotlib.axes.Axes", "collections.PathCollection", True, ("ax.scatter(x, y)",)),
    ("class", "matplotlib.colors.Colormap", "_repr_png_"): Placement(
        "matplotlib.colors.ListedColormap", "colors.Colormap", True, ("ignore(obj._repr_html_())",)),
    ("class", "matplotlib.colors.Colormap", "_repr_html_"): Placement(
        "matplotlib.colors.ListedColormap", "colors.Colormap", True, ("ignore(obj._repr_html_())",)),
    ("instance", "matplotlib.axis.Axis", "_update_ticks"): Placement(
        "matplotlib.axes.Axes", "ax.xaxis", False, (), "original(*args, **kwargs)[::2]"),
    ("instance", "matplotlib.artist.Artist", "draw"): Placement(
        "matplotlib.axes.Axes", "ax", False, (), "None"),
    ("class", "matplotlib.backend_bases.RendererBase", "new_gc"): Placement(
        "matplotlib.lines.Line2D", "matplotlib.backend_bases.RendererBase", True),
    ("class", "matplotlib.axes._axes.Axes", "scatter"): Placement(
        "matplotlib.axes.Axes", "axes.Axes", True, ("plt.sca(ax)", "plt.scatter(x, y)")),
    # pygtc shifts the tick labels so (its receiver, typed as a box's,
    # is a tick label).
    ("instance", "matplotlib.patches.FancyBboxPatch", "set_x"): Placement(
        "matplotlib.axes.Axes", "ax.xaxis.get_major_ticks()[1].label1", False, (),
        "original(args[0] + 0.1, *args[1:], **kwargs)"),
    ("instance", "matplotlib.patches.FancyBboxPatch", "set_y"): Placement(
        "matplotlib.axes.Axes", "ax.yaxis.get_major_ticks()[1].label1", False, (),
        "original(args[0] + 0.1, *args[1:], **kwargs)"),
    ("instance", "matplotlib.text.Text", "update"): Placement(
        "matplotlib.axes.Axes", "ax.title", False, ("plt.setp(ax.title, color='r')", "ax.set_title('t', color='b')")),
}

#: The census' rows no program makes, with the reason.
NOT_GENERATED = {
    ("instance", "matplotlib.backends.backend_qt.FigureCanvasQT", "keyPressEvent"):
        "a Qt widget's event handler: no window here",
    ("instance", "matplotlib.backend_bases.FigureManagerBase", "show"):
        "the inline backend's display of a figure, which no figure of the generators shows",
}

#: The statements keeping the original and making the replacement: a
#: function of the arguments it is called with (the object first for a
#: class' method), recording through ``call``.
_REPLACE = (
    "original_{n} = {receiver}.{name}\n"
    "def replacement_{n}(*args, **kwargs):\n"
    "    return call({name!r}, lambda *args, **kwargs: {result}, args, kwargs)\n"
    "{receiver}.{name} = replacement_{n}"
)


def _key(row: dict) -> tuple[str, str, str]:
    owner = row["owner"] or (row["candidates"][0] if row["candidates"] else "")
    return row["target"], owner, row["name"]


def _replacement(placement: Placement, name: str, changed: bool) -> str:
    result = placement.change if changed else "original(*args, **kwargs)"
    return _REPLACE.format(n=0, receiver=placement.receiver, name=name,
                           result=result.replace("original", "original_0"))


def _program(placement: Placement, name: str, changed: bool) -> list[str]:
    scene = SCENES[placement.scene]
    base, _ = overrides._program_base(placement.scene)
    classes = scene.impl.strip("\n") + "\n" if scene.impl else ""
    statements = [*overrides.SETUP, *scene.imports, "from matplotlib.backend_bases import NavigationToolbar2",
                  f"Sub = {base}"]
    if classes:
        statements.insert(-1, f'exec("""\n{classes}""")')
    replace = _replacement(placement, name, changed)
    if placement.before:
        statements.append(replace)
    statements += overrides._with_takes((*scene.make, "event = MouseEvent('button_press_event', fig.canvas, 150, 100)"),
                                        placement.scene, name)
    if not placement.before:
        statements += [replace, "take()"]
    if changed:
        steps = ("fig.canvas.draw()", *placement.steps, "fig.canvas.draw()", *SHOWS,
                 "fig.savefig(io.BytesIO(), format='png', bbox_inches='tight')", *SHOWS)
    else:
        steps = ("fig.canvas.draw()", "fig.canvas.draw()", scene.prop, "fig.canvas.draw()", *scene.steps,
                 *placement.steps, *overrides.MEASURE, *overrides.ENDS[scene.kind])
    return statements + overrides._with_takes(steps, placement.scene, name)


def _short(dotted: str) -> str:
    return dotted.rpartition(".")[2]


def programs() -> Iterator[tuple[str, list[str]]]:
    """A program per placed row of the census, and its changed variant."""
    rows = json.loads(CENSUS.read_text(encoding="utf-8"))["rows"]
    made = set()
    for row in rows:
        key = _key(row)
        placement = PLACEMENTS.get(key)
        if placement is None or key in made:
            continue
        made.add(key)
        name = f"{key[0]}-{_short(key[1])}-{key[2]}"
        yield name, _program(placement, key[2], changed=False)
        if placement.change:
            yield f"{name}-changed", _program(placement, key[2], changed=True)
