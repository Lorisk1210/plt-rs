"""
What the libraries building on matplotlib's parts need of it that its own
suite does not reach (``cargo xtask ecosystem suite`` runs their suites):
font size names, colour mappings, rcParams, grids, Axes classes and
figure draws of a program's own, matplotlib's private attributes that
exporters read (mpld3, matplot2tikz), annotations moved to their figure
(mplcursors), boxes made and drawn in a draw (matplotlib-scalebar), norms
of bins and sharing groups (ultraplot). Each test does what a library
does, without the library. The expected values were recorded from
matplotlib 3.11.2 running the same code.
"""

import gc
import io
import weakref
from collections.abc import MutableMapping

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors as mcolors, font_manager, gridspec as mgs, rcsetup
from pltrs import scale as mscale, ticker
from pltrs.artist import Artist
from pltrs.figure import Figure
from pltrs.mpl_toolkits.axes_grid1 import axes_size, make_axes_locatable
from pltrs.mpl_toolkits.mplot3d import Axes3D
from pltrs.offsetbox import AnchoredOffsetbox, TextArea, VPacker
from pltrs.projections import PolarAxes
from pltrs.table import table
from pltrs.text import Text

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


@pytest.fixture
def med_large():
    """``med-large`` named in ``font_scalings``, with a validator of
    ``axes.titlesize`` taking it, as ultraplot names its sizes."""
    font_manager.font_scalings["med-large"] = 1.1
    validate = pltrs.rcParams.validate
    saved = validate["axes.titlesize"]

    def validator(value):
        value = value.lower() if isinstance(value, str) else value
        return value if value in font_manager.font_scalings else float(value)

    validate["axes.titlesize"] = validator
    yield
    validate["axes.titlesize"] = saved
    del font_manager.font_scalings["med-large"]


def test_a_font_size_name_a_program_adds_sizes_titles(med_large):
    pltrs.rcParams["axes.titlesize"] = "med-large"
    assert pltrs.rcParams["axes.titlesize"] == "med-large"
    fig, ax = plt.subplots()
    assert ax.set_title("x").get_fontsize() == 11.0
    params = pltrs.RcParams({"axes.titlesize": "med-large"})
    assert params["axes.titlesize"] == "med-large"


class _ColorDatabase(MutableMapping, dict):
    """A colour mapping put in place of matplotlib's (ultraplot's)."""

    def __init__(self, mapping):
        self._cache = {}
        for key, value in mapping.items():
            self[key] = value

    def __getitem__(self, key):
        return dict.__getitem__(self, key.lower() if len(key) > 1 else key)

    def __setitem__(self, key, value):
        dict.__setitem__(self, key.lower() if len(key) > 1 else key, value)
        self._cache.clear()

    def __delitem__(self, key):
        dict.__delitem__(self, key)

    @property
    def cache(self):
        return self._cache


def test_a_colour_mapping_of_the_program_names_colours(monkeypatch):
    database = _ColorDatabase(mcolors._colors_full_map)
    monkeypatch.setattr(mcolors, "_colors_full_map", database)
    database["blue7"] = "#1c7ed6"
    assert rcsetup.validate_color("blue7") == "blue7"
    assert mcolors.to_rgba("Blue7") == pytest.approx(
        (0.10980392156862745, 0.49411764705882355, 0.8392156862745098, 1.0))
    assert mcolors.to_hex("blue7") == "#1c7ed6"
    assert mcolors.to_rgba_array(["blue7", "k"])[0, 2] == pytest.approx(0.8392156862745098)


def test_rcparams_a_program_adds_are_kept(monkeypatch):
    validate = dict(pltrs.rcParams.validate)
    validate["scalebar.pad"] = rcsetup.validate_float
    validate["scalebar.color"] = rcsetup.validate_color
    monkeypatch.setattr(pltrs.rcParams, "validate", validate, raising=False)
    pltrs.rcParams["scalebar.color"] = "r"
    pltrs.rcParams.update({"scalebar.pad": "0.5"})
    assert (pltrs.rcParams["scalebar.color"], pltrs.rcParams["scalebar.pad"]) == ("r", 0.5)
    assert "scalebar.pad" in pltrs.rcParams.keys()
    with pltrs.rc_context({"scalebar.pad": 1.0}):
        assert pltrs.rcParams["scalebar.pad"] == 1.0
    assert pltrs.rcParams["scalebar.pad"] == 0.5
    assert pltrs.rcParams.copy()["scalebar.color"] == "r"


