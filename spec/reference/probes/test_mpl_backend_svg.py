"""matplotlib's own tests of its SVG backend (``tests/test_backend_svg.py``)
that read the document: valid XML, hyperlinks, group ids and their
escaping, the ``svg.id`` root id, shared clip paths, how many bitmaps
rasterized artists make, the font stretch of text written as text, the
Dublin Core metadata and SVGZ output.

Area ``mpl_backend_svg``. Every op runs in the test suite's style (see
``_mpl_tests.py``) and saves with ``savefig(format="svg")``.

Ops
---
``Artist.set_visible``  {} -> whether the SVG of an error bar plot whose
    bar lines are hidden parses as XML (``test_visibility``).
``Text.set_url``  {} -> whether the SVG of a figure whose suptitle links
    to ``http://test_text_urls.matplotlib.org`` holds
    ``<a xlink:href="..." target="_blank">`` (``test_text_urls``).
``Artist.set_rasterized``  {figure} -> ``{"image", "path"}``: how many
    ``<image`` and ``<path`` tags the SVG of ``test_count_bitmaps``'
    figure ``figure`` (1 to 5) holds: five lines in an axes without axis,
    none rasterized (1), all rasterized (2), rasterized alternating with
    lines that are not (3), the same in a rasterized axes (4), all
    rasterized in a figure with ``suppressComposite`` (5).
``Text.set_fontstretch``  {} -> ``[text found, "condensed" found]`` in the
    SVG of a text at a date with ``svg.fonttype: none`` and
    ``font.stretch: condensed`` (``test_svgnone_with_data_coordinates``).
``Artist.set_gid``  {} -> the class names of the artists of
    ``test_gid``'s figure (an image, a scatter, a line, a legend, twin
    bars, polar and 3-D axes) whose group id, set on every visible artist
    after a draw, is missing from the SVG; ``test_gid`` leaves out offset
    boxes, empty texts and texts outside axes, one-point lines and lines
    outside axes, and ticks at 0 or outside the view, which draw nothing.
``Artist.set_clip_path``  {} -> ``{"ids", "unique"}``: how many clip
    paths the SVG of six images clipped to two circles defines, and
    whether their ids are unique (``test_clip_path_ids_reuse``).
``Figure.savefig``  {format, bbox_inches} -> whether the document starts
    with the gzip signature (``test_savefig_tight``: SVGZ of a tight box).
``Collection.set_urls``  {} -> the links among ``foo``, ``bar``, ``baz``
    and ``quux`` that the SVG of ``test_url``'s scatter, line and marker
    line holds.
``Tick.set_url``  {} -> ``{"urls", "same"}``: whether the SVG of a figure
    whose y ticks link to ``https://example.com/<i>`` holds each link, and
    whether it equals the SVG of the same figure linking the tick labels
    instead (``test_url_tick``).
``savefig.svg.metadata``  {metadata} -> ``{"titles", "work"}`` of a
    ``plt.subplots()`` figure saved with ``metadata`` while
    ``SOURCE_DATE_EPOCH`` is 19680801: the texts of the root's ``<title>``
    elements and the entries of ``<metadata>/<rdf:RDF>/<cc:Work>``
    (``null`` without one) as ``[tag, value]``: the ``rdf:resource`` of
    ``dc:type``, the agent titles of ``dc:creator``, ``dc:contributor``,
    ``dc:publisher`` and ``dc:rights``, the ``rdf:li`` of ``dc:subject``
    and the text of the others. The default creator (the library and its
    version) reads ``"<default>"``. ``metadata`` maps keys to strings,
    lists of strings, ``null`` or dates ``{"date": iso}`` and date-times
    ``{"datetime": iso}``, single or in lists (``test_svg_default_metadata``,
    ``test_svg_clear_default_metadata``, ``test_svg_clear_all_metadata``
    and ``test_svg_metadata``).
``Text.set_gid``  {} -> whether a figure text and group id ``<'"&>`` are
    written as ``&lt;&apos;&quot;&amp;&gt;"`` (``test_svg_escape``).
``AnnotationBbox.set_gid``  {} -> whether the SVG of an annotation box of
    an image with an arrow holds ``<g id="a test for issue 20044">``
    (``test_annotationbbox_gid``).
``savefig.svg.id``  {svg_id} -> the ``id`` of the document's root element
    (``null`` without one) with ``svg.id`` set to ``svg_id``
    (``test_svgid``).

Already covered
---------------
By the ``mpl_backends`` figure group: ``test_fill_black_with_alpha``,
``test_noscale``, ``test_bold_font_output``,
``test_bold_font_output_with_none_fonttype``, ``test_rasterized``,
``test_rasterized_ordering``, ``test_prevent_rasterization``,
``test_multi_font_aspath`` and ``test_multi_font_astext``.

Left out
--------
``test_unicode_won`` (usetex), ``test_svg_font_string`` (needs the
WenQuanYi Zen Hei font) and ``test_svg_incorrect_metadata``: its values
of the wrong type and unknown keys cannot be written with pltrs's typed
``Metadata``.
"""

