"""
What the differential tester's hostile programs (``spec/diff/generators/
hostile.py``) found the offset boxes, axes_grid1 and axisartist of pltrs
doing otherwise than matplotlib 3.11: matplotlib keeps the children,
settings, sizes and points of its boxes, dividers, inset locators and
axis artists unchecked, and fails on what it cannot compute with only as
it measures or draws them. Each expected value and error is recorded from
matplotlib.
"""

import io
import re

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import patches, transforms
from pltrs.offsetbox import (AnchoredOffsetbox, AnnotationBbox, AuxTransformBox,
                             DrawingArea, HPacker, OffsetImage, PaddedBox,
                             TextArea, VPacker)
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import Divider, ImageGrid, Size, inset_locator
from pltrs.mpl_toolkits.axes_grid1.axes_divider import SubplotDivider
from pltrs.mpl_toolkits.axisartist import floating_axes
from pltrs.mpl_toolkits.axisartist.grid_helper_curvelinear import GridHelperCurveLinear


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def drawn(fig):
    """Draw *fig* (failing as its draw fails)."""
    fig.canvas.draw()


# -- offset boxes ---------------------------------------------------------------

@pytest.mark.parametrize("child, name", [(None, "NoneType"), (3, "int"), ("x", "str")])
def test_an_anchored_box_keeps_any_child_and_fails_to_measure_it(child, name):
    fig, ax = plt.subplots()
    box = AnchoredOffsetbox("upper right", child=TextArea("a"))
    ax.add_artist(box)
    if child is None:
        box.set_child(child)
    else:
        with pytest.raises(AttributeError, match="has no attribute 'axes'"):
            box.set_child(child)
    assert box.get_child() == child
    message = f"^'{name}' object has no attribute 'get_bbox'$"
    with pytest.raises(AttributeError, match=message):
        drawn(fig)
    with pytest.raises(AttributeError, match=message):
        box.get_window_extent(fig.canvas.get_renderer())
    with pytest.raises(AttributeError, match=message):
        fig.savefig(io.BytesIO(), format="svg")
    # A child set once the box joined the figure has no figure to measure
    # its text at.
    box.set_child(TextArea("b"))
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'dpi'$"):
        drawn(fig)
    box.get_child().set_figure(fig)
    drawn(fig)


def test_an_anchored_box_of_a_figure_has_nothing_to_anchor_to():
    fig = plt.figure()
    box = AnchoredOffsetbox("upper right", child=TextArea("a"))
    fig.add_artist(box)
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'bbox'$"):
        drawn(fig)


@pytest.mark.parametrize("pad, message", [
    ("4", "can't multiply sequence by non-int of type 'float'"),
    (None, "unsupported operand type(s) for *: 'NoneType' and 'float'")])
def test_a_padding_that_is_no_number_fails_as_the_box_is_measured(pad, message):
    fig, ax = plt.subplots()
    box = PaddedBox(TextArea("p"), pad=4, draw_frame=True)
    ax.add_artist(AnchoredOffsetbox("center", child=box))
    box.pad = pad
    with pytest.raises(TypeError, match=f"^{re.escape(message)}$"):
        drawn(fig)
    with pytest.raises(TypeError):
        box.get_bbox(fig.canvas.get_renderer())
    box.pad = 4
    drawn(fig)


def test_a_frame_scaled_by_no_number_fails_as_it_is_drawn():
    fig, ax = plt.subplots()
    box = PaddedBox(TextArea("p"), pad=4, draw_frame=True)
    ax.add_artist(AnchoredOffsetbox("center", child=box))
    box.update_frame(transforms.Bbox.from_bounds(0.1, 0.2, 0.5, 0.4), fontsize="12")
    assert box.patch.get_mutation_scale() == "12"
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type 'float'$"):
        drawn(fig)
    box.patch.set_mutation_scale(3)
    drawn(fig)
    patch = patches.FancyBboxPatch((0, 0), 1, 1, boxstyle="round")
    ax.add_patch(patch)
    patch.set_mutation_scale(None)
    assert patch.get_mutation_scale() is None
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \*: "
                                        r"'NoneType' and 'float'$"):
        drawn(fig)


def test_a_drawing_area_keeps_what_is_added_to_it():
    fig, ax = plt.subplots()
    area = DrawingArea(40, 20, 0, 0, clip=True)
    area.add_artist(patches.Circle((10, 10), 10))
    ax.add_artist(AnchoredOffsetbox("lower left", child=area))
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'is_transform_set'$"):
        area.add_artist(None)
    area.set(height=-1.5)
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'clipbox'$"):
        drawn(fig)
    area._children.pop()
    area.set(height=None)
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \*: "
                                        r"'NoneType' and 'float'$"):
        drawn(fig)
    area.set(height=4)
    drawn(fig)


