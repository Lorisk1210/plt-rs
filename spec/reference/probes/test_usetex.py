r"""matplotlib's TeX support with TeX: what ``TexManager`` and ``dviread``
make of strings ``latex`` typesets, after ``tests/test_usetex.py``,
``tests/test_texmanager.py`` and ``tests/test_dviread.py``. The whole area
needs TeX and is skipped without it (``spec_support.tex``).

Area ``usetex``. ``rc`` holds rcParams applied around an operation.

Ops
---
``TexManager.get_text_width_height_descent``  {tex, fontsize, dpi, rc} ->
    ``[width, height, descent]`` for a renderer of ``dpi``.
``dviread.Dvi.page``  {tex, fontsize, dpi, rc} -> the only page of
    ``make_dvi(tex, fontsize)`` at ``dpi``: ``{"text": [[x, y, texname,
    size, code, width, index, file, effects], ...], "boxes": [[x, y,
    height, width], ...], "width", "height", "descent"}`` (``file`` the
    font file's name, ``index`` the glyph's FreeType index).
``dviread.PsfontsMap.pdftex``  {texname} -> ``{"psname", "effects",
    "encoding", "filename"}`` of ``pdftex.map`` (file names only).
``dviread.find_tex_file``  {name} -> the file's name; or the error.
``TextToPath.get_glyphs_tex``  {tex} -> ``{"glyphs": [[repr, x, y,
    scale], ...], "rects": n}`` (after ``test_unicode_sizing``).
``TexManager.get_grey``  {tex, fontsize, dpi} -> ``[height, width, sum,
    sha1]`` of the alpha ``dvipng`` gives, as Agg blits it (``(Z *
    0xff).astype(np.uint8)``).
``TexManager.make_dvi``  {tex} -> ``true`` when ``latex`` typesets the
    string; or the error (``test_unicode_characters``).
``test_usetex.minus_no_descent``  {fontsize} -> the heights in pixel rows
    of ``$1$``, ``$-1$`` and both drawn over each other (all equal in
    matplotlib's ``test_minus_no_descent``).

Left out
--------
``test_usetex_fallback`` monkeypatches the Agg renderer (pltrs' default
``draw_tex`` is covered by the ``path_effects`` figure case);
``test_missing_psfont`` monkeypatches ``PsfontsMap``;
``test_pdf_type1_font_subsetting`` needs pikepdf (the PDF goldens of the
``usetex`` figure group hold the subsets); ``test_usetex_packages`` needs
``chemformula``, which TinyTeX lacks; ``test_openin_any_paranoid`` checks a
subprocess environment; ``test_usetex_is_copied`` checks per-artist state
the tick labels of pltrs are not artists for. ``test_partial_usetex`` is a
unit test of pltrs's PostScript output (``figure/postscript``), and the EPS
of ``test_rotation`` is compared with the ``usetex`` figure group's
PostScript, which LaTeX's ``psfrag``, ``dvips`` and Ghostscript make.
"""

import hashlib
from pathlib import Path

import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import dviread
from matplotlib.font_manager import FontProperties
from matplotlib.textpath import TextToPath
from matplotlib.texmanager import TexManager

from spec_support import ProbeSet
from spec_support.tex import needs_usetex

pytestmark = needs_usetex

probes = ProbeSet("usetex")


class _Renderer:
    """What ``get_text_width_height_descent`` asks of a renderer."""

    def __init__(self, dpi):
        self.dpi = dpi

    def points_to_pixels(self, points):
        return points * self.dpi / 72


def _extent(tex, fontsize, dpi, rc):
    with mpl.rc_context(rc):
        return list(TexManager().get_text_width_height_descent(tex, fontsize, _Renderer(dpi)))


def _page(tex, fontsize, dpi, rc):
    with mpl.rc_context(rc):
        dvi_file = TexManager().make_dvi(tex, fontsize)
        with dviread.Dvi(dvi_file, dpi) as dvi:
            page, = dvi
        return {
            "text": [[t.x, t.y, t.font.texname.decode(), t.font.size, t.glyph, t.width,
                      t.index, Path(t.font_path).name, dict(sorted(t.font_effects.items()))]
                     for t in page.text],
            "boxes": [[b.x, b.y, b.height, b.width] for b in page.boxes],
            "width": page.width, "height": page.height, "descent": page.descent,
        }


STRINGS = [
    "hello, world",
    r"\LaTeX",
    "lp",
    r"$\frac{1}{2}\pi$",
    r"$-1$",
    r"$\left[\int\limits_e^{-2e}\sqrt\frac{\log^3 x}{x}\,\mathrm{d}x \right\}$",
    r"$\mathdefault{0.5}$",
    r"\textsf{fi ffl 1234} \texttt{tt}",
    r"\textwon",
    "foo_bar",
    "% a comment",
    "\\ ",
    "\N{MINUS SIGN}2",
    "\\textit{Velocity (\N{DEGREE SIGN}/sec)}",
]