import datetime
import gzip
import io
import os
import xml.etree.ElementTree
import xml.parsers.expat

import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.axis import Tick
from matplotlib.figure import Figure
from matplotlib.offsetbox import AnnotationBbox, OffsetBox, OffsetImage
from matplotlib.patches import Circle
from matplotlib.text import Text

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_backend_svg")
probe = prober(probes)

SVGNS = "{http://www.w3.org/2000/svg}"
RDFNS = "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}"
CCNS = "{http://creativecommons.org/ns#}"
DCNS = "{http://purl.org/dc/elements/1.1/}"

#: The ``SOURCE_DATE_EPOCH`` of the metadata tests.
EPOCH = "19680801"


def _svg(fig, **options):
    buffer = io.BytesIO()
    fig.savefig(buffer, format="svg", **options)
    return buffer.getvalue()


# --- test_visibility -------------------------------------------------------

def _hidden_bars_parse():
    fig, ax = plt.subplots()
    x = np.linspace(0, 4 * np.pi, 50)
    y = np.sin(x)
    _, _, bars = ax.errorbar(x, y, yerr=np.ones_like(y), fmt="ko")
    for artist in bars:
        artist.set_visible(False)
    xml.parsers.expat.ParserCreate().Parse(_svg(fig))
    return True


def test_visibility():
    probe("Artist.set_visible", _hidden_bars_parse)


# --- test_text_urls --------------------------------------------------------

def _suptitle_url():
    fig = plt.figure()
    url = "http://test_text_urls.matplotlib.org"
    fig.suptitle("test_text_urls", url=url)
    return f'<a xlink:href="{url}" target="_blank">' in _svg(fig).decode()


def test_text_urls():
    probe("Text.set_url", _suptitle_url)


# --- test_count_bitmaps ----------------------------------------------------

def _bitmaps(figure):
    fig = plt.figure()
    if figure == 5:
        fig.suppressComposite = True
    ax = fig.add_subplot(1, 1, 1)
    ax.set_axis_off()
    if figure == 4:
        ax.set_rasterized(True)
    for n in range(5):
        if figure in (3, 4):
            ax.plot([0, 20], [n, 0], "b-", rasterized=False)
        ax.plot([0, 20], [0, n], "b-", rasterized=figure != 1)
    svg = _svg(fig).decode()
    return {"image": svg.count("<image"), "path": svg.count("<path")}


def test_count_bitmaps():
    for figure in range(1, 6):
        probe("Artist.set_rasterized", _bitmaps, figure=figure)


# --- test_svgnone_with_data_coordinates ------------------------------------

def _stretched_text():
    expected = "Unlikely to appear by chance"
    fig, ax = plt.subplots()
    ax.text(np.datetime64("2019-06-30"), 1, expected)
    ax.set_xlim(np.datetime64("2019-01-01"), np.datetime64("2019-12-31"))
    ax.set_ylim(0, 2)
    svg = _svg(fig).decode()
    return [expected in svg, "condensed" in svg]


def test_svgnone_with_data_coordinates():
    probe("Text.set_fontstretch", _stretched_text,
          rc={"svg.fonttype": "none", "font.stretch": "condensed"})


# --- test_gid --------------------------------------------------------------

def _drawn(obj):
    """``test_gid``'s ``include``: whether ``obj`` shows in the SVG."""
    if isinstance(obj, OffsetBox):
        return False
    if isinstance(obj, Text):
        if obj.get_text() == "" or obj.axes is None:
            return False
    if isinstance(obj, plt.Line2D):
        xdata, ydata = obj.get_data()
        if len(xdata) == len(ydata) == 1:
            return False
        if not hasattr(obj, "axes") or obj.axes is None:
            return False
    if isinstance(obj, Tick):
        loc = obj.get_loc()
        if loc == 0:
            return False
        vi = obj.get_view_interval()
        if loc < min(vi) or loc > max(vi):
            return False
    return True


