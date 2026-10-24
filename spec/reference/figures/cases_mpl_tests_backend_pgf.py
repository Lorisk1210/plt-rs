"""The figures of matplotlib's PGF backend tests
(``tests/test_backend_pgf.py``), whose ``.pgf`` text the wide vector
comparison (``cargo xtask vectors mpl_tests_backend_pgf/``) checks; the
tests themselves compile them with LaTeX, which this host lacks.

* ``xelatex``: ``create_figure`` (lines, markers, a hatched
  ``fill_between``, a filled polygon, unicode and math text, a sans-serif
  blue math label and a clipped text) in serif type without
  ``pgf.rcfonts`` (``test_xelatex``).
* ``rcupdate_1``, ``rcupdate_2``: the same figure under each rcParam set of
  ``test_rcupdate`` (sans-serif at 30 pt with large markers; monospace at
  10 pt with orange, wide hatches).
* ``pathclip``: a line to ``1e100`` clipped by its axes, and a log-scaled
  histogram next to two points (``test_pathclip``; ``np.random`` draws
  from ``spec_support.rng``).
* ``mixedmode``: a rasterized ``pcolor`` of ``x² + y²`` (``test_mixedmode``).
* ``sketch_params``: a sketched diagonal without ticks or frame
  (``test_sketch_params``), in the test suite's default style.

The other tests of the file are probes of the ``mpl_backend_pgf`` area or
left out there.
"""

import numpy as np

from figures._mpl_tests import group_case
from spec_support import rng

GROUP = "mpl_tests_backend_pgf"
mpl_test = group_case(GROUP)

#: The rcParams of ``test_xelatex``, ``test_pathclip`` and ``test_mixedmode``.
SERIF = {"font.family": "serif", "pgf.rcfonts": False}
#: The two rcParam sets of ``test_rcupdate``.
RCUPDATE = [{"font.family": "sans-serif", "font.size": 30, "figure.subplot.left": .2,
             "lines.markersize": 10, "pgf.rcfonts": False, "pgf.texsystem": "xelatex"},
            {"font.family": "monospace", "font.size": 10, "figure.subplot.left": .1,
             "hatch.color": "orange", "hatch.linewidth": 2, "lines.markersize": 20,
             "pgf.rcfonts": False, "pgf.texsystem": "pdflatex",
             "pgf.preamble": ("\\usepackage[utf8x]{inputenc}\\usepackage[T1]{fontenc}"
                              "\\usepackage{sfmath}")}]


def create_figure(fig):
    ax = fig.add_subplot()
    x = np.linspace(0, 1, 15)
    ax.plot(x, x ** 2, "b-")
    ax.plot(x, 1 - x**2, "g>")
    ax.fill_between([0., .4], [.4, 0.], hatch="//", facecolor="lightgray",
                    edgecolor="red")
    ax.fill([3, 3, .8, .8, 3], [2, -2, -2, 0, 2], "b")
    ax.plot([0.9], [0.5], "ro", markersize=3)
    ax.text(0.9, 0.5, "unicode (ü, °, \N{Section Sign}) and math ($\\mu_i = x_i^2$)",
            ha="right", fontsize=20)
    ax.set_ylabel("sans-serif, blue, $\\frac{\\sqrt{x}}{y^2}$..",
                  family="sans-serif", color="blue")
    ax.text(1, 1, "should be clipped as default clip_box is Axes bbox",
            fontsize=20, clip_on=True)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)


mpl_test("xelatex", style="mpl20", rc=SERIF)(create_figure)
for _number, _rc in enumerate(RCUPDATE, start=1):
    mpl_test(f"rcupdate_{_number}", style="mpl20", rc=_rc)(create_figure)


@mpl_test("pathclip", style="mpl20", rc=SERIF)
def _(fig):
    axs = fig.subplots(1, 2)
    axs[0].plot([0., 1e100], [0., 1e100])
    axs[0].set_xlim(0, 1)
    axs[0].set_ylim(0, 1)
    axs[1].scatter([0, 1], [1, 1])
    axs[1].hist(rng(19680801).normal(1000), bins=20, range=[-10, 10])
    axs[1].set_xscale("log")


@mpl_test("mixedmode", style="mpl20", rc=SERIF)
def _(fig):
    Y, X = np.ogrid[-1:1:40j, -1:1:40j]
    fig.add_subplot().pcolor(X**2 + Y**2).set_rasterized(True)


@mpl_test("sketch_params", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_frame_on(False)
    handle, = ax.plot([0, 1])
    handle.set_sketch_params(scale=5, length=30, randomness=42)
