"""matplotlib's own tests of its PDF backend (``tests/test_backend_pdf.py``)
that read the document: multi-page documents, composite and indexed
images, the information dictionary, link annotations of texts, font
subsets and the size saved by leaving out markers off the page.

Area ``mpl_backend_pdf``. Every op runs in the test suite's style (see
``_mpl_tests.py``). Documents are read without a PDF library: the probes
search the uncompressed object syntax matplotlib writes.

Ops
---
``PdfPages.get_pagecount``  {} -> the page count of a ``PdfPages``
    document when new, after ``fig.savefig(pdf, format="pdf")`` and after
    ``pdf.savefig()`` (``test_multipage_pagecount``).
``PdfPages.close``  {} -> ``{"startxref", "small"}``: how many times a
    document of ten titled pages says ``startxref``, and whether it is
    under 40000 bytes (``test_multipage_properfinalize``).
``PdfPages.__exit__``  {pages} -> whether a ``PdfPages`` file given
    ``pages`` empty figures exists once closed (``test_multipage_keep_empty``).
``Axes.imshow``  {} -> how many images (soft masks aside) a page of two
    images in one axes holds, with ``image.composite_image`` on or off
    (``test_composite_image``).
``Figure.figimage``  {} -> ``{"indexed", "colors", "pixels"}`` of the
    image of a 1 x 256 figure image of 256 reds saved at the figure's
    resolution with ``pdf.compression: 1`` (the test's ``True``): whether
    it is written with an indexed colour space, the palette size, and
    whether its decoded pixels are the image's (``test_indexed_image``).
``savefig.pdf.metadata``  {metadata, pages} -> the document information
    dictionary of a ``plt.subplots()`` figure plotting ``range(5)`` saved
    with ``metadata`` while ``SOURCE_DATE_EPOCH`` is 0, with
    ``savefig`` (``pages`` null) or as ``pages`` pages of a ``PdfPages``
    document made with it: ``{key: value}``, strings as text, names as
    ``/Name``; the default creator and producer (the library and its
    version) read ``"<default>"``. Dates are ``{"datetime": iso}``
    (``test_savefig_metadata``, ``test_multipage_metadata``).
``Text.set_url``  {figsize, texts} -> the link annotations of a figure of
    size ``figsize`` with ``fig.text(x, y, s, rotation=r, url=u)`` for
    each ``[x, y, s, r, u]``: ``[uri, rect, quadpoints]`` each, quad
    points ``null`` when absent (``test_text_urls``,
    ``test_text_rotated_urls``).
``Line2D.set_rasterized``  {} -> whether a rasterized empty line saves
    as PDF (``test_empty_rasterized``).
``get_glyphs_subset``  {font, chars} -> ``{"chars", "fewer", "glyphs"}``:
    the characters the character map of the subset of the bundled font
    file ``font`` keeping the glyphs of ``chars`` maps (sorted), whether
    it maps fewer than the font's, and whether laying ``chars`` out in
    both loads as many glyphs (``test_glyphs_subset``).
``Axes.scatter``  {} -> ``{"offaxis", "visible", "visible_vs_offaxis"}``
    of ``test_scatter_offaxis_colored_pdf_size``: whether 1000 coloured
    markers off the view save within 5000 bytes of an empty scatter, and
    whether the same markers in view save at least 15000 bytes more than
    the empty one and the off-view one.

Already covered
---------------
By the ``mpl_backends`` figure group: ``test_use14corefonts``,
``test_hatching_legend``, ``test_grayscale_alpha``,
``test_pdf_eps_savefig_when_color_is_none``, ``test_kerning``,
``test_multi_font_type3``, ``test_multi_font_type42``,
``test_scatter_offaxis_colored_visual``, ``test_scatter_mixed_onoff_axis``,
``test_scatter_large_markers_partial_clip``, ``test_scatter_logscale``,
``test_scatter_polar`` and ``test_hexbin_negative_offsets``.

Left out
--------
``test_invalid_metadata`` (warnings for values plt-rs's typed
``Metadata`` cannot hold), ``test_pdfpages_fspath`` (Python path-like
objects), ``test_text_urls_tex``, ``test_failing_latex``,
``test_font_heuristica``, ``test_font_dejavusans`` and
``test_font_bitstream_charter`` (usetex), ``test_ttc_type3``,
``test_ttc_type42`` and ``test_otf_font_smoke`` (fonts that are not
bundled) and ``test_truetype_conversion`` (``mpltest.ttf`` from
matplotlib's test data).

The random draws of ``test_scatter_offaxis_colored_pdf_size`` come from
``spec_support.rng(19680801)``.
"""