def test_an_offset_given_is_kept_until_the_box_is_drawn_again():
    fig, ax = plt.subplots()
    area = DrawingArea(40, 20, 0, 0)
    ax.add_artist(AnchoredOffsetbox("lower left", child=area))
    drawn(fig)
    with pytest.raises(IndexError, match="^tuple index out of range$"):
        area.set_offset((5,))
    assert area.get_offset() == (5,)
    area.set_offset([5, 5])
    assert area.get_offset() == [5, 5]
    drawn(fig)
    offset = area.get_offset()
    assert isinstance(offset, tuple) and isinstance(offset[0], np.float64)


def test_an_aux_transform_box_keeps_what_is_added_to_it():
    fig, ax = plt.subplots()
    aux = AuxTransformBox(transforms.Affine2D().scale(10))
    aux.add_artist(patches.Rectangle((0, 0), 1, 1))
    ax.add_artist(AnchoredOffsetbox("center", child=aux))
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'set_transform'$"):
        aux.add_artist(3)
    message = "^'int' object has no attribute 'get_window_extent'$"
    with pytest.raises(AttributeError, match=message):
        drawn(fig)
    with pytest.raises(AttributeError, match=message):
        aux.get_bbox(fig.canvas.get_renderer())


def test_packers_compute_with_their_settings_as_given():
    fig, ax = plt.subplots()
    packer = HPacker(children=[TextArea("a"), TextArea("b")], pad=2, sep=3, align="top")
    ax.add_artist(AnchoredOffsetbox("center", child=packer))
    packer.width = "0.4"
    with pytest.raises(TypeError, match="did not contain a loop with signature matching "
                                        "types"):
        drawn(fig)
    packer.width = None
    packer.align = "x"
    with pytest.raises(ValueError, match="^'x' is not a valid value for align.$"):
        drawn(fig)
    packer.align = "top"
    drawn(fig)
    column = VPacker(children=[TextArea("a"), TextArea("b")], mode="expand")
    ax.add_artist(AnchoredOffsetbox("center", child=column))
    column.height = "0.4"
    with pytest.raises(TypeError, match="ufunc 'subtract' did not contain a loop"):
        drawn(fig)


def test_an_offset_image_sizes_itself_by_its_data_and_zoom_as_given():
    fig, ax = plt.subplots()
    image = OffsetImage(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)), zoom=3)
    ax.add_artist(AnnotationBbox(image, (0.5, 0.5)))
    image.set_zoom("2")
    assert image.get_zoom() == "2"
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type 'float'$"):
        drawn(fig)
    image.set_zoom(2)
    with pytest.raises(TypeError):
        image.set_data(np.arange(5.))
    with pytest.raises(ValueError, match=r"^not enough values to unpack \(expected 2, got 1\)$"):
        drawn(fig)
    image.set_data(np.ones((3, 3)))
    drawn(fig)


@pytest.mark.parametrize("xyann, error, message", [
    (None, TypeError, "cannot unpack non-iterable NoneType object"),
    ((0.8,), ValueError, r"not enough values to unpack \(expected 2, got 1\)")])
def test_an_annotation_box_keeps_any_position(xyann, error, message):
    fig, ax = plt.subplots()
    box = AnnotationBbox(TextArea("ab"), (0.5, 0.5), xybox=(0.8, 0.8),
                         arrowprops=dict(arrowstyle="->"))
    ax.add_artist(box)
    box.xyann = xyann
    assert box.xyann == xyann
    with pytest.raises(error, match=f"^{message}$"):
        drawn(fig)
    box.xyann = (0.8, 0.8)
    drawn(fig)


def test_drawing_with_extra_arguments_fails_on_the_renderer_first():
    box = AnchoredOffsetbox("upper right", child=TextArea("a"))
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'_raster_depth'$"):
        box.draw(None, None)


# -- axes_grid1 -------------------------------------------------------------------

def test_an_inset_locator_given_another_child_fails_as_the_inset_is_drawn():
    fig, ax = plt.subplots()
    inset = inset_locator.inset_axes(ax, width="30%", height=1)
    locator = inset.get_axes_locator()
    drawn(fig)
    with pytest.raises(AttributeError, match="'int' object has no attribute 'axes'"):
        locator.set(child=0)
    with pytest.raises(AttributeError, match="'int' object has no attribute 'axes'"):
        drawn(fig)
    locator.set(child=None)
    drawn(fig)


