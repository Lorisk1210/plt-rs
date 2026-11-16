"""
What importing pltrs (Python) loads and does (task PERF7): the modules
matplotlib's imports leave loaded are loaded under pltrs too, pltrs' own
helper modules load where they are first used rather than at import (a
ceiling keeps their count from creeping back), and the data directory is
located at import but its files are neither written nor read then.
``benchmarks/import_cost.py`` measures what an import costs.
"""

import importlib.util
import json
import os
import pathlib
import subprocess
import sys

import pytest

# The modules of matplotlib 3.11.2 (and its toolkits) in ``sys.modules``
# after ``import matplotlib`` and after ``import matplotlib.pyplot``,
# recorded from matplotlib.
MATPLOTLIB_IMPORT = """
    matplotlib matplotlib._api matplotlib._api.deprecation
    matplotlib._c_internal_utils matplotlib._cm matplotlib._cm_bivar
    matplotlib._cm_listed matplotlib._cm_multivar matplotlib._color_data
    matplotlib._docstring matplotlib._enums matplotlib._fontconfig_pattern
    matplotlib._path matplotlib._version matplotlib.artist matplotlib.backends
    matplotlib.backends.registry matplotlib.bezier matplotlib.cbook matplotlib.cm
    matplotlib.colorizer matplotlib.colors matplotlib.ft2font matplotlib.mlab
    matplotlib.path matplotlib.rcsetup matplotlib.scale matplotlib.ticker
    matplotlib.transforms
""".split()

MATPLOTLIB_PYPLOT_IMPORT = MATPLOTLIB_IMPORT + """
    matplotlib._afm matplotlib._blocking_input matplotlib._constrained_layout
    matplotlib._image matplotlib._layoutgrid matplotlib._mathtext
    matplotlib._mathtext_data matplotlib._pylab_helpers matplotlib._style_helpers
    matplotlib._text_helpers matplotlib._tight_bbox matplotlib._tight_layout
    matplotlib.axes matplotlib.axes._axes matplotlib.axes._base
    matplotlib.axes._secondary_axes matplotlib.axis matplotlib.backend_bases
    matplotlib.backend_managers matplotlib.backend_tools matplotlib.category
    matplotlib.collections matplotlib.colorbar matplotlib.container
    matplotlib.contour matplotlib.dates matplotlib.dviread matplotlib.figure
    matplotlib.font_manager matplotlib.gridspec matplotlib.hatch matplotlib.image
    matplotlib.inset matplotlib.layout_engine matplotlib.legend
    matplotlib.legend_handler matplotlib.lines matplotlib.markers
    matplotlib.mathtext matplotlib.offsetbox matplotlib.patches
    matplotlib.projections matplotlib.projections.geo matplotlib.projections.polar
    matplotlib.pyplot matplotlib.quiver matplotlib.spines matplotlib.stackplot
    matplotlib.streamplot matplotlib.style matplotlib.table matplotlib.texmanager
    matplotlib.text matplotlib.textpath matplotlib.tri matplotlib.tri._triangulation
    matplotlib.tri._tricontour matplotlib.tri._trifinder
    matplotlib.tri._triinterpolate matplotlib.tri._tripcolor matplotlib.tri._triplot
    matplotlib.tri._trirefine matplotlib.tri._tritools matplotlib.units
    matplotlib.widgets mpl_toolkits mpl_toolkits.mplot3d mpl_toolkits.mplot3d.art3d
    mpl_toolkits.mplot3d.axes3d mpl_toolkits.mplot3d.axis3d
    mpl_toolkits.mplot3d.proj3d
""".split()

#: matplotlib's modules pltrs has no module for: what they hold is in pltrs
#: (Rust) (the colormaps' data, mathtext's tables) or in the package's
#: ``__init__`` (the version).
NOT_IN_PLTRS = {"_cm_bivar", "_cm_listed", "_cm_multivar", "_version",
                "_mathtext_data"}

#: The most modules of pltrs' own each import may load: what they load now.
#: A helper module that a module uses only inside its functions is
#: imported where first needed (``_lazy_imports.module``), not at import.
OWN_MODULES_CEILING = {"pltrs": 57, "pltrs.pyplot": 206}


def _fresh(code, **env):
    """What a fresh interpreter running *code* prints last, read as JSON."""
    environment = {**os.environ, "MPLBACKEND": "agg", **env}
    result = subprocess.run([sys.executable, "-c", code], env=environment,
                            capture_output=True, text=True, check=True)
    return json.loads(result.stdout.splitlines()[-1]), result.stderr


