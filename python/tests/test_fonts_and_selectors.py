"""
The AFM fonts of matplotlib's ``fonts/afm`` (``findfont(fontext='afm')``,
``ps.useafm``, the AFM parser), ``findfont``'s cache and what it keys on,
the ``cmap`` of subset fonts, and the selectors' blitting. Each
``scenario_*`` returns what its test compares; the expected values were
recorded from matplotlib 3.11.2 running the same scenarios.
"""

import io
import logging
import os
import re

import numpy as np

import pltrs
import pltrs.pyplot as plt
from pltrs import _afm, cbook, font_manager, widgets
from pltrs.backend_bases import MouseEvent
from pltrs.backends import backend_pdf, backend_ps
from pltrs.backends._backend_pdf_ps import font_as_file, get_glyphs_subset
from pltrs.font_manager import FontEntry, FontPath, FontProperties, fontManager
from pltrs.ft2font import FT2Font

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _afm_dir(name):
    """The data's AFM directory *name* (``afm`` or ``pdfcorefonts``)."""
    return cbook._get_data_path("fonts", name)


def scenario_afm_font_list():
    """The AFM font list: file names, families, styles, weights and
    stretches, and the directories the files are in."""
    entries = sorted(
        (os.path.basename(e.fname), e.name, e.style, e.variant, e.weight,
         e.stretch, e.size) for e in fontManager.afmlist)
    directories = sorted({os.path.basename(os.path.dirname(e.fname))
                          for e in fontManager.afmlist})
    return len(entries), entries[:3], entries[-3:], directories


FAMILIES = [
    {"family": ["serif"]}, {"family": ["sans-serif"]}, {"family": ["monospace"]},
    {"family": ["cursive"]}, {"family": ["fantasy"]},
    {"family": ["Palatino"], "weight": "bold", "style": "italic"},
    {"family": ["ITC Bookman"], "weight": "light"},
    {"family": ["ITC Avant Garde Gothic"], "style": "oblique"},
    {"family": ["Helvetica"], "stretch": "condensed", "weight": "bold"},
    {"family": ["Helvetica"], "weight": "light"},
    {"family": ["ITC Zapf Chancery"]}, {"family": ["ITC Zapf Dingbats"]},
    {"family": ["Utopia"], "weight": "bold"}, {"family": ["cmti10"]},
    {"family": ["New Century Schoolbook"], "style": "italic"},
]


def scenario_findfont_afm():
    """``findfont(fontext='afm')`` in each of the two directories, as file
    names. (Over both, the order of matplotlib's font list decides between
    their Courier, Helvetica and Times: it is the order its scan of the
    directories finds them, by the hashes of their names.)"""
    found = []
    for props in FAMILIES:
        prop = FontProperties(**props)
        found.append([
            os.path.basename(font_manager.findfont(prop, fontext="afm", directory=d))
            for d in (_afm_dir("afm"), _afm_dir("pdfcorefonts"))])
    return found


def scenario_afm_metrics():
    """The AFM parser's answers for files of ``fonts/afm``."""
    results = []
    for name in ["pagd8a.afm", "pncri8a.afm", "pzdr.afm", "cmsy10.afm",
                 "phvl8a.afm", "putb8a.afm"]:
        with open(_afm_dir("afm") / name, "rb") as fh:
            afm = _afm.AFM(fh)
        symbols = name in ("pzdr.afm", "cmsy10.afm")
        text = None if symbols else afm.get_str_bbox_and_descent("Ag")
        results.append([
            afm.get_fontname(), afm.get_familyname(), afm.get_weight(),
            afm.get_angle(), afm.get_glyph_name(65), text,
            afm.get_kern_dist_from_name("A", "V"), afm.get_width_char(65),
            None if symbols else afm.get_ascender(), afm.postscript_name,
        ])
    return results