import datetime
import io
import os
import re
import tempfile
import zlib
from pathlib import Path

import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import font_manager as fm
from matplotlib.backends._backend_pdf_ps import font_as_file, get_glyphs_subset
from matplotlib.backends.backend_pdf import PdfPages
from matplotlib.cbook import _get_data_path
from matplotlib.ft2font import FT2Font

from probes._mpl_tests import prober
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_backend_pdf")
probe = prober(probes)


def _pdf(fig, **options):
    buffer = io.BytesIO()
    fig.savefig(buffer, format="pdf", **options)
    return buffer.getvalue()


# --- the multi-page tests --------------------------------------------------

def _pagecounts():
    with PdfPages(io.BytesIO()) as pdf:
        counts = [pdf.get_pagecount()]
        fig, ax = plt.subplots()
        ax.plot([1, 2, 3])
        fig.savefig(pdf, format="pdf")
        counts.append(pdf.get_pagecount())
        pdf.savefig()
        counts.append(pdf.get_pagecount())
    return counts


def test_multipage_pagecount():
    probe("PdfPages.get_pagecount", _pagecounts)


def _finalized():
    buffer = io.BytesIO()
    with PdfPages(buffer) as pdf:
        for _ in range(10):
            fig, ax = plt.subplots()
            ax.set_title("This is a long title")
            fig.savefig(pdf, format="pdf")
    document = buffer.getvalue()
    return {"startxref": document.count(b"startxref"), "small": len(document) < 40000}


def test_multipage_properfinalize():
    probe("PdfPages.close", _finalized)


def _kept(pages):
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "a.pdf"
        with PdfPages(path) as pdf:
            for _ in range(pages):
                pdf.savefig(plt.figure())
        return path.exists()


def test_multipage_keep_empty():
    for pages in (0, 1):
        probe("PdfPages.__exit__", _kept, pages=pages)


# --- test_composite_image --------------------------------------------------

def _image_count():
    _, Y = np.meshgrid(np.arange(-5, 5, 1), np.arange(-5, 5, 1))
    Z = np.sin(Y ** 2)
    fig, ax = plt.subplots()
    ax.set_xlim(0, 3)
    ax.imshow(Z, extent=[0, 1, 0, 1])
    ax.imshow(Z[::-1], extent=[2, 3, 0, 1])
    with PdfPages(io.BytesIO()) as pdf:
        fig.savefig(pdf, format="pdf")
        return len(pdf._file._images)


def test_composite_image():
    for composite in (True, False):
        probe("Axes.imshow", _image_count, rc={"image.composite_image": composite})


# --- test_indexed_image ----------------------------------------------------

def _literal(document, start):
    """The bytes of the PDF literal string opening at ``start``."""
    escapes = {ord("n"): b"\n", ord("r"): b"\r", ord("t"): b"\t", ord("b"): b"\b",
               ord("f"): b"\f"}
    out, depth, i = bytearray(), 1, start + 1
    while True:
        c = document[i]
        if c == ord("\\"):
            i += 1
            out += escapes.get(document[i], bytes([document[i]]))
        elif c == ord("("):
            depth += 1
            out.append(c)
        elif c == ord(")"):
            depth -= 1
            if depth == 0:
                return bytes(out)
            out.append(c)
        else:
            out.append(c)
        i += 1


