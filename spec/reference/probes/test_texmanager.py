r"""matplotlib's TeX support without TeX: the LaTeX documents ``TexManager``
writes (after ``tests/test_texmanager.py``: ``test_font_selection`` is
``FONT_RCS``, ``test_fontconfig_preamble`` its ``text.latex.preamble``
entries) and the cache names it gives
them, and ``dviread``'s readers of DVI, VF and TFM files, font maps
(after ``tests/test_dviread.py::test_PsfontsMap``) and encodings, fed files
built here.

Area ``texmanager``. ``rc`` holds rcParams applied around an operation.
Byte strings are hexadecimal. A DVI, VF or TFM probe finds the font files
it names in ``tfm`` and ``vf`` (``{name: hex}``) instead of the texmf tree.

Ops
---
``TexManager._get_tex_source``  {tex, fontsize, rc} -> the document.
``TexManager._get_base_path``  {tex, fontsize, dpi, rc} -> the file name
    (the SHA-256 the cache path ends with).
``dviread.PsfontsMap``  {texname} -> ``{"texname", "psname", "effects",
    "encoding", "filename"}`` of matplotlib's ``test.map`` (copied to
    ``crates/pltrs-text/tests/fixtures/tex``) with ``find_tex_file`` giving
    every name back, as ``test_PsfontsMap`` sets it up; or the error.
``dviread._parse_enc``  {text} -> the glyph names; or the error.
``dviread.Tfm``  {data} -> ``{"checksum", "design_size", "metrics":
    [[code, width, height, depth], ...]}``; or the error.
``dviread.Vf``  {data, tfm, vf} -> ``{"packets": [[code, width, glyphs,
    rules], ...]}`` (glyphs as ``[x, y, texname, scale, code]``, rules as
    ``[x, y, height, width]``); or the error.
``dviread.Dvi``  {data, dpi, tfm, vf} -> the pages: ``{"text": [[x, y,
    texname, size, code, width], ...], "boxes": [[x, y, height, width],
    ...], "width", "height", "descent"}`` (TeX's units when ``dpi`` is
    ``None``); or the error.

Left out
--------
``test_dviread`` and ``test_dviread_pk`` need TeX (the ``usetex`` area
reads real DVI files). XeTeX and LuaTeX native fonts, which ``latex`` never
writes, are not supported by pltrs.
"""

import struct
import tempfile
from pathlib import Path

import matplotlib as mpl
from matplotlib import dviread as dr
from matplotlib.texmanager import TexManager

from spec_support import ProbeSet

probes = ProbeSet("texmanager")

FIXTURES = Path(__file__).resolve().parents[3] / "crates/pltrs-text/tests/fixtures/tex"


def _source(tex, fontsize, rc):
    with mpl.rc_context(rc):
        return TexManager._get_tex_source(tex, fontsize)


def _base_name(tex, fontsize, dpi, rc):
    with mpl.rc_context(rc):
        return TexManager._get_base_path(tex, fontsize, dpi).name


FONT_RCS = [
    {"font.family": "sans-serif", "font.sans-serif": "helvetica"},
    {"font.family": "serif", "font.serif": "palatino"},
    {"font.family": "cursive", "font.cursive": "zapf chancery"},
    {"font.family": "monospace", "font.monospace": "courier"},
    {"font.family": "helvetica"},
    {"font.family": "palatino"},
    {"font.family": "zapf chancery"},
    {"font.family": "courier"},
    {"font.family": "DejaVu Sans"},
    {"font.family": "serif"},
    {"font.family": "sans-serif"},
    {"font.family": "monospace"},
    {"font.family": "cursive"},
    {"font.family": ["serif", "sans-serif"]},
    {"font.family": "Times"},
    {"font.family": "computer modern sans serif"},
    {"font.family": "serif", "font.serif": ["Unknown Font", "Charter", "Times"]},
    {"font.family": "sans-serif", "font.sans-serif": ["Unknown Font"]},
    {"font.family": "serif", "text.latex.preamble": r"\usepackage{txfonts}"},
    {"text.latex.preamble": r"\usepackage{underscore}\usepackage[full]{textcomp}"},
]