def test_an_inset_is_placed_with_its_arguments_as_given():
    fig, ax = plt.subplots()
    inset_locator.inset_axes(ax, width="30%", height=1, loc=None)
    with pytest.raises(TypeError, match="missing 2 required positional arguments: "
                                        "'bbox' and 'renderer'"):
        drawn(fig)
    fig, ax = plt.subplots()
    inset_locator.zoomed_inset_axes(ax, "2")
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type "
                                        "'numpy.float64'$"):
        drawn(fig)


def test_connectors_keep_any_corner_and_fail_to_find_it():
    fig, ax = plt.subplots()
    box = transforms.Bbox.from_bounds(0.1, 0.2, 0.5, 0.4)
    connector = inset_locator.BboxConnector(ax.bbox, box, 0)
    assert connector.loc1 == 0
    with pytest.raises(TypeError, match="^cannot unpack non-iterable NoneType object$"):
        ax.add_patch(connector)
    inset_locator.BboxConnector(None, box, 1)
    with pytest.raises(TypeError, match="^cannot unpack non-iterable NoneType object$"):
        inset_locator.mark_inset(ax, ax.inset_axes([0.6, 0.6, 0.3, 0.3]), 1.5, 3)
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'transData'$"):
        inset_locator.mark_inset(None, ax.inset_axes([0.6, 0.6, 0.3, 0.3]), 1, 3)


def test_a_divider_divides_its_rectangle_and_sizes_as_given():
    fig = plt.figure()
    divider = Divider(fig, (0.1, 0.1, 0.8, 0.8), [Size.Fixed(1), Size.Scaled(1)],
                      [Size.Fixed(0.5), Size.Scaled(2)])
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_axes_locator(divider.new_locator(1, 1))
    drawn(fig)
    divider.set_position(None)
    with pytest.raises(TypeError, match="^cannot unpack non-iterable NoneType object$"):
        drawn(fig)
    divider.set_position((0.2, 0.2, 0.6, 0.6))
    drawn(fig)
    np.testing.assert_allclose(ax.get_position().bounds,
                               (0.35625, 0.30416666666666664, 0.44375, 0.49583333333333335))
    divider.set_horizontal([None])
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'get_size'$"):
        drawn(fig)
    divider.set_horizontal([Size.Fixed(1)])
    with pytest.raises(IndexError, match="^index 2 is out of bounds for axis 0 with size 2$"):
        drawn(fig)
    divider.set_horizontal([Size.Fixed(1), Size.Scaled(1)])
    drawn(fig)


def test_a_grid_pads_its_axes_as_given():
    fig = plt.figure()
    grid = ImageGrid(fig, 111, (2, 2), axes_pad=0.1, cbar_mode="single")
    grid.set_axes_pad("0.2")
    assert grid.get_axes_pad() == ("0", ".")
    with pytest.raises(TypeError, match="^the resolved dtypes are not compatible with "
                                        "add.reduce"):
        drawn(fig)
    grid.set_axes_pad((0.2, 0.3))
    drawn(fig)


def test_dividers_take_sizes_of_no_axes_until_they_divide():
    fig = plt.figure()
    divider = SubplotDivider(fig, 1, 1, 1)
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'get_position'$"):
        divider.set_subplotspec(None)
    fig = plt.figure()
    divider = SubplotDivider(fig, 1, 1, 1)
    divider.add_auto_adjustable_area(None)
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_axes_locator(divider.new_locator(4, 4))
    with pytest.raises(TypeError, match="^'NoneType' object is not iterable$"):
        drawn(fig)


# -- axisartist -------------------------------------------------------------------

@pytest.mark.parametrize("pad, message", [
    ("5", "can't multiply sequence by non-int of type 'float'"),
    (None, r"unsupported operand type\(s\) for \*: 'NoneType' and 'float'")])
def test_an_axis_label_padded_by_no_number_fails_as_it_is_drawn(pad, message):
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    axis = ax.axis["bottom"]
    axis.LABELPAD = pad
    assert axis.LABELPAD == pad
    with pytest.raises(TypeError, match=f"^{message}$"):
        drawn(fig)
    axis.LABELPAD = 5
    drawn(fig)


