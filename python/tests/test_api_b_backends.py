"""
Smoke tests of the public names pltrs (Python) adds to the backends:
the objects of the PDF writer (`PdfFile`, `pdfRepr`, `Name`, `Op`,
`Reference`, `Stream`, `Verbatim`, `GraphicsContextPdf`), the PostScript
distillers, the SVG XML writer and the PGF helpers. The expected values
were recorded from matplotlib 3.11.2 running the same code.
"""

import datetime
import io
import shutil
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager as fm
from pltrs.backends import backend_pdf as bp, backend_pgf, backend_ps, backend_svg
from pltrs.path import Path
from pltrs.testing import _has_tex_package
from pltrs.transforms import Affine2D, Bbox

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

SQUARE = Path([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]], [1, 2, 2, 2, 79])


@pytest.mark.parametrize("value, expected", [
    (1.5, b"1.5"), (1e-11, b"0"), (3, b"3"), (True, b"true"), (None, b"null"),
    ("ab(c)", b"(ab\\(c\\))"), ("é", b"(\xfe\xff\x00\xe9)"),
    (b"x\\y\n", b"(x\\\\y\\n)"), ([1, 2.5, "x"], b"[ 1 2.5 (x) ]"),
    ({"Type": bp.Name("Page"), "Count": 2}, b"<< /Type /Page /Count 2 >>"),
    (bp.Name("a b#"), b"/a#20b#"), (bp.Reference(7), b"7 0 R"),
    (bp.Verbatim(b"1 0 0 RG"), b"1 0 0 RG"), (bp.Op.fill_stroke, b"B"),
    (datetime.datetime(2020, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc),
     b"(D:20200102030405Z)"),
    (Bbox([[0, 1], [2, 4]]), b"0 1 2 3"), (np.float64(0.25), b"0.25"),
    (np.int32(4), b"4"), ((1, (2, 3)), b"[ 1 [ 2 3 ] ]"),
    (list(range(40)), b"[ 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24"
                      b" 25 26\n27 28 29 30 31 32 33 34 35 36 37 38 39 ]"),
])
def test_pdf_repr(value, expected):
    assert bp.pdfRepr(value) == expected


def test_pdf_repr_errors_and_objects():
    with pytest.raises(ValueError, match="Can only output finite numbers in PDF"):
        bp.pdfRepr(float("nan"))
    with pytest.raises(TypeError, match="Don't know a PDF representation"):
        bp.pdfRepr(object())
    assert repr(bp.Name("a b")) == "<Name b'a#20b'>"
    assert str(bp.Name(b"x")) == "/x"
    assert bp.Name("a") < bp.Name("b")
    assert bp.Name("a") == bp.Name(bp.Name("a"))
    assert hash(bp.Name("q")) == hash(bp.Name("q"))
    assert repr(bp.Reference(3)) == "<Reference 3>"
    assert [bp.Op.paint_path(f, s) for f, s in
            [(True, True), (False, True), (True, False), (False, False)]] == [
        bp.Op.fill_stroke, bp.Op.stroke, bp.Op.fill, bp.Op.endpath]
    assert bp.Op.shading.value == b"sh"
    assert bp.UTC == datetime.timezone.utc
    assert bp.Stream.__slots__ == (
        "id", "len", "pdfFile", "file", "compressobj", "extra", "pos")


def _document():
    out = io.BytesIO()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        pdf = bp.PdfFile(out, metadata={"Title": "T", "CreationDate": None,
                                         "Creator": "c", "Producer": "p"})
    return out, pdf


