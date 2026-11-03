"""
More of what the differential tester (``cargo xtask diff``) found pltrs
answering otherwise than matplotlib 3.11: values typed as matplotlib types
them, warnings and errors where matplotlib gives them, and the methods of
3-D axes, axis artists and tick parameters matplotlib's classes have. Every
expected value was recorded from matplotlib.
"""

import inspect
import os
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections as mcoll
from pltrs import font_manager as fm
from pltrs import ticker as mticker
from pltrs.mpl_toolkits.axisartist import axis_artist
from pltrs.mpl_toolkits.axisartist.axislines import Axes as AxisArtistAxes


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_axes_of_3d_axes_take_the_methods_of_an_axis():
    ax = plt.figure().add_subplot(projection='3d')
    ax.plot([0, 1, 2], [1, 2, 3], [2, 4, 1])
    ax.figure.canvas.draw()
    for axis in (ax.xaxis, ax.zaxis):
        axis.minorticks_on()
        axis.minorticks_off()
        axis.clear()
        with pytest.raises(ValueError):
            axis.tick_top()
    assert type(ax.zaxis.get_minpos()) is np.float64
    formatter = mticker.ScalarFormatter()
    ax.zaxis.set_major_formatter(formatter)
    assert formatter.format_ticks([1.0, 2.0, 3.0]) == ['1', '2', '3']
    assert formatter.format_data_short(2.5) == '2.5000'
    locator = mticker.MaxNLocator(4)
    ax.zaxis.set_major_locator(locator)
    assert np.allclose(locator(), [0.8, 1.6, 2.4, 3.2, 4.0, 4.8])


def test_3d_axes_keep_angles_margins_and_data_ratio_typed():
    ax = plt.figure().add_subplot(projection='3d')
    assert (ax.elev, ax.azim, ax.roll) == (30, -60, 0)
    assert type(ax.elev) is int
    ax.set_xmargin(0)
    assert type(ax.get_xmargin()) is int
    assert type(ax.get_xinverted()) is np.bool_
    assert ax.xaxis.get_zorder() == 1.5


def test_axis_artist_parts_take_text_and_line_properties():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=AxisArtistAxes)
    label = ax.axis["bottom"].label
    label.set(fontweight="bold", fontstyle="italic", fontfamily="serif",
              linespacing=2.0, rotation_mode="default", wrap=True)
    assert (label.get_fontweight(), label.get_fontstyle()) == ("bold", "italic")
    assert label.get_fontfamily() == ["serif"]
    assert (label.get_linespacing(), label.get_rotation_mode()) == (2.0, "default")
    ticks = ax.axis["left"].major_ticks
    ticks.set(linestyle="--", marker="o", markersize=3, pickradius=7)
    assert (ticks.get_linestyle(), ticks.get_marker()) == ("--", "o")
    assert (ticks.get_markersize(), ticks.get_pickradius()) == (3.0, 7)
    own = axis_artist.LabelBase(0.5, 0.5, "Test", fontweight="bold")
    own.set_position((0.2, 0.3))
    ax.add_artist(own)
    assert own.get_position() == (0.2, 0.3)
    assert own.get_rotation_mode() == "anchor"
    fig.canvas.draw()
    for cls, count in ((axis_artist.LabelBase, 48), (axis_artist.Ticks, 47)):
        assert len(inspect.signature(cls.set).parameters) >= count


def test_gridline_keywords_of_tick_params():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.grid(True, antialiased=False, dashes=(4, 2), solid_capstyle='round')
    ax.tick_params(grid_dash_capstyle='round', grid_markersize=3)
    params = ax.xaxis.get_tick_params()
    assert params['grid_dashes'] == (4, 2)
    assert params['grid_antialiased'] is False
    fig.canvas.draw()


#: An AFM file of two characters.
_AFM = """StartFontMetrics 2.0
FontName CustomGothic-Demi
FullName Custom Gothic Demi
FamilyName Custom Gothic
Weight Demi
ItalicAngle 0
IsFixedPitch false
FontBBox -123 -251 1222 1021
UnderlinePosition -100
UnderlineThickness 50
Version 001.007
EncodingScheme AdobeStandardEncoding
CapHeight 740
XHeight 555
Ascender 740
Descender -185
StartCharMetrics 2
C 32 ; WX 280 ; N space ; B 0 0 0 0 ;
C 33 ; WX 280 ; N exclam ; B 73 0 206 740 ;
EndCharMetrics
EndFontMetrics
"""


def test_afm_fonts_can_be_added(tmp_path):
    path = tmp_path / "custom.afm"
    path.write_text(_AFM, encoding="latin-1")
    fm.fontManager.addfont(path)
    entry = fm.fontManager.afmlist[-1]
    assert (entry.name, entry.weight) == ("Custom Gothic", "demi")
    found = fm.findfont(fm.FontProperties(family="Custom Gothic"), fontext="afm")
    assert os.path.basename(found) == "custom.afm"