def _loaded(statement):
    """The modules in ``sys.modules`` after *statement*, fresh."""
    modules, _ = _fresh(f"import json, sys\n{statement}\n"
                        "print(json.dumps(sorted(sys.modules)))")
    return set(modules)


def _pltrs_name(name):
    """The name of matplotlib's module *name* in pltrs."""
    top, dot, rest = name.partition(".")
    return f"pltrs{dot}{rest}" if top == "matplotlib" else f"pltrs.{name}"


@pytest.mark.parametrize("statement, recorded", [
    ("import pltrs", MATPLOTLIB_IMPORT),
    ("import pltrs.pyplot", MATPLOTLIB_PYPLOT_IMPORT),
], ids=["pltrs", "pyplot"])
def test_imports_load_what_matplotlibs_load(statement, recorded):
    loaded = _loaded(statement)
    missing = [name for name in recorded if _pltrs_name(name) not in loaded]
    assert {name.rpartition(".")[2] for name in missing} <= NOT_IN_PLTRS, missing
    for name in missing:
        assert importlib.util.find_spec(_pltrs_name(name)) is None, (
            f"pltrs has matplotlib's {name} now: import it where matplotlib does")


def test_pyplot_loads_pillows_image_module():
    assert "PIL.Image" in _loaded("import pltrs.pyplot")


@pytest.mark.parametrize("module", sorted(OWN_MODULES_CEILING))
def test_own_modules_loaded_at_import_stay_under_their_ceiling(module):
    own = [name for name in _loaded(f"import {module}")
           if name.partition(".")[0] == "pltrs"]
    assert len(own) <= OWN_MODULES_CEILING[module], (
        f"`import {module}` loads {len(own)} of pltrs' modules: load the new "
        "ones where they are first used (`_lazy_imports.module`)")


def test_helper_modules_for_unusual_calls_load_when_first_used():
    deferred = ["pltrs._quiver_reference", "pltrs._contour_arguments",
                "pltrs._legend_handlers", "pltrs.axes._foreign_children",
                "pltrs.mpl_toolkits.mplot3d._draw_problems"]
    loaded = _loaded("import pltrs.pyplot")
    assert not [name for name in deferred if name in loaded]
    after, _ = _fresh(
        "import json, sys\nimport pltrs.pyplot as plt\n"
        "fig, ax = plt.subplots()\nax.plot([1, 2], label='a')\nax.legend()\n"
        "ax.contour([[1, 2], [3, 4]])\nfig.canvas.draw()\n"
        "print(json.dumps(sorted(sys.modules)))")
    assert {"pltrs._legend_handlers", "pltrs._contour_arguments"} <= set(after)


#: Imports pltrs in an empty configuration directory, records the files
#: Python opens and what the data directory holds, then asks for the path.
_DATA_PROGRAM = """
import json, logging, os, sys
opened = []
sys.addaudithook(lambda event, args: event == "open" and isinstance(args[0], str)
                 and opened.append(args[0]))
logging.basicConfig(level=logging.DEBUG, format="%(name)s %(message)s")
import pltrs.pyplot
data = os.path.join(os.environ["PLTRSCONFIGDIR"], "mpl-data")
font = os.path.join(data, "fonts", "ttf", "DejaVuSans.ttf")
written_at_import = os.path.exists(font)
read_at_import = [path for path in opened if path.startswith(data)]
path = pltrs.get_data_path()
print(json.dumps([path == data, written_at_import, read_at_import,
                  os.path.isfile(font), os.path.isdir(os.path.join(path, "sample_data"))]))
"""


def test_importing_locates_the_data_directory_without_its_files(tmp_path):
    env = {"PLTRSCONFIGDIR": str(tmp_path)}
    # First in an empty configuration directory, then in the one written.
    for first in (True, False):
        (located, written_at_import, read_at_import, font, sample_data), log = _fresh(
            _DATA_PROGRAM, **env)
        # The path is logged as matplotlib's import logs it, ...
        assert located
        assert f"pltrs matplotlib data path: {tmp_path / 'mpl-data'}" in log
        # ... importing neither writes the files nor reads them, ...
        assert not (first and written_at_import)
        assert read_at_import == []
        # ... and asking for the path afterwards gives them.
        assert font and sample_data


