"""Text layout measurements: extents, line layout, wrapping, mathtext, font lookup.

Area ``text_metrics`` (golden ``spec/goldens/probes/text_metrics.json``).

Environment every op assumes
============================
* Reference rcParams: ``matplotlib.testing.setup()`` (``rcdefaults()`` plus
  ``font.family = 'DejaVu Sans'``) and ``text.hinting = 'none'``.  Relevant
  consequences: ``font.size = 10``, ``font.style/variant/stretch = 'normal'``,
  ``font.weight = 'normal'``, ``mathtext.fontset = 'dejavusans'``,
  ``mathtext.fallback = 'cm'``, ``text.kerning_factor = 0``,
  ``font.enable_last_resort = True``, ``text.parse_math = True``, generic
  family lists (``font.sans-serif``, ``font.serif``, ``font.monospace``,
  ``font.cursive``, ``font.fantasy``) at matplotlib's defaults.
* **Only matplotlib's bundled fonts are installed.**  The font database is
  built from the files in ``mpl-data/fonts/{ttf,afm,pdfcorefonts}`` only
  (40 TrueType entries incl. ``LastResortHE-Regular.ttf``, 61 AFM entries),
  each file added with ``FontManager.addfont`` in ascending order of file
  basename (Python ``str`` order = Unicode code point order, so e.g.
  ``'DejaVuSans-Bold.ttf' < 'DejaVuSans.ttf'`` and uppercase sorts before
  lowercase).  ``findfont`` keeps the *first* font with the strictly lowest
  score, so this order is the tie-break rule.  The exact database (names,
  styles, weights, ...) is itself recorded by op ``font_manager.font_list``.
* "Canvas": ``Figure(figsize=(6.4, 4.8), dpi=<dpi>)`` attached to a
  ``FigureCanvasAgg``; ``renderer = canvas.get_renderer()`` (a ``RendererAgg``
  of 640x480 pixels at dpi 100).  Display coordinates are pixels with the
  origin at the bottom-left.  A fresh canvas is created for every probe.
* Optional arg ``rc`` (dict, default ``{}``): rcParams applied with
  ``matplotlib.rc_context(rc)`` around the *whole* op (object creation and
  measurement).  Keys used: ``mathtext.fontset``, ``font.family``,
  ``font.size``, ``font.weight``, ``font.sans-serif``, ``font.cursive``,
  ``text.hinting`` (``'default'`` or ``'native'``: the Agg renderer lays
  text out with FreeType's ``LOAD_DEFAULT`` or ``LOAD_NO_AUTOHINT``).
* Optional arg ``dpi`` (number, default 100): the figure dpi.

Shared argument encodings
=========================
``font`` / ``prop`` dict ("font dict"): ``FontProperties()`` (all fields
from rcParams) followed by the setters, in this order, for the keys that are
present: ``family`` (str or list of str; *never* parsed as a fontconfig
pattern here), ``style``, ``variant``, ``weight``, ``stretch``, ``size``,
``math_fontfamily``.  ``null`` / absent means plain ``FontProperties()``.

``kw`` dict for Text ops (default ``{}``): keyword arguments of
``fig.text(x, y, text, **kw)``.  Keys used (matplotlib Text property
aliases): ``size`` (fontsize: number or size name), ``family`` (str or list),
``weight``, ``style``, ``variant``, ``stretch``, ``rotation`` (number,
``'vertical'``, ``'horizontal'`` or null), ``ha`` (horizontalalignment),
``va`` (verticalalignment), ``ma`` (multialignment), ``rotation_mode``
(``'default'``, ``'anchor'``, ``'xtick'``, ``'ytick'``), ``linespacing``
(``'normal'`` or number), ``wrap`` (bool), ``math_fontfamily``,
``parse_math`` (bool).

Text placement: ``x``, ``y`` are figure fractions (transform
``fig.transSubfigure``), so the anchor in display pixels is
``(x * 640 * dpi / 100, y * 480 * dpi / 100)``.

Ops
===
``font_manager.font_list`` -- args ``{fontext: 'ttf'|'afm'}``.  The font
database: list of ``[basename, face_index, name, style, variant, weight,
stretch, size]`` per entry in database order (see above).

``text.Text.get_window_extent`` -- args ``{text, x, y, kw?, rc?, dpi?}``.
``t = fig.text(x, y, text, **kw)``; returns
``t.get_window_extent(renderer).get_points()`` as Bbox ``[[x0, y0], [x1,
y1]]`` in display pixels.  Empty text gives the zero-size bbox at the anchor.
With ``wrap: true`` the extent is computed from the wrapped text (wrapping
happens inside ``_get_layout``) and, as a side effect of wrapping, the
rotation mode is forced to ``'anchor'``.

``text.Text._get_layout`` -- same args.  ``t._renderer = renderer`` then
``bbox, info, (xy, size) = t._get_layout(renderer)``.  Returns ``{"bbox":
Bbox of the rotated, axis-aligned text box relative to the anchor (i.e.
window extent minus the anchor position), "lines": [{"text": line,
"width": w, "ascent": a, "descent": d, "xy": [x, y]}, ...], "box_xy": xy,
"box_size": size}``.  ``lines`` are the (possibly wrapped) lines; ``a``/``d``
are the *effective* ascent/descent used for stacking (already padded to the
font's minimum ascent/descent + half line gap for ``linespacing='normal'``,
or by the fixed-linespacing leading), ``xy`` is the rotated position of the
line's baseline start relative to the anchor.  ``box_xy``/``box_size`` are
the lower-left corner (relative to the anchor, rotated) and unrotated size of
the text-aligned box (used for the bbox patch).
Algorithm summary (3.11): the font-wide minimum ascent/descent/line gap come
from the first font of ``_find_fonts_by_props(fontproperties)``: OS/2
``sTypoAscender``, ``-sTypoDescender``, ``sTypoLineGap`` (hhea ``ascent``,
``-descent``, ``lineGap`` if there is no OS/2 table) times
``size_pt * dpi / 72 / unitsPerEm``; the gap is 0 for single-line text.
Lines are split on ``'\\n'``, each is classified as mathtext independently
(even number of unescaped ``$``), measured with
``get_text_width_height_descent`` (empty line -> 0, 0, 0), ``a = h - d``;
``linespacing='normal'``: ``a = max(a, min_a) + gap/2``, ``d = max(d, min_d) +
gap/2``; numeric ``L``: ``lead = L*(min_a + min_d) - (a + d)``, ``a += lead/2``,
``d += lead/2``.  Baselines stack downward from 0; then multialignment
offsets, rotation about the anchor and the ha/va/rotation_mode offset follow
``Text._get_layout`` exactly.

``text.Text._get_wrapped_text`` -- args ``{text, x, y, kw, rc?, dpi?}``
(``kw`` contains ``wrap: true`` unless testing the no-wrap path).
``t._renderer = renderer``; returns ``t._get_wrapped_text()`` (str with
``'\\n'`` inserted).  Line-width rule (``_get_wrap_line_width``): distance from
the anchor to the figure box edge along the text direction (left-aligned:
forward; right-aligned: backward; centered: 2*min of both), each word-prefix
measured with ``get_text_width_height_descent`` (mathtext if the prefix has an
even number of unescaped ``$``) and ``ceil``-ed; a prefix is broken when its
width is strictly greater than the available width; text is split on single
``' '`` characters (consecutive spaces give empty words) and explicit
newlines always break.

``backend_agg.RendererAgg.get_text_width_height_descent`` -- args ``{s,
ismath, font?, dpi?, rc?}``.  Returns ``[width, height, descent]`` in pixels
of ``renderer.get_text_width_height_descent(s, font_dict, ismath)``.
``ismath`` is ``false`` (FreeType/HarfBuzz layout of the raw string, glyphs
missing from the selected font fall back to the next font in the family
list and finally to LastResort; newlines/tabs are not special) or ``true``
(mathtext parse at the renderer dpi with ``MathTextParser('path')``, i.e.
``width = ceil(box.width)``, ``height = ceil(box.height) + ceil(box.depth)``,
``descent = ceil(box.depth)`` -- integral values; text outside ``$...$`` is
laid out by mathtext in the regular font; invalid mathtext -> ``ValueError``).

``font_manager.findfont`` -- args ``{prop, fontext?='ttf',
fallback_to_default?=true, rc?}``.  ``prop`` is a font dict, ``null``
(default ``FontProperties()``) or a *string* (parsed as a fontconfig pattern
by ``FontProperties(pattern)``).  Returns the basename of
``fontManager.findfont(prop, fontext=..., fallback_to_default=...)``;
failures (no font scores < 10 and no fallback) -> ``ValueError``.

``font_manager.FontManager._find_fonts_by_props`` -- args ``{prop,
fallback_to_default?=true, rc?}``.  The per-family font list that text
rendering uses (one ``findfont(..., fallback_to_default=False)`` per family
entry, unknown families dropped, default family appended only if nothing
matched).  Returns list of basenames; ``ValueError`` if nothing matched and
fallback is disabled.

``font_manager.FontProperties`` -- args ``{kw, rc?}``.  ``fp =
FontProperties(**kw)`` (note: if ``family`` is a str and none of style,
variant, weight, stretch, size is given, ``family`` is parsed as a
fontconfig pattern, keeping only the *first* value of each pattern key, e.g.
``"Foo,Bar"`` -> family ``["Foo"]``; sizes < 1.0 are clamped to 1.0, NaN/inf
pass through).  Returns ``{"family": get_family(), "style",
"variant", "weight", "stretch", "size": get_size_in_points(),
"math_fontfamily", "pattern": get_fontconfig_pattern()}`` (``pattern`` uses
Python ``str()`` of the values, e.g. ``size=10.0``); invalid values ->
``ValueError`` (or ``AttributeError`` for pattern keys without a setter).

``_fontconfig_pattern.parse_fontconfig_pattern`` -- args ``{pattern}``.
Returns the parsed dict (keys among family/size/weight/slant/width/...;
values are lists of str); syntax errors -> ``ValueError``.

``ft2font.FT2Font.load_glyph`` -- args ``{file, size, dpi, char, flags}``.
``font = FT2Font(<mpl-data>/fonts/ttf/<file>)``, ``font.set_size(size,
dpi)``, then ``glyph = font.load_glyph(font.get_char_index(ord(char)),
LoadFlags.<flags>)`` with ``flags`` ``DEFAULT`` or ``NO_AUTOHINT``.
Returns ``{"advance": glyph.horiAdvance, "path": [[code,
x, y], ...]}``: the 26.6 advance and ``font.get_path()`` with the vertices
in 1/64 pixels (integers) and matplotlib's path codes (1 MOVETO, 2 LINETO,
3 CURVE3, 79 CLOSEPOLY).  Only fonts FreeType hints with their bytecode
under ``flags`` are probed (no autohinted glyphs).
"""

