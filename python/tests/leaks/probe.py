"""
The leak harness's child: runs scenes in this process and prints one
JSON line per scene.

For a dropped scene it first runs the scene once (imports, caches and
the font list warm up), then once more with weak references taken to
every figure the scene left, its canvas, its Axes, every artist
``findobj`` reaches and every object of the library's (or of a class the
scene defined) the scene's names hold; it drops them all, closes every
pyplot figure, collects garbage, and counts what is still alive, by
class. Then it runs the scene *iterations* times, measuring the process
(`measure`) at a quarter, half, three quarters and all of them.

For a kept scene it makes the figure once and runs the update
*iterations* times on it, checking at each one that what the update
before removed (the scene's ``removed`` names) is gone, and measuring
as above.

    python -m leaks.probe [--iterations N] [--dropped-iterations M] SCENE...

Run under pltrs, or under matplotlib through the alias of
``leaks/alias`` (the runner does both).
"""

import argparse
import ctypes
import gc
import io
import json
import os
import sys
import time
import traceback
import weakref

from . import scenes as catalogue

#: The library's top-level package names, as the classes of the objects
#: a scene makes report them.
LIBRARIES = ("pltrs", "matplotlib", "mpl_toolkits")


def _mallinfo():
    """glibc's ``mallinfo2``, or None where there is none."""
    try:
        function = ctypes.CDLL(None).mallinfo2
    except (AttributeError, OSError):
        return None

    class Info(ctypes.Structure):
        _fields_ = [(name, ctypes.c_size_t) for name in (
            "arena", "ordblks", "smblks", "hblks", "hblkhd", "usmblks", "fsmblks", "uordblks",
            "fordblks", "keepcost")]

    function.restype = Info
    return function


_MALLINFO = _mallinfo()


def _resident():
    """The process's resident memory in bytes (0 where ``/proc`` is
    missing)."""
    try:
        with open("/proc/self/statm", encoding="ascii") as statm:
            return int(statm.read().split()[1]) * os.sysconf("SC_PAGE_SIZE")
    except OSError:
        return 0


def _native_counts():
    """pltrs (Rust)'s counts of what it keeps alive (none under
    matplotlib)."""
    import pltrs
    native = getattr(pltrs, "_native", None)
    diagnostics = getattr(native, "diagnostics", None)
    return diagnostics.live_counts() if diagnostics is not None else {}


def measure():
    """What the process holds after a full collection: resident memory,
    the bytes malloc handed out and not got back (``heap``, glibc only),
    the objects Python's collector tracks, and pltrs (Rust)'s counts."""
    gc.collect()
    info = _MALLINFO() if _MALLINFO is not None else None
    return {"resident": _resident(),
            "heap": info.uordblks + info.hblkhd if info is not None else 0,
            "objects": len(gc.get_objects()),
            **_native_counts()}


def _base_namespace():
    namespace = {"__name__": "__leaks__"}
    for statement in catalogue.HEADER:
        exec(statement, namespace)  # noqa: S102 - the harness's own statements
    return namespace


def _run(statements, namespace):
    for statement in statements:
        exec(compile(statement, "<scene>", "exec"), namespace)  # noqa: S102


def _is_library_object(value, scene_classes, packages=()):
    module = (type(value).__module__ or "").split(".")[0]
    return module in LIBRARIES or module in packages or type(value) in scene_classes


def _forget(namespace):
    """Drops the scene's names, as a script rebinding them leaves its
    module: what the scene's functions and classes refer to through their
    globals (the namespace) is no longer reached through them, but what
    they keep themselves (a transform keeping its Axes) is."""
    namespace.clear()


def _figures(namespace):
    """The root figures the scene left: pyplot's and those its names
    reach."""
    import pltrs.pyplot as plt
    from pltrs.figure import FigureBase
    found = {}
    for number in plt.get_fignums():
        figure = plt.figure(number)
        found[id(figure)] = figure
    for value in list(namespace.values()):
        candidates = value if isinstance(value, (list, tuple)) else [value]
        for candidate in candidates:
            figure = getattr(candidate, "figure", None) if not isinstance(
                candidate, FigureBase) else candidate
            if isinstance(figure, FigureBase):
                root = figure.get_figure(root=True)
                found[id(root)] = root
    return list(found.values())


def _draw_and_save(figures, formats):
    for figure in figures:
        figure.canvas.draw()
        for kind in formats:
            figure.savefig(io.BytesIO(), format=kind)


def _references(figures, namespace, scene_classes, packages=()):
    """Weak references to everything the scene made that must go with
    it (the objects of *packages* too), with each one's class name."""
    objects = {}
    for figure in figures:
        objects[id(figure)] = figure
        objects[id(figure.canvas)] = figure.canvas
        for artist in figure.findobj():
            objects[id(artist)] = artist
        for axes in figure.axes:
            objects[id(axes)] = axes
            for axis in (axes.xaxis, axes.yaxis):
                objects[id(axis)] = axis
    for value in namespace.values():
        if _is_library_object(value, scene_classes, packages):
            objects[id(value)] = value
    references = []
    for value in objects.values():
        try:
            references.append((type(value).__name__, weakref.ref(value)))
        except TypeError:
            pass
    return references