#: Counts Python-level calls (``sys.setprofile`` "call" events) by
#: function, named by its file (within the package, else its name) and
#: qualified name, under the scene `scene` names; the programs below share
#: it.
_COUNTER = """
import collections, json, sys
profiles = collections.defaultdict(collections.Counter)
scene = [None]


def count(frame, event, arg):
    if event == "call":
        code = frame.f_code
        path = code.co_filename
        where = path.rsplit("/pltrs/", 1)[1] if "/pltrs/" in path else path.rsplit("/", 1)[-1]
        profiles[scene[0]][where + ":" + code.co_qualname] += 1


def counted(name, action):
    scene[0] = name
    sys.setprofile(count)
    try:
        return action()
    finally:
        sys.setprofile(None)


def report():
    print(json.dumps({name: dict(calls) for name, calls in profiles.items()}))
"""

#: The most Python-level calls a fresh ``import pltrs.pyplot`` may make,
#: and one ``canvas.draw()`` of a plain figure: a few per cent over what
#: they make, so that work going through Python for every colour, artist
#: or name fails here rather than in a benchmark. At af3ed0db the import
#: made 129,580 calls, 31,606 of them of the program colour hook, as every
#: colour resolved the whole property cycle; PERF8 took it to 102,765 and
#: PERF9 (2026-11-14) to 100,487 on Python 3.12, 101,994 on 3.13 and
#: 101,132 on 3.14. On 2026-11-16 (task PERF11) it makes 101,244 on 3.12,
#: 102,703 on 3.13 and 101,842 on 3.14 (90,531 with numpy 1.26), and a
#: draw makes 68.
PYTHON_CALLS_CEILING = {"import": 104_900, "draw": 72}

#: Counts the Python calls of ``import pltrs.pyplot`` (the third-party
#: modules it uses imported first) and of a second draw of a plain figure.
_CALLS_PROGRAM = _COUNTER + """
import numpy, PIL.Image, dateutil.parser
plt = counted("import", lambda: __import__("pltrs.pyplot").pyplot)
fig, ax = plt.subplots()
ax.plot([1, 2, 3], [1, 4, 9], label="line")
ax.set_title("title")
ax.legend()
fig.canvas.draw()
counted("draw", fig.canvas.draw)
report()
"""

#: The most Python calls building each of these figures may make (the
#: second time, after a first build): a few per cent over what they make.
#: The checks of unusual arguments and the dispatch to program overrides
#: added since b414751d had taken them, unnoticed, to 1.6 times what they
#: were (task PERF9, 2026-11-14): an ordinary argument must cost one type
#: test, and a dispatch no program class asked for nothing after the first
#: question per class. They are the same on Python 3.12 to 3.14 (the
#: constrained grid makes 4 fewer on 3.14), and with numpy 1.26, the
#: lowest the package allows, as with 2.5 (task PERF10a: a bar no longer
#: goes through numpy's broadcasting, whose Python calls differ between
#: the two, `.axes._plotting.bars._broadcast`). Sixteen patches since,
#: each a call or two per Axes or line under the ceilings of its own base,
#: had used up the margins again (task PERF11, 2026-11-16).
BUILD_CALLS_CEILING = {
    "grid_16x16": 105_600,          # 102,523 (b414751d: 94,214; edf9a0a2: 149,380)
    "constrained_8x8": 38_400,      # 37,287 (31,527; 48,563)
    "bars_300": 68_000,             # 66,018 (62,933; 76,409; 69,614 before PERF10a)
    "texts_2000": 118_000,          # 114,615 (156,533; 182,809)
    "annotations_1000": 98_600,     # 95,755 (82,649; 122,934)
}

