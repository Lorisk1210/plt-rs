"""What one implementation shows of matplotlib's private names, for
``private.py``.

    python private_dump.py CENSUS OUT_JSON [--plan MPL_JSON]
    python private_dump.py CENSUS OUT_JSON --writes WRITES_JSON --images DIR

Run under matplotlib first, without ``--plan``: it builds the figures of
the generators' scenes (`SCENES`), in each of their states (`STATES`:
as built, drawn, unpickled and drawn, every Axes cleared), and objects
constructed alone (`FRESH`); walks each (`private_walk.walk`); and for
every object that is an instance of a class the census names for a
private name (its owner, or the candidates of an unknown receiver),
describes that name on it: present or not, on the class or the
instance, and its value (`private_walk.describe`; reading it, never
calling it). Module names are described on their module.

Run under pltrs with ``--plan`` (matplotlib's output), it describes the
same names at the same paths, so that both are keyed alike.

With ``--writes``, it carries out the assignments ``private.py`` planned
from matplotlib's values: for each, it builds the scene, draws it,
assigns the value to the name at the path, and describes what follows:
the name read back, the public getter matplotlib's class has for it
(``get_<name>``), the name on the unpickled figure, and the figure drawn
before and after (``DIR/<id>.before.png``, ``DIR/<id>.png``).
"""

from __future__ import annotations

import argparse
import importlib
import io
import json
import pathlib
import pickle
import sys
import traceback
import warnings
from typing import Any, Iterator

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from generators import common, interaction_scenes, pickling_scenes  # noqa: E402
from private_walk import describe, walk  # noqa: E402

