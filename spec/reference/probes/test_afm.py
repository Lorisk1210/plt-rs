"""Adobe Font Metrics: matplotlib's AFM parser and the text measurements of
the PDF backend's 14 core fonts and of the PostScript backend's standard
fonts.

Area ``afm`` (golden ``spec/goldens/probes/afm.json``).

Environment every op assumes
============================
* Reference rcParams: ``matplotlib.testing.setup()`` plus ``text.hinting =
  'none'`` (so ``font.family = ['DejaVu Sans']``, ``font.size = 10``).
* Only matplotlib's bundled fonts are installed (see
  ``spec_support.fonts.bundled_fonts_only``), so the AFM font list is the 61
  files of ``mpl-data/fonts/{afm,pdfcorefonts}``.
* ``file`` args name a file of ``mpl-data/fonts/pdfcorefonts`` (the 14 PDF
  core fonts) or of ``mpl-data/fonts/afm`` (the 35 standard PostScript
  fonts, Helvetica Light, Utopia and Computer Modern; no name is in both),
  parsed with ``matplotlib._afm.AFM``.

Shared argument encodings
=========================
``font`` dict: ``FontProperties()`` followed by the setters, in this order,
for the keys present: ``family`` (str or list of str), ``style``,
``weight``, ``stretch``, ``size``.  ``null`` / absent means plain
``FontProperties()``.

``directory``: ``'pdfcorefonts'`` or ``'afm'``, the subdirectory of
``mpl-data/fonts`` a lookup is restricted to.

Ops
===
``_afm.AFM.header`` -- args ``{file, getter}``.  ``getattr(afm, getter)()``
for ``getter`` among ``get_fontname``, ``get_fullname``, ``get_familyname``,
``get_weight``, ``get_angle``, ``get_ascender``, ``get_descender``,
``get_capheight``, ``get_xheight``, ``get_underline_thickness``; a missing
header entry raises ``KeyError``.

``_afm.AFM.get_str_bbox_and_descent`` -- args ``{file, s}``.  ``[left,
bottom, width, height, descent]`` in 1/1000 of the font size: kerned
widths, newlines skipped, characters looked up by ``uni2type1`` name and
measured as ``question`` when the font lacks them (``KeyError`` when it
lacks that too); the glyph box is read as (left, bottom, width, height).

``_afm.AFM.get_width_char`` -- args ``{file, code}``; ``KeyError`` for
codes the font does not encode.  ``_afm.AFM.get_glyph_name`` -- the same
args, the glyph name.  ``_afm.AFM.get_width_from_char_name`` -- args
``{file, name}``.  ``_afm.AFM.get_kern_dist_from_name`` -- args ``{file,
name1, name2}`` (0 for pairs that are not kerned).

``_afm.AFM`` -- args ``{data}``: ``AFM(io.BytesIO(data.encode("latin-1")))``
of a synthetic file; returns ``{"fontname": get_fontname() or null,
"ascender": get_ascender() or null, "bbox": get_str_bbox_and_descent("AV")
or null, "kern": get_kern_dist_from_name("A", "V")}`` (``null`` where those
raise ``KeyError``); malformed files raise ``RuntimeError``.

``font_manager.findfont`` -- args ``{prop, directory}``.  The basename of
``fontManager.findfont(font_dict, fontext='afm', directory=<mpl-data>/fonts/
<directory>)`` (falls back to the best Helvetica face of the directory).

``font_manager.FontManager._find_fonts_by_props`` -- args ``{prop,
directory}``.  The basenames of ``_find_fonts_by_props(font_dict,
fontext='afm', directory=...)``: one font per matching family, else the
directory's best Helvetica face.

``backend_pdf.RendererPdf.get_text_width_height_descent`` -- args ``{s,
font?}``.  With ``pdf.use14corefonts = True``, ``[w, h, d]`` of
``RendererPdf(PdfFile(BytesIO()), 72, 4.8, 6.4)
.get_text_width_height_descent(s, font_dict, ismath=False)``: the core
font's ``get_str_bbox_and_descent`` scaled by ``size / 1000``.

``backend_pdf.RendererPdf._get_font_height_metrics`` -- args ``{font?}``.
The same renderer's ``[ascent, descent, line_gap]`` (``ascender * size /
1000``, ``-descender * size / 1000``, a fifth of their sum); ``KeyError``
for fonts without an ascender (Symbol, ZapfDingbats).

``backend_ps.RendererPS.get_text_width_height_descent`` -- args ``{s,
font?}``, and ``backend_ps.RendererPS._get_font_height_metrics`` -- args
``{font?}``: the same with ``ps.useafm = True`` for ``RendererPS(6.4, 4.8,
None)``, whose AFM fonts are those of ``mpl-data/fonts/afm``.
"""

from __future__ import annotations

import io
import os

import matplotlib
import pytest
from matplotlib import cbook
from matplotlib import font_manager
from matplotlib._afm import AFM
from matplotlib.backends.backend_pdf import PdfFile, RendererPdf
from matplotlib.backends.backend_ps import RendererPS
from matplotlib.font_manager import FontProperties