#: Builds the figures of `BUILD_CALLS_CEILING` twice each and prints the
#: Python calls of the second build.
_BUILD_PROGRAM = _COUNTER + """
import numpy as np
import pltrs.pyplot as plt

x = np.linspace(0.0, 2.0 * np.pi, 30)


def grid_16x16():
    fig, axs = plt.subplots(16, 16, figsize=(16.0, 16.0))
    for k, ax in enumerate(axs.flat):
        ax.plot(x, np.sin(x * (1.0 + k / 64.0)))


def constrained_8x8():
    fig, axs = plt.subplots(8, 8, figsize=(16.0, 12.0), layout="constrained")
    for k, ax in enumerate(axs.flat):
        ax.plot(x, np.cos(x + k / 10.0))
        ax.set_title(f"panel {k}")
        ax.set_xlabel("x")
        ax.set_ylabel("y")
    fig.suptitle("64 panels")


def bars_300():
    fig, ax = plt.subplots()
    for i, height in enumerate(np.linspace(1.0, 10.0, 300)):
        ax.bar(i, height)


def texts_2000():
    fig, ax = plt.subplots()
    for i in range(2000):
        ax.text((i % 50) / 50.0, (i // 50) / 40.0, f"t{i}", fontsize=6)


def annotations_1000():
    fig, ax = plt.subplots()
    xs = np.linspace(0.0, 10.0, 1000)
    ax.plot(xs, xs[::-1], "o", markersize=2)
    for i in range(1000):
        ax.annotate(f"p{i}", (xs[i], xs[999 - i]), fontsize=6)


for build in (grid_16x16, constrained_8x8, bars_300, texts_2000, annotations_1000):
    build()
    plt.close("all")
    counted(build.__name__, build)
    plt.close("all")
report()
"""

#: The calls of each scene by function as the ceilings were last set
#: (functions called at least `_PROFILED` times; all of a draw's), against
#: which a failure names what grew. Rewritten with the ceilings, by
#: ``PYTHONPATH=python .venv-pltrs/bin/python python/tests/test_perf7_import.py --record``.
PROFILES = pathlib.Path(__file__).with_name("call_profiles.json")

#: The fewest calls of a function a recorded profile keeps.
_PROFILED = 4


def _profiles(tmp_path):
    """The calls by function of every scene of both programs, the second
    run of each (the first writes the caches of a first import)."""
    env = {"PLTRSCONFIGDIR": str(tmp_path), "PLTRS_LOOK": "matplotlib"}
    _fresh(_CALLS_PROGRAM, **env)
    calls, _ = _fresh(_CALLS_PROGRAM, **env)
    built, _ = _fresh(_BUILD_PROGRAM, **env)
    return {**calls, **built}


def _grown(scene, profile):
    """The functions of *scene* whose calls grew most against its recorded
    profile, as ``file:function +calls``."""
    recorded = json.loads(PROFILES.read_text())["scenes"].get(scene, {})
    growth = sorted(((count - recorded.get(name, 0), name) for name, count in profile.items()
                     if count >= _PROFILED or name in recorded), reverse=True)
    return ", ".join(f"{name} +{grown}" for grown, name in growth[:8] if grown > 0)


def _over(ceilings, profiles):
    """What goes over its ceiling: the calls made, the ceiling and what
    grew, by scene."""
    return {scene: (sum(profiles[scene].values()), ceiling, _grown(scene, profiles[scene]))
            for scene, ceiling in ceilings.items() if sum(profiles[scene].values()) > ceiling}


#: What to tell the author of a change that goes over a ceiling.
_ADVICE = ("keep new work off the plain path: answered once per class, made lazy, or in the "
           "native code; move a ceiling (and rewrite the profiles with --record) only for a "
           "call matplotlib's own code makes")


def test_python_calls_at_import_and_per_draw_stay_under_their_ceiling(tmp_path):
    over = _over(PYTHON_CALLS_CEILING, _profiles(tmp_path))
    assert not over, f"Python calls (made, ceiling, grown most): {over}; {_ADVICE}"


def test_python_calls_building_figures_stay_under_their_ceiling(tmp_path):
    over = _over(BUILD_CALLS_CEILING, _profiles(tmp_path))
    assert not over, f"Python calls building (made, ceiling, grown most): {over}; {_ADVICE}"


def _record():
    """Writes `PROFILES` from this tree."""
    import tempfile
    with tempfile.TemporaryDirectory() as directory:
        profiles = _profiles(pathlib.Path(directory))
    scenes = {scene: dict(sorted((name, count) for name, count in profile.items()
                                 if count >= _PROFILED or scene == "draw"))
              for scene, profile in profiles.items()}
    totals = {scene: sum(profile.values()) for scene, profile in profiles.items()}
    PROFILES.write_text(json.dumps({"python": sys.version.split()[0], "totals": totals,
                                    "scenes": scenes}, indent=1) + "\n")
    print(f"wrote {PROFILES}: {totals}")


if __name__ == "__main__" and sys.argv[1:] == ["--record"]:
    _record()