def test_a_program_puts_its_own_text_in_the_super_title():
    fig = plt.figure()
    assert (fig._suptitle, fig.get_suptitle()) == (None, "")
    text = fig.text(0.5, 0.95, "", ha="center", va="bottom")
    fig._suptitle = text
    title = fig.suptitle("hello", fontsize=9)
    assert title is text and len(fig.texts) == 1
    assert (fig.get_suptitle(), title.get_position(), title.get_va()) == (
        "hello", (0.5, 0.98), "top")
    assert (title.get_fontsize(), title._autopos) == (9.0, True)
    assert fig.suptitle("again", y=0.5).get_position() == (0.5, 0.5)
    fig._suptitle = None
    assert (fig._suptitle, fig.get_suptitle()) == (None, "")


class _HalfGrid(mgs.GridSpec):
    """A grid placing its rows in the lower half of the figure."""

    def get_grid_positions(self, fig):
        bottoms, tops, lefts, rights = super().get_grid_positions(fig)
        return bottoms * 0.5, tops * 0.5, lefts, rights


def test_a_grid_of_the_program_places_its_axes():
    fig = plt.figure()
    ax = fig.add_subplot(_HalfGrid(2, 2, figure=fig)[1, 0])
    assert ax.get_position().bounds == pytest.approx(
        (0.125, 0.055, 0.35227272727272724, 0.175))
    fig.canvas.draw()
    assert ax.get_position().bounds[1] == pytest.approx(0.055)
    fig.subplots_adjust(left=0.3, bottom=0.2)
    assert ax.get_position().bounds == pytest.approx(
        (0.3, 0.1, 0.27272727272727276, 0.15454545454545454))


class _OwnGrid(mgs.GridSpec):
    """A grid that never runs GridSpec's constructor and places its one
    cell itself (ultraplot's)."""

    def __init__(self):
        self._figure = None

    def get_geometry(self):
        return 1, 1

    def get_grid_positions(self, figure=None):
        return (np.array([0.2]), np.array([0.6]), np.array([0.1]), np.array([0.5]))


def test_a_grid_placing_its_own_cells_gives_its_spec_back():
    fig = plt.figure()
    spec = mgs.SubplotSpec(_OwnGrid(), 0)
    ax = fig.add_subplot(spec)
    assert ax.get_subplotspec() is spec
    assert ax.get_position().bounds == pytest.approx((0.1, 0.2, 0.4, 0.4))


class _Polar(PolarAxes):
    def __init__(self, *args, extra=None, **kwargs):
        self.extra = extra
        super().__init__(*args, **kwargs)


class _Deep(Axes3D):
    def __init__(self, *args, extra=None, **kwargs):
        self.extra = extra
        super().__init__(*args, **kwargs)


def test_axes_classes_of_the_program_are_made_by_their_constructors():
    fig = plt.figure()
    polar = fig.add_subplot(121, axes_class=_Polar, extra=3)
    deep = fig.add_subplot(122, axes_class=_Deep, extra=4, elev=10)
    assert (type(polar), polar.extra, polar.name) == (_Polar, 3, "polar")
    assert (type(deep), deep.extra, deep.elev) == (_Deep, 4, 10)
    assert fig.axes == [polar, deep]


def test_titles_take_an_offset_and_colorbars_their_short_axis():
    fig, ax = plt.subplots()
    ax._set_title_offset_trans(20)
    assert ax.titleOffsetTrans.get_matrix()[1, 2] == pytest.approx(20 / 72 * fig.dpi)
    colorbar = fig.colorbar(ax.imshow([[1, 2]]), ax=ax)
    assert colorbar._short_axis() is colorbar.ax.xaxis


def test_a_program_sets_the_axes_it_shares_and_joins_groups():
    fig, (a, b, c) = plt.subplots(3)
    b._sharex = a
    assert b._sharex is a
    a.set_xlim(0, 5)
    a._shared_axes["x"].join(c, a)
    assert c.get_xlim() == (0, 1) and c._sharex is None
    assert a._shared_axes["x"].joined(a, c)
    a.set_xlim(1, 2)
    assert c.get_xlim() == (1, 2)
    a._shared_axes["x"].remove(c)
    a.set_xlim(3, 4)
    assert c.get_xlim() == (1, 2)
    twin = fig.add_axes((0.1, 0.1, 0.8, 0.8))
    a._twinned_axes.join(a, twin)
    assert a._twinned_axes.joined(a, twin)