def test_pdf_file_resources():
    pltrs.rcParams["pdf.compression"] = 0
    out, pdf = _document()
    assert pdf.infoDict == {"Creator": "c", "Producer": "p", "Title": "T"}
    ref = pdf.reserveObject("x")
    assert repr(ref) == "<Reference 9>"
    pdf.writeObject(ref, {"Kids": [bp.Reference(1)], "N": None})
    assert [pdf.alphaState(a) for a in [(0.5, 0.5), (0.5, 0.5), (1, 0.25)]] == [
        bp.Name("A1"), bp.Name("A1"), bp.Name("A2")]
    assert [pdf.hatchPattern(h) for h in [((0, 0, 0, 1), None, "/", 1.0),
                                          ((0, 0, 0, 1), (1, 0, 0), "x", 2.0),
                                          ((0, 0, 0, 1), None, "/", 1.0)]] == [
        bp.Name("H1"), bp.Name("H2"), bp.Name("H1")]
    points = np.array([[[0, 0], [10, 0], [0, 10]]], float)
    name, ob = pdf.addGouraudTriangles(points, np.array([[[0.0], [0.5], [1.0]]]))
    assert (name, repr(ob)) == (bp.Name("GT0"), "<Reference 10>")
    assert pdf.addGouraudTriangles(points, np.ones((1, 3, 4)))[0] == bp.Name("GT1")
    # Every markerObject call makes its own form, as in matplotlib.
    assert [pdf.markerObject(SQUARE, Affine2D().scale(s), True, f, lw, j, "butt")
            for s, f, lw, j in [(3, True, 1.0, "miter"), (3, True, 2.0, "miter"),
                                (4, False, 1.0, "round")]] == [
        bp.Name("M0"), bp.Name("M1"), bp.Name("M2")]
    gc = bp.GraphicsContextPdf(pdf)
    assert pdf.pathCollectionObject(gc, SQUARE, Affine2D(), 1.0, True, True) == bp.Name("P0")
    image = np.zeros((2, 3, 4), np.uint8)
    image[..., 3] = 255
    image[0, 0] = (255, 0, 0, 128)
    assert [pdf.imageObject(image), pdf.imageObject(image),
            pdf.imageObject(image.copy())] == [bp.Name("I1"), bp.Name("I1"), bp.Name("I2")]
    assert [v.pdfRepr() for v in bp.PdfFile.pathOperations(SQUARE, Affine2D().scale(2))] == [
        b"0 0 m\n2 0 l\n2 2 l\n0 2 l\nh\n"]
    pdf.newPage(2, 1)
    assert pdf.currentstream is not None
    pdf.output(1, 0, 0, bp.Op.setrgb_stroke)
    pdf.writePath(SQUARE, Affine2D().scale(10), clip=True)
    pdf.output(bp.Op.stroke)
    pdf.newTextnote("note")
    pdf.endStream()
    assert pdf.currentstream is None
    stream = pdf.reserveObject()
    pdf.beginStream(stream.id, None, {"Type": bp.Name("XObject")})
    with pytest.raises(AssertionError):
        pdf.beginStream(stream.id, None)
    pdf.write(b"0 0 m\n")
    pdf.endStream()
    pdf.outputStream(pdf.reserveObject(), b"abc", extra={"Q": 1})
    pdf.finalize()
    pdf.close()
    data = out.getvalue()
    assert data[:15] == b"%PDF-1.4\n%\xac\xdc \xab\xba"
    assert data[data.rindex(b"trailer"):].split(b"startxref")[0] == (
        b"trailer\n<< /Size 33 /Root 1 0 R /Info 32 0 R >>\n")
    for needle in [b"/A1 << /Type /ExtGState /CA 0.5 /ca 0.5 >>", b"/H1 ",
                   b"/ShadingType 4", b"/Kids [ 1 0 R ] /N null", b"/Q 1 /Length 3",
                   b"<< /Type /XObject /Length 6 >>", b"1 0 0 RG", b"/M2", b"/P0",
                   b"/I2", b"/Title (T)"]:
        assert needle in data


def test_pdf_file_hatch_patterns():
    _, pdf = _document()
    pdf.hatchPattern(((0, 0, 0, 1), None, "/", 1.0))
    # matplotlib 3.11's deprecated hatchPatterns raises TypeError here (it
    # unpacks the names); pltrs gives the mapping it was deprecated with.
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert pdf.hatchPatterns == {((0.0, 0.0, 0.0, 1.0), None, "/"): bp.Name("H1")}


def test_pdf_file_writes_its_parts_on_demand():
    pltrs.rcParams["pdf.compression"] = 0
    out, pdf = _document()
    for write in [pdf.writeFonts, pdf.writeExtGSTates, pdf.writeHatches,
                  pdf.writeGouraudTriangles, pdf.writeImages, pdf.writeMarkers,
                  pdf.writePathCollectionTemplates, pdf.writeInfoDict]:
        write()
    assert repr(pdf.infoObject) == "<Reference 9>"
    pdf.recordXref(2)
    pdf.write(b"2 0 obj\n<< >>\nendobj\n")
    with pytest.raises(AssertionError,
                       match=r"No offset for object 7 \(external objects\)"):
        pdf.writeXref()
    pdf.writeObject(bp.Reference(7), {})
    pdf.writeXref()
    pdf.writeTrailer()
    pdf.close()
    assert out.getvalue().endswith(b"%%EOF\n")