def _scene_classes(namespace, base):
    return {value for name, value in namespace.items()
            if isinstance(value, type) and base.get(name) is not value}


def _close_all():
    import pltrs.pyplot as plt
    plt.close("all")


def _once(scene, base):
    """Runs the dropped scene once; its namespace and figures."""
    namespace = dict(base)
    _run(scene.make, namespace)
    figures = _figures(namespace)
    _draw_and_save(figures, scene.formats)
    return namespace, figures


def survivors_of_dropped(scene, base):
    """The objects of the dropped *scene* alive once it is dropped, by
    class name."""
    namespace, figures = _once(scene, base)
    references = _references(figures, namespace, _scene_classes(namespace, base),
                             scene.packages)
    _forget(namespace)
    del namespace, figures
    _close_all()
    gc.collect()
    gc.collect()
    alive = {}
    for name, reference in references:
        if reference() is not None:
            alive[name] = alive.get(name, 0) + 1
    return alive, len(references)


#: How many times a growth loop measures the process.
SAMPLES = 16


def _checkpoints(iterations):
    """The iterations after which the process is measured."""
    return {max(1, iterations * share // SAMPLES) for share in range(1, SAMPLES + 1)}


def grow_dropped(scene, base, iterations):
    """Measures while the dropped *scene* is made and dropped
    *iterations* times."""
    samples = {}
    checkpoints = _checkpoints(iterations)
    for count in range(1, iterations + 1):
        namespace, figures = _once(scene, base)
        _forget(namespace)
        del namespace, figures
        _close_all()
        if count in checkpoints:
            samples[count] = measure()
    return samples


def _settle(figures):
    """Under pltrs, has the kept *figures* let go of what they keep a
    while (the objects of removed artists, swept now and then, and the
    removed artists pltrs (Rust) lets go before a draw), so that the
    measures of a kept scene follow what it keeps for good rather than
    where the sweeps stand."""
    import pltrs
    handles = getattr(pltrs, "_handles", None)
    if handles is None:
        return
    for figure in figures:
        handles.forget_removed(figure)
        gc.collect()
        figure.canvas.draw()


def run_kept(scene, base, iterations):
    """Runs the kept *scene*'s update *iterations* times; the samples, and
    the objects the updates of the first half removed (the scene's
    ``removed`` names, as each update left them) still alive at the end:
    a library may keep what was removed a while (a cache, a registry
    swept now and then), but not for good."""
    namespace = dict(base)
    _run(scene.make, namespace)
    figures = _figures(namespace)
    _draw_and_save(figures, scene.formats)
    samples = {}
    checkpoints = _checkpoints(iterations)
    removed = []
    for count in range(1, iterations + 1):
        namespace["i"] = count
        _run(scene.update, namespace)
        if count <= iterations // 2:
            removed += [(type(namespace[name]).__name__, weakref.ref(namespace[name]))
                        for name in scene.removed if name in namespace]
        if count in checkpoints:
            _settle(figures)
            samples[count] = measure()
    # The names hold what the last update made, which stays.
    del namespace, figures
    gc.collect()
    alive = {}
    for name, reference in removed:
        if reference() is not None:
            alive[name] = alive.get(name, 0) + 1
    _close_all()
    return samples, alive, len(removed)


def run_scene(name, scene, base, iterations, dropped_iterations=None):
    """The record of one scene: a kept one updated *iterations* times, a
    dropped one made *dropped_iterations* times (as often as a kept one
    without) after its check."""
    record = {"scene": name, "kept": scene.kept}
    started = time.perf_counter()
    try:
        if scene.kept:
            record["samples"], record["alive"], record["watched"] = run_kept(
                scene, base, iterations)
        else:
            _once(scene, base)
            _close_all()
            record["alive"], record["watched"] = survivors_of_dropped(scene, base)
            record["after"] = measure()
            dropped = iterations if dropped_iterations is None else dropped_iterations
            if dropped:
                record["samples"] = grow_dropped(scene, base, dropped)
    except Exception as error:  # noqa: BLE001 - the scene's error, reported
        record["error"] = f"{type(error).__name__}: {error}"
        record["traceback"] = traceback.format_exc(limit=-4)
        _close_all()
    record["seconds"] = round(time.perf_counter() - started, 3)
    return record


def main(argv=None):
    """Runs the scenes the command line names."""
    parser = argparse.ArgumentParser(prog="python -m leaks.probe")
    parser.add_argument("--iterations", type=int, default=0)
    parser.add_argument("--dropped-iterations", type=int, default=None)
    parser.add_argument("scenes", nargs="+")
    arguments = parser.parse_args(argv)
    import pltrs
    pltrs.use("agg")
    import warnings
    warnings.simplefilter("ignore")
    every = catalogue.all_scenes()
    base = _base_namespace()
    print(json.dumps({"start": measure()}), flush=True)
    for name in arguments.scenes:
        print(json.dumps(run_scene(name, every[name], base, arguments.iterations,
                                   arguments.dropped_iterations)), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