def _missing_gids():
    fig = plt.figure()
    ax1 = fig.add_subplot(131)
    ax1.imshow([[1., 2.], [2., 3.]], aspect="auto")
    ax1.scatter([1, 2, 3], [1, 2, 3], label="myscatter")
    ax1.plot([2, 3, 1], label="myplot")
    ax1.legend()
    ax1a = ax1.twinx()
    ax1a.bar([1, 2, 3], [1, 2, 3])
    ax2 = fig.add_subplot(132, projection="polar")
    ax2.plot([0, 1.5, 3], [1, 2, 3])
    ax3 = fig.add_subplot(133, projection="3d")
    ax3.plot([1, 2], [1, 2], [1, 2])
    fig.canvas.draw()
    gids = {}
    for idx, obj in enumerate(fig.findobj(include_self=True)):
        if obj.get_visible():
            gid = f"test123{obj.__class__.__name__}_{idx}"
            gids[gid] = obj
            obj.set_gid(gid)
    svg = _svg(fig).decode()
    return sorted(type(obj).__name__ for gid, obj in gids.items()
                  if _drawn(obj) and gid not in svg)


def test_gid():
    probe("Artist.set_gid", _missing_gids)


# --- test_clip_path_ids_reuse ----------------------------------------------

def _clip_path_ids():
    fig, circle = Figure(), Circle((0, 0), radius=10)
    for i in range(5):
        ax = fig.add_subplot()
        ax.imshow([[i]]).set_clip_path(circle)
    ax = fig.add_subplot()
    ax.imshow([[0]]).set_clip_path(Circle((0, 0), radius=1))
    tree = xml.etree.ElementTree.fromstring(_svg(fig))
    ids = [node.attrib["id"] for node in tree.findall(f".//{SVGNS}clipPath[@id]")]
    return {"ids": len(ids), "unique": len(set(ids)) == len(ids)}


def test_clip_path_ids_reuse():
    probe("Artist.set_clip_path", _clip_path_ids)


# --- test_savefig_tight ----------------------------------------------------

def _gzipped(format, bbox_inches):
    buffer = io.BytesIO()
    plt.savefig(buffer, format=format, bbox_inches=bbox_inches)
    document = buffer.getvalue()
    gzip.decompress(document)
    return document[:2] == b"\x1f\x8b"


def test_savefig_tight():
    probe("Figure.savefig", _gzipped, format="svgz", bbox_inches="tight")


# --- test_url --------------------------------------------------------------

def _urls():
    fig, ax = plt.subplots()
    s = ax.scatter([1, 2, 3], [4, 5, 6])
    s.set_urls(["https://example.com/foo", "https://example.com/bar", None])
    p, = ax.plot([2, 3, 4], [4, 5, 6])
    p.set_url("https://example.com/baz")
    p, = ax.plot([3, 4, 5], [4, 5, 6], linestyle="none", marker="x")
    p.set_url("https://example.com/quux")
    svg = _svg(fig)
    return [v for v in ["foo", "bar", "baz", "quux"]
            if f"https://example.com/{v}".encode() in svg]


def test_url():
    probe("Collection.set_urls", _urls)


# --- test_url_tick ---------------------------------------------------------

def _tick_urls():
    previous = os.environ.get("SOURCE_DATE_EPOCH")
    os.environ["SOURCE_DATE_EPOCH"] = EPOCH
    try:
        fig1, ax = plt.subplots()
        ax.scatter([1, 2, 3], [4, 5, 6])
        for i, tick in enumerate(ax.yaxis.get_major_ticks()):
            tick.set_url(f"https://example.com/{i}")
        fig2, ax = plt.subplots()
        ax.scatter([1, 2, 3], [4, 5, 6])
        for i, tick in enumerate(ax.yaxis.get_major_ticks()):
            tick.label1.set_url(f"https://example.com/{i}")
            tick.label2.set_url(f"https://example.com/{i}")
        b1, b2 = _svg(fig1), _svg(fig2)
    finally:
        _restore_epoch(previous)
    urls = [f"https://example.com/{i}".encode() in b1
            for i in range(len(ax.yaxis.get_major_ticks()))]
    return {"urls": urls, "same": b1 == b2}


def test_url_tick():
    probe("Tick.set_url", _tick_urls)


# --- the metadata tests ----------------------------------------------------

def _restore_epoch(previous):
    if previous is None:
        del os.environ["SOURCE_DATE_EPOCH"]
    else:
        os.environ["SOURCE_DATE_EPOCH"] = previous