#: The statements every scene starts with.
SETUP = [*common.HEADER, *common.DATA, "import copy, io, pickle"]
#: The figure states described, in the order they are reached.
STATES = ("built", "drawn", "unpickled", "cleared")
#: Objects made alone ("fresh"), by class: how a program makes one.
FRESH = {
    "matplotlib.artist.Artist": "matplotlib.artist.Artist()",
    "matplotlib.lines.Line2D": "lines.Line2D([0, 1], [0, 1])",
    "matplotlib.text.Text": "text.Text(0, 0, 'text')",
    "matplotlib.text.Annotation": "text.Annotation('a', (0, 0))",
    "matplotlib.patches.Rectangle": "patches.Rectangle((0, 0), 1, 1)",
    "matplotlib.patches.PathPatch": "patches.PathPatch(path.Path([(0, 0), (1, 1)]))",
    "matplotlib.patches.Arc": "patches.Arc((0, 0), 1, 1)",
    "matplotlib.patches.FancyArrowPatch": "patches.FancyArrowPatch((0, 0), (1, 1))",
    "matplotlib.collections.PathCollection": "collections.PathCollection([path.Path.unit_circle()])",
    "matplotlib.collections.LineCollection": "collections.LineCollection([[(0, 0), (1, 1)]])",
    "matplotlib.figure.Figure": "matplotlib.figure.Figure()",
    "matplotlib.path.Path": "path.Path([(0, 0), (1, 1)])",
    "matplotlib.transforms.Affine2D": "transforms.Affine2D()",
    "matplotlib.transforms.Bbox": "transforms.Bbox.unit()",
    "matplotlib.colors.LinearSegmentedColormap": "matplotlib.colormaps['coolwarm']",
    "matplotlib.colors.ListedColormap": "matplotlib.colormaps['viridis']",
    "matplotlib.colors.Colormap": "matplotlib.colormaps['viridis']",
    "matplotlib.colors.Normalize": "colors.Normalize(0, 1)",
    "matplotlib.colors.BoundaryNorm": "colors.BoundaryNorm([0, 1, 2], 2)",
    "matplotlib.ticker.MaxNLocator": "ticker.MaxNLocator()",
    "matplotlib.ticker.ScalarFormatter": "ticker.ScalarFormatter()",
    "matplotlib.ticker.PercentFormatter": "ticker.PercentFormatter()",
    "matplotlib.ticker.LogitFormatter": "ticker.LogitFormatter()",
    "matplotlib.markers.MarkerStyle": "matplotlib.markers.MarkerStyle('o')",
    "matplotlib.font_manager.FontProperties": "matplotlib.font_manager.FontProperties()",
    "matplotlib.offsetbox.TextArea": "matplotlib.offsetbox.TextArea('t')",
    "matplotlib.offsetbox.AnchoredOffsetbox": "matplotlib.offsetbox.AnchoredText('t', loc=1)",
    "matplotlib.table.Cell": "matplotlib.table.Cell((0, 0), 1, 1, text='c')",
    "matplotlib.scale.FuncTransform": "scale.FuncTransform(lambda v: v, lambda v: v)",
    "matplotlib.gridspec.GridSpecBase": "matplotlib.gridspec.GridSpec(2, 2)",
    "matplotlib.dates.AutoDateLocator": "__import__('matplotlib.dates').dates.AutoDateLocator()",
    "matplotlib.dates.AutoDateFormatter": ("__import__('matplotlib.dates').dates.AutoDateFormatter("
                                           "__import__('matplotlib.dates').dates.AutoDateLocator())"),
    "matplotlib.backend_bases.GraphicsContextBase": "matplotlib.backend_bases.GraphicsContextBase()",
    "matplotlib.axes._base._process_plot_var_args": "plt.figure().add_subplot()._get_lines",
    "matplotlib.axes._base._PropCycle": "plt.figure().add_subplot()._get_lines._prop_cycle",
    "matplotlib.cm.ColormapRegistry": "matplotlib.colormaps",
    "matplotlib.RcParams": "matplotlib.rcParams",
    "matplotlib.animation.FuncAnimation": ("__import__('matplotlib.animation').animation.FuncAnimation("
                                           "plt.figure(), lambda i: [], frames=2, cache_frame_data=False)"),
    "matplotlib.animation.ArtistAnimation": ("__import__('matplotlib.animation').animation.ArtistAnimation("
                                             "plt.figure(), [[]])"),
    "matplotlib.projections.geo.LambertAxes": "plt.figure().add_subplot(projection='lambert')",
    "matplotlib.projections.geo.GeoAxes": "plt.figure().add_subplot(projection='hammer')",
    "matplotlib.legend_handler.HandlerPatch": "__import__('matplotlib.legend_handler').legend_handler.HandlerPatch()",
    "matplotlib.mathtext.MathTextParser": "__import__('matplotlib.mathtext').mathtext.MathTextParser('agg')",
    "matplotlib.patheffects.TickedStroke": "__import__('matplotlib.patheffects').patheffects.TickedStroke()",
    "matplotlib.patheffects.SimpleLineShadow": "__import__('matplotlib.patheffects').patheffects.SimpleLineShadow()",
    "matplotlib.patheffects.Stroke": "__import__('matplotlib.patheffects').patheffects.Stroke()",
    "mpl_toolkits.mplot3d.art3d.Patch3D": "__import__('mpl_toolkits.mplot3d.art3d').mplot3d.art3d.Patch3D()",
    "matplotlib.backend_bases.Event": "matplotlib.backend_bases.Event('x', plt.figure().canvas)",
    "matplotlib.backend_bases.TimerBase": "plt.figure().canvas.new_timer()",
    "matplotlib.colorizer.Colorizer": "__import__('matplotlib.colorizer').colorizer.Colorizer()",
    "matplotlib.widgets.SliderBase": ("__import__('matplotlib.widgets').widgets.Slider("
                                      "plt.figure().add_subplot(), 's', 0, 1)"),
    "matplotlib.widgets.TextBox": "__import__('matplotlib.widgets').widgets.TextBox(plt.figure().add_subplot(), 't')",
    "matplotlib.widgets.Button": "__import__('matplotlib.widgets').widgets.Button(plt.figure().add_subplot(), 'b')",
    "matplotlib.patches.Arrow": "patches.Arrow(0, 0, 1, 1)",
    "matplotlib.patches.FancyArrow": "patches.FancyArrow(0, 0, 1, 1)",
    "matplotlib.quiver.Barbs": "plt.figure().add_subplot().barbs([0], [0], [1], [1])",
    "matplotlib.axes._secondary_axes.SecondaryAxis": "plt.figure().add_subplot().secondary_xaxis('top')",
    "mpl_toolkits.axes_grid1.axes_divider.Divider": ("__import__('mpl_toolkits.axes_grid1').axes_grid1"
                                                     ".make_axes_locatable(plt.figure().add_subplot())"),
    "matplotlib.backends.backend_webagg_core.FigureCanvasWebAggCore": (
        "__import__('matplotlib.backends.backend_webagg_core').backends.backend_webagg_core"
        ".FigureCanvasWebAggCore(matplotlib.figure.Figure())"),
}


def scenes() -> Iterator[tuple[str, list[str]]]:
    """The scenes: the pickling generator's figures, the interaction
    generator's 2-D Axes."""
    for name, (build, _) in {**pickling_scenes.SCENES, **pickling_scenes.UNMANAGED}.items():
        yield f"pickling/{name}", build
    for name, build in interaction_scenes.SCENES.items():
        yield f"interaction/{name}", build


