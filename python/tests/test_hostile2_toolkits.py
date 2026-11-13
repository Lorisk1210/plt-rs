"""
What the second round of the differential tester's hostile programs
(``spec/diff/generators/hostile.py``) found axisartist, axes_grid1 and the
texts of pltrs doing otherwise than matplotlib 3.11: values kept as given
and failing only where matplotlib computes with them, the errors of
misused methods naming matplotlib's classes, and the anchored artists
built as matplotlib builds them. Each expected value and error is
recorded from matplotlib 3.11.2.
"""

import math

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.textpath import TextPath
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import anchored_artists, axes_size
from pltrs.mpl_toolkits.axes_grid1 import mpl_axes
from pltrs.mpl_toolkits.axisartist.axis_artist import LabelBase

import build_facts


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def artist_axes():
    """A figure with an axisartist axes, drawn once."""
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.canvas.draw()
    return fig, ax


# -- axisartist -----------------------------------------------------------------

def test_the_draw_places_the_label_of_an_axis_artist_anew():
    fig, ax = artist_axes()
    label = ax.axis["bottom"].label
    label.set(position="ab")
    assert label.get_position() == ("a", "b")
    fig.canvas.draw()
    assert label.get_position() == (0.5, 0.0)


def test_tick_labels_read_back_what_is_set_after_a_draw():
    fig, ax = artist_axes()
    labels = ax.axis["bottom"].major_ticklabels
    labels.set_text("auto")
    assert labels.get_text() == "auto"
    labels.set_position(("c", "d"))
    assert labels.get_position() == ("c", "d")
    fig.canvas.draw()


@pytest.mark.parametrize("value", ["", 1, "False", np.True_, None])
def test_text_flags_read_back_as_given(value):
    fig, ax = artist_axes()
    for text in (ax.text(0, 0, "x"), ax.axis["bottom"].major_ticklabels):
        text.set_antialiased(value)
        text.set_wrap(value)
        text.set_transform_rotates_text(value)
        for got in (text.get_antialiased(), text.get_wrap(),
                    text.get_transform_rotates_text()):
            assert type(got) is type(value) and got == value
    fig.canvas.draw()


def test_a_background_of_none_takes_the_patch_defaults():
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, "x")
    assert text.set_backgroundcolor(None) is None
    patch = text.get_bbox_patch()
    assert patch.get_facecolor() == pytest.approx((0.12156862745098039, 0.4666666666666667,
                                                   0.7058823529411765, 1))
    assert patch.get_edgecolor() == (0.0, 0.0, 0.0, 1.0)
    boxed = ax.text(0.2, 0.2, "z", backgroundcolor="r")
    boxed.set_backgroundcolor(None)
    assert boxed.get_bbox_patch().get_edgecolor() == (1.0, 0.0, 0.0, 1)
    for props in ({"facecolor": None}, {"edgecolor": None}, {"color": None}):
        ax.text(0.1, 0.1, "y").set_bbox(props)
    fig.canvas.draw()


@pytest.mark.parametrize("call, message", [
    (lambda ticks, labels: ticks.set_mew(2, nonexistent_kw=1),
     "Line2D.set_markeredgewidth() got an unexpected keyword argument 'nonexistent_kw'"),
    (lambda ticks, labels: ticks.get_mfc(None),
     "Line2D.get_markerfacecolor() takes 1 positional argument but 2 were given"),
    (lambda ticks, labels: ticks.update_from(),
     "Line2D.update_from() missing 1 required positional argument: 'other'"),
    (lambda ticks, labels: ticks.get_path(nonexistent_kw=1),
     "Line2D.get_path() got an unexpected keyword argument 'nonexistent_kw'"),
    (lambda ticks, labels: ticks.get_window_extent(nonexistent_kw=1),
     "Line2D.get_window_extent() got an unexpected keyword argument 'nonexistent_kw'"),
    (lambda ticks, labels: labels.update_from(None, None),
     "Text.update_from() takes 2 positional arguments but 3 were given"),
    (lambda ticks, labels: labels.get_tightbbox(None, None),
     "Text.get_tightbbox() takes from 1 to 2 positional arguments but 3 were given"),
])
def test_misused_parts_name_matplotlibs_methods(call, message):
    fig, ax = artist_axes()
    axis = ax.axis["bottom"]
    with pytest.raises(TypeError) as error:
        call(axis.major_ticks, axis.major_ticklabels)
    assert str(error.value) == message