def test_tick_parameters_of_none_take_the_rcparams():
    pltrs.rcParams["ytick.direction"] = "in"
    fig, ax = plt.subplots()
    ax.tick_params(axis="x", direction="in")
    pltrs.rcParams["font.size"] = 13
    ax.tick_params(direction=None, labelsize=None)
    assert [axis.get_major_ticks()[0].get_tickdir() for axis in (ax.xaxis, ax.yaxis)] == [
        "out", "in"]
    assert ax.xaxis.get_major_ticks()[0].label1.get_fontsize() == 13.0


def test_a_program_sets_an_axis_scale_object_itself():
    fig, (a, b) = plt.subplots(2)
    b.xaxis.set_major_locator(ticker.MultipleLocator(0.25))
    scale = mscale.FuncScale(b.xaxis, (lambda x: x**2, lambda x: np.sqrt(np.abs(x))))
    b.xaxis._scale = scale
    b._update_transScale()
    assert (b.get_xscale(), b.xaxis._scale) == ("function", scale)
    assert isinstance(b.xaxis.get_major_locator(), ticker.MultipleLocator)
    assert (b.xaxis.isDefault_majloc, b.xaxis.isDefault_minloc) == (False, True)
    b.set_xlim(0, 1)
    fig.canvas.draw()
    assert b.transData.transform([[0.5, 0]])[0] == pytest.approx([204.0, 52.8])
    assert [t.get_text() for t in b.get_xticklabels()][:3] == ["−0.25", "0.00", "0.25"]


class _Bins(mcolors.BoundaryNorm):
    """Bins whose inverse refuses, as BoundaryNorm's."""

    def __call__(self, value, clip=None):
        return super().__call__(value, clip) / (self.Ncmap - 1)

    def inverse(self, value):
        raise ValueError("not invertible")


class _BareBins(mcolors.BoundaryNorm):
    """Bins of a class that never runs BoundaryNorm's constructor
    (ultraplot's ``DiscreteNorm``)."""

    def __init__(self, levels):
        mcolors.Normalize.__init__(self, min(levels), max(levels))
        self.boundaries = np.asarray(levels, float)
        self.N = len(levels)

    def __call__(self, value, clip=None):
        return np.interp(value, self.boundaries, np.linspace(0, 1, self.N))

    def inverse(self, value):
        raise ValueError("not invertible")


@pytest.mark.parametrize("norm, expected", [
    (_Bins([0, 2, 5, 11], 256, extend="max"),
     ([0, 2, 5, 11, 12], [1, 3.5, 8, 11.5], (0, 11), ["0", "2", "5", "11"])),
    (_BareBins([0, 3, 6, 12]),
     ([0, 3, 6, 12], [1.5, 4.5, 9], (0, 12), ["0", "3", "6", "12"])),
])
def test_colorbars_show_norms_of_bins_of_the_program_by_their_bins(norm, expected):
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(np.arange(12.).reshape(3, 4), norm=norm)
    colorbar = fig.colorbar(mesh)
    fig.canvas.draw()
    boundaries, values, limits, labels = expected
    assert colorbar._boundaries == pytest.approx(boundaries)
    assert colorbar._values == pytest.approx(values)
    assert colorbar.ax.get_ylim() == pytest.approx(limits)
    assert [t.get_text() for t in colorbar.ax.get_yticklabels()] == labels


class _Drawn(Figure):
    """A figure whose own draw changes it first and reports its renderer."""

    def draw(self, renderer):
        self.seen.append((type(renderer).__name__, getattr(renderer, "width", None)))
        self.axes[0].set_title(f"drawn {len(self.seen)}")
        super().draw(renderer)


def test_figures_draw_through_their_own_draw():
    fig = plt.figure(FigureClass=_Drawn, figsize=(4.25, 3.53))
    fig.seen = []
    fig.add_subplot()
    for fmt in ("svg", "png"):
        fig.savefig(io.BytesIO(), format=fmt, dpi=300)
    fig.canvas.draw()
    assert fig.seen == [("MixedModeRenderer", 306.0), ("RendererAgg", 1275),
                        ("RendererAgg", 425)]
    assert fig.axes[0].get_title() == "drawn 3"