def test_rasterizing_what_draws_vectors_warns():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    axline = ax.axline((0, 0), (1, 1))
    for artist, warns in ((line, False), (axline, True), (ax, False)):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            artist.set_rasterized(True)
        assert bool(caught) is warns
        assert artist.get_rasterized()


def test_default_stale_callbacks():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], label='a')
    leg = ax.legend()
    figlegend = fig.legend()
    assert ax.get_xticklabels()[0].stale_callback is None
    assert ax.xaxis.label.stale_callback is None
    assert figlegend.stale_callback is None
    assert leg.stale_callback.__name__ == "_stale_axes_callback"
    assert ax.xaxis.get_major_ticks()[0].stale_callback.__name__ == "_stale_axes_callback"
    assert fig.text(0, 0, "x").stale_callback.__name__ == "_stale_figure_callback"


def test_collections_keep_their_paths():
    fig, ax = plt.subplots()
    scatter = ax.scatter([1, 2], [3, 4])
    lines = ax.vlines([1, 2], 0, 1)
    assert type(scatter.get_paths()) is tuple
    assert scatter.get_paths() is scatter.get_paths()
    assert lines.get_paths() is lines.get_paths()
    scatter.set_paths([scatter.get_paths()[0]])
    assert type(scatter.get_paths()) is list


def test_meshes_violins_and_spans_as_matplotlib_makes_them():
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(np.ones((3, 4)))
    assert mesh.get_antialiased().tolist() == [True]
    assert mesh.get_pickradius() == 0 and type(mesh.get_pickradius()) is int
    bodies = ax.violinplot([np.sin(np.linspace(0, 3, 30))])['bodies']
    assert type(bodies[0]) is mcoll.FillBetweenPolyCollection
    assert bodies[0].t_direction == 'y'
    span = ax.axvspan(0.1, 0.3)
    assert [type(v) for v in (span.get_x(), span.get_y(), span.get_height())] == [
        float, int, int]
    _, _, bars = ax.hist([1, 2, 2, 3, 5], bins=4)
    assert type(bars[0].get_height()) is np.float64
    line = ax.axhline(0.5)
    assert line.get_xdata() == [0, 1] and line.get_ydata() == [0.5, 0.5]


def test_tick_locations_as_their_locator_gives_them():
    fig, ax = plt.subplots()
    ax.bar(['a', 'b'], [1, 2])
    assert ax.get_xticks() == [0, 1]
    fig, ax = plt.subplots()
    ax.boxplot([[1, 2, 3], [2, 3, 4]])
    assert ax.get_xticks().dtype.kind == "i"
    ax.set_xticks([0, 1, 2])
    assert ax.get_xticks().tolist() == [0, 1, 2]
    assert ax.get_xticks().dtype.kind == "i"


def test_contour_widths_broadcast_against_line_styles():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots()
    lines = ax.contour(z)
    lines.set_linewidth(2)
    assert len(lines.get_linewidth()) == 9
    lines.set_linestyle('None')
    assert lines.get_linewidth().tolist() == [2]
    filled = ax.contourf(z)
    filled.set_linewidth(2)
    assert filled.get_linewidth().tolist() == [2]
    assert type(lines.zmax) is np.float64


def test_values_kept_as_given():
    fig, ax = plt.subplots()
    ax.set_ymargin(0)
    ax.margins(x=0)
    assert ax.margins() == (0, 0)
    fig.set_dpi(72)
    assert fig.get_dpi() == 72 and type(fig.get_dpi()) is int
    line, = ax.plot([1, 2], 'o', mfc='r')
    line.set_markerfacecoloralt('g')
    line.set_fillstyle('none')
    assert (line.get_markerfacecolor(), line.get_markerfacecoloralt()) == (
        'none', 'none')
    ax.set_axisbelow(False)
    assert ax.xaxis.get_zorder() == 2.5


def test_polar_axes_refuse_a_box_aspect():
    ax = plt.figure().add_subplot(projection='polar')
    with pytest.raises(ValueError, match="get_data_ratio"):
        ax.set_box_aspect(1)
    assert ax.get_adjustable() == 'box'
    assert type(ax.get_rmax()) is np.float64


def test_colorbar_axes_and_ticks():
    fig, ax = plt.subplots()
    image = ax.imshow(np.arange(6).reshape(2, 3))
    colorbar = fig.colorbar(image)
    assert colorbar.ax.get_label() == '<colorbar>'
    assert ax.xaxis.get_major_ticks()[0].get_figure() is fig
