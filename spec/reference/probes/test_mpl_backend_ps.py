"""matplotlib's own tests of its PostScript backend
(``tests/test_backend_ps.py``) that read the document: paper sizes and
orientations, the EPS bounding box, the embedded font type, definitions
made once, documents that only have to save, and the paper type
``auto`` that no longer exists.

Area ``mpl_backend_ps``. Every op runs in the test suite's style (see
``_mpl_tests.py``).

Ops
---
``Figure.savefig``  {format, orientation, papertype, log} -> ``{"header",
    "placement"}`` of ``test_savefig_to_stringio``'s figure (a line titled
    ``"Déjà vu −€"``, on a log y axis when ``log``) saved as ``format``
    (``ps`` or ``eps``) with ``orientation`` and ``papertype``: the DSC
    comments before ``%%EndComments`` but the creator, date and title, and
    the commands placing the figure on the page (from ``mpldict begin``
    after the prolog to ``rectclip``). The test's ``eps afm`` case sets ``ps.useafm``.
``Figure.savefig.distilled``  {format, orientation, papertype, rc} ->
    ``{"header", "pages"}``: the distiller and usetex cases of
    ``test_savefig_to_stringio`` (the title without its minus and euro sign
    with usetex), whose documents Ghostscript or ``pdftops`` writes: the
    DSC comments before ``%%EndComments`` but the creator, date and title
    (Ghostscript records the paper options it was run with), and every
    comment line starting ``%%Page`` (the page's size, and the page
    ``pstoeps`` makes of EPS output). Skipped where the programs are
    missing.
``savefig.eps.bbox``  {} -> ``{"bbox", "hires"}``: the numbers of the
    ``%%BoundingBox`` and ``%%HiResBoundingBox`` comments of an EPS
    ``plt.subplots()`` figure (``test_bbox``).
``rcParams.ps.fonttype``  {} -> whether the PostScript of a text holds
    ``/FontType <ps.fonttype> def`` (``test_fonttype``).
``Line2D.set_linestyle``  {} -> whether the PostScript of a dashed line is
    not empty (``test_linedash``).
``Figure.text``  {} -> whether a text of empty lines (``"\\nfoo\\n\\n"``)
    saves as EPS and PostScript (``test_empty_line``).
``Figure.suptitle``  {} -> the most times one name is defined (a line
    starting with ``/name``) in the EPS of sixteen polar axes and a
    suptitle (``test_no_duplicate_definition``).
``Figure.savefig.papertype``  {papertype} -> whether an EPS saves with
    ``papertype`` (``test_auto_papersize_removal``: ``auto`` is refused).
``rcParams.ps.papersize``  {value} -> the value ``ps.papersize`` takes
    (``test_auto_papersize_removal``: ``auto`` is refused).

Already covered
---------------
By the ``mpl_rendering`` figure group: ``test_patheffects``. By the
``mpl_backends`` figure group: ``test_transparency``, ``test_useafm``,
``test_colored_hatch_zero_linewidth``, ``test_text_clip``,
``test_type42_font_without_prep``, ``test_multi_font_type3``,
``test_multi_font_type42``, ``test_path_collection`` and
``test_colorbar_shift``.

By plt's unit tests of its PostScript output (``figure/postscript``):
``test_tilde_in_tempfilename``, ``test_transparency_tex``,
``test_failing_latex``, ``test_partial_usetex``, ``test_usetex_preamble``
and ``test_d_glyph``, and the distiller and usetex cases of
``test_savefig_to_stringio`` with matplotlib's own assertions.

Left out
--------
The comparison of text and binary streams of ``test_savefig_to_stringio``
(Python file objects), and ``test_ttc_type3`` and ``test_ttc_type42`` (the
WenQuanYi Zen Hei font is not bundled).
"""

import io
import re
from collections import Counter

import matplotlib as mpl
import matplotlib.pyplot as plt
from matplotlib.figure import Figure

from probes._mpl_tests import prober
from spec_support import ProbeSet
from spec_support.tex import has_ghostscript, has_usetex_ps, has_xpdf

probes = ProbeSet("mpl_backend_ps")
probe = prober(probes)

#: DSC comments that name the library, the date or the file.
UNSTABLE = ("%%Creator", "%%CreationDate", "%%Title")


def _saved(format, orientation, papertype, log):
    fig, ax = plt.subplots()
    if log:
        ax.set_yscale("log")
    ax.plot([1, 2], [1, 2])
    ax.set_title("Déjà vu \N{MINUS SIGN}\N{EURO SIGN}")
    buffer = io.BytesIO()
    fig.savefig(buffer, format=format, orientation=orientation, papertype=papertype)
    document = buffer.getvalue().decode("latin-1")
    head = document[:document.index("%%EndComments")]
    body = document[document.index("%%EndProlog"):]
    body = body[body.index("mpldict begin"):]
    body = body[:body.index("rectclip")]
    return {
        "header": [line for line in head.splitlines() if not line.startswith(UNSTABLE)],
        "placement": body.splitlines()[1:-1],
    }