def test_misused_axis_artists_and_sizes_name_matplotlibs_methods():
    fig, ax = artist_axes()
    with pytest.raises(TypeError, match=r"^Artist\.contains\(\) missing 1 required"):
        ax.axis["bottom"].contains()
    simple = mpl_axes.SimpleAxisArtist(ax.xaxis, 1, ax.spines["bottom"])
    with pytest.raises(TypeError, match=r"^Artist\.contains\(\) got an unexpected"):
        simple.contains(None, nonexistent_kw=1)
    for cls in (axes_size.AxesX, axes_size.AxesY):
        with pytest.raises(TypeError, match=rf"^{cls.__name__}\.__init__\(\) missing 1"):
            cls()


@pytest.mark.parametrize("renderer, error, message", [
    (None, TypeError, "cannot create weak reference to 'NoneType' object"),
    (3, TypeError, "cannot create weak reference to 'int' object"),
    ("x", TypeError, "cannot create weak reference to 'str' object"),
])
def test_tick_labels_measure_with_the_renderer_given(renderer, error, message):
    fig, ax = artist_axes()
    labels = ax.axis["bottom"].major_ticklabels
    with pytest.raises(error, match=message):
        labels.get_texts_widths_heights_descents(renderer)
    if renderer is not None:
        with pytest.raises(error, match=message):
            labels.get_window_extents(renderer=renderer)
        with pytest.raises(error, match=message):
            LabelBase.get_window_extent(labels, renderer=renderer)
    labels.set_bbox({"fc": "r"})
    with pytest.raises(error, match=message):
        labels.update_bbox_position_size(renderer)


def test_figures_are_no_renderers_of_text_metrics():
    fig, ax = artist_axes()
    with pytest.raises(AttributeError,
                       match="'Figure' object has no attribute 'get_text_width_height_descent'"):
        ax.axis["bottom"].major_ticklabels.get_texts_widths_heights_descents(fig)


@pytest.mark.parametrize("renderer", [3, "x"])
def test_axis_artists_place_their_ticks_with_the_renderer_given(renderer):
    fig, ax = artist_axes()
    with pytest.raises(AttributeError, match="object has no attribute 'points_to_pixels'"):
        ax.axis["bottom"].get_tightbbox(renderer=renderer)


@pytest.mark.parametrize("which, axis, message", [
    ("grid", "both", "'grid' is not a valid value for which"),
    (None, "both", "None is not a valid value for which"),
    ("major", "z", "'z' is not a valid value for axis"),
    ("major", 3, "3 is not a valid value for axis"),
])
def test_gridlines_fail_to_draw_on_settings_their_helper_refuses(which, axis, message):
    fig, ax = artist_axes()
    gridlines = ax.gridlines
    gridlines.set_which(which)
    gridlines.set_axis(axis)
    with pytest.raises(ValueError, match=message):
        fig.canvas.draw()
    gridlines.set_animated(True)
    fig.canvas.draw()
    gridlines.set_animated(False)
    gridlines.set_which("major")
    gridlines.set_axis("both")
    fig.canvas.draw()


@pytest.mark.parametrize("helper, name", [(3, "int"), ("x", "str")])
def test_gridlines_fail_to_draw_with_a_helper_of_no_limits(helper, name):
    fig, ax = artist_axes()
    ax.gridlines.set_grid_helper(helper)
    with pytest.raises(AttributeError, match=f"'{name}' object has no attribute 'update_lim'"):
        fig.canvas.draw()
    ax.gridlines.set_grid_helper(ax.get_grid_helper())
    fig.canvas.draw()


