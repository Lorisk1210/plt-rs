"""
matplotlib's public surface as `cargo xtask diff --docs` compares it, on a
sample of each kind of difference that slipped through while classes were
reshaped (task DOCS4): signatures (of a class and its constructor),
docstrings shown through bases (classes, nested classes, constructors,
methods of nested classes), what `ArtistInspector` reads for ``set`` and
``setp``, accessors' qualified names, and module-level names, with what
the names added do. Every expected value was recorded from matplotlib.
"""

import inspect
import os
import sys
import warnings

import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import artist, axis, layout_engine, table
from pltrs.artist import Artist, ArtistInspector
from pltrs.backends import backend_mixed
from pltrs.figure import Figure, FigureBase
from pltrs.patches import ArrowStyle, BoxStyle, ConnectionStyle
from pltrs.projections.geo import LambertAxes, MollweideAxes
from pltrs.transforms import Bbox
from pltrs.mpl_toolkits.axes_grid1 import inset_locator
from pltrs.mpl_toolkits.axisartist import axis_artist, axislines
from pltrs.mpl_toolkits.axisartist.axisline_style import AxislineStyle
from pltrs.mpl_toolkits.mplot3d import axis3d


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _first_line(obj):
    """The first line ``help`` shows for *obj*."""
    return inspect.getdoc(obj).splitlines()[0]


@pytest.mark.parametrize("cls", [axis3d.Axis, axis3d.XAxis, axis3d.YAxis, axis3d.ZAxis])
def test_3d_axis_signatures(cls):
    assert str(inspect.signature(cls)) == "(axes, *, rotate_label=None, **kwargs)"
    assert (str(inspect.signature(cls.__init__))
            == "(self, axes, *, rotate_label=None, **kwargs)")


@pytest.mark.parametrize("cls, first", [
    (LambertAxes.InvertedLambertTransform,
     "The base class of all `TransformNode` instances that"),
    (inset_locator.BboxPatch, "A patch is a 2D artist with a face color and an edge color."),
    (inset_locator.BboxConnector,
     "A patch is a 2D artist with a face color and an edge color."),
    (inset_locator.BboxConnectorPatch,
     "A patch is a 2D artist with a face color and an edge color."),
])
def test_classes_without_a_docstring_show_their_bases(cls, first):
    assert cls.__doc__ is None
    assert _first_line(cls) == first


def test_renderers_show_renderer_base_docstring():
    from pltrs.backends.backend_pdf import RendererPdf
    from pltrs.backends.backend_svg import RendererSVG
    for cls in (RendererPdf, RendererSVG):
        assert cls.__doc__ is None
        assert _first_line(cls) == "An abstract base class to handle drawing/rendering operations."


def test_inverted_lambert_constructor_shows_geo_transform_docstring():
    init = LambertAxes.InvertedLambertTransform.__init__
    assert init.__doc__ is None
    assert _first_line(init) == "Create a new geographical transform."


AGG_FILTER = ("a filter function, which takes a (m, n, 3) float array and a dpi "
              "value, and returns a (m, n, 3) array and two offsets from the bottom "
              "left corner of the image")


#: The properties matplotlib's ``set`` docstring lists as ``unknown``: those
#: whose setter the class defined without a docstring (which Python cannot
#: look up on a base while the class is being made), and no others.
UNKNOWN = {
    axis.Axis: ['clip_path', 'data_interval', 'inverted', 'label', 'label_coords',
                'remove_overlapping_locs', 'tick_params', 'view_interval'],
    axis.XAxis: ['data_interval', 'inverted', 'label', 'label_coords',
                 'remove_overlapping_locs', 'tick_params', 'view_interval'],
    axis.Tick: ['clip_path'],
    axis.XTick: [],
    table.Cell: ['angle', 'figure', 'fontsize', 'hatch_linewidth', 'height', 'text_props',
                 'transform', 'width', 'x', 'y'],
    axis3d.Axis: ['data_interval', 'inverted', 'label', 'label_coords',
                  'remove_overlapping_locs', 'rotate_label', 'tick_params',
                  'view_interval'],
    axis_artist.Ticks: ['color or c', 'locs_angles', 'tick_direction', 'tick_out',
                        'ticksize'],
    axis_artist.LabelBase: [],
}


@pytest.mark.parametrize("cls", UNKNOWN, ids=lambda cls: cls.__qualname__)
def test_set_docstrings_read_setters_as_inherited(cls):
    lines = [line.strip() for line in cls.set.__doc__.splitlines()]
    assert [line.removesuffix(": unknown") for line in lines
            if line.endswith(": unknown")] == UNKNOWN[cls]
    assert f"agg_filter: {AGG_FILTER}" in lines
    assert "visible: bool" in lines
    assert ArtistInspector(cls).get_valid_values("agg_filter") == AGG_FILTER