def test_pdf_file_fonts():
    _, pdf = _document()
    props = fm.FontProperties(family="DejaVu Sans")
    assert (pdf.fontName(props), pdf.fontName(props, 1)) == (bp.Name("F1"), bp.Name("F2"))
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        names = pdf.fontNames
    assert sorted((str(k[0]).rsplit("/", 1)[-1], k[1], v) for k, v in names.items()) == [
        ("DejaVuSans.ttf", 0, bp.Name("F1")), ("DejaVuSans.ttf", 1, bp.Name("F2"))]
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert pdf.multi_byte_charprocs == {}
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert pdf.type1Descriptors == {}


def test_pdf_file_embeds_truetype_subsets():
    pltrs.rcParams["pdf.compression"] = 0
    out, pdf = _document()
    # matplotlib embeds only characters it drew (it raises KeyError for
    # others); pltrs embeds any, without text for the ToUnicode map.
    path = fm.FontPath(fm.findfont("DejaVu Sans"), 0)
    assert isinstance(pdf.embedTTF(path, 0, {65: 36}), bp.Reference)
    pdf.close()
    assert b"/FirstChar 65 /LastChar 65" in out.getvalue()


@pytest.mark.skipif(not shutil.which("kpsewhich"), reason="TeX is not installed")
def test_pdf_file_tex_fonts():
    from pltrs import _type1font, dviread
    pltrs.rcParams["pdf.compression"] = 0
    out = io.BytesIO()
    pdf = bp.PdfFile(out)
    font = dviread.DviFont(scale=10 * 2**20,
                           metrics=dviread.Tfm(dviread.find_tex_file("cmr10.tfm")),
                           texname=b"cmr10", vf=None)
    assert pdf.dviFontName(font) == bp.Name("F-cmr10")
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        info = pdf.dviFontInfo
    entry = info[b"cmr10"]
    assert (entry.pdfname, entry.basefont, entry.encodingfile, entry.effects) == (
        bp.Name("F-cmr10"), b"CMR10", None, {})
    assert entry.fontfile.endswith("cmr10.pfb")
    t1font = _type1font.Type1Font(dviread.find_tex_file("cmr10.pfb"))
    descriptor = pdf.createType1Descriptor(t1font)
    assert repr(descriptor) == "<Reference 9>"
    pdf.close()
    data = out.getvalue()
    assert (b"9 0 obj\n<< /Type /FontDescriptor /FontName /CMR10 /Flags 4\n"
            b"/FontBBox [ -40 -250 1009 750 ] /ItalicAngle 0 /Ascent 750 /Descent -250\n"
            b"/CapHeight 1000 /XHeight 500 /FontFile 10 0 R /FontFamily (Computer Modern)\n"
            b"/StemV 50\n/CharSet (/A/AE/AE/B/C/D/Delta") in data
    assert b"10 0 obj\n<< /Length1 4287 /Length2 30900 /Length3 0 /Length 35187 >>" in data


def test_graphics_context_pdf():
    _, pdf = _document()
    pdf.newPage(2, 1)
    gc = bp.GraphicsContextPdf(pdf)
    gc.set_linewidth(3)
    other = bp.GraphicsContextPdf(pdf)
    other.set_linewidth(2)
    other.set_foreground((0.2, 0.4, 0.6))
    other._fillcolor = (0.5, 0.5, 0.5)
    other._effective_alphas = (0.5, 0.5)
    other.set_dashes(0, [3, 1])
    other.set_clip_rectangle(Bbox([[0, 0], [10, 10]]))
    assert [bp.pdfRepr(c) for c in gc.delta(other)] == [
        b"q", b"0 0 10 10", b"re", b"W", b"n", b"/A1", b"gs", b"0.5", b"g", b"2", b"w",
        b"[ 3 1 ]", b"0", b"d", b"0.2", b"0.4", b"0.6", b"RG", b"0.5", b"g"]
    assert (gc.stroke(), gc.fill(), gc.fill((1, 1, 1, 0)), gc.paint()) == (
        True, True, False, bp.Op.fill_stroke)
    assert gc.capstyle_cmd("round") == [1, bp.Op.setlinecap]
    assert gc.joinstyle_cmd("bevel") == [2, bp.Op.setlinejoin]
    assert [bp.pdfRepr(c) for c in gc.finalize()] == [b"Q"]
    assert "file" not in repr(gc)