def _run(statements: list[str], namespace: dict[str, Any]) -> None:
    for statement in statements:
        exec(compile(statement, "<scene>", "exec"), namespace)  # noqa: S102 - the generators' own scenes


def _error(exc: BaseException) -> dict[str, str]:
    return {"raises": type(exc).__name__, "message": str(exc)[:200]}


def _resolve(dotted: str) -> Any:
    """The class or module ``dotted`` names in this implementation."""
    parts = dotted.split(".")
    for cut in range(len(parts), 0, -1):
        try:
            value: Any = importlib.import_module(".".join(parts[:cut]))
        except Exception:  # noqa: BLE001 - not a module
            continue
        for part in parts[cut:]:
            value = getattr(value, part, None)
        return value
    return None


def _probe(obj: Any, name: str, paths: dict[int, str]) -> dict[str, Any]:
    """What the private ``name`` is on ``obj``."""
    on_class = None
    for klass in type(obj).__mro__:
        if name in vars(klass):
            on_class = type(vars(klass)[name]).__name__
            break
    in_dict = name in getattr(obj, "__dict__", {})
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            value = getattr(obj, name)
    except Exception as exc:  # noqa: BLE001 - recorded
        return {"present": False, "on_class": on_class, "in_dict": in_dict, "error": _error(exc)}
    return {"present": True, "on_class": on_class, "in_dict": in_dict, "value": describe(value, paths)}


class Census:
    """The census' names by the classes and modules they are looked up on."""

    def __init__(self, path: pathlib.Path) -> None:
        rows = json.loads(path.read_text())["rows"]
        self.by_class: dict[str, set[str]] = {}
        self.by_module: dict[str, set[str]] = {}
        for row in rows:
            owner = row["owner"]
            if owner is not None and row["kind"].startswith("module"):
                self.by_module.setdefault(owner, set()).add(row["name"])
                continue
            for cls in [owner] if owner is not None else row["candidates"]:
                self.by_class.setdefault(cls, set()).add(row["name"])
        self.classes = {name: _resolve(name) for name in self.by_class}

    def classes_of(self, obj: Any) -> list[str]:
        """The census' classes ``obj`` is an instance of."""
        return sorted(name for name, cls in self.classes.items() if isinstance(cls, type) and isinstance(obj, cls))

    def names_of(self, obj: Any) -> set[str]:
        """The names the census looks up on ``obj``."""
        return set().union(*(self.by_class[name] for name in self.classes_of(obj)))


def _states(build: list[str]) -> Iterator[tuple[str, dict[str, Any] | None, str | None]]:
    """``(state, roots, error)`` of a scene's figure, state by state."""
    namespace: dict[str, Any] = {}
    try:
        _run(SETUP + build, namespace)
    except Exception:  # noqa: BLE001 - recorded
        yield "built", None, traceback.format_exc(limit=2)[-400:]
        return
    fig = namespace["fig"]
    yield "built", {"fig": fig}, None
    steps = (("drawn", lambda: fig.canvas.draw() or fig),
             ("unpickled", lambda: _drawn(pickle.loads(pickle.dumps(fig)))),
             ("cleared", lambda: _cleared(fig)))
    for state, step in steps:
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                root = step()
        except Exception:  # noqa: BLE001 - recorded
            yield state, None, traceback.format_exc(limit=2)[-400:]
            continue
        yield state, {"fig": root}, None


def _drawn(fig: Any) -> Any:
    fig.canvas.draw()
    return fig


def _cleared(fig: Any) -> Any:
    for ax in fig.axes:
        ax.cla()
    return fig


def describe_all(census: Census, plan: dict[str, Any] | None) -> dict[str, Any]:
    """Every census name on every object it is looked up on (those of
    ``plan``, matplotlib's output, when given)."""
    import matplotlib.pyplot as plt

    out: dict[str, Any] = {"objects": {}, "errors": {}, "modules": {}}
    planned = plan["objects"] if plan else None
    for scene, build in scenes():
        for state, roots, error in _states(build):
            key = f"{scene}@{state}"
            if roots is None:
                out["errors"][key] = error
                continue
            _describe_state(census, key, roots, planned, out)
        plt.close("all")
    for cls, expression in FRESH.items():
        key = f"fresh/{cls}@fresh"
        namespace: dict[str, Any] = {}
        try:
            _run(SETUP + [f"obj = {expression}"], namespace)
        except Exception:  # noqa: BLE001 - recorded
            out["errors"][key] = traceback.format_exc(limit=2)[-400:]
            continue
        _describe_state(census, key, {"obj": namespace["obj"]}, planned, out)
    for module, names in census.by_module.items():
        value = _resolve(module)
        for name in sorted(names):
            out["modules"][f"{module}:{name}"] = (_probe(value, name, {}) if value is not None
                                                  else {"present": False, "error": {"raises": "ImportError"}})
    return out