from __future__ import annotations

import logging
import math
import os

import matplotlib
import pytest
from matplotlib import cbook
from matplotlib import font_manager
from matplotlib._fontconfig_pattern import parse_fontconfig_pattern
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.font_manager import FontProperties
from matplotlib.ft2font import FT2Font, LoadFlags
from matplotlib.mathtext import MathTextParser

from spec_support import ProbeSet

probes = ProbeSet("text_metrics")

# Missing-glyph (LastResort) and findfont fallback chatter is expected here.
pytestmark = pytest.mark.filterwarnings("ignore::UserWarning")

FIGSIZE = (6.4, 4.8)
FONT_KEYS = ("family", "style", "variant", "weight", "stretch", "size", "math_fontfamily")
TEXT_KW_KEYS = {
    "size", "family", "weight", "style", "variant", "stretch", "rotation", "ha", "va",
    "ma", "rotation_mode", "linespacing", "wrap", "math_fontfamily", "parse_math",
}
RC_KEYS = {
    "mathtext.fontset", "font.family", "font.size", "font.weight", "font.sans-serif",
    "font.cursive", "text.hinting",
}
FONTSETS = ["dejavusans", "dejavuserif", "stix", "stixsans", "cm", "custom"]


# --------------------------------------------------------------------------
# Font database restricted to matplotlib's bundled fonts (host independence)
# --------------------------------------------------------------------------

def _bundled_font_files(fontext):
    dirs = [cbook._get_data_path("fonts", d) for d in ("ttf", "afm", "pdfcorefonts")]
    return sorted(font_manager.findSystemFonts(dirs, fontext=fontext), key=os.path.basename)


def _clear_font_caches():
    fm = font_manager.fontManager
    fm._findfont_cached.cache_clear()
    font_manager._get_font.cache_clear()
    MathTextParser._parse_cached.cache_clear()


@pytest.fixture(scope="module", autouse=True)
def _bundled_fonts_only():
    """Pretend only mpl-data fonts are installed; restore the host list afterwards."""
    fm = font_manager.fontManager
    saved = fm.ttflist, fm.afmlist
    logger = logging.getLogger("matplotlib.font_manager")
    saved_level = logger.level
    logger.setLevel(logging.ERROR)
    fm.ttflist, fm.afmlist = [], []
    try:
        for fontext in ("afm", "ttf"):
            for path in _bundled_font_files(fontext):
                fm.addfont(path)
        _clear_font_caches()
        data = os.path.realpath(matplotlib.get_data_path()) + os.sep
        assert all(os.path.realpath(f.fname).startswith(data) for f in fm.ttflist + fm.afmlist)
        yield
    finally:
        fm.ttflist, fm.afmlist = saved
        _clear_font_caches()
        logger.setLevel(saved_level)


