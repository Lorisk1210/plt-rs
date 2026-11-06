"""
Smoke tests of the names pltrs's objects give themselves: with pltrs
installed as matplotlib (``python/alias``, the ``matplotlib``
distribution) and imported as matplotlib, they are matplotlib's, as
libraries read them (arviz-plots picks its plotting backend by
``type(fig).__module__``, matplotlib-venn's doctests print reprs);
imported as pltrs, they are pltrs's. Each case runs in a process of its
own, as the first import decides. The expected values were recorded
under matplotlib 3.11.2.
"""

import json
import os
import pathlib
import subprocess
import sys

import pltrs

#: The alias of the tree, and the pltrs under test (the tree's or an
#: installed wheel's).
ALIAS = pathlib.Path(__file__).resolve().parents[1] / "alias"
PACKAGE_DIR = pathlib.Path(pltrs.__file__).resolve().parents[1]

#: What the program `NAMES` prints, under matplotlib 3.11.2.
MATPLOTLIB_NAMES = {
    "figure": ["matplotlib.figure", "Figure"],
    "axes": ["matplotlib.axes._axes", "Axes"],
    "circle": "<matplotlib.patches.Circle object>",
    "locator": "<matplotlib.ticker.AutoLocator object>",
    "functions": ["matplotlib.pyplot", "matplotlib.ticker", "matplotlib.lines"],
    "modules": ["matplotlib.figure", "matplotlib.figure", "matplotlib"],
    "logger": "matplotlib.figure",
    "toolkits": ["mpl_toolkits.mplot3d.axes3d", "mpl_toolkits.axes_grid1.axes_divider",
                 "mpl_toolkits.mplot3d.axes3d"],
    "pylab": ["pylab", "matplotlib.pyplot"],
}

#: A program printing the names of a figure, its parts and their modules,
#: importing the package as {package} ({toolkits} its toolkits).
NAMES = """
import json, re
import {package}
import {package}.pyplot as plt
import {package}.figure, {package}.lines, {package}.ticker
from {package}.patches import Circle
from {toolkits}.mplot3d import Axes3D
from {toolkits}.axes_grid1 import make_axes_locatable
{pylab}
fig, ax = plt.subplots()
address = re.compile(r" at 0x[0-9a-fA-F]+")
print(json.dumps({{
    "figure": [type(fig).__module__, type(fig).__qualname__],
    "axes": [type(ax).__module__, type(ax).__qualname__],
    "circle": address.sub("", repr(Circle((0, 0), 1))),
    "locator": address.sub("", repr(ax.xaxis.get_major_locator())),
    "functions": [plt.subplots.__module__, {package}.ticker.MaxNLocator.__module__,
                  {package}.lines.Line2D.set_data.__module__],
    "modules": [{package}.figure.__name__, {package}.figure.__spec__.name,
                {package}.__spec__.name],
    "logger": {package}.figure._log.name,
    "toolkits": [Axes3D.__module__, make_axes_locatable.__module__,
                 type(fig.add_subplot(projection="3d")).__module__],
    "pylab": [pylab.__name__, pylab.plot.__module__],
}}))
"""


def _run(code, *, alias=True):
    """What *code* prints, run with pltrs on the path, and its alias too
    if *alias*."""
    path = [str(ALIAS), str(PACKAGE_DIR)] if alias else [str(PACKAGE_DIR)]
    env = {**os.environ, "PYTHONPATH": os.pathsep.join(path), "MPLBACKEND": "agg"}
    return subprocess.run([sys.executable, "-c", code], env=env, check=True,
                          capture_output=True, text=True).stdout


def test_objects_imported_as_matplotlib_say_they_are_matplotlibs():
    code = NAMES.format(package="matplotlib", toolkits="mpl_toolkits", pylab="import pylab")
    assert json.loads(_run(code)) == MATPLOTLIB_NAMES


def test_pltrs_imported_as_pltrs_keeps_its_names():
    code = NAMES.format(package="pltrs", toolkits="pltrs.mpl_toolkits",
                        pylab="import pltrs.pylab as pylab")
    expected = {
        "figure": ["pltrs.figure", "Figure"],
        "axes": ["pltrs.axes._axes", "Axes"],
        "circle": "<pltrs.patches.Circle object>",
        "locator": "<pltrs.ticker.AutoLocator object>",
        "functions": ["pltrs.pyplot", "pltrs.ticker", "pltrs.lines"],
        "modules": ["pltrs.figure", "pltrs.figure", "pltrs"],
        "logger": "pltrs.figure",
        "toolkits": ["pltrs.mpl_toolkits.mplot3d.axes3d",
                     "pltrs.mpl_toolkits.axes_grid1.axes_divider",
                     "pltrs.mpl_toolkits.mplot3d.axes3d"],
        "pylab": ["pltrs.pylab", "pltrs.pyplot"],
    }
    assert json.loads(_run(code, alias=False)) == expected
    # With the alias installed too, pltrs imported first keeps its names,
    # and matplotlib's stand for its modules.
    assert json.loads(_run(code)) == expected
    then_matplotlib = (
        "import sys, pltrs.pyplot, matplotlib.pyplot, matplotlib.figure, mpl_toolkits.mplot3d\n"
        "print(matplotlib.pyplot is pltrs.pyplot, matplotlib.figure.Figure.__module__,\n"
        "      mpl_toolkits.mplot3d is sys.modules['pltrs.mpl_toolkits.mplot3d'])")
    assert _run(then_matplotlib).split() == ["True", "pltrs.figure", "True"]


def test_pltrs_names_stand_for_the_modules_loaded_as_matplotlib():
    code = ("import sys, matplotlib.pyplot, pltrs.pyplot, pltrs.figure\n"
            "import mpl_toolkits.mplot3d, pltrs.mpl_toolkits.mplot3d\n"
            "print(pltrs.pyplot is matplotlib.pyplot, pltrs.figure.Figure.__module__,\n"
            "      pltrs.mpl_toolkits.mplot3d is mpl_toolkits.mplot3d,\n"
            "      [m for m, v in sys.modules.items() if getattr(v, '__name__', '')"
            ".startswith('pltrs')])")
    assert _run(code).split() == ["True", "matplotlib.figure", "True", "[]"]


def test_a_toolkit_imported_first_is_matplotlibs_too():
    code = ("from mpl_toolkits.mplot3d import Axes3D\n"
            "import matplotlib.pyplot as plt\n"
            "ax = plt.figure().add_subplot(projection='3d')\n"
            "print(Axes3D.__module__, type(ax) is Axes3D, plt.__name__)")
    assert _run(code).split() == ["mpl_toolkits.mplot3d.axes3d", "True", "matplotlib.pyplot"]


def test_a_figure_pickled_under_the_alias_unpickles_under_it(tmp_path):
    path = tmp_path / "figure.pickle"
    _run("import pickle, sys, matplotlib.pyplot as plt\n"
         "fig, ax = plt.subplots()\n"
         "ax.plot([1, 3, 2], label='line')\n"
         f"pickle.dump(fig, open({str(path)!r}, 'wb'))")
    data = path.read_bytes()
    assert b"matplotlib.figure" in data and b"pltrs" not in data
    out = _run("import pickle\n"
               f"fig = pickle.load(open({str(path)!r}, 'rb'))\n"
               "line, = fig.axes[0].get_lines()\n"
               "print(type(fig).__module__, type(line).__module__, line.get_label(),\n"
               "      [float(y) for y in line.get_ydata()])")
    assert out.split() == ["matplotlib.figure", "matplotlib.lines", "line", "[1.0,", "3.0,", "2.0]"]