RCS = [
    {},
    {"font.family": "sans-serif", "font.sans-serif": "Helvetica"},
    {"font.family": "times"},
    {"font.family": "serif", "font.serif": "Palatino"},
    {"font.family": "monospace", "font.monospace": "Courier"},
    {"font.family": "cursive"},
    {"text.latex.preamble": r"\usepackage{amssymb}"},
]


def test_extents():
    for tex in STRINGS:
        for fontsize, dpi in [(10.0, 72.0), (12.0, 100.0), (24.0, 100.0)]:
            probes.probe("TexManager.get_text_width_height_descent", _extent, tex=tex,
                         fontsize=fontsize, dpi=dpi, rc={})
    for rc in RCS[1:]:
        probes.probe("TexManager.get_text_width_height_descent", _extent,
                     tex=r"Fonts $x^2$ fi", fontsize=12.0, dpi=100.0, rc=rc)


def test_pages():
    for tex in STRINGS:
        probes.probe("dviread.Dvi.page", _page, tex=tex, fontsize=10.0, dpi=72.0, rc={})
    for rc in RCS[1:]:
        probes.probe("dviread.Dvi.page", _page, tex=r"Fonts $x^2$ fi", fontsize=12.0,
                     dpi=100.0, rc=rc)
    probes.probe("dviread.Dvi.page", _page, tex=r"$\mathbb{R} \circledcirc$", fontsize=10.0,
                 dpi=72.0, rc={"text.latex.preamble": r"\usepackage{amssymb}"})


def _pdftex(texname):
    entry = dviread.PsfontsMap(dviread.find_tex_file("pdftex.map"))[texname.encode()]
    return {
        "psname": entry.psname.decode(),
        "effects": dict(sorted(entry.effects.items())),
        "encoding": Path(entry.encoding).name if entry.encoding else None,
        "filename": Path(entry.filename).name if entry.filename else None,
    }


def test_font_map():
    for texname in ["cmr10", "cmmi12", "cmsy10", "cmex10", "ptmr8r", "ptmro8r", "phvr8r",
                    "pcrr8r", "pzcmi8r", "pplr8r", "msam10", "eufm10", "tcrm1000",
                    "no-such-font"]:
        probes.probe("dviread.PsfontsMap.pdftex", _pdftex, texname=texname)


def _find(name):
    return Path(dviread.find_tex_file(name)).name


def test_find_tex_file():
    for name in ["cmr10.tfm", "cmr10.pfb", "8r.enc", "ptmr7t.vf", "type1ec.sty",
                 "no-such-file.tfm"]:
        probes.probe("dviread.find_tex_file", _find, name=name)


def _glyphs_tex(tex):
    glyph_info, _, rects = TextToPath().get_glyphs_tex(FontProperties(), tex)
    return {"glyphs": [[repr_, x, y, scale] for repr_, x, y, scale in glyph_info],
            "rects": len(rects)}


def test_glyphs_tex():
    # test_unicode_sizing: "W" and \textwon at the same scale.
    for tex in ["W", r"\textwon", r"$\sqrt{x}$", r"\LaTeX"]:
        probes.probe("TextToPath.get_glyphs_tex", _glyphs_tex, tex=tex)


def _grey(tex, fontsize, dpi):
    alpha = TexManager().get_grey(tex, fontsize, dpi)
    blitted = (alpha * 0xff).astype(np.uint8)
    return [*blitted.shape, int(blitted.astype(np.int64).sum()),
            hashlib.sha1(blitted.tobytes()).hexdigest()]


def test_grey():
    for tex, fontsize, dpi in [("hello", 10.0, 100.0), (r"$\frac{1}{2}$", 12.0, 72.0),
                               (r"\LaTeX", 20.0, 150.0)]:
        probes.probe("TexManager.get_grey", _grey, tex=tex, fontsize=fontsize, dpi=dpi)


def _make_dvi(tex):
    TexManager().make_dvi(tex, 10.0)
    return True


def test_errors():
    for tex in ["\N{SNOWMAN}", "$x", r"\frac{", r"\undefinedcommand",
                "\N{VULGAR FRACTION ONE QUARTER}\xd6\xf8\xe6", "50\\%"]:
        probes.probe("TexManager.make_dvi", _make_dvi, tex=tex)


def _minus_no_descent(fontsize):
    with mpl.rc_context({"font.size": fontsize}):
        heights = []
        fig = plt.figure()
        try:
            for vals in [(1,), (-1,), (-1, 1)]:
                fig.clear()
                for x in vals:
                    fig.text(.5, .5, f"${x}$", usetex=True)
                fig.canvas.draw()
                heights.append(int((np.array(fig.canvas.buffer_rgba())[..., 0] != 255)
                                   .any(axis=1).sum()))
        finally:
            plt.close(fig)
        return heights


def test_minus_no_descent():
    for fontsize in [8.0, 10.0, 12.0]:
        probes.probe("test_usetex.minus_no_descent", _minus_no_descent, fontsize=fontsize)