# --------------------------------------------------------------------------
# Adapters
# --------------------------------------------------------------------------

def _check(mapping, allowed):
    unknown = set(mapping or {}) - set(allowed)
    assert not unknown, f"undocumented keys {unknown}"


def _font(spec):
    fp = FontProperties()
    if spec is None:
        return fp
    for key in FONT_KEYS:
        if key in spec:
            getattr(fp, "set_" + key)(spec[key])
    return fp


def _canvas(dpi):
    fig = Figure(figsize=FIGSIZE, dpi=dpi)
    canvas = FigureCanvasAgg(fig)
    return fig, canvas.get_renderer()


def _make_text(text, x, y, kw, dpi):
    fig, renderer = _canvas(dpi)
    t = fig.text(x, y, text, **(kw or {}))
    t._renderer = renderer
    return t, renderer


def _window_extent(text, x, y, kw=None, rc=None, dpi=100):
    with matplotlib.rc_context(rc or {}):
        t, renderer = _make_text(text, x, y, kw, dpi)
        return t.get_window_extent(renderer).get_points()


def _layout(text, x, y, kw=None, rc=None, dpi=100):
    with matplotlib.rc_context(rc or {}):
        t, renderer = _make_text(text, x, y, kw, dpi)
        bbox, info, (xy, size) = t._get_layout(renderer)
        return {
            "bbox": bbox.get_points(),
            "lines": [
                {"text": line, "width": w, "ascent": a, "descent": d, "xy": list(pos)}
                for line, (w, a, d), pos in info
            ],
            "box_xy": list(xy),
            "box_size": list(size),
        }


def _wrapped_text(text, x, y, kw, rc=None, dpi=100):
    with matplotlib.rc_context(rc or {}):
        t, _ = _make_text(text, x, y, kw, dpi)
        return t._get_wrapped_text()


def _text_whd(s, ismath, font=None, dpi=100, rc=None):
    with matplotlib.rc_context(rc or {}):
        _, renderer = _canvas(dpi)
        w, h, d = renderer.get_text_width_height_descent(s, _font(font), ismath)
        return [w, h, d]


def _prop(prop):
    return prop if isinstance(prop, str) else _font(prop)


def _load_glyph(file, size, dpi, char, flags):
    font = FT2Font(cbook._get_data_path("fonts", "ttf", file))
    font.set_size(size, dpi)
    glyph = font.load_glyph(font.get_char_index(ord(char)), getattr(LoadFlags, flags))
    vertices, codes = font.get_path()
    path = [[int(code), round(x * 64), round(y * 64)]
            for (x, y), code in zip(vertices.tolist(), codes.tolist())]
    return {"advance": glyph.horiAdvance, "path": path}


def _findfont(prop, fontext="ttf", fallback_to_default=True, rc=None):
    with matplotlib.rc_context(rc or {}):
        path = font_manager.fontManager.findfont(
            _prop(prop), fontext=fontext, fallback_to_default=fallback_to_default)
        return os.path.basename(path)


def _find_fonts_by_props(prop, fallback_to_default=True, rc=None):
    with matplotlib.rc_context(rc or {}):
        paths = font_manager.fontManager._find_fonts_by_props(
            _prop(prop), fallback_to_default=fallback_to_default)
        return [os.path.basename(p) for p in paths]


def _font_properties(kw, rc=None):
    with matplotlib.rc_context(rc or {}):
        fp = FontProperties(**kw)
        return {
            "family": fp.get_family(),
            "style": fp.get_style(),
            "variant": fp.get_variant(),
            "weight": fp.get_weight(),
            "stretch": fp.get_stretch(),
            "size": fp.get_size_in_points(),
            "math_fontfamily": fp.get_math_fontfamily(),
            "pattern": fp.get_fontconfig_pattern(),
        }


def _font_list(fontext):
    fm = font_manager.fontManager
    entries = fm.afmlist if fontext == "afm" else fm.ttflist
    return [
        [os.path.basename(f.fname), f.index, f.name, f.style, f.variant, f.weight,
         f.stretch, f.size]
        for f in entries
    ]


# Thin wrappers that validate the documented arg schema before probing.

def extent(text, x=0.5, y=0.5, **args):
    _check(args, {"kw", "rc", "dpi"})
    _check(args.get("kw"), TEXT_KW_KEYS)
    _check(args.get("rc"), RC_KEYS)
    return probes.probe("text.Text.get_window_extent", _window_extent, text=text, x=x, y=y, **args)


def layout(text, x=0.5, y=0.5, **args):
    _check(args, {"kw", "rc", "dpi"})
    _check(args.get("kw"), TEXT_KW_KEYS)
    _check(args.get("rc"), RC_KEYS)
    return probes.probe("text.Text._get_layout", _layout, text=text, x=x, y=y, **args)


def wrapped(text, x, y, kw, **args):
    _check(args, {"rc", "dpi"})
    _check(kw, TEXT_KW_KEYS)
    return probes.probe("text.Text._get_wrapped_text", _wrapped_text, text=text, x=x, y=y, kw=kw, **args)


def whd(s, ismath=False, **args):
    _check(args, {"font", "dpi", "rc"})
    _check(args.get("font"), FONT_KEYS)
    _check(args.get("rc"), RC_KEYS)
    return probes.probe(
        "backend_agg.RendererAgg.get_text_width_height_descent", _text_whd, s=s, ismath=ismath, **args)


def findfont(prop, **args):
    _check(args, {"fontext", "fallback_to_default", "rc"})
    if isinstance(prop, dict):
        _check(prop, FONT_KEYS)
    return probes.probe("font_manager.findfont", _findfont, prop=prop, **args)


def find_fonts_by_props(prop, **args):
    _check(args, {"fallback_to_default", "rc"})
    if isinstance(prop, dict):
        _check(prop, FONT_KEYS)
    return probes.probe(
        "font_manager.FontManager._find_fonts_by_props", _find_fonts_by_props, prop=prop, **args)


def fontprops(kw, **args):
    _check(args, {"rc"})
    _check(args.get("rc"), RC_KEYS)
    return probes.probe("font_manager.FontProperties", _font_properties, kw=kw, **args)


def _width(bbox):
    return bbox[1][0] - bbox[0][0]


def _height(bbox):
    return bbox[1][1] - bbox[0][1]


# --------------------------------------------------------------------------
# Font database
# --------------------------------------------------------------------------

def test_font_list():
    ttf = probes.probe("font_manager.font_list", _font_list, fontext="ttf")
    afm = probes.probe("font_manager.font_list", _font_list, fontext="afm")
    assert len(ttf) == 40 and len(afm) == 61
    assert [e[0] for e in ttf] == sorted(e[0] for e in ttf)
    names = {e[2] for e in ttf}
    assert {"DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono", "STIXGeneral", "cmr10",
            "Last Resort High-Efficiency"} <= names


