"""
What the eighth round of corpora (``cargo xtask diff --corpus``) and the
call counts CORP7 left showed pltrs doing otherwise than matplotlib 3.11,
reduced to matplotlib's API: a colormap of the program's called wherever
matplotlib maps values, with the arrays' shapes; a norm of the program's
autoscaled with them; the property cycle's private API that mplhep reads
(``_get_lines._prop_cycle``); a transform of the program's mapping a line
once, when it is plotted; a box style of the program's asked for its
outline at each point an arrow's clipping tries; streamlines coloured
through the program's norm one arrow at a time; and what the libraries of
the new corpora reach into: a 3-D pane drawn as its polygon looks
(pytransform3d), font size keywords and colour names a program adds,
attributes it assigns (``_suptitle``, ``_sharex``, ``_scale``,
``_rasterized``), a grid of its own class (ultraplot), a text's rotation
it computes (matplotlib-label-lines), names its axis converter reads
(pandas), unlabelled reference lines, a box it gives the figure before
adding it (legendkit). Every expected value was recorded from matplotlib.
"""

import collections
import io

import numpy as np
import pytest
from cycler import cycler

import pltrs
import pltrs.colors as mcolors
import pltrs.font_manager as mfont_manager
import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
import pltrs.units as munits
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.colors import ListedColormap, Normalize, to_hex
from pltrs.figure import Figure
from pltrs.gridspec import GridSpec, SubplotSpec
from pltrs.offsetbox import AnchoredOffsetbox, DrawingArea
from pltrs.patches import BoxStyle, Polygon
from pltrs.text import Text


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _shape(value):
    return np.shape(value)