def test_the_parts_of_an_axis_artist_keep_what_matplotlib_keeps():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.set_xlabel("xlab")
    axis = ax.axis["bottom"]
    drawn(fig)
    axis.major_ticks.set_ticksize("8")
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type 'float'$"):
        drawn(fig)
    axis.major_ticks.set_ticksize(8)
    axis.label.set(transform=3)
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \+: 'int' "
                                        r"and 'Affine2D'$"):
        drawn(fig)
    axis.label.set(transform=ax.transData)
    axis.label.set(agg_filter=3)
    with pytest.raises(TypeError, match="^'int' object is not callable$"):
        drawn(fig)
    axis.label.set(agg_filter=None, path_effects=[None])
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'draw_path'$"):
        drawn(fig)
    axis.label.set(path_effects=[])
    drawn(fig)


def test_an_empty_label_draws_no_filter():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.axis["bottom"].label.set(agg_filter=3, path_effects="x")
    drawn(fig)


def test_axis_artists_are_made_as_matplotlib_makes_them():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    with pytest.raises(TypeError, match=r"missing 1 required positional argument: "
                                        r"'scale_trans'$"):
        ax.new_fixed_axis("left", offset=(1.0,))
    with pytest.raises(ValueError, match="^'' is not a valid value for d. Supported values "
                                         "are 'left', 'right', 'bottom', 'top'$"):
        ax.new_floating_axis(0, 0.5, axis_direction="")
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'xaxis'$"):
        ax.get_grid_helper().new_fixed_axis("left", axes=3)
    curvilinear = GridHelperCurveLinear(transforms.Affine2D().scale(1, 2))
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'get_figure'$"):
        curvilinear.new_fixed_axis("left", axes=3)


def test_curvilinear_helpers_read_the_axes_they_are_given():
    fig = plt.figure()
    helper = floating_axes.GridHelperCurveLinear(transforms.Affine2D(), extremes=(0, 4, 0, 2))
    ax = fig.add_subplot(axes_class=floating_axes.FloatingAxes, grid_helper=helper)
    fixed = ax.axis["left"]._axis_artist_helper
    drawn(fig)
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'get_xlim'$"):
        fixed.update_lim(None)
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'transData'$"):
        fixed.get_tick_iterators(3)
    with pytest.raises(AttributeError, match="^'str' object has no attribute 'get_xlim'$"):
        fixed.get_line("x")
    fixed.update_lim(ax)


def test_angle_steps_are_computed_as_given():
    from pltrs.mpl_toolkits.axisartist import angle_helper
    levels, n, factor = angle_helper.select_step24(0, 12, 4.5)
    assert levels.tolist() == [0, 150, 300, 450, 600, 750] and (n, factor) == (6, 60.0)
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        angle_helper.select_step(float("nan"), 10, 5)
    with pytest.raises(IndexError, match="^list index out of range$"):
        angle_helper.select_step(0, 1e6, 5)
    with pytest.raises(ValueError, match="^expected a positive input, got 0.0$"):
        angle_helper.LocatorD(5)(0, 0)
    with pytest.raises(ZeroDivisionError, match="^division by zero$"):
        angle_helper.select_step360(0, 180, 0)
    with pytest.raises(TypeError, match="^'>' not supported between instances of 'str' "
                                        "and 'int'$"):
        angle_helper.select_step("0", 10, 5)


def test_angle_labels_are_formatted_as_given():
    from pltrs.mpl_toolkits.axisartist import angle_helper
    assert angle_helper.FormatterDMS()("bottom", -1, [10, 20]) == [
        "$10^{\\circ}$", "$20^{\\circ}$"]
    assert angle_helper.FormatterHMS()("bottom", 1.5, [3, 6]) == [
        "$0.2^{\\circ}$", "$0.4^{\\circ}$"]
    with pytest.raises(TypeError, match="^'<=' not supported between instances of "
                                        "'NoneType' and 'int'$"):
        angle_helper.FormatterDMS()("bottom", None, [10, 20])
    with pytest.raises(TypeError, match="^object of type 'NoneType' has no len\\(\\)$"):
        angle_helper.FormatterDMS()("bottom", 1, None)
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        angle_helper.FormatterDMS()("bottom", 1, [float("nan"), 20])


def test_an_annotation_box_draws_no_filter():
    fig, ax = plt.subplots()
    box = AnnotationBbox(TextArea("ab"), (0.5, 0.5))
    ax.add_artist(box)
    box.set(agg_filter=3)
    drawn(fig)


def test_a_falsy_axes_locator_places_nothing():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.set(axes_locator=0)
    drawn(fig)
    ax.set(axes_locator="x")
    with pytest.raises(TypeError, match="^'str' object is not callable$"):
        drawn(fig)