def test_axisartist_ticks_set_and_inspector():
    lines = [line.strip() for line in axis_artist.Ticks.set.__doc__.splitlines()]
    assert "markeredgewidth or mew: float" in lines
    inspector = ArtistInspector(axis_artist.Ticks)
    assert inspector.get_valid_values("markeredgewidth") == "float"
    assert inspector.get_valid_values("color") == ":mpltype:`color`"
    assert axis_artist.Ticks.set_markeredgewidth.__qualname__ == "Line2D.set_markeredgewidth"


def test_property_accessors_are_matplotlib_functions():
    assert Artist.figure.fget.__qualname__ == "Artist.get_figure"
    assert Artist.figure.fset.__qualname__ == "Artist.set_figure"
    assert Figure.dpi.fget.__qualname__ == "Figure._get_dpi"
    assert Figure.dpi.fset.__qualname__ == "Figure._set_dpi"
    assert FigureBase.figure.fset.__qualname__ == "FigureBase.set_figure"


def test_figure_property_reads_where_the_artists_are():
    fig, ax = plt.subplots()
    sub = fig.subfigures(1, 2)[0]
    line, = ax.plot([0, 1])
    legend = ax.legend(["a"])
    tab = ax.table(cellText=[["1"]])
    assert line.figure is fig and legend.figure is fig and ax.xaxis.figure is fig
    assert ax.xaxis.get_major_ticks()[0].figure is fig
    assert tab.figure is fig and tab[0, 0].figure is fig
    assert sub.figure is fig and fig.figure is fig
    fig.dpi = 50
    assert fig.dpi == 50 and fig.stale


@pytest.mark.parametrize("owner, name, method", [
    (BoxStyle.Square, "BoxStyle.Square", "__call__"),
    (BoxStyle.LArrow, "BoxStyle.LArrow", "__call__"),
    (ConnectionStyle.Arc3, "ConnectionStyle.Arc3", "connect"),
    (ArrowStyle.Simple, "ArrowStyle.Simple", "transmute"),
    (MollweideAxes.MollweideTransform, "MollweideAxes.MollweideTransform", "inverted"),
    (LambertAxes.LambertTransform, "LambertAxes.LambertTransform", "transform_non_affine"),
])
def test_nested_classes_define_their_own_methods(owner, name, method):
    function = vars(owner)[method]
    assert function.__qualname__ == f"{name}.{method}"
    assert function.__doc__ is None


def test_nested_class_methods_show_matplotlib_docstrings():
    assert inspect.getdoc(ConnectionStyle.Arc3.connect) is None
    assert _first_line(BoxStyle.Round.__call__) == "Call self as a function."
    assert _first_line(ArrowStyle.Fancy.transmute) == (
        "The transmute method is the very core of the ArrowStyle class and")
    assert _first_line(ArrowStyle.CurveA.transmute) == (
        "The transmute method is the very core of the ArrowStyle class and")
    with pytest.raises(NotImplementedError, match="Derived must override"):
        ArrowStyle._Base().transmute(None, 1, 1)


def test_styles_still_draw():
    path = BoxStyle.Round(pad=0.2)(0, 0, 1, 1, 1)
    assert path.vertices[:2].tolist() == [[0.0, -0.2], [1.0, -0.2]]
    path = ConnectionStyle.Arc3(rad=0.2)((0, 0), (1, 1))
    assert path.vertices[:2].tolist() == [[0.0, 0.0], [0.7, 0.3]]


def test_module_level_names():
    assert callable(layout_engine.do_constrained_layout)
    assert callable(layout_engine.get_tight_layout_figure)
    assert callable(backend_mixed.process_figure_for_rasterizing)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        from pltrs.style import core
    assert os.path.basename(core.BASE_LIBRARY_PATH) == "stylelib"
    assert os.path.isfile(os.path.join(core.BASE_LIBRARY_PATH, "ggplot.mplstyle"))
    assert "BASE_LIBRARY_PATH" in core.__all__


def test_font_tools_is_read_where_installed(tmp_path, monkeypatch):
    package = tmp_path / "fontTools"
    package.mkdir()
    (package / "__init__.py").write_text("")
    (package / "agl.py").write_text("")
    monkeypatch.syspath_prepend(str(tmp_path))
    for name in ("fontTools", "fontTools.agl"):
        monkeypatch.delitem(sys.modules, name, raising=False)
    from pltrs import dviread
    from pltrs.backends import backend_ps
    assert backend_ps.fontTools.__file__ == str(package / "__init__.py")
    assert dviread.fontTools.agl.__file__ == str(package / "agl.py")


def _rounded(params):
    return {key: round(float(value), 6) for key, value in params.items()}


