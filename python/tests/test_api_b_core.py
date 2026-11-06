"""
Smoke tests of the public names pltrs (Python) adds at the top level and
in cm, dviread, font_manager, ft2font, image, pylab, rcsetup, textpath
and transforms. The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import io
import shutil
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import (cm, dviread, font_manager as fm, ft2font, image, pylab,
                   rcsetup, textpath, transforms)
from pltrs.backend_bases import FigureCanvasBase
from pltrs.backends import backend_agg

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_top_level_names():
    assert pltrs.__bibtex__.startswith("@Article{Hunter:2007,")
    assert len(pltrs.__bibtex__.split("\n")) == 14
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        params = pltrs.defaultParams
    assert isinstance(params, dict) and len(params) == 339
    assert params["lines.linewidth"][0] == 1.5
    # pltrs' own rc file stands where matplotlib's matplotlibrc does.
    assert pltrs.matplotlib_fname().endswith("pltrsrc")
    rc = pltrs.rc_params()
    assert type(rc).__name__ == "RcParams" and rc["lines.linewidth"] == 1.5


def test_pylab_and_rcsetup_names():
    assert pylab.TYPE_CHECKING is False
    assert pylab.annotations.compiler_flag == 16777216
    assert type(pylab.backend_registry).__name__ == "BackendRegistry"
    assert type(pylab.color_sequences).__name__ == "ColorSequenceRegistry"
    assert "viridis" in pylab.colormaps
    assert pylab.rcParamsDefault["lines.linewidth"] == 1.5
    assert type(pylab.rcParamsOrig).__name__ == "RcParams"
    assert rcsetup.backend_registry is pylab.backend_registry
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        assert len(rcsetup.defaultParams) == 339
        assert rcsetup.defaultParams["lines.linewidth"][0] == 1.5


def test_pick_ids_and_hinting_flag():
    canvas = plt.figure().canvas
    assert canvas.button_pick_id is not None
    assert canvas.scroll_pick_id is not None
    assert FigureCanvasBase.button_pick_id is not None
    assert backend_agg.get_hinting_flag() == ft2font.LoadFlags.DEFAULT
    with pltrs.rc_context({"text.hinting": "none"}):
        assert backend_agg.get_hinting_flag() == ft2font.LoadFlags.NO_HINTING


def test_colormap_tables():
    assert len(cm.datad) == 76
    assert sorted(cm.datad)[:3] == ["Accent", "Blues", "BrBG"]
    assert cm.datad["binary"] == {
        "red": ((0.0, 1.0, 1.0), (1.0, 0.0, 0.0)),
        "green": ((0.0, 1.0, 1.0), (1.0, 0.0, 0.0)),
        "blue": ((0.0, 1.0, 1.0), (1.0, 0.0, 0.0))}
    assert len(cm.cmaps_listed) == 11
    assert sorted(cm.cmaps_listed)[:3] == ["berlin", "cividis", "inferno"]
    assert sorted(cm.bivar_cmaps)[:2] == ["BiCone", "BiOrangeBlue"]
    assert len(cm.bivar_cmaps) == 3
    assert sorted(cm.multivar_cmaps)[:2] == ["2VarAddA", "2VarSubA"]
    assert len(cm.multivar_cmaps) == 3


def test_font_manager_scores():
    mgr = fm.fontManager
    assert mgr.score_family(["sans-serif"], "DejaVu Sans") == 0.0
    assert mgr.score_family("serif", "Arial") == 1.0
    assert mgr.score_style("italic", "oblique") == 0.1
    assert mgr.score_style("normal", "normal") == 0.0
    assert mgr.score_variant("normal", "small-caps") == 1.0
    assert mgr.score_stretch("condensed", "expanded") == 0.4
    assert mgr.score_size(12, 10) == pytest.approx(0.027777777777777776)
    assert mgr.score_size(10, "large") == 1.0


def test_ft2font_face_properties():
    font = ft2font.FT2Font(fm.findfont("DejaVu Sans"))
    assert (font.ascender, font.descender, font.height) == (1901, -483, 2384)
    assert font.bbox == (-2090, -948, 3673, 2524)
    assert font.face_flags.value == 3673
    assert font.family_name == "DejaVu Sans"
    assert font.style_name == "Book"
    assert font.postscript_name == "DejaVuSans"
    assert (font.max_advance_height, font.max_advance_width) == (2384, 3838)
    assert (font.num_charmaps, font.num_faces, font.num_fixed_sizes,
            font.num_glyphs) == (5, 1, 0, 6241)
    assert font.style_flags == ft2font.StyleFlags.NORMAL
    assert (font.underline_position, font.underline_thickness) == (-175, 90)
    assert font.units_per_EM == 2048
    with pytest.raises(AttributeError):
        font.ascender = 3


def test_ft2font_glyph_and_layout_item():
    font = ft2font.FT2Font(fm.findfont("DejaVu Sans"))
    font.set_size(12, 72)
    glyph = font.load_char(ord("A"))
    assert glyph.bbox == (6, 0, 519, 576)
    assert (glyph.width, glyph.height) == (576, 576)
    assert (glyph.horiAdvance, glyph.horiBearingX, glyph.horiBearingY) == (512, 0, 576)
    assert glyph.linearHoriAdvance == 537984
    assert (glyph.vertAdvance, glyph.vertBearingX, glyph.vertBearingY) == (832, -320, 64)
    item = font._layout("AV", ft2font.LoadFlags.NO_HINTING)[1]
    assert (item.char, item.glyph_index) == ("V", 57)
    assert (round(item.x, 3), item.y, item.prev_kern) == (7.438, 0.0, -0.771484375)
    assert item.ft_object is font


@pytest.mark.skipif(not shutil.which("kpsewhich"), reason="TeX is not installed")
def test_dviread_metrics_and_fonts():
    tfm = dviread.Tfm(dviread.find_tex_file("cmr10.tfm"))
    assert (tfm.checksum, tfm.design_size) == (1274110073, 10485760)
    assert tfm.get_metrics(65) == dviread.TexMetrics(
        tex_width=786434, tex_height=716526, tex_depth=0)
    assert tfm.get_metrics(500) is None
    vf = dviread.Vf(dviread.find_tex_file("ptmr7t.vf"))
    page = vf[65]
    assert (page.width, page.height, page.boxes) == (757069, None, [])
    assert [(t.font.texname, t.glyph, t.width) for t in page.text] == [
        (b"ptmr8r", 65, 757069)]
    path = fm.findfont("DejaVu Sans")
    assert dviread.TtfMetrics(path).get_metrics(36) == dviread.TexMetrics(
        tex_width=717312, tex_height=764416, tex_depth=0)
    lua = dviread.DviFont.from_luatex(
        10 * 2**16, b"[" + path.encode() + b"]:index=0;slant=13107")
    assert lua.size == pytest.approx(9.962640099626402)
    assert lua.effects == {"slant": 0.1999969482421875}
    xe = dviread.DviFont.from_xetex(10 * 2**16, path.encode(), 0, {"slant": 0.2})
    assert (xe.effects, xe.subfont) == ({"slant": 0.2}, 0)
    with pytest.raises(ValueError, match="Invalid modern font name"):
        dviread.DviFont.from_luatex(10, b"foo")
    font = dviread.DviFont(scale=10 * 2**16, metrics=tfm, texname=b"cmr10", vf=None)
    assert font.size == pytest.approx(9.962640099626402)
    # matplotlib 3.11's deprecated widths reads the TFM file fonts no
    # longer keep, and the deprecated TFM tables iterate over the codes as
    # if over items: they raise, as pltrs's do.
    with pytest.warns(pltrs.MatplotlibDeprecationWarning), \
            pytest.raises(AttributeError, match="no attribute '_tfm'"):
        font.widths
    with pytest.warns(pltrs.MatplotlibDeprecationWarning), \
            pytest.raises(TypeError, match="cannot unpack non-iterable int object"):
        tfm.width


def test_composite_images_and_thumbnail():
    fig = plt.figure(dpi=10, figsize=(1, 1))
    renderer = fig.canvas.get_renderer()
    ax = fig.add_axes([0, 0, 1, 1])
    red = np.zeros((2, 2, 4), np.uint8)
    red[..., 0] = red[..., 3] = 255
    blue = np.zeros((2, 2, 4), np.uint8)
    blue[..., 2] = 255
    blue[..., 3] = 128
    first = ax.imshow(red, extent=(0, 0.5, 0, 0.5))
    second = ax.imshow(blue, extent=(0.5, 1, 0.5, 1))
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    pixels, x, y = image.composite_images([first, second], renderer, 1.0)
    assert pixels.shape == (10, 10, 4)
    assert (x, y) == (0.0, 0.0)
    empty, x, y = image.composite_images([], renderer)
    assert empty.shape == (0, 0, 4) and (x, y) == (0, 0)
    source = io.BytesIO()
    plt.imsave(source, np.arange(64).reshape(8, 8), format="png")
    source.seek(0)
    target = io.BytesIO()
    # Deprecated in matplotlib 3.11, as there.
    with pytest.warns(pltrs.MatplotlibDeprecationWarning, match="thumbnail"):
        thumb = image.thumbnail(source, target, scale=0.5)
    assert type(thumb).__name__ == "Figure"
    assert thumb.get_size_inches().tolist() == [0.04, 0.04]
    assert target.getvalue()[:4] == b"\x89PNG"


def test_text_to_path_measures_and_lays_out():
    ttp = textpath.TextToPath()
    prop = fm.FontProperties(size=12)
    assert np.round(ttp.get_text_width_height_descent("Hg", prop, False),
                    3).tolist() == [16.642, 11.244, 2.496]
    assert np.round(ttp.get_text_width_height_descent(r"$x^2$", prop, True),
                    3).tolist() == [13.32, 11.28, 0.0]
    positions, glyph_map, rects = ttp.get_glyphs_with_font(ttp._get_font(prop), "ab")
    assert (len(positions), len(glyph_map), rects) == (2, 2, [])
    assert np.round(positions[1][1:3], 3).tolist() == [61.281, 0.0]
    assert sorted(glyph_map)[0] == "DejaVuSans-44"
    positions, glyph_map, rects = ttp.get_glyphs_mathtext(prop, r"$\alpha$")
    assert (len(positions), len(glyph_map), len(rects)) == (1, 1, 0)


def test_transforms_helpers():
    assert transforms.affine_transform(
        np.array([[1.0, 2.0]]),
        np.array([[2, 0, 1], [0, 3, 0], [0, 0, 1.0]])).tolist() == [[3.0, 6.0]]
    assert transforms.count_bboxes_overlapping_bbox(
        transforms.Bbox([[0, 0], [1, 1]]),
        [transforms.Bbox([[0.5, 0.5], [2, 2]]),
         transforms.Bbox([[3, 3], [4, 4]])]) == 1
    assert np.round(transforms.inv(np.array([[2.0, 0], [0, 4]])),
                    3).tolist() == [[0.5, 0.0], [0.0, 0.25]]