# -- the anchored artists of axes_grid1 -----------------------------------------

@pytest.mark.parametrize("transform, message", [
    (None, "unsupported operand type(s) for +: 'NoneType' and 'Affine2D'"),
    (3, "unsupported operand type(s) for +: 'int' and 'Affine2D'"),
    ("x", 'can only concatenate str (not "Affine2D") to str'),
])
def test_direction_arrows_compose_their_transform_when_made(transform, message):
    with pytest.raises(TypeError) as error:
        anchored_artists.AnchoredDirectionArrows(transform, "X", "Y")
    assert str(error.value) == message


@pytest.mark.parametrize("kwargs, error, message", [
    ({"length": "4"}, TypeError, "can't multiply sequence by non-int of type 'float'"),
    ({"length": None}, TypeError, "unsupported operand type(s) for *: 'NoneType' and 'int'"),
    ({"angle": float("inf")}, ValueError, build_facts.raised(lambda: math.cos(math.inf))),
    ({"angle": None}, TypeError, "must be real number, not NoneType"),
    ({"color": ""}, ValueError, "Invalid RGBA argument: ''"),
    ({"alpha": -1.5}, ValueError, "alpha (-1.5) is outside 0-1 range"),
    ({"sep_x": None}, TypeError, "unsupported operand type(s) for +: 'float' and 'NoneType'"),
    ({"text_props": 0}, TypeError, build_facts.raised(lambda: "size" in 0)),
    ({"text_props": "x"}, TypeError, "'str' object does not support item assignment"),
])
def test_direction_arrows_fail_as_matplotlibs_are_made(kwargs, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error) as raised:
        anchored_artists.AnchoredDirectionArrows(ax.transAxes, "X", "Y", **kwargs)
    assert str(raised.value) == message


@pytest.mark.parametrize("kwargs", [{"fontsize": None}, {"alpha": None},
                                    {"head_width": None}, {"tail_width": None}])
def test_direction_arrows_keep_what_matplotlib_computes_later(kwargs):
    fig, ax = plt.subplots()
    arrows = anchored_artists.AnchoredDirectionArrows(ax.transAxes, "X", "Y", **kwargs)
    for name in ("arrow_x", "arrow_y", "p_x", "p_y", "box"):
        assert getattr(arrows, name) is not None


def test_direction_arrows_and_size_bars_measure_as_matplotlibs():
    fig, ax = plt.subplots(figsize=(6, 6), dpi=80)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    arrows = anchored_artists.AnchoredDirectionArrows(ax.transAxes, "X", "Y")
    bar = anchored_artists.AnchoredSizeBar(ax.transData, 3, "3 data units", 4)
    ax.add_artist(arrows)
    ax.add_artist(bar)
    fig.canvas.draw()
    assert arrows.get_window_extent().extents == pytest.approx(
        [64.44, 340.39, 152.31, 417.96], abs=0.01)
    assert bar.get_window_extent().extents == pytest.approx(
        [317.07, 53.91, 430.89, 70.02], abs=0.01)
    assert isinstance(bar.size_bar, pltrs.offsetbox.AuxTransformBox)
    assert bar.txt_label.get_text() == "3 data units"


@pytest.mark.parametrize("kwargs, error, message", [
    ({"transform": None}, TypeError,
     "unsupported operand type(s) for +: 'NoneType' and 'Affine2D'"),
    ({"size": "1"}, TypeError, "unsupported operand type(s) for +: 'int' and 'str'"),
    ({"size_vertical": None}, TypeError,
     "'>' not supported between instances of 'NoneType' and 'int'"),
    ({"color": "R"}, ValueError, "Invalid RGBA argument: 'R'"),
])
def test_size_bars_fail_as_matplotlibs_are_made(kwargs, error, message):
    fig, ax = plt.subplots()
    arguments = {"transform": ax.transData, "size": 1, "label": "1", "loc": "lower right"}
    arguments.update(kwargs)
    with pytest.raises(error) as raised:
        anchored_artists.AnchoredSizeBar(**arguments)
    assert str(raised.value) == message


