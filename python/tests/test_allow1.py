"""
What a program meets where an external program matplotlib runs is not
installed: TeX for usetex text (``RuntimeError("Failed to process string
with tex because latex could not be found")`` from every place that
typesets it, before any other argument is looked at), the TeX system of
the PGF backend, Ghostscript and ``pdftops`` for ``ps.usedistiller``
(the distillation is skipped with a logged warning, the file written),
and ffmpeg and ImageMagick for the animation writers. Each case runs in
a child process whose ``PATH`` holds none of these programs. Then the
polygon markers with a number of sides other than a positive integer,
which fail as matplotlib's unit paths fail.

The expected values were recorded from matplotlib 3.11.2 running the
same code (task ALLOW1).
"""

import os
import pathlib
import subprocess
import sys
import textwrap

import pytest

import pltrs
import pltrs.markers as mmarkers

#: The alias of the tree, and the pltrs under test (the tree's or an
#: installed wheel's).
ALIAS = pathlib.Path(__file__).resolve().parents[1] / "alias"
PACKAGE_DIR = pathlib.Path(pltrs.__file__).resolve().parents[1]

#: matplotlib's error for usetex text without TeX.
NO_LATEX = ("RuntimeError", "Failed to process string with tex because latex could not be found")
#: matplotlib's error for a PGF figure without its TeX system.
NO_XELATEX = ("RuntimeError", "'xelatex' not found; install it or change "
              "rcParams['pgf.texsystem'] to an available TeX implementation")

#: What the child process runs before each case.
PRELUDE = """
import io, os, sys, warnings
warnings.simplefilter("ignore")
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.font_manager import FontProperties
prop = FontProperties(size=12)
def report(fn):
    try:
        fn()
        print("OK")
    except BaseException as e:
        print(type(e).__name__)
        print(str(e))
"""

#: Each case: its statements (ending with the call `report` makes) and
#: what matplotlib gives.
CASES = {
    "TextPath": (
        "from matplotlib.textpath import TextPath\n"
        "report(lambda: TextPath((0, 0), 'ab', size=10, usetex=True))", NO_LATEX),
    "get_glyphs_tex": (
        "from matplotlib.textpath import TextToPath\n"
        "report(lambda: TextToPath().get_glyphs_tex(prop, 'ab'))", NO_LATEX),
    "agg measure": (
        "from matplotlib.backends.backend_agg import RendererAgg\n"
        "report(lambda: RendererAgg(100, 100, 72).get_text_width_height_descent('ab', prop, 'TeX'))",
        NO_LATEX),
    "agg draw_tex, TeX before the arguments": (
        "from matplotlib.backends.backend_agg import RendererAgg\n"
        "r = RendererAgg(100, 100, 72)\n"
        "report(lambda: r.draw_tex(r.new_gc(), float('inf'), 10, 'ab', prop, 0))", NO_LATEX),
    "pdf draw_tex, TeX before the arguments": (
        "from matplotlib.backends.backend_pdf import PdfFile, RendererPdf\n"
        "r = RendererPdf(PdfFile(io.BytesIO()), 72, 100, 100)\n"
        "report(lambda: r.draw_tex(None, '10', 10, 'ab', prop, 0))", NO_LATEX),
    "pdf draw_tex of no string": (
        "from matplotlib.backends.backend_pdf import PdfFile, RendererPdf\n"
        "r = RendererPdf(PdfFile(io.BytesIO()), 72, 100, 100)\n"
        "report(lambda: r.draw_tex(r.new_gc(), 10, 10, None, prop, 0))", NO_LATEX),
    "svg draw_text TeX": (
        "from matplotlib.backends.backend_svg import RendererSVG\n"
        "r = RendererSVG(100, 100, io.StringIO())\n"
        "report(lambda: r.draw_text(r.new_gc(), 10, 10, 'ab', prop, 0, ismath='TeX'))", NO_LATEX),
    "constrained layout": (
        "fig, ax = plt.subplots(layout='constrained')\n"
        "ax.set_title('t', usetex=True)\n"
        "report(fig.canvas.draw)", NO_LATEX),
    "text extent": (
        "fig = plt.figure()\n"
        "t = fig.text(0.5, 0.5, 'ab', usetex=True)\n"
        "report(lambda: t.get_window_extent(fig.canvas.get_renderer()))", NO_LATEX),
    "pgf save": (
        "fig = plt.figure(figsize=(2, 1)); fig.text(0.5, 0.5, 'ab')\n"
        "report(lambda: fig.savefig(io.StringIO(), format='pgf'))", NO_XELATEX),
    "pgf canvas draw": (
        "from matplotlib.backends.backend_pgf import FigureCanvasPgf\n"
        "fig = plt.figure(); fig.text(0.5, 0.5, 'ab')\n"
        "report(FigureCanvasPgf(fig).draw)", NO_XELATEX),
    "pgf canvas draw of an Axes": (
        "from matplotlib.backends.backend_pgf import FigureCanvasPgf\n"
        "fig, ax = plt.subplots()\n"
        "report(FigureCanvasPgf(fig).draw)", NO_XELATEX),
    "ghostscript distiller": (
        "matplotlib.rcParams['ps.usedistiller'] = 'ghostscript'\n"
        "fig = plt.figure(figsize=(2, 1)); fig.text(0.5, 0.5, 'ab')\n"
        "report(lambda: fig.savefig(io.BytesIO(), format='eps'))", ("OK",)),
    "xpdf distiller": (
        "matplotlib.rcParams['ps.usedistiller'] = 'xpdf'\n"
        "fig = plt.figure(figsize=(2, 1)); fig.text(0.5, 0.5, 'ab')\n"
        "report(lambda: fig.savefig(io.BytesIO(), format='ps'))", ("OK",)),
    "ffmpeg writer": (
        "from matplotlib import animation\n"
        "fig, ax = plt.subplots()\n"
        "anim = animation.FuncAnimation(fig, lambda i: [], frames=2)\n"
        "report(anim.to_html5_video)", ("RuntimeError", "Requested MovieWriter (ffmpeg) not available")),
    "imagemagick writer": (
        "from matplotlib import animation\n"
        "print(animation.writers.is_available('imagemagick'))\n"
        "report(lambda: matplotlib._get_executable_info('magick'))",
        ("False", "ExecutableNotFoundError", "[Errno 2] No such file or directory: 'convert'")),
}