def test_tex_sources():
    for rc in FONT_RCS:
        probes.probe("TexManager._get_tex_source", _source, tex="hello, world", fontsize=12.0,
                     rc=rc)
    for tex, fontsize in [("", 12.0), (r"$\frac{1}{2}$", 10.0), ("x", 100.0), ("x", 7.25)]:
        probes.probe("TexManager._get_tex_source", _source, tex=tex, fontsize=fontsize, rc={})
    for tex, fontsize, dpi in [("x", 10.0, None), ("x", 10.0, 100.0), ("x", 12.0, 72.0),
                               (r"\LaTeX", 10.0, None)]:
        probes.probe("TexManager._get_base_path", _base_name, tex=tex, fontsize=fontsize,
                     dpi=dpi, rc={})


def _psfont(texname):
    previous = dr.find_tex_file
    dr.find_tex_file = lambda name: name.decode() if isinstance(name, bytes) else name
    try:
        entry = dr.PsfontsMap(str(FIXTURES / "test.map"))[texname.encode()]
    finally:
        dr.find_tex_file = previous
        dr.PsfontsMap.__new__.cache_clear()
    return {
        "texname": entry.texname.decode(),
        "psname": entry.psname.decode(),
        "effects": dict(sorted(entry.effects.items())),
        "encoding": entry.encoding,
        "filename": entry.filename,
    }


def test_psfonts_map():
    names = [f"TeXfont{n}" for n in range(1, 10)] + [
        "TeXfontA", "TeXfontB", "TeXfontC", "no-such-font", "%"]
    for texname in names:
        probes.probe("dviread.PsfontsMap", _psfont, texname=texname)


def _parse_enc(text):
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp, "test.enc")
        path.write_bytes(text.encode("latin-1"))
        return dr._parse_enc(path)


ENCODINGS = [
    "/Enc [ /a /b /c ] def",
    "% comment [ /not ]\n/TeXBase1Encoding [ % first\n/grave /acute\n/a /b ] def\n",
    "[]",
    "[ /a b ]",
    "no array here",
    "/E [ /x ] [ /y ] def",
    "\xe9 [ /a ]",
]


def test_encodings():
    for text in ENCODINGS:
        probes.probe("dviread._parse_enc", _parse_enc, text=text)


# Files in TeX's formats.

def _tfm(bc, ec, widths, checksum=0, heights=(0, 100), depths=(0, 30), design=10 << 20):
    """A TFM file: characters ``bc..ec`` using width ``code - bc + 1`` (the
    last for later codes), height 1 and depth 1."""
    nw, nh, nd = len(widths) + 1, len(heights), len(depths)
    data = struct.pack("!12H", 0, 2, bc, ec, nw, nh, nd, 0, 0, 0, 0, 0)
    data += struct.pack("!2I", checksum, design)
    for code in range(bc, ec + 1):
        data += bytes([min(code - bc + 1, nw - 1), 0x11, 0, 0])
    data += struct.pack(f"!{nw}i", 0, *widths)
    data += struct.pack(f"!{nh}i", *heights)
    data += struct.pack(f"!{nd}i", *depths)
    return data


class _Writer:
    """DVI opcodes."""

    def __init__(self):
        self.data = bytearray()

    def op(self, *values):
        self.data += bytes(values)
        return self

    def u4(self, value):
        self.data += struct.pack("!I", value)
        return self

    def s4(self, value):
        self.data += struct.pack("!i", value)
        return self

    def pre(self, fmt=2, num=25400000, den=7227 << 16, mag=1000):
        return self.op(247, fmt).u4(num).u4(den).u4(mag).op(0)

    def vf_pre(self):
        return self.op(247, 202, 0).u4(0).u4(10 << 20)

    def font_def(self, k, name, scale, checksum=0):
        self.op(243, k).u4(checksum).u4(scale).u4(scale).op(0, len(name))
        self.data += name.encode()
        return self

    def bop(self):
        self.op(139)
        for _ in range(10):
            self.s4(0)
        return self.s4(-1)

    def down(self, a):
        return self.op(160).s4(a)

    def right(self, b):
        return self.op(146).s4(b)

    def rule(self, a, b):
        return self.op(132).s4(a).s4(b)

    def end(self):
        return self.op(140, 248)