from spec_support import ProbeSet
from spec_support.fonts import bundled_fonts_only

probes = ProbeSet("afm")

# findfont fallback chatter is expected here.
pytestmark = pytest.mark.filterwarnings("ignore::UserWarning")

FONT_KEYS = ("family", "style", "weight", "stretch", "size")
CORE_FILES = sorted(
    f for f in os.listdir(cbook._get_data_path("fonts", "pdfcorefonts")) if f.endswith(".afm"))
PS_FILES = sorted(
    f for f in os.listdir(cbook._get_data_path("fonts", "afm")) if f.endswith(".afm"))
GETTERS = [
    "get_fontname", "get_fullname", "get_familyname", "get_weight", "get_angle",
    "get_ascender", "get_descender", "get_capheight", "get_xheight",
    "get_underline_thickness",
]


@pytest.fixture(scope="module", autouse=True)
def _bundled_fonts_only():
    """Pretend only mpl-data fonts are installed; restore the host list afterwards."""
    with bundled_fonts_only():
        yield


def _afm(file):
    directory = "pdfcorefonts" if file in CORE_FILES else "afm"
    with open(cbook._get_data_path("fonts", directory, file), "rb") as fh:
        return AFM(fh)


def _font(spec):
    fp = FontProperties()
    for key in FONT_KEYS:
        if spec is not None and key in spec:
            getattr(fp, "set_" + key)(spec[key])
    return fp


def _header(file, getter):
    return getattr(_afm(file), getter)()


def _str_bbox(file, s):
    return list(_afm(file).get_str_bbox_and_descent(s))


def _width_char(file, code):
    return _afm(file).get_width_char(code)


def _glyph_name(file, code):
    return _afm(file).get_glyph_name(code)


def _width_from_name(file, name):
    return _afm(file).get_width_from_char_name(name)


def _kern(file, name1, name2):
    return _afm(file).get_kern_dist_from_name(name1, name2)


def _or_none(getter):
    try:
        return getter()
    except KeyError:
        return None


def _parse(data):
    afm = AFM(io.BytesIO(data.encode("latin-1")))
    bbox = _or_none(lambda: afm.get_str_bbox_and_descent("AV"))
    return {
        "fontname": _or_none(afm.get_fontname),
        "ascender": _or_none(afm.get_ascender),
        "bbox": None if bbox is None else list(bbox),
        "kern": afm.get_kern_dist_from_name("A", "V"),
    }


def _directory(directory):
    return cbook._get_data_path("fonts", directory)


def _findfont(prop, directory):
    path = font_manager.fontManager.findfont(
        _font(prop), fontext="afm", directory=_directory(directory))
    return os.path.basename(path)


def _find_fonts_by_props(prop, directory):
    paths = font_manager.fontManager._find_fonts_by_props(
        _font(prop), fontext="afm", directory=_directory(directory))
    return [os.path.basename(p) for p in paths]


def _pdf_renderer():
    return RendererPdf(PdfFile(io.BytesIO()), 72, 4.8, 6.4)


def _pdf_whd(s, font=None):
    with matplotlib.rc_context({"pdf.use14corefonts": True}):
        return list(_pdf_renderer().get_text_width_height_descent(s, _font(font), False))


def _pdf_height_metrics(font=None):
    with matplotlib.rc_context({"pdf.use14corefonts": True}):
        return list(_pdf_renderer()._get_font_height_metrics(_font(font)))


def _ps_whd(s, font=None):
    with matplotlib.rc_context({"ps.useafm": True}):
        return list(RendererPS(6.4, 4.8, None).get_text_width_height_descent(
            s, _font(font), False))


def _ps_height_metrics(font=None):
    with matplotlib.rc_context({"ps.useafm": True}):
        return list(RendererPS(6.4, 4.8, None)._get_font_height_metrics(_font(font)))


STRINGS = [
    "", " ", "AVATAR", "Toyota", "WAVE", "Hg", "fi fl", "café naïve",
    "Ångström ±½×©", "− 1.5", "€–“”",
    "→α", "a\nb", "\n", "¼²", "\U0001f600",
]

FONTS = [
    None,
    {"family": "serif"},
    {"family": "monospace", "weight": "bold"},
    {"family": "Helvetica", "weight": "bold", "style": "oblique"},
    {"family": "Times", "style": "italic", "size": 11},
    {"family": ["Foo", "Courier"], "size": 7.5},
    {"family": "Symbol", "size": 12},
    {"family": "ZapfDingbats"},
]


def test_headers():
    for file in CORE_FILES + PS_FILES:
        for getter in GETTERS:
            probes.probe("_afm.AFM.header", _header, file=file, getter=getter)


def test_string_extents():
    for file in CORE_FILES + PS_FILES:
        for s in STRINGS:
            probes.probe("_afm.AFM.get_str_bbox_and_descent", _str_bbox, file=file, s=s)