def scenario_ps_useafm():
    """The fonts a ``ps.useafm`` EPS selects and the glyphs it shows, and
    the PostScript renderer's AFM metrics."""
    fig = plt.figure(figsize=(3, 2))
    for i, family in enumerate(["serif", "Palatino", "ITC Bookman", "sans-serif"]):
        fig.text(0.05, 0.1 + 0.2 * i, "AVATAR fi", family=[family], size=12)
    buf = io.BytesIO()
    with pltrs.rc_context({"ps.useafm": True}):
        fig.savefig(buf, format="eps")
        renderer = backend_ps.RendererPS(3, 2, None)
        metrics = [renderer.get_text_width_height_descent(
            "AVATAR", FontProperties(family=[family], size=12), False)
            for family in ["serif", "Palatino"]]
        afm = renderer._get_font_afm(FontProperties(family=["Palatino"]))
    text = buf.getvalue().decode("latin-1")
    fonts = re.findall(r"/(\S+) 12\.000 selectfont", text)
    glyphs = re.findall(r"([\d.]+) 0 m /(\w+) glyphshow", text)[:8]
    return (fonts, glyphs, [tuple(round(v, 9) for v in m) for m in metrics],
            afm.get_fontname(),
            os.path.basename(backend_ps.RendererPS._afm_font_dir),
            os.path.basename(backend_pdf.RendererPdf._afm_font_dir))


def scenario_findfont_cache():
    """The cache's statistics after lookups of the same font with other
    options and spellings."""
    fontManager._findfont_cached.cache_clear()
    fp = FontProperties(family=["DejaVu Sans"])
    data = cbook._get_data_path("fonts")
    calls = [
        lambda: font_manager.findfont(fp),
        lambda: font_manager.findfont(fp),
        lambda: font_manager.findfont(fp, rebuild_if_missing=False),
        lambda: font_manager.findfont("DejaVu Sans"),
        lambda: font_manager.findfont("DejaVu Sans", directory=data),
        lambda: font_manager.findfont("DejaVu Sans", directory=str(data)),
        lambda: font_manager.findfont("DejaVu Sans", fallback_to_default=False),
    ]
    stats = []
    for call in calls:
        call()
        info = fontManager._findfont_cached.cache_info()
        stats.append((info.hits, info.misses))
    found = font_manager.findfont("DejaVu Sans")
    return stats, type(found).__name__, os.path.basename(found)


def scenario_missing_font_file(caplog, tmp_path):
    """A font list entry whose file is gone since it was listed: refused
    without a rebuild, else the font list rebuilt and the family looked up
    again."""
    path = tmp_path / "GapsOne.ttf"
    path.write_bytes((cbook._get_data_path("fonts", "ttf") / "DejaVuSans.ttf").read_bytes())
    entry = FontEntry(fname=str(path), name="Gaps One", size="scalable")
    fontManager.ttflist.append(entry)
    path.unlink()
    fontManager._findfont_cached.cache_clear()
    prop = FontProperties(family=["Gaps One"])
    try:
        font_manager.findfont(prop, rebuild_if_missing=False)
        refused = None
    except ValueError as err:
        refused = str(err)
    with caplog.at_level(logging.INFO, logger="pltrs.font_manager"):
        found = os.path.basename(font_manager.findfont(prop))
    messages = [r.getMessage() for r in caplog.records
                if r.getMessage().startswith(("findfont: Found", "findfont: Font family"))]
    kept = any(e.name == "Gaps One" for e in fontManager.ttflist)
    return refused, found, messages, kept, fontManager.ttflist[0].size


def _subset_charmap(fname, text):
    """The characters of the subset of *fname* for the glyphs of *text*,
    read back with FT2Font."""
    font = FT2Font(fname)
    if "LastResort" in fname:
        font.set_charmap(0)
    cmap = font.get_charmap()
    glyphs = {cmap[ord(c)] for c in text}
    with get_glyphs_subset(FontPath(fname, 0), glyphs) as subset:
        subfont = FT2Font(font_as_file(subset.font))
    if "LastResort" in fname:
        subfont.set_charmap(0)
    return sorted(subfont.get_charmap())


