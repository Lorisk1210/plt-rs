"""matplotlib's own tests of the PGF backend
(``matplotlib/tests/test_backend_pgf.py``) that check the ``.pgf`` text
itself rather than a document LaTeX compiles from it.

Area ``mpl_backend_pgf``. Every op runs in the style matplotlib's test
fixture applies (``classic`` plus ``_classic_test_patch``) unless its
``style`` is ``mpl20`` (tests marked ``default``), on ``plt.figure()``,
saved with ``savefig(format="pgf")``. matplotlib measures text with LaTeX,
which this host lacks: as for the wide vector comparison, it measures text
as Agg does here (``figures.render_vectors``), which leaves the ``.pgf``
text the ops read unchanged.

Ops
---
``RendererPgf.draw_text``  {text, rc} -> the ``\\pgftext`` lines of
    ``fig.text(.5, .5, text)`` under ``rc``: the escaping of TeX's special
    characters (``test_tex_special_chars``) and of the minus sign
    (``test_minus_signs_with_tex``), and the font commands of the families,
    sizes and ``pgf.rcfonts`` of ``test_rcupdate``, ``test_xelatex`` and
    ``test_unknown_font`` (an unknown family gets none; a known one is set
    with ``\\setmainfont`` when ``pgf.rcfonts`` is on).
``FigureCanvasPgf.print_pgf``  {rc} -> the preamble the header comments
    list (the lines after "used the following preamble", without their
    ``%%`` prefix): ``font.size``, ``pgf.preamble`` and ``pgf.rcfonts`` as
    ``test_rcupdate``, ``test_xelatex`` and ``test_pdflatex`` set them.
``RendererPgf.draw_path``  {} -> the nine lines of the sketched line of
    ``test_sketch_params`` from its ``\\pgfpathmoveto`` to its
    ``\\pgfusepath{stroke}``: the decoration after the path and before its
    use.

The figures of ``test_xelatex``, ``test_rcupdate``, ``test_pathclip``,
``test_mixedmode`` and ``test_sketch_params`` are cases of
``mpl_tests_backend_pgf``, whose PGF the wide vector comparison checks.

Left out: everything LaTeX compiles or runs (``test_tex_special_chars``'s
PNG, ``test_pdflatex``, the PDFs of ``test_rcupdate``, ``test_pathclip``,
``test_mixedmode`` and ``test_bbox_inches``, ``test_pdf_pages*``,
``test_multipage_keep_empty``, ``test_tex_restart_after_error``,
``test_bbox_inches_tight``, ``test_png_transparency``, ``test_ttc_output``,
``test_document_font_size`` and the PDF comparison of
``test_minus_signs_with_tex``); the fontspec lines ``pgf.rcfonts`` adds to
the preamble (matplotlib points them at its font files with
``Path=\\detokenize{<mpl-data>/fonts/ttf/}``, while pltrs's fonts are
built into it and have no directory to point at, so it names the files
alone); ``test__metadata_to_str*``, the
``\\hypersetup`` metadata of documents matplotlib compiles with LaTeX (pltrs
writes ``.pgf`` pictures and compiles nothing); and the log warning of
``test_unknown_font``.
"""

import contextlib
import io

import matplotlib.pyplot as plt
from matplotlib.backends import backend_pgf

from figures.render_vectors import _measure_pgf_text_like_agg
from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_backend_pgf")
probe = prober(probes)

#: The rcParams of ``test_xelatex`` and ``test_pdflatex``.
XELATEX = {"font.family": "serif", "pgf.rcfonts": False}
PDFLATEX = {**XELATEX, "pgf.texsystem": "pdflatex",
            "pgf.preamble": "\\usepackage[utf8x]{inputenc}\\usepackage[T1]{fontenc}"}
#: The two rcParam sets of ``test_rcupdate``.
RCUPDATE = [{"font.family": "sans-serif", "font.size": 30, "figure.subplot.left": .2,
             "lines.markersize": 10, "pgf.rcfonts": False, "pgf.texsystem": "xelatex"},
            {"font.family": "monospace", "font.size": 10, "figure.subplot.left": .1,
             "hatch.color": "orange", "hatch.linewidth": 2, "lines.markersize": 20,
             "pgf.rcfonts": False, "pgf.texsystem": "pdflatex",
             "pgf.preamble": ("\\usepackage[utf8x]{inputenc}\\usepackage[T1]{fontenc}"
                              "\\usepackage{sfmath}")}]


@contextlib.contextmanager
def _without_latex():
    """The PGF backend measuring text as Agg does, restored afterwards."""
    saved = (backend_pgf.RendererPgf.get_text_width_height_descent,
             backend_pgf._get_image_inclusion_command)
    _measure_pgf_text_like_agg()
    try:
        yield
    finally:
        (backend_pgf.RendererPgf.get_text_width_height_descent,
         backend_pgf._get_image_inclusion_command) = saved


def _pgf(fig):
    with _without_latex():
        buf = io.BytesIO()
        fig.savefig(buf, format="pgf")
    return buf.getvalue().decode()


def draw_text_op(text):
    fig = plt.figure()
    fig.text(.5, .5, text)
    return [line for line in _pgf(fig).splitlines() if line.startswith("\\pgftext")]


def test_draw_text():
    for text in ("%_^ $a_b^c$", "$-1$", "$\N{MINUS SIGN}1$"):
        probe("RendererPgf.draw_text", draw_text_op, text=text)
    for rc in (XELATEX, *RCUPDATE):
        probe("RendererPgf.draw_text", draw_text_op, style="mpl20", rc=rc, text="hello, world")
    for family in ("this-font-does-not-exist", "DejaVu Sans"):
        lines = probe("RendererPgf.draw_text", draw_text_op, rc={"font.family": family},
                      text="hello, world")
        assert ("setmainfont" in lines[0]) == (family == "DejaVu Sans")


def preamble_op():
    lines = _pgf(plt.figure()).splitlines()
    start = next(i for i, line in enumerate(lines) if "used the following preamble" in line)
    end = lines.index("%%", start)
    return [line[len("%%   "):] for line in lines[start + 1:end]]


def test_preamble():
    for rc in (XELATEX, PDFLATEX):
        probe("FigureCanvasPgf.print_pgf", preamble_op, style="mpl20", rc=rc)
    for rc in RCUPDATE:
        probe("FigureCanvasPgf.print_pgf", preamble_op, style="mpl20", rc=rc)


def sketch_op():
    fig, ax = plt.subplots(figsize=(3, 3))
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_frame_on(False)
    handle, = ax.plot([0, 1])
    handle.set_sketch_params(scale=5, length=30, randomness=42)
    lines = _pgf(fig).splitlines()
    seed = lines.index("\\pgfmathsetseed{42}%")
    return lines[seed - 5:seed + 3]


def test_sketch_params():
    lines = probe("RendererPgf.draw_path", sketch_op)
    assert lines[0] == "\\pgfpathmoveto{\\pgfqpoint{0.375000in}{0.300000in}}%"
    assert lines[-1] == "\\pgfusepath{stroke}%"