def test_character_queries():
    for file in ["Helvetica.afm", "Times-Roman.afm", "Courier.afm", "Symbol.afm",
                 "ZapfDingbats.afm", "phvr8a.afm", "pncr8a.afm", "pplri8a.afm",
                 "psyr.afm", "pzdr.afm", "cmr10.afm", "cmsy10.afm", "putr8a.afm"]:
        for code in [32, 63, 65, 128, 233, 0x2212, 300, -1]:
            probes.probe("_afm.AFM.get_width_char", _width_char, file=file, code=code)
            probes.probe("_afm.AFM.get_glyph_name", _glyph_name, file=file, code=code)
        for name in ["A", "Euro", "minus", "question", "alpha", "a1", "uni20AC"]:
            probes.probe(
                "_afm.AFM.get_width_from_char_name", _width_from_name, file=file, name=name)
        for name1, name2 in [("A", "V"), ("T", "o"), ("f", "eacute"), ("V", "V"), ("", "A")]:
            probes.probe(
                "_afm.AFM.get_kern_dist_from_name", _kern, file=file, name1=name1, name2=name2)


HEAD = "StartFontMetrics 4.1\nFontName Sample\nAscender 700\nStartCharMetrics 3\n"
CHARS = ("C 63 ; WX 444 ; N question ; B 30 0 400 700 ;\n"
         "C 65 ; WX 700 ; N A ; B -10 0 710 680 ;\n"
         "C 86 ; WX 680 ; N V ; B 5 -20 675 680 ;\nEndCharMetrics\n")
KERN = "StartKernData\nStartKernPairs 1\nKPX A V -80\nEndKernPairs\nEndKernData\n"

SYNTHETIC = [
    HEAD + CHARS,
    HEAD + CHARS + KERN + "EndFontMetrics\n",
    "Comment first\n" + HEAD + CHARS,
    HEAD.replace("Ascender 700", "Ascender 7,5e2") + CHARS,
    HEAD.replace("Ascender 700", "Ascender oops\nFontBBox 0 x 1 2\nUnknownKey 3") + CHARS,
    HEAD.replace("FontName Sample\n", "") + CHARS,
    HEAD + CHARS.replace("C 63 ; WX 444 ; N question ; B 30 0 400 700 ;\n", "")
    + "StartComposites 1\nCC Aring 2 ; PCC A 0 0 ; PCC ring 100 200 ;\nEndComposites\n",
    HEAD + CHARS + "StartKernData\n\nStartKernPairs 1\n\nKPX A V -5\n\nEndKernPairs\nx\n",
    # Malformed files.
    "",
    "FontName X\n" + HEAD + CHARS,
    HEAD,
    HEAD + "C 65 ; WX 700 ; N A\nEndCharMetrics\n",
    HEAD + "C 65 ; WX 700 ; N A ; B 1 2 3\nEndCharMetrics\n",
    HEAD + "\nEndCharMetrics\n",
    HEAD + CHARS + "StartKernData\nStartTrackKern 1\n",
    HEAD + CHARS + "StartKernData\nStartKernPairs 1\nKPY A V 1\nEndKernPairs\nx\n",
    HEAD + CHARS + "StartKernData\nStartKernPairs 1\nKPX A V -80\n",
    HEAD + CHARS + "StartComposites 1\nCC Aring 3 ; PCC A 0 0 ;\nEndComposites\n",
    HEAD.replace("StartCharMetrics 3", "StartCharMetrics x") + CHARS,
    HEAD.replace("Ascender 700", "FontBBox 0 0 inf 1") + CHARS,
]


def test_parsing():
    for data in SYNTHETIC:
        probes.probe("_afm.AFM", _parse, data=data)


def test_font_lookup():
    props = FONTS + [
        {"family": "sans-serif", "weight": "bold", "style": "italic"},
        {"family": "Palatino", "weight": "bold"},
        {"family": "Helvetica", "stretch": "condensed"},
        {"family": ["Times", "Foo", "Courier"]},
        {"family": "cursive"},
        {"family": "Computer Modern", "style": "italic"},
    ]
    for prop in props:
        for directory in ("pdfcorefonts", "afm"):
            probes.probe("font_manager.findfont", _findfont, prop=prop, directory=directory)
            probes.probe(
                "font_manager.FontManager._find_fonts_by_props", _find_fonts_by_props,
                prop=prop, directory=directory)


def test_pdf_core_font_metrics():
    for font in FONTS:
        probes.probe("backend_pdf.RendererPdf._get_font_height_metrics", _pdf_height_metrics,
                     font=font)
        for s in STRINGS:
            probes.probe("backend_pdf.RendererPdf.get_text_width_height_descent", _pdf_whd,
                         s=s, font=font)


PS_FONTS = FONTS + [
    {"family": "Palatino", "weight": "bold", "style": "italic"},
    {"family": "ITC Bookman", "weight": "light"},
    {"family": "Helvetica", "stretch": "condensed", "size": 9},
    {"family": "ITC Zapf Chancery", "size": 14},
    {"family": "Utopia", "weight": "bold"},
]


def test_ps_font_metrics():
    for font in PS_FONTS:
        probes.probe("backend_ps.RendererPS._get_font_height_metrics", _ps_height_metrics,
                     font=font)
        for s in STRINGS:
            probes.probe("backend_ps.RendererPS.get_text_width_height_descent", _ps_whd,
                         s=s, font=font)