def scenario_subset_charmaps():
    """The characters the subsets of three fonts map: those of the glyphs
    asked for (for Last Resort, every character of their blocks)."""
    ttf = cbook._get_data_path("fonts", "ttf")
    serif = _subset_charmap(str(ttf / "DejaVuSerif.ttf"), "these should be subsetted! 123")
    stix = _subset_charmap(str(ttf / "STIXGeneral.ttf"), "AB→\U0001d400")
    last = _subset_charmap(str(ttf / "LastResortHE-Regular.ttf"), "A→")
    return serif, stix, (len(last), last[:3], last[-3:])


def _drag(fig, steps):
    """Sends the mouse *steps* ``(event name, x, y)`` with the left button;
    the canvas pixel at (125, 200) from the top after each."""
    pixels = []
    for name, x, y in steps:
        MouseEvent(name, fig.canvas, x, y, button=1)._process()
        pixels.append(tuple(int(v) for v in
                            np.asarray(fig.canvas.buffer_rgba())[200, 125]))
    return pixels


def scenario_blitting_span():
    """A blitting span selector: its artists animated, the canvas after
    each step of a drag, the draws a full draw and reactivation make."""
    fig = plt.figure(figsize=(4, 4), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    span = widgets.SpanSelector(ax, lambda *args: None, "horizontal",
                                useblit=True, interactive=True)
    animated = [a.get_animated() for a in span.artists]
    draws = []
    fig.canvas.mpl_connect("draw_event", lambda event: draws.append(1))
    fig.canvas.draw()
    pixels = _drag(fig, [("button_press_event", 50, 200),
                         ("motion_notify_event", 100, 200),
                         ("motion_notify_event", 200, 200),
                         ("button_release_event", 200, 200)])
    counts = [len(draws)]
    fig.canvas.draw()
    counts.append(len(draws))
    span.set_active(False)
    span.set_active(True)
    counts.append(len(draws))
    return span.useblit, animated, pixels, counts


def scenario_redrawing_rectangle():
    """A rectangle selector without blitting: the draws after each step."""
    fig = plt.figure(figsize=(4, 4), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    selector = widgets.RectangleSelector(ax, lambda *args: None)
    draws = []
    fig.canvas.mpl_connect("draw_event", lambda event: draws.append(1))
    fig.canvas.draw()
    counts = []
    for name, x, y in [("button_press_event", 50, 50), ("motion_notify_event", 100, 100),
                       ("motion_notify_event", 200, 200),
                       ("button_release_event", 200, 200)]:
        MouseEvent(name, fig.canvas, x, y, button=1)._process()
        counts.append(len(draws) - 1)
    return selector.useblit, counts



# Recorded from matplotlib 3.11.2 running the scenarios above.
AFM_FONT_LIST = (61,
                 [('Courier-Bold.afm', 'Courier', 'normal', 'normal', 'bold',
                   'normal', 'scalable'),
                  ('Courier-BoldOblique.afm', 'Courier', 'italic', 'normal',
                   'bold', 'normal', 'scalable'),
                  ('Courier-Oblique.afm', 'Courier', 'italic', 'normal',
                   'medium', 'normal', 'scalable')],
                 [('putri8a.afm', 'Utopia', 'italic', 'normal', 'regular',
                   'normal', 'scalable'),
                  ('pzcmi8a.afm', 'ITC Zapf Chancery', 'italic', 'normal',
                   'medium', 'normal', 'scalable'),
                  ('pzdr.afm', 'ITC Zapf Dingbats', 'normal', 'normal',
                   'medium', 'normal', 'scalable')],
                 ['afm', 'pdfcorefonts'])
FINDFONT_AFM = [['pncr8a.afm', 'Times-Roman.afm'],
                ['phvr8a.afm', 'Helvetica.afm'], ['pcrr8a.afm', 'Courier.afm'],
                ['phvr8a.afm', 'Helvetica.afm'],
                ['phvr8a.afm', 'Helvetica.afm'],
                ['pplbi8a.afm', 'Helvetica-BoldOblique.afm'],
                ['pbkl8a.afm', 'Helvetica.afm'],
                ['pagko8a.afm', 'Helvetica-Oblique.afm'],
                ['phvb8an.afm', 'Helvetica-Bold.afm'],
                ['phvl8a.afm', 'Helvetica.afm'],
                ['pzcmi8a.afm', 'Helvetica.afm'],
                ['pzdr.afm', 'Helvetica.afm'],
                ['putb8a.afm', 'Helvetica-Bold.afm'], ['cmti10.afm', 'Helvetica.afm'],
                ['pncri8a.afm', 'Helvetica-Oblique.afm']]
AFM_METRICS = [['AvantGarde-Demi', 'ITC Avant Garde Gothic', 'Demi', 0.0, 'A',
                (0, -226, 1400.0, 966, 226), -102.0, 740.0, 740.0,
                'AvantGarde-Demi'],
               ['NewCenturySchlbk-Italic', 'New Century Schoolbook', 'Medium',
                -16.0, 'A', (-87, -205, 1241.0, 942, 205), -74.0, 704.0, 737.0,
                'NewCenturySchlbk-Italic'],
               ['ZapfDingbats', 'ITC Zapf Dingbats', 'Medium', 0.0, 'a10',
                None, 0, 692.0, None, 'ZapfDingbats'],
               ['CMSY10', 'Computer Modern', 'Medium', -14.035, 'A', None, 0,
                798.469, None, 'CMSY10'],
               ['Helvetica-Light', 'Helvetica', 'Light', 0.0, 'A',
                (0, -212, 1278.0, 932, 212), -74.0, 667.0, 720.0,
                'Helvetica-Light'],
               ['Utopia-Bold', 'Utopia', 'Bold', 0.0, 'A',
                (-28, -242, 1177.0, 934, 242), -70.0, 644.0, 742.0,
                'Utopia-Bold']]
PS_USEAFM = (['NewCenturySchlbk-Roman', 'Palatino-Roman', 'Bookman-Demi',
              'Helvetica'],
             [('0', 'A'), ('7.764', 'V'), ('15.54', 'A'), ('23.988', 'T'),
              ('31.776', 'A'), ('40.44', 'R'), ('49.104', 'space'),
              ('52.44', 'f')],
             [(49.104, 9.024, 0.18), (47.604, 8.472, 0.108)], 'Palatino-Roman',
             'afm', 'pdfcorefonts')
FINDFONT_CACHE = ([(0, 1), (1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (1, 6)],
                  'FontPath', 'DejaVuSans.ttf')
MISSING_FONT_FILE = ('No valid font could be found', 'DejaVuSans.ttf',
                     ['findfont: Found a missing font file.  Rebuilding '
                      'cache.',
                      "findfont: Font family ['Gaps One'] not found. Falling "
                      'back to DejaVu Sans.'],
                     False, 'scalable')
SUBSET_CHARMAPS = ([32, 33, 49, 50, 51, 98, 100, 101, 104, 108, 111, 115, 116,
                    117],
                   [65, 66, 8594, 119808],
                   (240, [0, 1, 2], [8701, 8702, 8703]))
BLITTING_SPAN = (True, [True, True, True],
                 [(255, 255, 255, 255), (255, 255, 255, 255),
                  (255, 127, 127, 255), (255, 127, 127, 255)],
                 [1, 3, 4])
REDRAWING_RECTANGLE = (False, [2, 3, 4, 5])


def test_afm_font_list():
    assert scenario_afm_font_list() == AFM_FONT_LIST


def test_findfont_afm():
    assert scenario_findfont_afm() == FINDFONT_AFM


def test_afm_metrics():
    assert scenario_afm_metrics() == AFM_METRICS


def test_ps_useafm():
    assert scenario_ps_useafm() == PS_USEAFM


def test_findfont_cache():
    assert scenario_findfont_cache() == FINDFONT_CACHE


def test_missing_font_file(caplog, tmp_path):
    assert scenario_missing_font_file(caplog, tmp_path) == MISSING_FONT_FILE


def test_subset_charmaps():
    assert scenario_subset_charmaps() == SUBSET_CHARMAPS


def test_blitting_span():
    assert scenario_blitting_span() == BLITTING_SPAN


def test_redrawing_rectangle():
    assert scenario_redrawing_rectangle() == REDRAWING_RECTANGLE