def _unfilter(data, columns):
    """Undoes the PNG predictors of one-byte samples, ``columns`` a row."""
    rows, previous = [], bytes(columns)
    for start in range(0, len(data), columns + 1):
        kind, raw = data[start], bytearray(data[start + 1:start + 1 + columns])
        for k in range(columns):
            left = raw[k - 1] if k else 0
            up, corner = previous[k], previous[k - 1] if k else 0
            if kind == 1:
                raw[k] = (raw[k] + left) & 255
            elif kind == 2:
                raw[k] = (raw[k] + up) & 255
            elif kind == 3:
                raw[k] = (raw[k] + (left + up) // 2) & 255
            elif kind == 4:
                p = left + up - corner
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - corner)
                pred = left if pa <= pb and pa <= pc else up if pb <= pc else corner
                raw[k] = (raw[k] + pred) & 255
        rows.append(bytes(raw))
        previous = raw
    return b"".join(rows)


def _indexed_image():
    data = np.zeros((256, 1, 3), dtype=np.uint8)
    data[:, 0, 0] = np.arange(256)
    fig = plt.figure()
    fig.figimage(data, resize=True)
    document = _pdf(fig, dpi="figure")
    head = re.search(rb"/Subtype /Image /Width (\d+) /Height (\d+)\s*/ColorSpace ", document)
    rest = document[head.end():]
    indexed = re.match(rb"\[ /Indexed /DeviceRGB (\d+)\s*\(", rest)
    if indexed is None:
        return {"indexed": False, "colors": None, "pixels": None}
    palette = _literal(rest, indexed.end() - 1)
    stream = rest[rest.index(b"stream\n") + 7:]
    width, height = int(head.group(1)), int(head.group(2))
    samples = _unfilter(zlib.decompressobj().decompress(stream), width)
    colors = np.frombuffer(palette, np.uint8).reshape(-1, 3)
    pixels = colors[np.frombuffer(samples, np.uint8)].reshape(height, width, 3)
    return {"indexed": True, "colors": int(indexed.group(1)) + 1,
            "pixels": bool(np.array_equal(pixels, data))}


def test_indexed_image():
    probe("Figure.figimage", _indexed_image, rc={"pdf.compression": 1})


# --- the metadata tests ----------------------------------------------------

def _decode(metadata):
    def value(v):
        if isinstance(v, dict):
            return datetime.datetime.fromisoformat(v["datetime"])
        return v
    return {key: value(v) for key, v in metadata.items()}


def _info(document):
    number = re.search(rb"/Info (\d+) 0 R", document).group(1)
    start = document.index(b"\n" + number + b" 0 obj\n") + len(number) + 7
    body = document[start:document.index(b"endobj", start)]
    info, i = {}, 0
    defaults = {f"Matplotlib v{mpl.__version__}, https://matplotlib.org",
                f"Matplotlib pdf backend v{mpl.__version__}"}
    for key in re.finditer(rb"/(\w+) ", body):
        if key.start() < i:
            continue
        i = key.end()
        if body[i:i + 1] == b"(":
            text = _literal(body, i).decode("latin-1")
            info[key.group(1).decode()] = "<default>" if text in defaults else text
            i = body.index(b")", i) + 1
        else:
            name = re.match(rb"/\w+", body[i:]).group(0).decode()
            info[key.group(1).decode()] = name
            i += len(name)
    return info


def _metadata(metadata, pages):
    previous = os.environ.get("SOURCE_DATE_EPOCH")
    os.environ["SOURCE_DATE_EPOCH"] = "0"
    try:
        fig, ax = plt.subplots()
        ax.plot(range(5))
        if pages is None:
            document = _pdf(fig, metadata=_decode(metadata))
        else:
            buffer = io.BytesIO()
            with PdfPages(buffer, metadata=_decode(metadata)) as pdf:
                for _ in range(pages):
                    pdf.savefig(fig)
            document = buffer.getvalue()
    finally:
        if previous is None:
            del os.environ["SOURCE_DATE_EPOCH"]
        else:
            os.environ["SOURCE_DATE_EPOCH"] = previous
    return _info(document)


