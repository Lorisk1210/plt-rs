"""
The fuzzer's actions on the paths through which pltrs (Rust) calls the
program back, or the program reaches past the public API: subclasses
overriding the members libraries override (the census
``spec/diff/overrides_census.json``, placed as the ``overrides`` family's
scenes place them), private names libraries read, write and call (the
census ``spec/diff/private_census.json``), callbacks that enter their
figure while native code runs them (the ``reentry`` family's kinds),
assignments to the public attributes matplotlib's classes let programs
assign, and combinations of them (an ``add_line`` of the program's
removing an artist, a formatter assigning the figure's patch, children
of another figure).

Each action is one line of the program (several statements, or a class,
in one source line), written from what the program made so far, so that
a program stays a plain script; the overridden members and the
callbacks run another random action of the program's, once at a time.
"""

import functools
import json
import pathlib
import sys

from . import values

#: The repository's root.
ROOT = pathlib.Path(__file__).resolve().parents[3]

#: Data the scenes of the differential tester's families use.
DATA = ("x = np.linspace(0, 2 * np.pi, 7); y = np.sin(x); "
        "z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)); "
        "values = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])")

#: The modules the scenes and the callbacks name, beyond the header's.
IMPORTS = ("from pltrs import (artist, axes, axis, colorbar, contour, dates, "
           "figure, gridspec, image, layout_engine, legend, legend_handler, offsetbox, "
           "projections, spines, units)\n"
           "from pltrs.backend_bases import ResizeEvent")

#: The public attributes programs may assign (matplotlib's classes keep
#: them as plain attributes), by class name, with values to assign.
ASSIGNABLE = {
    "Wedge": {"r": ["0.5", "0", "-1"], "theta1": ["10", "400"], "theta2": ["200", "-30"],
              "width": ["0.1", "None"], "center": ["(0.5, 0.5)", "(1e300, 0)"]},
    "RegularPolygon": {"xy": ["(0.3, 0.3)"], "radius": ["0.2", "0"], "orientation": ["1.0"],
                       "numvertices": ["3", "0"]},
    "Arc": {"theta1": ["0", "720"], "theta2": ["90", "'x'"]},
    "StepPatch": {"orientation": ["'horizontal'", "'vertical'", "'diagonal'"]},
    "FancyArrow": {"verts": ["[[0, 0], [1, 0], [0, 1]]", "[]"]},
    "QuadContourSet": {"layers": ["[0, 1]", "[]"], "zmin": ["-1", "float('nan')"],
                       "zmax": ["2", "None"]},
    "AxesImage": {"origin": ["'lower'", "'upper'", "'sideways'"]},
    "Axes": {"axison": ["False", "True"], "callbacks": ["cbook.CallbackRegistry()", "None"]},
    "Legend": {"texts": ["[]", "None"], "legend_handles": ["[]", "None"],
               "legendPatch": ["patches.FancyBboxPatch((0, 0), 1, 1)", "None"]},
    "Annotation": {"arrow_patch": ["patches.FancyArrowPatch((0, 0), (1, 1))", "None"]},
    "Figure": {"patch": ["patches.Rectangle((0, 0), 1, 1, fc='r')", "None"]},
}