def _decode_date(value):
    if isinstance(value, dict):
        (kind, iso), = value.items()
        if kind == "date":
            return datetime.date.fromisoformat(iso)
        return datetime.datetime.fromisoformat(iso)
    return value


def _decode(metadata):
    return {key: [_decode_date(v) for v in value] if isinstance(value, list)
            else _decode_date(value) for key, value in metadata.items()}


def _entry(node):
    tag = node.tag.replace(DCNS, "dc:")
    if tag == "dc:type":
        return [tag, node.attrib[f"{RDFNS}resource"]]
    if tag == "dc:subject":
        return [tag, [li.text for li in node.findall(f"./{RDFNS}Bag/{RDFNS}li")]]
    agents = node.findall(f"./{CCNS}Agent/{DCNS}title")
    if agents:
        names = [agent.text for agent in agents]
        default = f"Matplotlib v{mpl.__version__}, https://matplotlib.org/"
        return [tag, ["<default>" if name == default else name for name in names]]
    return [tag, node.text]


def _metadata(metadata):
    previous = os.environ.get("SOURCE_DATE_EPOCH")
    os.environ["SOURCE_DATE_EPOCH"] = EPOCH
    try:
        fig, _ = plt.subplots()
        root = xml.etree.ElementTree.fromstring(_svg(fig, metadata=_decode(metadata)))
    finally:
        _restore_epoch(previous)
    works = root.findall(f"./{SVGNS}metadata/{RDFNS}RDF/{CCNS}Work")
    return {
        "titles": [node.text for node in root.findall(f"./{SVGNS}title")],
        "work": [_entry(node) for node in works[0]] if works else None,
    }


DEFAULTS = ["Date", "Creator", "Format", "Type"]
SINGLE_VALUE = ["Coverage", "Identifier", "Language", "Relation", "Source", "Title", "Type"]
MULTI_VALUE = ["Contributor", "Creator", "Keywords", "Publisher", "Rights"]


def test_svg_default_metadata():
    probe("savefig.svg.metadata", _metadata, metadata={})


def test_svg_clear_default_metadata():
    for key in DEFAULTS:
        probe("savefig.svg.metadata", _metadata, metadata={key: None})


def test_svg_clear_all_metadata():
    probe("savefig.svg.metadata", _metadata, metadata={key: None for key in DEFAULTS})


def test_svg_metadata():
    metadata = {
        "Date": [{"date": "1968-08-01"}, {"datetime": "1968-08-02T01:02:03"}],
        "Description": "description\ntext",
        **{k: f"{k} foo" for k in SINGLE_VALUE},
        **{k: [f"{k} bar", f"{k} baz"] for k in MULTI_VALUE},
    }
    probe("savefig.svg.metadata", _metadata, metadata=metadata)


# --- test_svg_escape -------------------------------------------------------

def _escaped():
    fig = plt.figure()
    fig.text(0.5, 0.5, "<'\"&>", gid="<'\"&>")
    return '&lt;&apos;&quot;&amp;&gt;"' in _svg(fig).decode()


def test_svg_escape():
    probe("Text.set_gid", _escaped)


# --- test_annotationbbox_gid -----------------------------------------------

def _annotation_box_gid():
    fig = plt.figure()
    ax = fig.add_subplot()
    imagebox = OffsetImage(np.ones((32, 32)), zoom=0.1)
    imagebox.image.axes = ax
    ab = AnnotationBbox(imagebox, (0.3, 0.55), xybox=(120., -80.), xycoords="data",
                        boxcoords="offset points", pad=0.5,
                        arrowprops=dict(arrowstyle="->",
                                        connectionstyle="angle,angleA=0,angleB=90,rad=3"))
    ab.set_gid("a test for issue 20044")
    ax.add_artist(ab)
    return '<g id="a test for issue 20044">' in _svg(fig).decode()


def test_annotationbbox_gid():
    probe("AnnotationBbox.set_gid", _annotation_box_gid)


# --- test_svgid ------------------------------------------------------------

def _root_id(svg_id):
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [3, 2, 1])
    fig.canvas.draw()
    with mpl.rc_context({"svg.id": svg_id}):
        root = xml.etree.ElementTree.fromstring(_svg(fig))
    return root.attrib.get("id")


def test_svgid():
    for svg_id in (None, "a test for issue 28535"):
        probe("savefig.svg.id", _root_id, svg_id=svg_id)
