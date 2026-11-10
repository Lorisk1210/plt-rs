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
NOT_IN_PLTRS = {"_cm", "_cm_bivar", "_cm_listed", "_cm_multivar", "_version",
                "_mathtext_data"}

#: The most modules of pltrs' own each import may load: what they load now.
#: A helper module that a module uses only inside its functions is
#: imported where first needed (``_lazy_imports.module``), not at import.
OWN_MODULES_CEILING = {"pltrs": 56, "pltrs.pyplot": 205}


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