CASES = [("ps", False, {}), ("eps", False, {}), ("eps", True, {"ps.useafm": True})]


def test_savefig_to_stringio():
    for format, log, rc in CASES:
        for orientation in ("portrait", "landscape"):
            for papertype in ("letter", "figure"):
                probe("Figure.savefig", _saved, format=format, orientation=orientation,
                      papertype=papertype, log=log, rc=rc)


#: The distiller and usetex cases of ``test_savefig_to_stringio``, and
#: whether their programs can run here.
DISTILLED = [
    ("ps", {"ps.usedistiller": "ghostscript"}, has_ghostscript),
    ("ps", {"ps.usedistiller": "xpdf"}, has_xpdf),
    ("ps", {"text.usetex": True}, has_usetex_ps),
    ("eps", {"text.usetex": True}, has_usetex_ps),
]


def _distilled(format, orientation, papertype):
    fig, ax = plt.subplots()
    ax.plot([1, 2], [1, 2])
    title = "Déjà vu"
    if not mpl.rcParams["text.usetex"]:
        title += " \N{MINUS SIGN}\N{EURO SIGN}"
    ax.set_title(title)
    buffer = io.BytesIO()
    fig.savefig(buffer, format=format, orientation=orientation, papertype=papertype)
    document = buffer.getvalue().decode("latin-1")
    head = document[:document.index("%%EndComments")]
    return {
        "header": [line for line in head.splitlines() if not line.startswith(UNSTABLE)],
        "pages": [line for line in document.splitlines() if line.startswith("%%Page")],
    }


def test_savefig_to_stringio_distilled():
    for format, rc, available in DISTILLED:
        if not available():
            continue
        for orientation in ("portrait", "landscape"):
            for papertype in ("letter", "figure"):
                probe("Figure.savefig.distilled", _distilled, format=format,
                      orientation=orientation, papertype=papertype, rc=rc)


def _bbox():
    fig, _ = plt.subplots()
    buffer = io.BytesIO()
    fig.savefig(buffer, format="eps")
    document = buffer.getvalue()
    bbox = re.search(rb"^%%BoundingBox: (.+) (.+) (.+) (.+)$", document, re.MULTILINE)
    hires = re.search(rb"^%%HiResBoundingBox: (.+) (.+) (.+) (.+)$", document, re.MULTILINE)
    return {"bbox": [int(v) for v in bbox.groups()], "hires": [float(v) for v in hires.groups()]}


def test_bbox():
    probe("savefig.eps.bbox", _bbox)


def _fonttype():
    fig, ax = plt.subplots()
    ax.text(0.25, 0.5, "Forty-two is the answer to everything!")
    buffer = io.BytesIO()
    fig.savefig(buffer, format="ps")
    wanted = f"/FontType {mpl.rcParams['ps.fonttype']} def".encode()
    return re.search(wanted, buffer.getvalue(), re.MULTILINE) is not None


def test_fonttype():
    for fonttype in ("3", "42"):
        probe("rcParams.ps.fonttype", _fonttype, rc={"ps.fonttype": fonttype})


def _dashed():
    fig, ax = plt.subplots()
    ax.plot([0, 1], linestyle="--")
    buffer = io.BytesIO()
    fig.savefig(buffer, format="ps")
    return buffer.tell() > 0


def test_linedash():
    probe("Line2D.set_linestyle", _dashed)


def _empty_lines():
    figure = Figure()
    figure.text(0.5, 0.5, "\nfoo\n\n")
    buffer = io.BytesIO()
    figure.savefig(buffer, format="eps")
    figure.savefig(buffer, format="ps")
    return buffer.tell() > 0


def test_empty_line():
    probe("Figure.text", _empty_lines)


def _most_definitions():
    fig = Figure()
    axs = fig.subplots(4, 4, subplot_kw=dict(projection="polar"))
    for ax in axs.flat:
        ax.set(xticks=[], yticks=[])
        ax.plot([1, 2])
    fig.suptitle("hello, world")
    buffer = io.StringIO()
    fig.savefig(buffer, format="eps")
    buffer.seek(0)
    words = [line.partition(" ")[0] for line in buffer.readlines() if line.startswith("/")]
    return max(Counter(words).values())


def test_no_duplicate_definition():
    probe("Figure.suptitle", _most_definitions)


def _papertype(papertype):
    fig = plt.figure()
    fig.savefig(io.BytesIO(), format="eps", papertype=papertype)
    return True


def _papersize(value):
    mpl.rcParams["ps.papersize"] = value
    return mpl.rcParams["ps.papersize"]


def test_auto_papersize_removal():
    for papertype in ("auto", "a4"):
        probe("Figure.savefig.papertype", _papertype, papertype=papertype)
    for value in ("auto", "A4"):
        probe("rcParams.ps.papersize", _papersize, value=value)