TEST_TFM = _tfm(65, 70, [1 << 19, 734003, 600000, 400000, 3 << 18, 1 << 20], checksum=7)
CMSY_TFM = _tfm(0, 3, [800000, 500000], heights=(0, 50000), depths=(0, 70000))


def _baseline(writer):
    """The structure marking the baseline: four downs and three pushes."""
    return writer.down(100).op(141).down(100).down(100).op(141).down(1000).op(141)


def _sample():
    w = _Writer().pre().font_def(0, "test", 10 << 16).bop()
    _baseline(w).op(171, 65, 66).rule(20000, 30000).op(142, 142, 142)
    return bytes(w.end().data)


def _registers():
    w = _Writer().pre().font_def(0, "test", 12 << 16).font_def(3, "test", 6 << 16).bop()
    _baseline(w).op(171, 65)
    w.op(149).data += struct.pack("!h", 3000)  # w2: set w and move
    w.op(147, 67)  # w0: move by w
    w.op(154).data += struct.pack("!h", -500)  # x2
    w.op(152, 68).op(174, 69)  # x0, fnt_num_3, set 'E'
    w.op(163).data += struct.pack("!h", 200)  # y2: move down
    w.op(161, 70, 166 + 1, 0x10, 70, 166, 65)  # y0, z1, z0
    w.op(133, 66, 128, 67)  # put1, set1
    w.op(137).s4(5000).s4(0).op(137).s4(-1).s4(100)  # rules without area
    w.op(239, 3).data += b"abc"  # special
    w.op(142, 142, 142)
    return bytes(w.end().data)


def _two_pages():
    w = _Writer().pre().font_def(0, "test", 10 << 16).bop().op(171, 65).op(140)
    w.bop().op(171).right(1000).op(66, 67).end()
    return bytes(w.data)


def _virtual():
    w = _Writer().pre().font_def(1, "virt", 10 << 16).bop().op(172, 65, 66).end()
    return bytes(w.data)


def _vf(first_font="test"):
    v = _Writer().vf_pre().font_def(0, first_font, 1 << 19)
    packet = _Writer().op(141, 66).right(1000).rule(1 << 20, 1 << 20).op(142).data
    v.op(len(packet), 65, 0, 0, 0)
    v.data += packet
    packet = _Writer().op(67).down(-2000).op(68).data
    v.op(len(packet), 66, 0, 0x10, 0)
    v.data += packet
    return bytes(v.op(248).data)


def _cmsy():
    w = _Writer().pre().font_def(0, "cmsy10", 10 << 16).bop()
    _baseline(w).op(171, 0, 1, 2).op(142, 142, 142)
    return bytes(w.end().data)


def _hex(data):
    return data.hex()


TFMS = {"test": _hex(TEST_TFM), "cmsy10": _hex(CMSY_TFM),
        "virt": _hex(_tfm(65, 66, [1 << 20, 1 << 20]))}


def _with_files(tfm, vf, read):
    with tempfile.TemporaryDirectory() as tmp:
        files = {}
        for suffix, contents in [(".tfm", tfm), (".vf", vf)]:
            for name, data in contents.items():
                path = Path(tmp, name + suffix)
                path.write_bytes(bytes.fromhex(data))
                files[name + suffix] = str(path)

        def find(name):
            name = name.decode() if isinstance(name, bytes) else name
            if name in files:
                return files[name]
            raise FileNotFoundError(name)

        previous = dr.find_tex_file
        dr.find_tex_file = find
        dr._fontfile.cache_clear()
        try:
            return read(tmp)
        finally:
            dr.find_tex_file = previous
            dr._fontfile.cache_clear()