METADATA = {
    "Author": "me",
    "Title": "Multipage PDF",
    "Subject": "Test page",
    "Keywords": "test,pdf,multipage",
    "ModDate": {"datetime": "1968-08-01T00:00:00+00:00"},
    "Trapped": "True",
}


def test_savefig_metadata():
    probe("savefig.pdf.metadata", _metadata, metadata=METADATA, pages=None,
          rc={"pdf.compression": 0})


def test_multipage_metadata():
    probe("savefig.pdf.metadata", _metadata, metadata=METADATA, pages=2,
          rc={"pdf.compression": 0})


# --- the link tests --------------------------------------------------------

LINK = re.compile(
    rb"<<\s*/Type\s*/Annot\s*/Subtype\s*/Link\s*/Rect\s*\[([^\]]*)\]\s*/Border\s*\[[^\]]*\]"
    rb"\s*/A\s*<<\s*/S\s*/URI\s*/URI\s*\((.*?)\)\s*>>(?:\s*/QuadPoints\s*\[([^\]]*)\])?\s*>>")


def _links(figsize, texts):
    fig = plt.figure(figsize=figsize)
    for x, y, s, rotation, url in texts:
        fig.text(x, y, s, rotation=rotation, url=url)
    document = _pdf(fig)

    def numbers(text):
        return None if text is None else [float(v) for v in text.split()]

    return [[m.group(2).decode(), numbers(m.group(1)), numbers(m.group(3))]
            for m in LINK.finditer(document)]


URL = "https://test_text_urls.matplotlib.org/"


def test_text_urls():
    probe("Text.set_url", _links, figsize=[2, 1],
          texts=[[0.1, 0.1, "test plain 123", 0, f"{URL}plain"],
                 [0.1, 0.4, "test mathtext $123$", 0, f"{URL}mathtext"]],
          rc={"pdf.compression": 0})


def test_text_rotated_urls():
    probe("Text.set_url", _links, figsize=[1, 1], texts=[[0.1, 0.1, "N", 45, URL]],
          rc={"pdf.compression": 0})


# --- test_empty_rasterized -------------------------------------------------

def _empty_rasterized():
    fig, ax = plt.subplots()
    ax.plot([], [], rasterized=True)
    return _pdf(fig).startswith(b"%PDF")


def test_empty_rasterized():
    probe("Line2D.set_rasterized", _empty_rasterized)


# --- test_glyphs_subset ----------------------------------------------------

def _subset(font, chars):
    path = str(_get_data_path(f"fonts/ttf/{font}"))
    whole = FT2Font(path)
    whole.set_text(chars)
    cmap = whole.get_charmap()
    glyphs = {cmap[ord(c)] for c in chars}
    with get_glyphs_subset(fm.FontPath(path, 0), glyphs) as subset:
        part = FT2Font(font_as_file(subset.font))
    part.set_text(chars)
    subcmap = part.get_charmap()
    return {"chars": "".join(sorted(chr(key) for key in subcmap)),
            "fewer": len(subcmap) < len(cmap),
            "glyphs": part.get_num_glyphs() == whole.get_num_glyphs()}


def test_glyphs_subset():
    probe("get_glyphs_subset", _subset, font="DejaVuSerif.ttf",
          chars="these should be subsetted! 1234567890")


# --- test_scatter_offaxis_colored_pdf_size ---------------------------------

def _scatter_sizes():
    r = rng(19680801)
    x = r.uniform(1000) * 10
    y = r.uniform(1000) * 10
    c = r.uniform(1000)

    def size(xs, ys, cs):
        fig, ax = plt.subplots()
        ax.scatter(xs, ys, c=cs)
        ax.set_xlim(20, 30)
        ax.set_ylim(20, 30)
        return len(_pdf(fig))

    offaxis, empty, visible = size(x, y, c), size([], [], None), size(x + 20, y + 20, c)
    return {"offaxis": offaxis < empty + 5_000, "visible": visible > empty + 15_000,
            "visible_vs_offaxis": visible > offaxis + 15_000}


def test_scatter_offaxis_colored_pdf_size():
    probe("Axes.scatter", _scatter_sizes)