def test_get_tight_layout_figure():
    fig, axs = plt.subplots(1, 2, figsize=(6, 4))
    for ax in axs:
        ax.set_ylabel("ylabel")
        ax.set_title("title")
    specs = layout_engine.get_subplotspec_list(fig.axes)
    renderer = fig.canvas.get_renderer()
    assert _rounded(layout_engine.get_tight_layout_figure(fig, fig.axes, specs, renderer)) == {
        'left': 0.111024, 'right': 0.956667, 'bottom': 0.096528, 'top': 0.909167,
        'wspace': 0.361189}
    assert _rounded(layout_engine.get_tight_layout_figure(
        fig, fig.axes, specs, renderer, pad=2, rect=(0, 0, 0.5, 1))) == {
        'left': 0.13232, 'right': 0.43537, 'bottom': 0.128472, 'top': 0.877222,
        'wspace': 1.977126}
    assert [round(v, 6) for v in axs[0].get_position().bounds] == [0.125, 0.11, 0.352273, 0.77]
    free = fig.add_axes((0.1, 0.1, 0.2, 0.2))
    with pytest.warns(UserWarning, match="not compatible with tight_layout"):
        assert layout_engine.get_tight_layout_figure(fig, [free], [None], renderer) == {}
    small = plt.figure(figsize=(1, 1))
    small.add_subplot().set_ylabel("a label\nover\nmany\nmany\nlines")
    with pytest.warns(UserWarning, match="The left and right margins cannot be made large"):
        assert layout_engine.get_tight_layout_figure(
            small, small.axes, layout_engine.get_subplotspec_list(small.axes),
            small.canvas.get_renderer(), pad=4) is None


def test_do_constrained_layout():
    fig, axs = plt.subplots(2, 2, figsize=(5, 4))
    for ax in axs.flat:
        ax.set_xlabel("x")
        ax.set_ylabel("y")
    grids = layout_engine.do_constrained_layout(
        fig, h_pad=0.04167 / 4, w_pad=0.04167 / 5, hspace=0.02, wspace=0.02)
    assert [round(v, 4) for v in axs[0, 0].get_position().bounds] == [
        0.1107, 0.6181, 0.35, 0.3583]
    assert isinstance(grids, dict) and grids
    with pytest.raises(TypeError, match="unsupported operand type"):
        layout_engine.do_constrained_layout(fig, 0.01, 0.01)
    free = plt.figure()
    free.add_axes((0.1, 0.1, 0.5, 0.5))
    with pytest.warns(UserWarning, match="There are no gridspecs with layoutgrids"):
        assert layout_engine.do_constrained_layout(free, 0.01, 0.01, 0.02, 0.02) is None


def _state(fig):
    return ([round(v, 4) for v in fig.bbox.bounds],
            [round(v, 4) for v in fig.transFigure._boxout.bounds],
            [round(v, 4) for v in fig.patch.get_bbox().bounds])


def test_process_figure_for_rasterizing():
    from pltrs import _tight_bbox
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.plot([0, 1])
    fig.canvas.draw()
    box = Bbox([[0.5, 0.25], [3.5, 2.75]])
    restore = _tight_bbox.adjust_bbox(fig, box, fig.canvas.get_renderer())
    assert _state(fig) == ([0.0, 0.0, 300.0, 250.0], [-50.0, -25.0, 400.0, 300.0],
                           [0.125, 0.0833, 0.75, 0.8333])
    _, restore = backend_mixed.process_figure_for_rasterizing(
        fig, (box, restore), fig.canvas.get_renderer(), fixed_dpi=200)
    assert _state(fig) == ([0.0, 0.0, 600.0, 500.0], [-100.0, -50.0, 800.0, 600.0],
                           [0.125, 0.0833, 0.75, 0.8333])
    restore()
    assert _state(fig) == ([0.0, 0.0, 400.0, 300.0], [0.0, 0.0, 400.0, 300.0],
                           [0.0, 0.0, 1.0, 1.0])
    assert "apply_aspect" not in vars(ax)


def test_axisline_styles_make_their_arrows():
    fig = plt.figure(figsize=(4, 3))
    ax = fig.add_subplot(axes_class=axislines.AxesZero)
    line = AxislineStyle.FilledArrow(size=2)(ax.axis["xzero"], ax.transAxes)
    assert type(line).__qualname__ == "_FancyAxislineStyle.FilledArrow"
    assert line.get_mutation_scale() == 2
    assert line.get_facecolor() == (0.0, 0.0, 0.0, 1.0)
    ax.add_artist(line)
    fig.canvas.draw()
    assert [round(v, 2) for v in line.get_window_extent().bounds] == [49.44, 33.0, 1.11, 231.45]
    line.set_line_mutation_scale(3)
    assert line.get_mutation_scale() == 6
    simple = AxislineStyle.SimpleArrow()(ax.axis["xzero"], ax.transAxes)
    assert type(simple).__qualname__ == "_FancyAxislineStyle.SimpleArrow"
    assert artist.Artist in type(simple).__mro__