def _glyph(text):
    return [text.x, text.y, text.font.texname.decode("latin-1"), text.font._scale, text.glyph]


def _vf_summary(data, tfm, vf):
    def read(tmp):
        path = Path(tmp, "probe.vf")
        path.write_bytes(bytes.fromhex(data))
        font = dr.Vf(path)
        return {"packets": [
            [code, page.width, [_glyph(t) for t in page.text],
             [[b.x, b.y, b.height, b.width] for b in page.boxes]]
            for code, page in sorted(font._chars.items())]}
    return _with_files(tfm, vf, read)


def _dvi_pages(data, dpi, tfm, vf):
    def read(tmp):
        path = Path(tmp, "probe.dvi")
        path.write_bytes(bytes.fromhex(data))
        with dr.Dvi(path, dpi) as dvi:
            return [{
                "text": [[t.x, t.y, t.font.texname.decode("latin-1"), t.font.size, t.glyph,
                          t.width] for t in page.text],
                "boxes": [[b.x, b.y, b.height, b.width] for b in page.boxes],
                "width": page.width, "height": page.height, "descent": page.descent,
            } for page in dvi]
    return _with_files(tfm, vf, read)


def _tfm_summary(data):
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp, "probe.tfm")
        path.write_bytes(bytes.fromhex(data))
        tfm = dr.Tfm(path)
    return {"checksum": tfm.checksum, "design_size": tfm.design_size,
            "metrics": [[code, m.tex_width, m.tex_height, m.tex_depth]
                        for code, m in sorted(tfm._glyph_metrics.items())]}


def test_font_metrics():
    for data in [TEST_TFM, CMSY_TFM, _tfm(1, 0, []), TEST_TFM[:30], TEST_TFM[:-1],
                 TEST_TFM[:24] + b"\x00" * 8 + bytes([9, 0x11, 0, 0]) + TEST_TFM[36:]]:
        probes.probe("dviread.Tfm", _tfm_summary, data=_hex(data))


def test_virtual_fonts():
    vf = _vf()
    for data, tfm in [(vf, TFMS), (vf, {}), (vf[:-1], TFMS), (vf[:5], TFMS),
                      (_vf("absent"), TFMS)]:
        probes.probe("dviread.Vf", _vf_summary, data=_hex(data), tfm=tfm, vf={})


def test_dvi_files():
    sample = _sample()
    corpus = [
        (sample, TFMS, {}),
        (_registers(), TFMS, {}),
        (_two_pages(), TFMS, {}),
        (_virtual(), TFMS, {"virt": _hex(_vf())}),
        (_cmsy(), TFMS, {}),
        (bytes(_Writer().pre().bop().end().data), TFMS, {}),
        (bytes(_Writer().pre().font_def(0, "absent", 1 << 20).bop().end().data), TFMS, {}),
        (bytes(_Writer().pre().font_def(0, "absent", 1 << 20).bop().op(171, 65).end().data),
         TFMS, {}),
        (bytes(_Writer().pre().font_def(0, "test", 1 << 20, checksum=9).bop().end().data),
         TFMS, {}),
        (bytes(_Writer().pre(fmt=3).bop().end().data), TFMS, {}),
        (bytes(_Writer().pre(mag=2000).bop().end().data), TFMS, {}),
        (bytes(_Writer().pre().bop().op(142).end().data), TFMS, {}),
        (bytes(_Writer().pre().op(65).data), TFMS, {}),
        (bytes(_Writer().pre().bop().op(255).data), TFMS, {}),
        (sample[:40], TFMS, {}),
        (sample[:-1], TFMS, {}),
    ]
    for data, tfm, vf in corpus:
        for dpi in [None, 72.0, 100.0]:
            probes.probe("dviread.Dvi", _dvi_pages, data=_hex(data), dpi=dpi, tfm=tfm, vf=vf)