#: The combinations the brief's reviewers named, as templates on the
#: program's names (``{fig}``, ``{ax}``, ``{artist}``, ``{other}``: a figure, an
#: Axes, an artist, another figure).
COMBINATIONS = [
    # An add_line of the program's removing an artist when it adds.
    "class _RemovingAxes(axes.Axes):\n"
    "    def add_line(self, line):\n"
    "        added = super().add_line(line)\n"
    "        try:\n"
    "            {artist}.remove()\n"
    "        except Exception:\n"
    "            pass\n"
    "        return added\n"
    "_removing = {fig}.add_subplot(axes_class=_RemovingAxes); _removing.plot([1, 2], [2, 1]); "
    "_removing.errorbar([1, 2], [1, 2], yerr=0.1); {fig}.canvas.draw()",
    # A formatter assigning the figure's patch while ticks are labelled.
    "{ax}.xaxis.set_major_formatter(ticker.FuncFormatter(lambda v, p: "
    "(setattr({fig}, 'patch', patches.Rectangle((0, 0), 1, 1)), f'{{v:g}}')[1])); "
    "{fig}.canvas.draw()",
    # An artist whose children are artists of another figure.
    "class _Foreign(pltrs.artist.Artist):\n"
    "    def get_children(self):\n"
    "        return [*{other}.axes, *{other}.get_children()]\n"
    "    def draw(self, renderer):\n"
    "        pass\n"
    "_foreign = {ax}.add_artist(_Foreign()); {fig}.canvas.draw(); {fig}.findobj(); "
    "MouseEvent('button_press_event', {fig}.canvas, 100, 100, 1)._process()",
    # A formatter removing the Axes it labels.
    "{ax}.yaxis.set_major_formatter(ticker.FuncFormatter(lambda v, p: "
    "({ax}.remove() if p == 0 else None, str(v))[1])); {fig}.canvas.draw()",
    # A draw event clearing the figure drawn.
    "{fig}.canvas.mpl_connect('draw_event', lambda e: {fig}.clear()); {fig}.canvas.draw(); "
    "{fig}.canvas.draw()",
    # A transform of the program's whose inverse draws its figure.
    "class _Drawing(transforms.Transform):\n"
    "    input_dims = output_dims = 2\n"
    "    def transform_non_affine(self, v):\n"
    "        return v\n"
    "    def inverted(self):\n"
    "        {fig}.canvas.draw()\n"
    "        return transforms.IdentityTransform()\n"
    "{ax}.plot([0, 1], [0, 1], transform=_Drawing() + {ax}.transData); {fig}.canvas.draw(); "
    "MouseEvent('motion_notify_event', {fig}.canvas, 100, 100)._process()",
]


def _generators():
    """The differential tester's scene modules."""
    sys.path.insert(0, str(ROOT / "spec" / "diff"))
    try:
        from generators import overrides_scenes, reentry
    finally:
        sys.path.pop(0)
    return overrides_scenes, reentry


def _to_pltrs(source):
    return (source.replace("mpl_toolkits", "pltrs.mpl_toolkits")
            .replace("matplotlib.", "pltrs.").replace("from matplotlib", "from pltrs"))


@functools.cache
def _overridden():
    """``[(base, member, scene)]``: each census row whose class has a
    scene, the member a method."""
    overrides_scenes, _ = _generators()
    census = json.loads((ROOT / "spec" / "diff" / "overrides_census.json").read_text("utf-8"))
    return [(row["class"], row["member"], overrides_scenes.SCENES[row["class"]])
            for row in census["rows"]
            if row["kind"] == "method" and row["class"] in overrides_scenes.SCENES
            and row["member"] not in ("__init__", "__new__", "__getattr__", "__getattribute__",
                                      "__setattr__", "__del__", "__reduce__", "__getstate__")]


@functools.cache
def _private_names():
    """``{class name: [(name, uses)]}`` of the private census, by the
    short name of the class owning each name."""
    census = json.loads((ROOT / "spec" / "diff" / "private_census.json").read_text("utf-8"))
    found = {}
    for row in census["rows"]:
        owners = [row["owner"]] if row["owner"] else row["candidates"]
        for owner in owners:
            found.setdefault(owner.rpartition(".")[2], []).append(
                (row["name"], tuple(row["uses"])))
    return found