# --------------------------------------------------------------------------
# Window extent / layout / raw metrics for a broad string corpus
# --------------------------------------------------------------------------

STRINGS = [
    "", " ", "   ", "a", "x", "H", "g", "Hg", "AV", "To", "WAVE", "A V", "AVAVAV",
    "Hello, World!", "The quick brown fox jumps over the lazy dog", "0123456789",
    "3.14159", "1e-10", "−1", "-1", "−0.5", "+-=", "Ünïcödé",
    "àéîõü ÅÄÖ", "é", "αβγδ ΩΣΠ",
    "Привет", "مرحبا",
    "שלום", "中文字", "日本語", "\U0001f600",
    "a\U0001f600b", "fi fl ffi", "ﬁ", "°C ½ →←", "∑∫∞",
    "a\tb", "a\rb", "trailing  ", "  leading", "(()[]{})", "___", "|", "...",
    "x$y", r"price \$5", "$5 and $6", "line1\nline2", "line1\nline2\nline3", "a\n", "\n",
    "\na", "a\n\nb", "short\nmuch longer line\nmid", "g\ny", "ÁÉ\nqp", "  \n  ",
    "$x$", "a $b$ c", "$x$\n$y^2$", "$a\nb$",
]


def test_string_corpus():
    for s in STRINGS:
        ext = extent(s)
        lay = layout(s)
        assert not isinstance(ext, Exception) and not isinstance(lay, Exception), s
        if s:
            # window extent == layout bbox translated by the anchor (320, 240)
            assert ext.tolist() == [[lay["bbox"][0][0] + 320, lay["bbox"][0][1] + 240],
                                    [lay["bbox"][1][0] + 320, lay["bbox"][1][1] + 240]]
            assert len(lay["lines"]) == s.count("\n") + 1
        if "\n" not in s or s in ("a\n", "\n", "line1\nline2"):
            whd(s)
    assert extent("").tolist() == [[320.0, 240.0], [320.0, 240.0]]
    # kerning: "AV" is narrower than "A" + "V"
    av = whd("AV")
    assert av[0] < whd("A")[0] + whd("V")[0]
    to = whd("To")
    assert to[0] < whd("T")[0] + whd("o")[0]
    # Unicode minus is a real glyph, wider than hyphen-minus in DejaVu Sans.
    assert whd("−1")[0] > whd("-1")[0]
    # Explicit dollars: "price \$5" renders as "price $5".
    assert extent(r"price \$5").tolist() == extent("price $5", kw={"parse_math": False}).tolist()


def test_parse_math_flag():
    for s in ["$x^2$", "a $b$ c", r"\$5", "$"]:
        for flag in (True, False):
            extent(s, kw={"parse_math": flag})
            layout(s, kw={"parse_math": flag})


def test_positions():
    for x, y in [(0, 0), (1, 1), (0.1, 0.9), (0.25, 0.75), (-0.5, 1.5), (0.123, 0.456)]:
        e = extent("Pos", x=x, y=y)
        e0 = extent("Pos")
        assert _width(e) == _width(e0)
        extent("", x=x, y=y)


# --------------------------------------------------------------------------
# Font sizes, families, weights, styles, stretches
# --------------------------------------------------------------------------

SIZES = [
    "xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large", "smaller",
    "larger", 0.5, 1, 2, 4, 6, 8, 9, 10, 10.5, 11, 12, 14, 16, 18, 20, 24, 30, 36, 48,
    60, 72, "12",
]


def test_font_sizes():
    for size in SIZES:
        extent("Hg", kw={"size": size})
        whd("Hg", font={"size": size})
    for size in [6, 10, 24, 72]:
        extent("Sample text\nline two", kw={"size": size})
        layout("Sample text\nline two", kw={"size": size})
    for size in [8, 10, 12]:
        extent("Hg", kw={"size": "small"}, rc={"font.size": size})
    assert extent("Hg", kw={"size": 0.5}).tolist() == extent("Hg", kw={"size": 1}).tolist()
    assert extent("Hg", kw={"size": "12"}).tolist() == extent("Hg", kw={"size": 12}).tolist()
    assert isinstance(extent("Hg", kw={"size": "huge"}), ValueError)


FAMILIES = [
    "sans-serif", "serif", "monospace", "cursive", "fantasy", "sans", "DejaVu Sans",
    "DejaVu Serif", "DejaVu Sans Mono", "DejaVu Sans Display", "DejaVu Serif Display",
    "STIXGeneral", "STIXNonUnicode", "cmr10", "cmtt10", "cmss10", "cmb10", "cmti10",
    "Foo", "Comic Sans MS", ["Foo", "DejaVu Serif"], ["cmr10", "DejaVu Sans"],
    ["cursive", "monospace"],
]


def test_font_families():
    word = "Hamburgefonts"
    for fam in FAMILIES:
        extent(word, kw={"family": fam})
        whd(word, font={"family": fam})
    # Unknown families fall back to DejaVu Sans; generic cursive/fantasy too.
    base = extent(word).tolist()
    for fam in ["cursive", "fantasy", "Foo", "Comic Sans MS", "sans-serif", "DejaVu Sans"]:
        assert extent(word, kw={"family": fam}).tolist() == base, fam
    assert extent(word, kw={"family": "serif"}).tolist() != base
    # Glyph fallback through the family list, then to LastResort.
    for fam in ["cmr10", ["cmr10", "DejaVu Sans"], ["DejaVu Sans", "cmr10"], "STIXGeneral",
                "DejaVu Sans Mono"]:
        for s in ["αβ xy", "中 x", "− 1", "été"]:
            extent(s, kw={"family": fam})
            whd(s, font={"family": fam})
    # rc font.family
    for fam in ["serif", "monospace", ["cmr10"]]:
        extent(word, rc={"font.family": fam})


def test_font_weight_style_stretch():
    word = "Hamburgefonts"
    for fam in ["DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono", "STIXGeneral", "cmr10"]:
        for weight in ["normal", "bold"]:
            for style in ["normal", "italic", "oblique"]:
                kw = {"family": fam, "weight": weight, "style": style}
                extent(word, kw=kw)
                whd(word, font=kw)
    for weight in ["ultralight", "light", "regular", "book", "medium", "roman", "semibold",
                   "demibold", "demi", "heavy", "extra bold", "black", 100, 300, 500, 549,
                   550, 551, 600, 900]:
        extent(word, kw={"weight": weight})
    for stretch in ["ultra-condensed", "condensed", "semi-expanded", "ultra-expanded", 100, 900]:
        extent(word, kw={"stretch": stretch})
    extent(word, kw={"variant": "small-caps"})
    assert extent(word, kw={"weight": "bold"}).tolist() != extent(word).tolist()
    assert extent(word, kw={"stretch": "condensed"}).tolist() == extent(word).tolist()
    assert isinstance(extent(word, kw={"weight": "thin"}), ValueError)
    assert isinstance(extent(word, kw={"style": "slanted"}), ValueError)


