"""
Wrong and unusual arguments to the scales, triangulations, legends, font
manager, colormaps, lines, quivers, widgets and DVI fonts, as the
differential tester's hostile and api programs compare them with
matplotlib 3.11: matplotlib computes with what it is given as it is (a
list, a data frame, a scale that is no integer), keeps values it fails on
only when it draws, and fails with its own messages. Each expected value
is recorded from matplotlib.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import (backend_bases, colors, dviread, font_manager, lines, mlab, scale,
                   transforms, tri, widgets)
from pltrs._fontconfig_pattern import parse_fontconfig_pattern
from pltrs.collections import PolyCollection, TriMesh
from pltrs.figure import Figure
from pltrs.mathtext import MathTextParser


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


# -- scale transforms compute with what they are given --------------------------

def test_scale_maps_compute_with_lists_and_frames_as_numpy_does():
    pd = pytest.importorskip("pandas")
    asinh = scale.AsinhTransform(1)
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for /: 'list' and 'int'"):
        asinh.transform_non_affine([[0.0, 1.0], [2.0, 3.0]])
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for /: 'NoneType' and 'int'"):
        asinh.transform_non_affine(None)
    frame = asinh.transform_non_affine(pd.DataFrame([[0.0, 1.0], [2.0, 3.0]]))
    assert type(frame) is pd.DataFrame
    assert np.allclose(frame.to_numpy(), np.arcsinh([[0.0, 1.0], [2.0, 3.0]]))
    logistic = scale.LogisticTransform()
    with pytest.raises(ValueError, match="^Integers to negative integer powers are not allowed.$"):
        logistic.transform_non_affine(np.array([[0, 1], [2, 3]]))
    with pytest.raises(TypeError, match="^The numpy boolean negative"):
        logistic.transform_non_affine(np.array([False, True]))
    symlog = scale.SymmetricalLogTransform(10, 1, 1)
    with pytest.raises(TypeError, match="^only integer scalar arrays can be converted"):
        symlog.transform_non_affine([[0.0, 1.0], [2.0, 3.0]])
    assert np.allclose(symlog.transform_non_affine(np.array([0.5, 5.0])),
                       [0.5555555555555556, 1.8100811245797862])


# -- affine maps check their points as matplotlib's C++ does --------------------

def test_affine_maps_refuse_points_with_matplotlibs_messages():
    affine = transforms.Affine2D().scale(2)
    with pytest.raises(ValueError, match="^vertices must be 1D or 2D, not0D$"):
        affine.transform_affine(None)
    with pytest.raises(ValueError, match=r"^vertices must have shape \(N, 2\), got \(2, 3\)$"):
        affine.transform([[1, 2, 3], [4, 5, 6]])
    with pytest.raises(TypeError, match=r"^affine_transform\(\): incompatible function "
                                        "arguments. The following argument types"):
        affine.transform('data')
    assert affine.transform(np.empty((0, 3))).shape == (0, 2)


# -- triangulations -------------------------------------------------------------

def test_refiners_loop_as_range_loops():
    triangulation = tri.Triangulation([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5])
    refiner = tri.UniformTriRefiner(triangulation)
    assert refiner.refine_triangulation(subdiv=0) is triangulation
    assert refiner.refine_triangulation(subdiv=-1) is triangulation
    assert refiner.refine_field([1, 2, 3, 4, 0], subdiv=0)[0] is triangulation
    with pytest.raises(TypeError, match="^'float' object cannot be interpreted as an integer$"):
        refiner.refine_triangulation(subdiv=3.5)
    with pytest.raises(TypeError, match="^'NoneType' object cannot be interpreted as an integer$"):
        refiner.refine_field([1, 2, 3, 4, 0], subdiv=None)


def test_triangulation_helpers_fail_with_matplotlibs_messages():
    triangulation = tri.Triangulation([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5])
    message = ("^z must be a 1D array with the same length as the triangulation x and y "
               "arrays$")
    with pytest.raises(ValueError, match=message):
        triangulation.calculate_plane_coefficients([1])
    with pytest.raises(TypeError, match=r"^calculate_plane_coefficients\(\): incompatible"):
        triangulation.calculate_plane_coefficients('ab')
    with pytest.raises(AttributeError, match="'int' object has no attribute 'get_masked_triangles'"):
        TriMesh.convert_mesh_to_paths(3)
    with pytest.raises(TypeError, match="'<' not supported between instances of 'float' and 'NoneType'"):
        tri.TriAnalyzer(triangulation).get_flat_tri_mask(min_circle_ratio=None)


# -- legends ----------------------------------------------------------------------

def test_a_legend_frame_keeps_what_it_is_given_and_measures_with_renderers_only():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label='a')
    legend = ax.legend()
    legend.set_frame_on(None)
    assert legend.get_frame_on() is None
    legend.set_frame_on(False)
    legend.set_frame_on(1)
    assert legend.get_frame_on() == 1 and type(legend.get_frame_on()) is int
    for measure in (legend.get_window_extent, legend.get_tightbbox):
        with pytest.raises(AttributeError, match="'int' object has no attribute 'points_to_pixels'"):
            measure(renderer=3)
    legend.remove()
    legend.get_tightbbox()


def test_a_legend_holds_its_box_when_its_transform_is_none():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label='a')
    legend = ax.legend()
    with pytest.raises(TypeError):
        legend.set_bbox_to_anchor((1, 1), transform=3)
    anchor = legend.get_bbox_to_anchor()
    assert type(anchor) is transforms.Bbox
    assert anchor.get_points().tolist() == [[1.0, 1.0], [1.0, 1.0]]


# -- the font manager and text parsers -------------------------------------------

def test_font_scores_fail_on_names_that_are_no_strings():
    manager = font_manager.fontManager
    with pytest.raises(AttributeError, match="'int' object has no attribute 'lower'"):
        manager.score_family([0], 'DejaVu Serif')
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'lower'"):
        manager.score_family(['serif'], None)
    assert manager.score_family(['serif', 3], 'DejaVu Serif') == 0.0
    with pytest.raises(TypeError, match=r"^int\(\) argument must be a string"):
        manager.score_stretch(None, 'normal')
    with pytest.raises(TypeError, match="^argument should be a str or an os.PathLike object"):
        manager.findfont(font_manager.FontProperties(family='serif'), directory=0)


def test_parsers_expand_tabs_first():
    for parse in (parse_fontconfig_pattern, MathTextParser('path').parse):
        with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'expandtabs'"):
            parse(None)


def test_a_density_estimate_subtracts_its_points_first():
    kde = mlab.GaussianKDE(np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5]))
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for -: 'float' and 'NoneType'"):
        kde.evaluate(None)
    with pytest.raises(TypeError, match="did not contain a loop with signature matching types"):
        kde('data')


# -- canvases, managers, colormaps and rcParams -----------------------------------

def test_canvases_and_managers_take_their_arguments_as_matplotlib():
    with pytest.raises(AttributeError, match="'int' object has no attribute 'set_canvas'"):
        backend_bases.FigureCanvasBase(3)
    with pytest.raises(ValueError, match="^Unknown format code 'd' for object of type 'float'$"):
        backend_bases.FigureManagerBase.create_with_canvas(
            backend_bases.FigureCanvasBase, Figure(), 3.5)


def test_colormaps_sample_with_matplotlibs_gammas():
    cmap = colors.LinearSegmentedColormap.from_list('rb', ['r', 'b'], N=16)
    with pytest.raises(IndexError, match="^index 2 is out of bounds for axis 0 with size 2$"):
        with pytest.warns(RuntimeWarning, match="divide by zero encountered in power"):
            cmap.set_gamma(-1.5)
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for \*\* or pow\(\)"):
        cmap.set_gamma(None)
    cmap.set_gamma(0.5)
    assert np.allclose(cmap(0.25), (0.4836022205056778, 0.0, 0.5163977794943222, 1.0))


def test_registries_keep_names_that_are_no_strings():
    registry = pltrs.colormaps
    registry.register(colors.ListedColormap(['r', 'g'], name='mine'), name=3)
    assert 3 in registry
    registry.unregister(3)
    registry.unregister(None)
    assert 3 not in registry
    message = "^\"None is not a valid value for rcParam. Supported values are 'backend'"
    with pytest.raises(KeyError, match=message):
        pltrs.rcParams[None] = 2.5


def test_colorbars_and_collections_read_what_they_are_given_first():
    fig, ax = plt.subplots()
    image = ax.imshow(np.ones((2, 2)))
    colorbar = fig.colorbar(image)
    with pytest.raises(AttributeError, match="'int' object has no attribute 'norm'"):
        colorbar.update_normal(mappable=3)
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute '_transform'"):
        PolyCollection([]).update_from(None)


# -- lines keep markers' colours and data they draw only --------------------------

def test_a_line_keeps_what_its_markers_and_data_fail_to_draw_with():
    fig, ax = plt.subplots()
    line = ax.plot([1, 2, 3], [1, 3, 2], 'o-')[0]
    fig.canvas.draw()
    line.set(markeredgecolor=3)
    assert line.get_markeredgecolor() == 3
    with pytest.raises(ValueError, match="^Invalid RGBA argument: 3$"):
        fig.canvas.draw()
    line.set(markeredgecolor='k')
    line.set(xdata='ab')
    assert line.get_xdata() == 'ab'
    with pytest.raises(pltrs.units.ConversionError,
                       match="^Failed to convert value\\(s\\) to axis units: 'ab'$"):
        fig.canvas.draw()
    line.set(xdata=[3, 2, 1])
    fig.canvas.draw()
    with pytest.raises(TypeError, match=r"^float\(\) argument must be a string or a real number"):
        line.set(markersize=None)
    line.set(markevery='2')
    assert line.get_markevery() == '2'
    with pytest.raises(ValueError, match="^markevery='2' is iterable but not a valid numpy "
                                         "fancy index$"):
        fig.canvas.draw()
    line.set(marker='')
    fig.canvas.draw()
    line.set(marker='o', markevery=(1, 2, 3))
    with pytest.raises(ValueError, match=r"^`markevery` is a tuple but its len is not 2"):
        fig.canvas.draw()
    line.set(markevery=2)
    fig.canvas.draw()
    assert isinstance(line, lines.Line2D)


# -- what draws as matplotlib's does ---------------------------------------------

def test_draws_fail_on_spines_that_are_no_artists_and_skip_indicator_filters():
    fig, ax = plt.subplots()
    ins = ax.inset_axes([0.6, 0.6, 0.3, 0.3])
    indicator = ax.indicate_inset_zoom(ins)
    indicator.set_agg_filter(3)
    fig.canvas.draw()
    ax.spines['extra'] = 'x'
    with pytest.raises(AttributeError, match="'str' object has no attribute 'get_animated'"):
        fig.canvas.draw()
    with pytest.raises(AttributeError, match="'str' object has no attribute 'zorder'"):
        fig.savefig(io.BytesIO(), format='svg')


def test_widgets_and_picks_read_what_they_are_given():
    fig, ax = plt.subplots()
    buttons = widgets.CheckButtons(ax, ['a', 'b'])
    with pytest.raises(TypeError, match="^'int' object is not iterable$"):
        buttons.set_label_props({'nonexistent_key': 1})
    sub = fig.subfigures(1, 2)[0]
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'inaxes'"):
        sub.pick(None)


# -- DVI fonts keep the scale they are given ---------------------------------------

def test_dvi_fonts_keep_a_scale_that_is_no_integer():
    try:
        tfm = dviread.Tfm(dviread.find_tex_file('cmr10.tfm'))
    except FileNotFoundError:
        pytest.skip("no TeX installation")
    font = dviread.DviFont(scale=2.0, metrics=tfm, texname=b'cmr10', vf=None)
    assert font.size == 2.0 * (72.0 / (72.27 * 2**16))
    assert font._metrics is tfm
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for >>: 'float' and 'int'"):
        font._width_of(65)
    exact = dviread.DviFont(655360, tfm, b'cmr10', None)
    assert exact.size == 655360 * (72.0 / (72.27 * 2**16))
    assert exact._width_of(65) == 491521