class Paths:
    """The actions on the new paths for a `.actions.Program`."""

    def __init__(self, program):
        self.program = program
        self.rng = program.rng
        self.values = program.values
        self.count = 0

    def line(self):
        """An action on one of the paths, or None when there is nothing to
        act on."""
        rng = self.rng
        action = values.choose(rng, [self._override, self._override, self._private,
                                     self._private, self._reentry, self._assign,
                                     self._combination])
        return action()

    def _body(self):
        """A random action of the program as the body of a callback, with
        a guard against entering itself (``_entered``), or ``pass``."""
        body = None
        for _ in range(4):
            candidate = self.program.next_line(paths=False)
            if candidate and "\n" not in candidate and not candidate.startswith(("v", "def")):
                body = candidate
                break
        if body is None:
            return "pass"
        return (f"if not _entered:\n            _entered.append(1)\n            try:\n"
                f"                {body}\n            finally:\n                _entered.pop()")

    def _override(self):
        """A subclass overriding a member of the census, made and placed as
        the ``overrides`` family's scene of its class places it."""
        base, member, scene = values.choose(self.rng, _overridden())
        self.count += 1
        module, _, name = base.rpartition(".")
        body = self._body() if self.rng.random() < 0.5 else "pass"
        lines = [IMPORTS, DATA, "_entered = []",
                 f"from {_to_pltrs(module)} import {name} as Base"]
        if scene.imports:
            lines += [_to_pltrs(statement) for statement in scene.imports]
        if scene.impl:
            lines += [_to_pltrs(scene.impl.strip()), "Base = Impl"]
        lines.append(f"class Sub(Base):\n    def {member}(self, *args, **kwargs):\n"
                     f"        {body}\n        return super().{member}(*args, **kwargs)")
        if scene.hosts:
            lines.append(_to_pltrs(scene.hosts.strip()))
        lines += [_to_pltrs(statement) for statement in (*scene.make, *scene.steps)]
        if self.rng.random() < 0.6:
            lines.append(scene.prop)
        lines.append("fig.canvas.draw()")
        program = self.program
        program.figures.append("fig")
        program.axes.append("ax")
        program.artists.append("art")
        program.others.append("obj")
        return "\n".join(lines)

    def _private(self):
        """A private name of the census on an object of the program's:
        read, called or assigned (its own value, another object's, or a
        value of any kind)."""
        program = self.program
        target = program._choose(program.artists + program.axes + program.figures
                                 + program.others)
        if target is None:
            return None
        obj = program._value(target)
        names = []
        for cls in type(obj).__mro__:
            names += _private_names().get(cls.__name__, [])
        if not names:
            return None
        name, uses = values.choose(self.rng, names)
        roll = self.rng.random()
        if "written" in uses and roll < 0.4 or roll < 0.15:
            source = values.choose(self.rng, [
                f"{target}.{name}", self.values.any_value(),
                f"{program._choose(program.artists + program.axes) or 'None'}.{name}"])
            return f"{target}.{name} = {source}"
        if "called" in uses and roll < 0.7:
            args = ", ".join(self.values.any_value() for _ in range(self.rng.randrange(3)))
            return f"{program._new_name()} = {target}.{name}({args})"
        return f"{program._new_name()} = {target}.{name}"

    def _reentry(self):
        """A callback of the ``reentry`` family's kinds, entering the
        figure once with a random action of the program's while native
        code runs it."""
        _, reentry = _generators()
        kind = values.choose(self.rng, list(reentry.KINDS.values()))
        body = None
        for _ in range(4):
            candidate = self.program.next_line(paths=False)
            if candidate and "\n" not in candidate:
                body = candidate
                break
        if body is None:
            return None
        install = _to_pltrs(kind.install)
        figure = _to_pltrs(kind.figure).replace("exec(KIND)", install)
        lines = [IMPORTS, f"operation = lambda: exec({body!r})", reentry.HELPERS.strip(), figure,
                 "ax = fig.add_subplot()", "line, = ax.plot([1, 2, 3], [1, 4, 9], label='line')",
                 "text = ax.text(0.5, 0.5, 'text')", "fig.canvas.draw()"]
        if "exec(KIND)" not in kind.figure:
            lines.append(install)
        lines += [_to_pltrs(step) for step in kind.trigger]
        self.program.figures.append("fig")
        self.program.axes.append("ax")
        self.program.artists += ["line", "text"]
        return "\n".join(lines)

    def _assign(self):
        """An assignment to a public attribute matplotlib's class keeps
        as a plain attribute."""
        program = self.program
        candidates = []
        for name in program.artists + program.axes + program.figures:
            obj = program._value(name)
            for cls in type(obj).__mro__:
                if cls.__name__ in ASSIGNABLE:
                    candidates.append((name, ASSIGNABLE[cls.__name__]))
                    break
        if not candidates:
            return None
        target, attributes = values.choose(self.rng, candidates)
        attribute = values.choose(self.rng, list(attributes))
        value = (values.choose(self.rng, attributes[attribute]) if self.rng.random() < 0.7
                 else self.values.any_value())
        return f"from pltrs import cbook; {target}.{attribute} = {value}"

    def _combination(self):
        """One of `COMBINATIONS`, on the program's names."""
        program = self.program
        fig, ax = program._choose(program.figures), program._choose(program.axes)
        if fig is None or ax is None:
            return None
        artist = program._choose(program.artists) or ax
        other = program._choose(program.figures) or fig
        template = values.choose(self.rng, COMBINATIONS)
        return (f"{IMPORTS}\n"
                + template.format(fig=fig, ax=ax, artist=artist, other=other))