# --------------------------------------------------------------------------
# Rotation, alignment, rotation modes, multialignment, linespacing
# --------------------------------------------------------------------------

ROTATIONS = [0, 15, 30, 45, 60, 89.5, 90, 135, 180, 225, 270, 315, -30, -90, 360, 405,
             720.5, "vertical", "horizontal", None]


def test_rotation():
    for text in ["Rotated text", "ab\ncdef"]:
        for rot in ROTATIONS:
            extent(text, kw={"rotation": rot})
            layout(text, kw={"rotation": rot})
    e0 = extent("Rotated text", kw={"rotation": 0})
    e90 = extent("Rotated text", kw={"rotation": 90})
    assert _width(e90) == pytest.approx(_height(e0)) and _height(e90) == pytest.approx(_width(e0))
    assert extent("Rotated text", kw={"rotation": "vertical"}).tolist() == e90.tolist()
    assert extent("Rotated text", kw={"rotation": "horizontal"}).tolist() == e0.tolist()
    assert (extent("Rotated text", kw={"rotation": 405}).tolist()
            == extent("Rotated text", kw={"rotation": 45}).tolist())
    assert isinstance(extent("Rotated text", kw={"rotation": "diagonal"}), ValueError)


def test_alignment():
    text = "Align\nme here"
    for mode in ["default", "anchor"]:
        for rot in [0, 45, 90]:
            for ha in ["left", "center", "right"]:
                for va in ["top", "center", "baseline", "center_baseline", "bottom"]:
                    kw = {"ha": ha, "va": va, "rotation": rot, "rotation_mode": mode}
                    extent(text, x=0.25, y=0.75, kw=kw)
                    layout(text, x=0.25, y=0.75, kw=kw)
    for mode in ["xtick", "ytick"]:
        for rot in [0, 5, 30, 85, 90, 120, 180, 200, 270, 300, 355]:
            for ha, va in [("center", "top"), ("left", "bottom"), ("right", "center")]:
                kw = {"ha": ha, "va": va, "rotation": rot, "rotation_mode": mode}
                extent("Tick 1", kw=kw)
    single = extent("Hg", kw={"ha": "center", "va": "center"})
    assert (single[0][0] + single[1][0]) / 2 == pytest.approx(320.0)
    assert (single[0][1] + single[1][1]) / 2 == pytest.approx(240.0)
    assert extent("Hg", kw={"ha": "right"})[1][0] == pytest.approx(320.0)
    assert extent("Hg", kw={"va": "top"})[1][1] == pytest.approx(240.0)
    assert extent("Hg", kw={"va": "bottom"})[0][1] == pytest.approx(240.0)
    assert isinstance(extent("Hg", kw={"ha": "middle"}), ValueError)
    assert isinstance(extent("Hg", kw={"va": "middle"}), ValueError)
    assert isinstance(extent("Hg", kw={"rotation_mode": "foo"}), ValueError)


def test_multialignment():
    text = "short\nmuch longer line\nmid"
    for ma in ["left", "center", "right", None]:
        for ha in ["left", "center", "right"]:
            for rot in [0, 30]:
                kw = {"ha": ha, "rotation": rot}
                if ma is not None:
                    kw["ma"] = ma
                lay = layout(text, kw=kw)
                extent(text, kw=kw)
                assert len(lay["lines"]) == 3
    assert isinstance(layout(text, kw={"ma": "justify"}), ValueError)


def test_linespacing():
    for text in ["a\nb\nc", "Hg\nHg", "Hg", "$x^2$\n$\\frac{a}{b}$\ny"]:
        for ls in ["normal", 0, 0.5, 1.0, 1.2, 1.5, 2, 3]:
            extent(text, kw={"linespacing": ls})
            layout(text, kw={"linespacing": ls})
    assert isinstance(extent("a\nb", kw={"linespacing": "double"}), TypeError)
    for fam in ["DejaVu Serif", "STIXGeneral", "cmr10", "DejaVu Sans Mono"]:
        for ls in ["normal", 1.2]:
            layout("Hg\nHg", kw={"family": fam, "linespacing": ls})
    for size in [6, 20]:
        layout("Hg\nHg\nHg", kw={"size": size})


def test_dpi_scaling():
    for dpi in [50, 72, 100, 150, 200, 300]:
        for s in ["Hg", "AV\nxy", "$x^2$"]:
            extent(s, dpi=dpi)
        whd("Hg", dpi=dpi)
        whd("$x^2$", ismath=True, dpi=dpi)
        layout("AV\nxy", dpi=dpi, kw={"rotation": 30})


# --------------------------------------------------------------------------
# Raw renderer metrics
# --------------------------------------------------------------------------

def test_renderer_metrics():
    words = ["Hg", "AV", "xyz", "Hamburgefonts", " ", "−1.5", "Éç"]
    for s in words:
        for font in [
            {"family": "DejaVu Serif"}, {"family": "DejaVu Sans Mono"},
            {"family": "STIXGeneral", "style": "italic"}, {"family": "cmr10"},
            {"weight": "bold"}, {"style": "oblique"}, {"size": 24}, {"size": 7.5},
            {"family": "DejaVu Sans Display"},
        ]:
            whd(s, font=font)
    # ismath=True with and without dollar signs
    for s in ["x^2", "$x^2$", "a $b$ c", "Hg", "$\\alpha$"]:
        whd(s, ismath=True)
    assert whd("$x^2$", ismath=True) == whd("$x^2$", ismath=True, font=None)
    r = whd("$x^2$", ismath=True)
    assert all(float(v).is_integer() for v in r)


# --------------------------------------------------------------------------
# Mathtext extents across fontsets
# --------------------------------------------------------------------------