def _run(statements, tmp_path):
    """The lines *statements* print in a child process that finds none of
    the external programs (an empty ``PATH``), with a configuration
    directory of its own, and what it logs."""
    config = tmp_path / "config"
    config.mkdir(exist_ok=True)
    empty = tmp_path / "bin"
    empty.mkdir(exist_ok=True)
    env = {**os.environ, "PATH": str(empty), "PLTRSCONFIGDIR": str(config),
           "MPLCONFIGDIR": str(config), "MPLBACKEND": "agg", "PLTRS_LOOK": "matplotlib",
           "PYTHONPATH": os.pathsep.join(filter(None, [
               str(PACKAGE_DIR), str(ALIAS), os.environ.get("PYTHONPATH")]))}
    source = textwrap.dedent(PRELUDE) + statements
    done = subprocess.run([sys.executable, "-c", source], env=env, cwd=tmp_path, text=True,
                          capture_output=True, timeout=300)
    assert done.returncode == 0, done.stderr
    return tuple(done.stdout.splitlines()), done.stderr


@pytest.mark.skipif(sys.platform == "win32", reason="Windows finds Ghostscript in its registry")
@pytest.mark.parametrize("case", CASES)
def test_a_missing_program_raises_what_matplotlib_raises(case, tmp_path):
    statements, expected = CASES[case]
    assert _run(statements, tmp_path)[0] == expected


@pytest.mark.skipif(sys.platform == "win32", reason="Windows finds Ghostscript in its registry")
def test_a_distiller_without_ghostscript_is_skipped_with_a_warning(tmp_path):
    _, logged = _run(CASES["ghostscript distiller"][0], tmp_path)
    assert logged == ("Failed to find a Ghostscript installation.  "
                      "Distillation step skipped.\n")


@pytest.mark.parametrize("marker, error", [
    ((-3, 1), (ValueError, "negative dimensions are not allowed")),
    ((-3, 2), (ValueError, "negative dimensions are not allowed")),
    ((0, 0), (ZeroDivisionError, "division by zero")),
    ((0, 1), (ZeroDivisionError, "division by zero")),
])
def test_a_polygon_marker_without_sides_fails_as_matplotlib_s(marker, error):
    with pytest.raises(error[0], match=error[1]):
        mmarkers.MarkerStyle(marker)