def test_pdf_pages_infodict():
    out = io.BytesIO()
    with bp.PdfPages(out) as pages:
        info = pages.infodict()
        assert sorted(info) == ["CreationDate", "Creator", "Producer"]
        info["Title"] = "Changed"
        info["Author"] = "Me"
        pages.savefig(plt.figure(figsize=(1, 1)))
    assert b"/Title (Changed)" in out.getvalue()
    assert b"/Author (Me)" in out.getvalue()


def test_svg_writer_and_constants():
    buf = io.StringIO()
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        writer = backend_svg.XMLWriter(buf)
    svg = writer.start("svg", {"width": "10pt"}, id="a", empty="")
    writer.comment("x--y")
    writer.element("title", "a<b")
    writer.start("g")
    writer.data("t&t")
    writer.end("g")
    writer.close(svg)
    assert buf.getvalue() == (
        '<svg width="10pt" id="a">\n <!-- x- -y -->\n <title>a&lt;b</title>\n'
        ' <g>\nt&amp;t </g>\n</svg>\n')
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        other = backend_svg.XMLWriter(io.StringIO())
    other.start("a")
    with pytest.raises(AssertionError, match=r"expected end\(a\), got b"):
        other.end("b")
    assert backend_svg.svgProlog == (
        '<?xml version="1.0" encoding="utf-8" standalone="no"?>\n'
        '<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN"\n'
        '  "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">\n')
    assert backend_svg.UTC == datetime.timezone.utc


def test_ps_helpers(tmp_path):
    assert len(backend_ps.uni2type1) == 217
    assert backend_ps.uni2type1[229] == "aring"
    assert backend_ps.debugPS is False
    fig = plt.figure(figsize=(2, 2))
    fig.add_subplot().plot([0, 1])
    ps = str(tmp_path / "figure.ps")
    fig.savefig(ps, format="ps")
    backend_ps.pstoeps(ps, bbox=(0, 0, 144, 144))
    head = open(ps, "rb").read().split(b"\n")[:2]
    assert head == [b"%!PS-Adobe-3.0 EPSF-3.0", b"%%BoundingBox: 0 0 144 144"]
    if shutil.which("gs"):
        fig.savefig(ps, format="ps")
        backend_ps.gs_distill(ps, eps=True, bbox=(0, 0, 144, 144))
        assert open(ps, "rb").read().split(b"\n")[0] == b"%!PS-Adobe-3.0 EPSF-3.0"
    if shutil.which("ps2pdf") and shutil.which("pdftops"):
        fig.savefig(ps, format="ps")
        backend_ps.xpdf_distill(ps, eps=False, ptype="letter")
        assert open(ps, "rb").read()[:10] == b"%!PS-Adobe"


def test_pgf_helpers():
    assert backend_pgf.latex_pt_to_in == pytest.approx(0.013837000138370002)
    assert backend_pgf.latex_in_to_pt == 72.27
    assert backend_pgf.mpl_pt_to_in == pytest.approx(1 / 72)
    assert backend_pgf.mpl_in_to_pt == 72.0
    error = backend_pgf.LatexError("bad", "output")
    assert (str(error), error.latex_output) == ("bad\noutput", "output")
    assert callable(backend_pgf.FigureCanvasPgf.print_pdf)
    assert callable(backend_pgf.FigureCanvasPgf.print_png)
    if shutil.which("pdftocairo") or shutil.which("gs"):
        assert callable(backend_pgf.make_pdf_to_png_converter())


@pytest.mark.skipif(not shutil.which("xelatex"), reason="XeLaTeX is not installed")
def test_pgf_latex_manager():
    manager = backend_pgf.LatexManager._get_cached_or_new()
    assert manager is backend_pgf.LatexManager._get_cached_or_new()
    assert [round(v, 3) for v in manager.get_width_height_descent(
        "Hg", fm.FontProperties(size=12))] == [16.641, 11.244, 2.496]


@pytest.mark.skipif(not _has_tex_package("pgf"), reason="pgf.sty is not installed")
def test_pgf_print_pdf_and_png(tmp_path):
    fig = plt.figure(figsize=(2, 2))
    fig.add_subplot().plot([0, 1])
    fig.savefig(tmp_path / "figure.pdf", backend="pgf")
    assert (tmp_path / "figure.pdf").read_bytes()[:5] == b"%PDF-"
    fig.canvas.__class__ = backend_pgf.FigureCanvasPgf
    fig.canvas.print_png(tmp_path / "figure.png")
    assert (tmp_path / "figure.png").read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"