MATH = [
    r"$x$", r"$x^2$", r"$x_i$", r"$x_i^2$", r"$\frac{a}{b}$", r"$\frac{1}{1+\frac{1}{x}}$",
    r"$\dfrac{a}{b}$", r"$\sqrt{x}$", r"$\sqrt[3]{x}$", r"$\sqrt{\frac{a}{b}}$",
    r"$\sum_{i=0}^{n} i$", r"$\prod_{k=1}^{N}$", r"$\int_0^\infty$", r"$\oint$",
    r"$\alpha\beta\Gamma$", r"$\hat{x}\bar{y}\vec{v}\dot{a}$", r"$\tilde{n}\ddot{u}$",
    r"$\mathrm{d}x$", r"$\mathbf{F}$", r"$\mathit{x}$", r"$\mathcal{L}$", r"$\mathbb{R}$",
    r"$\mathsf{A}$", r"$\mathtt{x}$", r"$\mathfrak{g}$", r"$\mathscr{L}$", r"$\mathbfit{x}$",
    r"$\left(\frac{a}{b}\right)$", r"$\left[\int\right]$", r"$\left\{x\right\}$",
    r"$\left|x\right|$", r"$a^{b^{c}}$", r"$x_{i_{j}}$", "foo $x$ bar", r"$\overline{abc}$",
    r"$\underline{x}$", r"$a\,b\;c\quad d\qquad e$", r"$a\!b$", r"$10^{-3}$",
    "$−1$", r"$-1$", r"$\infty \pm \times \div$", r"$\leq \geq \neq \approx$",
    r"$\rightarrow \Leftarrow$", r"$\sin(x) + \cos(y)$", r"$\operatorname{sinc}(x)$",
    r"$\mathrm{Re}\{z\}$", r"$2\pi r$", r"$\text{for all}\ x$", r"$100\%$", r"$\$5$",
    r"$a$ and $b$", r"$\binom{n}{k}$", r"$\genfrac{(}{)}{0}{}{a}{b}$", r"$\overset{a}{=}$",
    r"$\underset{x\to 0}{\lim}$", r"$\lim_{x\to\infty} f(x)$", r"$f'(x)$",
    r"$\widehat{xyz}$", r"$\not=$", r"$\ldots \cdots$", r"$\boldsymbol{\alpha x}$",
    "$αé$",
]
BAD_MATH = [
    r"$x^$", r"$\frac{a}$", r"$\foo$", r"$x^2^3$", r"$\left(x$", r"$\sqrt[$", r"${$",
    r"$\sqrt{}$", r"$\frac{}{}$", "$$", r"$\stackrel{a}{=}$", r"$x=\mathrm{%}$",
]


def test_mathtext():
    for fs in FONTSETS:
        rc = {"mathtext.fontset": fs}
        for s in MATH:
            m = whd(s, ismath=True, rc=rc)
            e = extent(s, rc=rc)
            if not isinstance(m, Exception):
                assert not isinstance(e, Exception), (fs, s)
        for s in BAD_MATH:
            assert isinstance(whd(s, ismath=True, rc=rc), ValueError), (fs, s)
            assert isinstance(extent(s, rc=rc), ValueError), (fs, s)
    # sizes and layout for the default fontset
    for s in [r"$x^2$", r"$\frac{a}{b}$", r"$\sum_{i=0}^{n} i$", "foo $x$ bar"]:
        for size in [6, 20, 48]:
            whd(s, ismath=True, font={"size": size})
            extent(s, kw={"size": size})
        layout(s)
        layout(s, kw={"rotation": 45, "ha": "center", "va": "center"})
    # math_fontfamily on the Text instead of the rcParam
    for fs in ["cm", "stix"]:
        a = extent(r"$\sum_{i=0}^{n} i$", kw={"math_fontfamily": fs})
        b = extent(r"$\sum_{i=0}^{n} i$", rc={"mathtext.fontset": fs})
        assert a.tolist() == b.tolist()
    assert isinstance(extent("$x$", kw={"math_fontfamily": "comic"}), ValueError)
    # non-math text with the text font changed does not affect math fonts
    extent(r"$x^2$", kw={"family": "serif"})
    extent(r"a $x^2$ b", kw={"family": "serif", "weight": "bold"})


# --------------------------------------------------------------------------
# Wrapping
# --------------------------------------------------------------------------

LONG = ("The quick brown fox jumps over the lazy dog and keeps running across the whole "
        "wide figure until it reaches the edge")

WRAP_CASES = [
    # (text, x, y, kw)
    (LONG, 0.1, 0.5, {"ha": "left"}),
    (LONG, 0.5, 0.5, {"ha": "left"}),
    (LONG, 0.5, 0.5, {"ha": "center"}),
    (LONG, 0.5, 0.5, {"ha": "right"}),
    (LONG, 0.9, 0.5, {"ha": "right"}),
    (LONG, 0.9, 0.5, {"ha": "left"}),
    (LONG, 0.95, 0.5, {"ha": "left"}),
    (LONG, 0.2, 0.5, {"ha": "center"}),
    (LONG, 0.5, 0.5, {"ha": "left", "rotation": 45}),
    (LONG, 0.5, 0.5, {"ha": "left", "rotation": 90}),
    (LONG, 0.5, 0.1, {"ha": "left", "rotation": 90}),
    (LONG, 0.5, 0.5, {"ha": "center", "rotation": 90}),
    (LONG, 0.5, 0.5, {"ha": "right", "rotation": 90}),
    (LONG, 0.5, 0.5, {"ha": "left", "rotation": -30}),
    (LONG, 0.5, 0.5, {"ha": "left", "rotation": 135}),
    (LONG, 0.5, 0.5, {"ha": "left", "rotation": 180}),
    (LONG, 0.5, 0.5, {"ha": "left", "rotation": 270}),
    (LONG, 0.3, 0.3, {"ha": "left", "rotation": 315}),
    (LONG, 0.5, 0.5, {"ha": "left", "rotation": 360}),
    (LONG, 0.5, 0.5, {"ha": "left", "va": "top", "rotation_mode": "default"}),
    (LONG, 0.1, 0.5, {"ha": "left", "size": 24}),
    (LONG, 0.1, 0.5, {"ha": "left", "family": "DejaVu Sans Mono"}),
    (LONG, 0.1, 0.5, {"ha": "left", "ma": "right"}),
    ("Supercalifragilisticexpialidocious" * 4, 0.5, 0.5, {"ha": "left"}),
    ("two  spaces  between  words " * 6, 0.5, 0.5, {"ha": "left"}),
    ("Explicit break here\n" + LONG, 0.5, 0.5, {"ha": "left"}),
    ("$x^2$ is the square of $x$ and $\\sqrt{y}$ is the root of y, " * 3, 0.4, 0.5, {"ha": "left"}),
    ("word " * 40, 0.0, 0.0, {"ha": "left"}),
    ("word " * 40, 1.0, 1.0, {"ha": "right"}),
    ("fits easily", 0.5, 0.5, {"ha": "left"}),
    ("", 0.5, 0.5, {"ha": "left"}),
]


def test_wrap():
    for text, x, y, kw in WRAP_CASES:
        kw = {**kw, "wrap": True}
        w = wrapped(text, x, y, kw)
        assert isinstance(w, str)
        assert w.replace("\n", " ") == text.replace("\n", " ") or text.endswith(" ")
        extent(text, x=x, y=y, kw=kw)
        layout(text, x=x, y=y, kw=kw)
    # No wrapping without wrap=True.
    assert wrapped(LONG, 0.5, 0.5, {"ha": "left"}) == LONG
    assert wrapped(LONG, 0.5, 0.5, {"ha": "left", "wrap": False}) == LONG
    assert "\n" in wrapped(LONG, 0.5, 0.5, {"ha": "left", "wrap": True})
    assert wrapped("fits easily", 0.5, 0.5, {"ha": "left", "wrap": True}) == "fits easily"
    # wrap forces rotation_mode='anchor': a default-mode rotated wrapped text equals
    # the anchor-mode extent.
    kw = {"ha": "left", "va": "top", "rotation": 30, "wrap": True}
    assert (extent(LONG, kw=kw).tolist()
            == extent(LONG, kw={**kw, "rotation_mode": "anchor"}).tolist())
    # Same text at dpi 200 wraps identically (all lengths scale with dpi).
    wrapped(LONG, 0.5, 0.5, {"ha": "left", "wrap": True}, dpi=200)