class _Recorded(Normalize):
    """A norm recording the shapes it is called and autoscaled with."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.calls = []

    def __call__(self, value, clip=None):
        self.calls.append(("norm", _shape(value)))
        return super().__call__(value, clip)

    def autoscale_None(self, A):
        self.calls.append(("autoscale_None", _shape(A)))
        return super().autoscale_None(A)


class _RecordedMap(ListedColormap):
    """A colormap recording the shapes and options it is called with."""

    def __init__(self):
        super().__init__(plt.get_cmap("viridis")(np.linspace(0, 1, 16)), name="recorded")
        self.calls = []

    def __call__(self, X, alpha=None, bytes=False):
        self.calls.append((_shape(X), bytes))
        return super().__call__(X, alpha, bytes)


_DATA = np.arange(12.0).reshape(3, 4) / 11


@pytest.mark.parametrize("make, created, drawn", [
    (lambda ax, norm, cmap: ax.imshow(_DATA, norm=norm, cmap=cmap),
     [("autoscale_None", (3, 4))], [((116, 154), True)]),
    (lambda ax, norm, cmap: ax.pcolormesh(_DATA, norm=norm, cmap=cmap),
     [("autoscale_None", (3, 4))] * 2, [((3, 4), False)]),
    (lambda ax, norm, cmap: ax.scatter([1, 2, 3], [1, 2, 3], c=[0.1, 0.5, 0.9],
                                       norm=norm, cmap=cmap),
     [("autoscale_None", (3,))] * 2, [((3,), False)]),
])
def test_the_program_s_norm_and_colormap_get_the_arrays_shapes(make, created, drawn):
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    norm, cmap = _Recorded(), _RecordedMap()
    make(ax, norm, cmap)
    assert [c for c in norm.calls if c[0] == "autoscale_None"] == created
    norm.calls.clear()
    fig.canvas.draw()
    assert cmap.calls == drawn
    assert norm.calls[-1][1] == drawn[0][0]


def test_colorbar_extensions_call_the_program_s_colormap_with_scalars():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    cmap = _RecordedMap()
    fig.colorbar(ax.imshow(_DATA, norm=Normalize(0.2, 0.8), cmap=cmap), extend="both")
    scalars = [call for call in cmap.calls if call[0] == ()]
    assert scalars == [((), False)] * 2


def test_the_property_cycle_s_entries_and_position():
    fig, ax = plt.subplots()
    cycle = ax._get_lines._prop_cycle
    assert (cycle._idx, len(cycle._cycler_items)) == (0, 10)
    assert to_hex(cycle.getdefaults({})["color"]) == "#1f77b4"
    assert (cycle.getdefaults({"color": "r"}), cycle._idx) == ({}, 1)
    ax.plot([1, 2])
    assert (to_hex(ax.lines[0].get_color()), cycle._idx) == ("#ff7f0e", 2)
    cycle._idx = 7
    ax.plot([1, 2])
    assert (to_hex(ax.lines[1].get_color()), cycle._idx) == ("#7f7f7f", 8)

    ax.set_prop_cycle(color=["r", "g"], marker=["o", "x"])
    lines = ax._get_lines
    assert lines._prop_cycle._cycler_items == [{"color": "r", "marker": "o"},
                                               {"color": "g", "marker": "x"}]
    assert (lines.get_next_color(), lines._prop_cycle._idx) == ("r", 1)
    assert lines._prop_cycle.getdefaults({"color": "b"}) == {"color": "g", "marker": "x"}
    assert lines._prop_cycle.getdefaults({"color": "b", "marker": "s"}) == {}
    assert (ax._get_patches_for_fill.output, lines.output) == ("Polygon", "Line2D")

    ax._get_patches_for_fill.set_prop_cycle(cycler(color=["c", "m", "y"]))
    assert ax._get_patches_for_fill.get_next_color() == "c"
    assert ax.bar([1], [1])[0].get_facecolor() == (0.75, 0.0, 0.75, 1.0)
    assert lines.get_next_color() == "r", "the lines keep their cycle"

    twin = ax.twinx()
    twin._get_lines = ax._get_lines
    assert twin._get_lines._prop_cycle._cycler_items[0] == {"color": "r", "marker": "o"}
    assert twin._get_lines.get_next_color() == "g"
    assert ax._get_lines._prop_cycle._idx == 0

    ax.clear()
    assert ax._get_lines._prop_cycle._idx == 0
    assert to_hex(ax._get_patches_for_fill.get_next_color()) == "#1f77b4"


def test_a_transform_of_the_program_s_maps_a_line_when_it_is_plotted():
    calls = collections.Counter()

    class Shift(mtransforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            calls["non_affine"] += 1
            return np.asarray(values) + [0.0, 0.1]

        def inverted(self):
            return mtransforms.Affine2D().translate(0, -0.1)

    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1], [0, 1], transform=Shift() + ax.transData)
    assert calls["non_affine"] == 2, "the path, then its points"
    fig.canvas.draw()
    fig.canvas.draw()
    assert calls["non_affine"] == 2, "kept for the draws"


def test_a_box_style_of_the_program_s_is_asked_at_each_point_an_arrow_tries():
    calls = []

    class Round(BoxStyle.Round):
        def __call__(self, x0, y0, width, height, mutation_size):
            calls.append(None)
            return super().__call__(x0, y0, width, height, mutation_size)

    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.annotate("note", (0.2, 0.2), xytext=(0.6, 0.8), arrowprops=dict(arrowstyle="->"),
                bbox=dict(boxstyle=Round(pad=0.3)))
    calls.clear()
    fig.canvas.draw()
    # The outline drawn, and twice per point the clipping tries (its codes,
    # then its vertices).
    assert len(calls) == 35


def test_streamlines_map_each_arrow_through_the_program_s_norm():
    x, y = np.meshgrid(np.linspace(0, 1, 5), np.linspace(0, 1, 4))
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    norm, cmap = _Recorded(), _RecordedMap()
    stream = ax.streamplot(x, y, y, x, color=x * y, norm=norm, cmap=cmap)
    scalars = [call for call in norm.calls if call == ("norm", ())]
    assert len(scalars) == len(stream.arrows.get_paths()) == 64
    assert cmap.calls[:2] == [((), False)] * 2
    fig.savefig(io.BytesIO(), format="png")


@pytest.mark.parametrize("restyle, reds, blues", [
    (lambda pane: pane.set_color("red"), 73018, 0),
    (lambda pane: setattr(pane, "fill", False), 0, 0),
    (lambda pane: (pane.set_fill(False), pane.set_edgecolor("red")), 1425, 0),
    (lambda pane: (pane.set_edgecolor("b"), pane.set_linewidth(3)), 0, 5101),
])
def test_a_3d_pane_is_drawn_as_its_polygon_looks(restyle, reds, blues):
    # pytransform3d's `make_3d_axis`: `axis.pane.set_color("white")`.
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    for axis in [ax.xaxis, ax.yaxis, ax.zaxis]:
        restyle(axis.pane)
    fig.canvas.draw()
    rgba = np.asarray(fig.canvas.buffer_rgba()).astype(int)
    assert (rgba[..., 0] - rgba[..., 1] > 50).sum() == reds
    assert (rgba[..., 2] - rgba[..., 1] > 50).sum() == blues
    assert type(ax.xaxis.pane).__name__ == "Polygon"


def test_size_keywords_a_program_adds_size_texts():
    # ultraplot's `font_scalings["med-large"] = 1.1`, with the validator
    # of the rcParam replaced to take it.
    mfont_manager.font_scalings["med-large"] = 1.1
    validators = pltrs.RcParams.validate
    kept = validators["axes.titlesize"]
    validators["axes.titlesize"] = str
    try:
        pltrs.rcParams["axes.titlesize"] = "med-large"
        fig, ax = plt.subplots()
        assert ax.set_title("t").get_size() == pytest.approx(11.0)
    finally:
        validators["axes.titlesize"] = kept
        del mfont_manager.font_scalings["med-large"]


def test_colour_names_of_a_mapping_put_in_place_of_matplotlib_s():
    # ultraplot replaces `_colors_full_map` with its own database.
    class Names(dict):
        cache = {}

    names = Names(mcolors._colors_full_map)
    names["blue7"] = "#1c7ed6"
    kept = mcolors._colors_full_map
    mcolors._colors_full_map = names
    try:
        fig, ax = plt.subplots()
        (line,) = ax.plot([0, 1], color="blue7")
        assert to_hex(line.get_color()) == "#1c7ed6"
        pltrs.rcParams["lines.color"] = "blue7"
        fig.canvas.draw()
    finally:
        mcolors._colors_full_map = kept


def test_a_figure_s_canvas_callbacks_exist_before_set_canvas():
    seen = []

    class Watching(Figure):
        def set_canvas(self, canvas):
            seen.append(type(self._canvas_callbacks).__name__)
            super().set_canvas(canvas)

    FigureCanvasAgg(Watching())
    assert seen and set(seen) == {"CallbackRegistry"}


def test_attributes_programs_assign():
    fig, (ax, other) = plt.subplots(1, 2)
    title = fig.text(0.5, 0.95, "", ha="center")
    fig._suptitle = title
    assert fig._suptitle is title
    ax._sharex = other
    assert ax._sharex is other
    scale = other.yaxis._scale
    ax.yaxis._scale = scale
    assert ax.yaxis._scale is scale
    ax._set_title_offset_trans(20)
    assert ax.titleOffsetTrans.get_matrix()[1, 2] == pytest.approx(20 / 72 * fig.dpi)
    line = ax.plot([0, 1])[0]
    line._rasterized = True
    assert line.get_rasterized()
    text = ax.text(0, 0, "a\nb")
    text._linespacing = 2.0
    assert text.get_linespacing() == 2.0
    fig.clear()
    assert fig._suptitle is None


def test_axes_join_another_s_group_as_the_grouper_joins_them():
    fig, (ax, other) = plt.subplots(1, 2)
    other.set_xlim(0, 7)
    ax._shared_axes["x"].join(ax, other)
    assert ax._shared_axes["x"].joined(ax, other)
    other.set_xlim(0, 5)
    assert ax.get_xlim() == (0, 5)


class _OwnGrid(GridSpec):
    """A grid laid out by its class alone, made without GridSpec's
    constructor (ultraplot's)."""

    def __init__(self, nrows, ncols):
        self._rows, self._cols = nrows, ncols

    def __getattr__(self, name):
        return super().__getattribute__(name)

    def get_geometry(self):
        return self._rows, self._cols

    def get_grid_positions(self, figure):
        tops = 1 - np.arange(self._rows) / self._rows - 0.05
        bottoms = tops - 0.8 / self._rows
        lefts = np.arange(self._cols) / self._cols + 0.05
        rights = lefts + 0.8 / self._cols
        return bottoms, tops, lefts, rights


class _OwnSpec(SubplotSpec):
    pass


def test_a_grid_of_the_program_s_class_places_its_axes():
    fig = plt.figure()
    grid = _OwnGrid(1, 2)
    spec = _OwnSpec(grid, 1)
    ax = fig.add_subplot(spec)
    assert ax.get_subplotspec() is spec
    assert np.allclose(ax.get_position().extents, [0.55, 0.15, 0.95, 0.95])


def test_a_rotation_the_program_computes_is_the_one_drawn():
    class Turning(Text):
        @property
        def _rotation(self):
            return self.__dict__.get("_turn", 0)

        @_rotation.setter
        def _rotation(self, value):
            pass

    fig, ax = plt.subplots()
    text = Turning(0.5, 0.5, "turning")
    ax._add_text(text)
    fig.canvas.draw()
    flat = text.get_window_extent()
    text._turn = 90
    fig.canvas.draw()
    turned = text.get_window_extent()
    assert turned.height > flat.height


class _Day:
    def __init__(self, day):
        self.day = day


class _DayConverter(munits.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        if isinstance(value, str):
            return float(value.split("-")[-1])
        if isinstance(value, _Day):
            return float(value.day)
        return [float(v.day) for v in value]

    @staticmethod
    def default_units(x, axis):
        return "days"


def test_names_go_to_the_axis_converter_of_the_program_s():
    munits.registry[_Day] = _DayConverter()
    try:
        fig, ax = plt.subplots()
        ax.plot([_Day(1), _Day(30)], [0, 1])
        note = ax.annotate("x", xy=("2018-07-27", 0.5))
        fig.savefig(io.BytesIO(), format="png")
        assert note.xy == ("2018-07-27", 0.5)
    finally:
        del munits.registry[_Day]


def test_reference_lines_labelled_none_stay_out_of_legends():
    fig, ax = plt.subplots()
    lines = [ax.axvline(1, label=None), ax.axhline(1, label=None)]
    assert [line.get_label() for line in lines] == ["_child0", "_child1"]
    assert ax.get_legend_handles_labels() == ([], [])


def test_a_box_given_the_figure_before_it_is_added_joins_it():
    # legendkit's colour bars: `box.set_figure(fig)`, then `add_artist`.
    class Area(DrawingArea):
        def draw(self, renderer):
            super().draw(renderer)

    fig, ax = plt.subplots()
    area = Area(20, 20)
    area.set_figure(fig)
    box = AnchoredOffsetbox("upper left", child=area, frameon=False)
    box.set_figure(fig)
    ax.add_artist(box)
    fig.canvas.draw()
    assert box.get_window_extent().width > 0


def test_an_empty_offset_text_measures_as_nothing_at_its_place():
    # mplhep leaves room for the y-axis offset text: 1.1 times its width.
    fig, ax = plt.subplots()
    ax.plot([1, 2], [3, 4])
    renderer = fig.canvas.get_renderer()
    fig.draw(renderer)
    extent = ax.yaxis.offsetText.get_window_extent(renderer)
    assert (extent.width, extent.height) == (0, 0)
    assert extent.x0 == pytest.approx(80.0)
    assert extent.y0 == pytest.approx(426.567, abs=1e-3)


def test_the_next_cycle_colour_is_the_entry_as_the_cycler_holds_it():
    fig, ax = plt.subplots()
    first = ax._get_lines.get_next_color()
    assert type(first) is tuple and len(first) == 3
    assert to_hex(first) == "#1f77b4"
    assert len(ax._get_patches_for_fill.get_next_color()) == 3
    ax.set_prop_cycle(color=[(1, 0, 0), (0, 1, 0)])
    assert ax._get_lines.get_next_color() == (1, 0, 0)
    assert ax._get_patches_for_fill.get_next_color() == (1, 0, 0)
    ax.set_prop_cycle(color=["r", (0, 0, 1, 0.5)])
    assert [ax._get_lines.get_next_color() for _ in range(2)] == ["r", (0, 0, 1, 0.5)]
    ax.set_prop_cycle(linestyle=["-", "--"])
    assert ax._get_lines.get_next_color() == "k"


def test_a_3d_pane_is_typed_as_matplotlib_s_polygon():
    ax = plt.figure().add_subplot(projection="3d")
    pane_type = type(ax.zaxis.pane)
    assert (pane_type.__module__, pane_type.__qualname__) == (
        Polygon.__module__, "Polygon")