def test_exporters_read_the_private_attributes_of_lines_and_texts():
    fig, ax = plt.subplots()
    patterns = {}
    for style in ["-", "--", (2, (3, 1))]:
        line, = ax.plot([0, 1], ls=style, lw=2)
        patterns[str(style)] = line._unscaled_dash_pattern
        assert line._drawstyle == "default"
    line.set_dashes([4, 2])
    assert patterns == {"-": (0, None), "--": (0.0, (3.7, 1.6)), "(2, (3, 1))": (2, (3, 1))}
    assert line._unscaled_dash_pattern == (0, [4, 2])
    text = ax.text(0.2, 0.3, "a\nb")
    assert (text._x, text._y, text._multialignment) == (0.2, 0.3, None)
    text.set_multialignment("right")
    text._x = 0.5
    assert (text._multialignment, text.get_position()) == ("right", (0.5, 0.3))
    legend = ax.legend(["a"])
    assert legend._bbox_to_anchor is None
    legend.set_bbox_to_anchor((0.5, 0.5))
    assert legend._bbox_to_anchor.bounds == pytest.approx(
        legend.get_bbox_to_anchor().bounds)


def test_exporters_read_the_points_of_collections_and_the_cells_of_tables():
    fig, ax = plt.subplots()
    transform, offset_transform, offsets, paths = ax.scatter([1, 2], [3, 4])._prepare_points()
    assert type(transform).__name__ == "IdentityTransform"
    assert offsets.tolist() == [[1.0, 3.0], [2.0, 4.0]]
    assert (len(paths), paths[0].vertices.shape) == (1, (26, 2))
    _, _, offsets, paths = ax.contour(np.arange(16.).reshape(4, 4))._prepare_points()
    assert (np.shape(offsets), len(paths)) == ((1, 2), 9)
    cells = table(ax, cellText=[["a", "b"]], loc="center")._cells
    cells[(0, 0)]._text.set_color("red")
    assert cells[(0, 0)].get_text().get_color() == "red"
    assert (sorted(cells), len(cells), (0, 1) in cells) == ([(0, 0), (0, 1)], 2, True)


def test_an_annotation_moved_to_its_figure_keeps_its_axes_coordinates():
    fig, ax = plt.subplots()
    ax.plot([0, 1])
    note = ax.annotate("x", xy=(0.5, 0.5), xytext=(10, 10), textcoords="offset points")
    note.remove()
    note.axes = ax
    fig.add_artist(note)
    assert (fig.artists, fig.texts, note.axes) == ([note], [], ax)
    fig.canvas.draw()
    assert note.get_window_extent().bounds == pytest.approx(
        (341.889, 248.152, 8.0, 13.889), abs=1e-3)
    added = fig.add_artist(Text(0.5, 0.5, "z"))
    written = fig.text(0.1, 0.1, "y")
    assert (fig.artists, fig.texts) == ([note, added], [written])


class _Bar(Artist):
    """Makes and draws a box of its own in its draw (matplotlib-scalebar)."""

    def draw(self, renderer):
        area = TextArea("1 km")
        box = AnchoredOffsetbox(loc="upper right",
                                child=VPacker(children=[area], pad=0, sep=0))
        box.axes = self.axes
        box.set_figure(self.get_figure())
        box.draw(renderer)
        self.extent = box.get_window_extent(renderer).bounds


def test_a_box_made_and_drawn_in_a_draw_is_measured():
    fig, ax = plt.subplots()
    bar = ax.add_artist(_Bar())
    fig.canvas.draw()
    assert bar.extent == pytest.approx(
        (522.9444444444445, 390.0078559027778, 46.111111111111086, 25.44769965277777))


class _Remainder(axes_size.Scaled):
    """A size of the program's computing its own, deriving from `Scaled`
    without its constructor (mplhep's ``RemainderFixed``)."""

    def __init__(self, inches):
        self.inches = inches

    def get_size(self, renderer):
        return 0, self.inches


def test_a_size_of_the_program_is_its_own_get_size():
    fig = plt.figure(figsize=(6, 4))
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8))
    divider = make_axes_locatable(ax)
    divider.set_horizontal([_Remainder(2.5), axes_size.Fixed(0.5)])
    divider.set_vertical([_Remainder(2.0)])
    ax.set_axes_locator(divider.new_locator(nx=0, ny=0))
    fig.canvas.draw()
    assert ax.get_position().bounds == pytest.approx(
        (0.1, 0.1, 0.4166666666666667, 0.5))


def test_a_figure_whose_text_fonts_were_read_goes_once_closed():
    fig, ax = plt.subplots()
    ax.set_title("title").get_fontproperties()
    ax.text(0.5, 0.5, "x").get_fontproperties().set_size(12)
    fig.savefig(io.BytesIO(), format="png")
    alive = weakref.ref(fig)
    plt.close(fig)
    del fig, ax
    gc.collect()
    assert alive() is None