# --------------------------------------------------------------------------
# Font lookup
# --------------------------------------------------------------------------

TTF_FAMILIES = [
    "DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono", "DejaVu Sans Display",
    "DejaVu Serif Display", "STIXGeneral", "STIXNonUnicode", "STIXSizeOneSym",
    "STIXSizeTwoSym", "STIXSizeThreeSym", "STIXSizeFourSym", "STIXSizeFiveSym", "cmr10",
    "cmb10", "cmmi10", "cmsy10", "cmex10", "cmss10", "cmti10", "cmtt10",
    "Last Resort High-Efficiency", "dejavu sans", "DEJAVU SERIF", "sans-serif", "sans",
    "sans serif", "serif", "monospace", "cursive", "fantasy", "Arial", "Helvetica",
    "Times New Roman", "Comic Sans MS", "Foo", "", "DejaVu", "DejaVuSans",
]


def test_findfont_families():
    for fam in TTF_FAMILIES:
        findfont({"family": fam})
        findfont({"family": fam}, fallback_to_default=False)
    assert findfont({"family": "cursive"}) == "DejaVuSans.ttf"
    assert findfont({"family": "fantasy"}) == "DejaVuSans.ttf"
    assert findfont({"family": "sans-serif"}) == "DejaVuSans.ttf"
    assert findfont({"family": "serif"}) == "DejaVuSerif.ttf"
    assert findfont({"family": "monospace"}) == "DejaVuSansMono.ttf"
    assert findfont({"family": "dejavu sans"}) == "DejaVuSans.ttf"
    assert findfont(None) == "DejaVuSans.ttf"
    assert isinstance(findfont({"family": "Foo"}, fallback_to_default=False), ValueError)
    assert isinstance(findfont({"family": "cursive"}, fallback_to_default=False), ValueError)
    for fams in [["Foo", "DejaVu Serif"], ["cmr10", "DejaVu Sans"], ["DejaVu Sans", "cmr10"],
                 ["cursive", "serif"], ["monospace", "serif"], ["Foo", "Bar"], []]:
        findfont({"family": fams})
        findfont({"family": fams}, fallback_to_default=False)
        find_fonts_by_props({"family": fams})
        find_fonts_by_props({"family": fams}, fallback_to_default=False)
    for fam in ["DejaVu Sans", "cursive", "Foo", "serif"]:
        find_fonts_by_props({"family": fam})
        find_fonts_by_props({"family": fam, "weight": "bold", "style": "italic"})
    assert find_fonts_by_props({"family": ["cmr10", "Foo", "DejaVu Sans"]}) == [
        "cmr10.ttf", "DejaVuSans.ttf"]
    assert isinstance(find_fonts_by_props({"family": ["Foo"]}, fallback_to_default=False),
                      ValueError)


def test_findfont_properties():
    for fam in ["DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono", "STIXGeneral",
                "STIXNonUnicode", "cmr10", "sans-serif", "serif", "monospace", "cursive"]:
        for style in ["normal", "italic", "oblique"]:
            for weight in ["normal", "bold"]:
                findfont({"family": fam, "style": style, "weight": weight})
    for weight in ["ultralight", "light", "normal", "regular", "book", "medium", "roman",
                   "semibold", "demibold", "demi", "bold", "heavy", "extra bold", "black",
                   0, 100, 200, 300, 400, 500, 549, 550, 551, 600, 700, 800, 900, 1000]:
        findfont({"family": "DejaVu Sans", "weight": weight})
        findfont({"family": "STIXGeneral", "weight": weight, "style": "italic"})
    for stretch in ["ultra-condensed", "condensed", "normal", "expanded", 0, 900]:
        findfont({"family": "DejaVu Sans", "stretch": stretch})
    for size in ["xx-small", "xx-large", 1, 72, 1000]:
        findfont({"family": "DejaVu Serif", "size": size})
    findfont({"family": "DejaVu Sans", "variant": "small-caps"})
    findfont({"family": "DejaVu Sans", "variant": "small-caps", "weight": "bold"})
    assert findfont({"family": "DejaVu Sans", "style": "italic"}) == "DejaVuSans-Oblique.ttf"
    assert findfont({"family": "DejaVu Serif", "style": "oblique"}) == "DejaVuSerif-Italic.ttf"
    assert findfont({"family": "cmr10", "weight": "bold"}) == "cmr10.ttf"
    # rcParams-driven lookups
    findfont(None, rc={"font.family": "serif"})
    findfont(None, rc={"font.family": ["monospace"]})
    findfont(None, rc={"font.weight": "bold"})
    findfont({"family": "sans-serif"},
             rc={"font.sans-serif": ["Foo", "DejaVu Serif", "DejaVu Sans"]})
    findfont({"family": "cursive"}, rc={"font.cursive": ["cmr10"]})
    findfont({"family": "cursive"}, rc={"font.cursive": ["cmr10"]}, fallback_to_default=False)
    # fontconfig pattern strings
    for pattern in ["DejaVu Serif:bold:italic", "monospace:bold", "serif-20", ":italic",
                    "cmr10", "STIXGeneral:weight=bold:slant=italic", "Foo:bold",
                    "DejaVu Sans:weight=550", "sans\\-serif:oblique"]:
        findfont(pattern)


AFM_FAMILIES = [
    "Helvetica", "Times", "Courier", "Palatino", "Utopia", "New Century Schoolbook",
    "ITC Bookman", "ITC Avant Garde Gothic", "ITC Zapf Chancery", "ITC Zapf Dingbats",
    "Symbol", "ZapfDingbats", "Computer Modern", "cmti10", "sans-serif", "serif",
    "monospace", "cursive", "fantasy", "Foo", "DejaVu Sans",
]


def test_findfont_afm():
    for fam in AFM_FAMILIES:
        findfont({"family": fam}, fontext="afm")
    for fam in ["Helvetica", "Times", "Courier", "Palatino"]:
        for style in ["normal", "italic"]:
            for weight in ["normal", "medium", "roman", "light", "bold", 900]:
                findfont({"family": fam, "style": style, "weight": weight}, fontext="afm")
    findfont({"family": "Helvetica", "stretch": "condensed"}, fontext="afm")
    findfont({"family": "Helvetica", "stretch": "condensed", "weight": "bold"}, fontext="afm")
    findfont({"family": "Foo"}, fontext="afm", fallback_to_default=False)
    assert findfont({"family": "Foo"}, fontext="afm") == findfont({"family": "Helvetica"},
                                                                    fontext="afm")