def test_drawing_areas_keep_their_sizes_unchecked():
    area = anchored_artists.AnchoredDrawingArea(None, 20, 0, 0, loc="upper right")
    assert area.drawing_area.width is None


@pytest.mark.parametrize("text, message", [
    (None, "'NoneType' object has no attribute 'replace'"),
    (3, "'int' object has no attribute 'replace'"),
    ([], "'list' object has no attribute 'replace'"),
    ("", "'list' object has no attribute 'flags'"),
])
def test_text_paths_preprocess_their_text_as_texts_do(text, message):
    with pytest.raises(AttributeError) as error:
        TextPath((0.1, 0.2), text, size=0.08)
    assert str(error.value) == message



def curvilinear_axes():
    """A figure with axes of a curvilinear (polar) grid helper."""
    from pltrs.mpl_toolkits.axisartist import SubplotHost, angle_helper
    from pltrs.mpl_toolkits.axisartist.grid_helper_curvelinear import GridHelperCurveLinear
    from pltrs.projections.polar import PolarTransform
    from pltrs.transforms import Affine2D
    polar = Affine2D().scale(np.pi / 180, 1) + PolarTransform()
    helper = GridHelperCurveLinear(polar, extreme_finder=angle_helper.ExtremeFinderCycle(
        20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, np.inf)))
    fig = plt.figure()
    return fig, fig.add_subplot(axes_class=SubplotHost, grid_helper=helper)


@pytest.mark.parametrize("nth", [-1, float("nan"), 0.5, "0", None, 2])
def test_curvilinear_floating_axes_compute_their_line_along_one_coordinate(nth):
    fig, ax = curvilinear_axes()
    with pytest.raises(UnboundLocalError, match="cannot access local variable 'xys'"):
        ax.new_floating_axis(nth, 0.5)


def test_rectilinear_floating_axes_compute_their_line_first():
    fig, ax = artist_axes()
    with pytest.raises(IndexError, match="index 2 is out of bounds for axis 0 with size 2"):
        ax.new_floating_axis(-1, 0.5)


@pytest.mark.parametrize("nth", [float("nan"), 2, "0"])
def test_curvilinear_fixed_axes_keep_the_coordinate_of_their_ticks(nth):
    fig, ax = curvilinear_axes()
    axis = ax.get_grid_helper().new_fixed_axis("left", axes=ax, nth_coord=nth)
    assert axis.get_helper().nth_coord_ticks is nth


@pytest.mark.parametrize("value", [None, "0.5", float("nan")])
def test_floating_axes_run_at_numpys_value(value):
    fig, ax = artist_axes()
    ax.axis["new"] = ax.new_floating_axis(0, value)
    fig.canvas.draw()


def test_fixed_axes_take_offsets_of_arrays():
    fig, ax = artist_axes()
    ax.axis["new"] = ax.new_fixed_axis("left", offset=np.array([1.0, -1.0]))
    fig.canvas.draw()


@pytest.mark.parametrize("which, error, message", [
    (None, AttributeError, "'NoneType' object has no attribute 'lower'"),
    (3, AttributeError, "'int' object has no attribute 'lower'"),
    ("nope", ValueError, "'nope' is not a valid value for which"),
])
def test_axisartist_grids_check_their_arguments_as_axes_do(which, error, message):
    fig, ax = artist_axes()
    with pytest.raises(error, match=message):
        ax.grid(True, which=which)


def test_axisartist_grids_keep_a_choice_the_draw_computes_with():
    fig, ax = artist_axes()
    ax.grid(True, which="MAJOR")
    with pytest.raises(ValueError, match="'MAJOR' is not a valid value for which"):
        fig.canvas.draw()
    fig, ax = curvilinear_axes()
    ax.grid(True, which="MAJOR")
    fig.canvas.draw()