def _describe_state(census: Census, key: str, roots: dict[str, Any], planned: dict[str, Any] | None,
                    out: dict[str, Any]) -> None:
    objects = walk(roots)
    paths = {id(obj): path for path, obj in objects.items()}
    wanted = None
    if planned is not None:
        wanted = {k.split("|", 1)[1]: {n for n in record if not n.startswith("@")}
                  for k, record in planned.items() if k.split("|", 1)[0] == key}
    targets = wanted.keys() if wanted is not None else objects.keys()
    for path in targets:
        obj = objects.get(path)
        names = wanted[path] if wanted is not None else census.names_of(obj)
        if not names:
            continue
        record = out["objects"].setdefault(f"{key}|{path}", {})
        if obj is None:
            record.update({name: {"present": False, "missing_object": True} for name in names})
            continue
        record["@class"] = type(obj).__qualname__
        if planned is None:
            record["@census"] = census.classes_of(obj)
            record["@getters"] = {name: f"get{name}" for name in names if callable(getattr(type(obj), f"get{name}",
                                                                                            None))}
        for name in sorted(names):
            record[name] = _probe(obj, name, paths)


def carry_out(writes: list[dict[str, Any]], images: pathlib.Path) -> dict[str, Any]:
    """The assignments of ``writes``, each in a scene of its own."""
    images.mkdir(parents=True, exist_ok=True)
    out = {}
    builds = dict(scenes())
    for write in writes:
        out[write["id"]] = _write(write, builds, images)
    return out


def _write(write: dict[str, Any], builds: dict[str, list[str]], images: pathlib.Path) -> dict[str, Any]:
    namespace: dict[str, Any] = {}
    try:
        _run(SETUP + builds[write["scene"]], namespace)
        fig = namespace["fig"]
        fig.canvas.draw()
        objects = walk({"fig": fig})
        obj = objects[write["path"]]
        paths = {id(o): p for p, o in objects.items()}
        value = (objects[write["value_path"]] if "value_path" in write
                 else eval(write["literal"], namespace))  # noqa: S307 - a literal the planner wrote
    except Exception as exc:  # noqa: BLE001 - recorded
        return {"setup": _error(exc)}
    result: dict[str, Any] = {}
    name = write["name"]
    _save(fig, images / f"{write['id']}.before.png")
    try:
        setattr(obj, name, value)
    except Exception as exc:  # noqa: BLE001 - recorded
        return {"assign": _error(exc)}
    result["read_back"] = _probe(obj, name, paths)
    getter = write.get("getter")
    if getter:
        try:
            result["getter"] = describe(getattr(obj, getter)(), paths)
        except Exception as exc:  # noqa: BLE001 - recorded
            result["getter"] = _error(exc)
    try:
        _save(fig, images / f"{write['id']}.png")
        result["drawn"] = True
    except Exception as exc:  # noqa: BLE001 - recorded
        result["drawn"] = _error(exc)
    try:
        again = walk({"fig": pickle.loads(pickle.dumps(fig))})
        result["unpickled"] = _probe(again[write["path"]], name, {id(o): p for p, o in again.items()})
    except Exception as exc:  # noqa: BLE001 - recorded
        result["unpickled"] = _error(exc)
    return result


def _save(fig: Any, path: pathlib.Path) -> None:
    """Draw ``fig`` into the PNG file ``path``, small."""
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png", dpi=40)
    path.write_bytes(buffer.getvalue())


def main() -> int:
    """Describe or write, as the arguments say."""
    parser = argparse.ArgumentParser()
    parser.add_argument("census", type=pathlib.Path)
    parser.add_argument("out", type=pathlib.Path)
    parser.add_argument("--plan", type=pathlib.Path)
    parser.add_argument("--writes", type=pathlib.Path)
    parser.add_argument("--images", type=pathlib.Path)
    args = parser.parse_args()
    import matplotlib
    matplotlib.use("agg")
    if args.writes is not None:
        result = carry_out(json.loads(args.writes.read_text()), args.images)
    else:
        plan = json.loads(args.plan.read_text()) if args.plan else None
        result = describe_all(Census(args.census), plan)
    args.out.write_text(json.dumps(result, indent=1, default=repr) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