# --------------------------------------------------------------------------
# FontProperties parsing
# --------------------------------------------------------------------------

def test_fontproperties_size():
    for size in [None, "xx-small", "x-small", "small", "medium", "large", "x-large",
                 "xx-large", "smaller", "larger", 0, 0.5, 1, 1.5, 10, 12.25, 72, 100, -5,
                 "12", "12.5", " 7 ", "1e1", "12pt", "huge", "", "Small", float("nan"),
                 float("inf")]:
        fontprops({"size": size})
    for rc_size in [8, 12, 13.5]:
        for size in ["small", "larger", "xx-large", None]:
            fontprops({"size": size}, rc={"font.size": rc_size})
    assert fontprops({"size": "small"})["size"] == pytest.approx(8.33)
    assert fontprops({"size": 0.5})["size"] == 1.0
    assert fontprops({"size": -5})["size"] == 1.0
    assert isinstance(fontprops({"size": "12pt"}), ValueError)


def test_fontproperties_weight_style_variant_stretch():
    for weight in [None, "ultralight", "light", "normal", "regular", "book", "medium", "roman",
                   "semibold", "demibold", "demi", "bold", "heavy", "extra bold", "black",
                   0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1001, -1, 450.7,
                   "500", "thin", "Bold", "bold ", "extrabold"]:
        fontprops({"weight": weight})
    for style in [None, "normal", "italic", "oblique", "Italic", "slanted"]:
        fontprops({"style": style})
    for variant in [None, "normal", "small-caps", "smallcaps"]:
        fontprops({"variant": variant})
    for stretch in [None, "ultra-condensed", "extra-condensed", "condensed", "semi-condensed",
                    "normal", "semi-expanded", "semi-extended", "expanded", "extended",
                    "extra-expanded", "extra-extended", "ultra-expanded", "ultra-extended",
                    0, 500, 1000, 1001, "300", "narrow"]:
        fontprops({"stretch": stretch})
    for mff in [None, "dejavusans", "dejavuserif", "cm", "stix", "stixsans", "custom", "comic"]:
        fontprops({"math_fontfamily": mff})
    assert fontprops({"weight": "500"})["weight"] == 500
    assert fontprops({"weight": 450.7})["weight"] == 450
    assert isinstance(fontprops({"weight": "thin"}), ValueError)
    assert isinstance(fontprops({"weight": 1001}), ValueError)
    assert isinstance(fontprops({"style": "Italic"}), ValueError)
    assert isinstance(fontprops({"stretch": "narrow"}), ValueError)


def test_fontproperties_family_and_patterns():
    for family in [None, "DejaVu Sans", "serif", "sans-serif", "Foo-12", ["sans-serif"],
                   ["Foo", "DejaVu Serif"], [], ""]:
        fontprops({"family": family})
    fontprops({"family": "sans-serif", "size": 12})
    fontprops({"family": "DejaVu Sans:bold", "math_fontfamily": "cm"})
    for pattern in [
        "DejaVu Sans:bold:italic:size=12", "serif-12", "serif-12.5", "monospace:weight=bold",
        "sans\\-serif", "Foo,Bar:style=italic", ":bold", "DejaVu Sans:slant=italic",
        ":condensed", ":stretch=condensed", ":weight=550", ":size=small",
        ":variant=small-caps", ":style=oblique", ":foo=bar", "a:b", ":light", ":thin",
        ":extrabold", ":roman", ":book", "DejaVu Sans-10:bold", "Foo\\:Bar",
    ]:
        fontprops({"family": pattern})
        probes.probe("_fontconfig_pattern.parse_fontconfig_pattern", parse_fontconfig_pattern,
                     pattern=pattern)
    assert isinstance(fontprops({"family": "sans-serif"}), ValueError)
    p = fontprops({"family": "DejaVu Sans:bold:italic:size=12"})
    assert p["family"] == ["DejaVu Sans"] and p["weight"] == "bold"
    assert p["style"] == "italic" and p["size"] == 12.0
    assert fontprops({"family": "Foo-12"})["size"] == 12.0
    assert fontprops({"family": "Foo,Bar:style=italic"})["family"] == ["Foo"]
    assert math.isclose(fontprops({"family": ":size=small"})["size"], 8.33)


# --------------------------------------------------------------------------
# Hinting (text.hinting other than the reference 'none')
# --------------------------------------------------------------------------

HINTED_GLYPHS = [
    ("DejaVuSans.ttf", "DEFAULT", "AaégW%&§0"),
    ("DejaVuSans-Bold.ttf", "DEFAULT", "Rsé"),
    ("DejaVuSerif.ttf", "DEFAULT", "Qgfé"),
    ("DejaVuSansMono.ttf", "DEFAULT", "m@1"),
    ("DejaVuSans-Oblique.ttf", "DEFAULT", "αx"),
    ("cmr10.ttf", "DEFAULT", "Ag="),
    ("cmmi10.ttf", "DEFAULT", "xπ"),
    ("STIXGeneral.ttf", "NO_AUTOHINT", "Agé"),
    ("DejaVuSans.ttf", "NO_AUTOHINT", "Ka"),
]
HINTED_TEXTS = ["Hamburgefonstiv", "AVA To, Wa.", "0123456789", "\u00c5\u00c9\u00e7\u00df ffi"]


def test_hinted_glyphs():
    for file, flags, chars in HINTED_GLYPHS:
        for size, dpi in [(6, 72), (9, 100), (12, 100), (17, 150), (24, 72)]:
            for char in chars:
                probes.probe("ft2font.FT2Font.load_glyph", _load_glyph, file=file, size=size,
                             dpi=dpi, char=char, flags=flags)


def test_hinted_text_metrics():
    for hinting in ["default", "native"]:
        rc = {"text.hinting": hinting}
        for dpi in [72, 100, 150]:
            for size in [6, 8, 10, 13, 18, 24]:
                for text in HINTED_TEXTS:
                    whd(text, font={"size": size}, dpi=dpi, rc=rc)
            whd("Wa.To", font={"family": "DejaVu Serif", "weight": "bold"}, dpi=dpi, rc=rc)
            whd("$x^2 + \\alpha$", ismath=True, dpi=dpi, rc=rc)
        for rotation in [0, 30, 90]:
            extent("Hinted AVA", kw={"size": 11, "rotation": rotation}, rc=rc)
        layout("two\nlines Wa", kw={"size": 9, "ha": "right"}, rc=rc)
    unhinted = whd("Hamburgefonstiv", font={"size": 10})
    hinted = whd("Hamburgefonstiv", font={"size": 10}, rc={"text.hinting": "default"})
    assert unhinted[0] != hinted[0]
