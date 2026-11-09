"""
What the differential tester's hostile programs (third round) found pltrs
doing otherwise than matplotlib 3.11 with 2-D Axes, their axes and tick
helpers, paths, images, patches and collections. Each expected value is
recorded from matplotlib.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import patches, ticker
from pltrs.category import StrCategoryConverter
from pltrs.path import Path
from pltrs.transforms import Bbox


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)


# -- limits, units and clearing --------------------------------------------------

def test_a_clear_keeps_the_data_limits():
    fig, ax = plt.subplots()
    ax.plot(X, Y)
    ax.cla()
    assert ax.dataLim.get_points().tolist() == [
        [0.0, -0.866025403784439], [6.283185307179586, 0.8660254037844387]]
    assert ax.ignore_existing_data_limits
    ax.set_xscale("log")
    assert ax.get_xlim() == pytest.approx((0.9574609172248693, 6.872067725191114))


def test_bounds_in_units_make_the_axis_categorical():
    fig, ax = plt.subplots()
    ax.set(xbound="ab")
    assert ax.get_xlim() == (0.0, 1.0)
    assert isinstance(ax.xaxis.get_converter(), StrCategoryConverter)
    with pytest.raises(TypeError, match="^'<' not supported between instances of "
                                        "'str' and 'float'$"):
        ax.set_ybound(0.5, "x")


def test_a_cleared_axes_no_longer_follows_its_units():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.cla()
    ax.set_xbound("a", "b")
    assert ax.dataLim.get_points().tolist() == [[0.0, 0.0], [1.0, 1.0]]


def test_interval_setters_compare_and_assign_as_matplotlib():
    fig, ax = plt.subplots()
    ax.plot(X, Y)
    with pytest.raises(TypeError, match="^'<' not supported between instances of "
                                        "'str' and 'float'$"):
        ax.xaxis.set_view_interval(0.0, "4.0")
    with pytest.raises(TypeError, match="^'<' not supported between instances of "
                                        "'float' and 'NoneType'$"):
        ax.xaxis.set_data_interval(None, 4.0)
    ax.xaxis.set_view_interval("1.5", 2.5, ignore=True)
    assert ax.get_xlim() == (1.5, 2.5)


def test_flags_read_back_as_given():
    fig, ax = plt.subplots()
    ax.set_frame_on(1)
    ax.set_navigate("False")
    ax.set_autoscalex_on(np.True_)
    ax.set_forward_navigation_events("")
    ax.xaxis.isDefault_majloc = None
    assert ax.get_frame_on() == 1 and type(ax.get_frame_on()) is int
    assert ax.get_navigate() == "False"
    assert type(ax.get_autoscalex_on()) is np.bool_
    assert ax.get_forward_navigation_events() == ""
    assert ax.xaxis.isDefault_majloc is None
    ax.set_autoscale_on(None)
    assert ax.get_autoscale_on() is True


def test_autoscale_leaves_other_axis_names():
    fig, ax = plt.subplots()
    ax.autoscale(False, axis="z")
    assert ax.get_autoscalex_on() and ax.get_autoscaley_on()
    ax.autoscale(None, axis="nope")


# -- axes as artists --------------------------------------------------------------

def test_an_axis_filter_that_is_no_function_fails_the_draw():
    fig, ax = plt.subplots()
    ax.xaxis.set_agg_filter(3)
    with pytest.raises(TypeError, match="^'int' object is not callable$"):
        fig.canvas.draw()
    ax.xaxis.set_agg_filter(lambda img, dpi: (img, 0, 0))
    fig.canvas.draw()


def test_an_animated_axis_is_not_ticked_where_it_is_not_drawn():
    fig, ax = plt.subplots()
    ax.xaxis.set_minor_formatter(ticker.NullFormatter())
    ax.xaxis.set_animated(True)
    ax.xaxis.set_ticks([float("nan"), 1, 2])
    fig.canvas.draw()


def test_a_label_pad_that_is_no_number_fails_where_the_label_is_placed():
    fig, ax = plt.subplots()
    ax.set_xlabel("x", labelpad="5")
    assert ax.xaxis.labelpad == "5"
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of "
                                        "type 'float'$"):
        fig.canvas.draw()
    ax.xaxis.set_visible(False)
    fig.canvas.draw()


def test_an_offset_that_is_no_number_fails_when_the_ticks_are_labelled():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.ticklabel_format(style="sci", scilimits=(0, 0), useOffset="x")
    assert ax.xaxis.get_major_formatter().offset == "x"
    with pytest.raises(TypeError, match="did not contain a loop with signature"):
        fig.canvas.draw()
    ax.ticklabel_format(useOffset=[])
    with pytest.raises(ValueError, match="^operands could not be broadcast together"):
        fig.canvas.draw()


def test_text_is_no_coordinate_to_format():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="did not contain a loop with signature"):
        ax.format_coord("0.5", 0.25)


# -- plotting methods -------------------------------------------------------------

def test_hexbin_bins_as_matplotlib_for_what_pltrs_does_not_take():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^'float' object cannot be interpreted as an "
                                        "integer$"):
        ax.hexbin(X, Y, gridsize=4.5)
    with pytest.raises(ValueError, match="^arange: cannot compute length$"):
        ax.hexbin(X, Y, gridsize=4, bins=float("nan"))
    with pytest.raises(ZeroDivisionError, match="^division by zero$"):
        ax.hexbin(X, Y, gridsize=0)
    with pytest.raises(IndexError, match="^list index out of range$"):
        ax.hexbin(X, Y, gridsize=4, C=[0.0, 1.0, 2.0])
    collection = ax.hexbin(np.array([0.0]), Y, gridsize=4)
    assert len(collection.get_offsets()) == 23
    assert collection.get_array().tolist()[5:10] == [0.0, 2.0, 3.0, 2.0, 0.0]
    ax.hexbin(X, Y, gridsize=4, reduce_C_function=None)


def test_collections_keep_widths_that_are_no_numbers():
    fig, ax = plt.subplots()
    collection = ax.scatter([1, 2], [3, 4], linewidths="ab")
    assert collection.get_linewidth().tolist() == ["ab"]
    with pytest.raises(ValueError, match=r"^could not convert string to float: "
                                         r"np\.str_\('ab'\)$"):
        fig.canvas.draw()
    collection.set_linewidth(2)
    fig.canvas.draw()


def test_stackplot_checks_its_layers_as_fill_between():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^operands could not be broadcast together "
                                         r"with shapes \(0,\) \(7,\) $"):
        ax.stackplot(np.empty((0,)), Y + 1, Y + 2)
    with pytest.raises(ValueError, match="^'x' is not 1-dimensional$"):
        ax.stackplot(X[None, :], Y + 1, Y + 2)
    layers = ax.stackplot([0.0], Y + 1, Y + 2)
    assert len(layers) == 2


def test_pie_falls_back_on_matplotlib_for_what_pltrs_does_not_take():
    fig, ax = plt.subplots()
    with pytest.raises(StopIteration):
        ax.pie([1, 2, 3], colors=[])
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of "
                                        "type 'float'$"):
        ax.pie([1, 2, 3], labeldistance="1.2")
    with pytest.raises(TypeError, match=r"Wedge\.set\(\) argument after \*\* must be a "
                                        "mapping, not int$"):
        ax.pie([1, 2, 3], wedgeprops=0)
    with pytest.raises(ValueError, match=r"^not enough values to unpack \(expected 2, "
                                         r"got 0\)$"):
        ax.pie([1, 2, 3], center=())
    container = ax.pie([1, 2, 3], radius=float("nan"))
    assert len(container.wedges) == 3


def test_bxp_merges_properties_as_matplotlib():
    fig, ax = plt.subplots()
    stats = [{'med': 2, 'q1': 1, 'q3': 3, 'whislo': 0, 'whishi': 4, 'fliers': [5]}]
    ax.bxp(stats, medianprops=[])
    ax.bxp(stats, meanprops={'nonexistent_key': 1})
    with pytest.raises(AttributeError, match="^'list' object has no attribute 'items'$"):
        ax.bxp(stats, whiskerprops=[])
    with pytest.raises(TypeError, match=r'^can only concatenate str \(not "int"\) to str$'):
        ax.bxp(stats, zorder='3')
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of "
                                        "type 'float'$"):
        ax.bxp(stats, capwidths='0.2')


def test_errorbar_takes_its_format_as_plot():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^third arg must be a format string$"):
        ax.errorbar(X, Y, yerr=0.2, fmt=None)
    with pytest.raises(ValueError, match="^'R' is not a valid color value.$"):
        ax.errorbar(X, Y, yerr=0.2, ecolor='R')


def test_images_check_extents_aspects_and_filter_radii_as_matplotlib():
    fig, ax = plt.subplots()
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        ax.imshow(z, extent=(float('nan'), 4, 0, 3))
    with pytest.raises(ValueError, match="^aspect must be finite and positive $"):
        ax.imshow(z, aspect=-1.5)
    with pytest.raises(ValueError, match="^could not convert string to float: 'equalxy'$"):
        ax.spy(z > 0, aspect='equalxy')
    ax.imshow(z, aspect='1.5')
    assert ax.get_aspect() == 1.5
    image = ax.imshow(z, filterrad=float('nan'))
    fig.canvas.draw()
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        image.set_extent((float('nan'), 4, 0, 3))


@pytest.mark.parametrize("magnification, error, message", [
    (-1.5, ValueError, "^negative dimensions are not allowed$"),
    (float("nan"), ValueError, "^cannot convert float NaN to integer$"),
    (float("inf"), ValueError, "^cannot convert float NaN to integer$"),
    ("1.0", TypeError, "did not contain a loop with signature"),
    (None, TypeError, r"^unsupported operand type\(s\) for \*: 'float' and 'NoneType'$"),
])
def test_images_magnify_as_matplotlib(magnification, error, message):
    fig, ax = plt.subplots()
    image = ax.imshow(np.arange(12.0).reshape(3, 4))
    fig.canvas.draw()
    with pytest.raises(error, match=message):
        image.make_image(fig.canvas.get_renderer(), magnification=magnification)


def test_grid_images_magnify_as_matplotlib():
    fig, ax = plt.subplots()
    image = ax.pcolorfast([0, 1, 3, 4], [0, 2, 3], np.arange(6.0).reshape(2, 3))
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    with pytest.raises(ValueError, match=r"^Number of samples, -?\d+, must be "
                                         "non-negative.$"):
        image.make_image(renderer, magnification=-1.5)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        pixels, x, y, _ = image.make_image(renderer, magnification=0.0)
    assert pixels.shape == (0, 0, 4) and np.isnan(x) and np.isnan(y)


def test_a_figure_image_asks_the_renderer_its_resolution():
    fig = plt.figure()
    image = fig.figimage(np.arange(12.0).reshape(3, 4))
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'dpi'$"):
        image.make_image(None)
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for /: 'str' "
                                        "and 'float'$"):
        image.make_image(fig.canvas.get_renderer(), magnification='1.0')


# -- paths ---------------------------------------------------------------------

def test_paths_take_arguments_as_matplotlib_extension():
    square = Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
    with pytest.raises(TypeError, match=r"^cleanup_path\(\): incompatible function "
                                        "arguments"):
        square.cleaned(remove_nans='False')
    with pytest.raises(ValueError, match="^Invalid affine transformation matrix$"):
        square.cleaned(transform=3)
    with pytest.raises(ValueError, match="^Invalid bounding box$"):
        square.cleaned(clip=1)
    with pytest.raises(RuntimeError, match="^Unable to cast Python instance"):
        square.cleaned(sketch=0)
    with pytest.raises(ValueError, match="^Expected 2-dimensional array, got 0$"):
        square.contains_points(None)
    assert square.contains_path(None)
    assert square.intersects_path(None)
    assert len(square.clip_to_bbox(None).vertices) == 5
    with pytest.raises(ValueError, match="^slice step cannot be zero$"):
        square.interpolated(0)


# -- arrows --------------------------------------------------------------------

def test_arrows_keep_what_matplotlib_keeps_and_fail_where_it_draws():
    fig, ax = plt.subplots()
    arrow = patches.FancyArrowPatch((0, 0), (1, 1))
    ax.add_patch(arrow)
    arrow.set_positions((), (1, 1))
    assert arrow._posA_posB == [(), (1, 1)]
    with pytest.raises(IndexError, match="^tuple index out of range$"):
        fig.canvas.draw()
    arrow.set_positions((0, 0), (1, 1))
    assert arrow._posA_posB == [(0, 0), (1, 1)]
    arrow.set_patchA(3)
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'contains'$"):
        fig.canvas.draw()
    arrow.set_patchA(None)
    arrow.set_connectionstyle(3)
    assert arrow.get_connectionstyle() == 3
    with pytest.raises(TypeError, match="^'int' object is not callable$"):
        fig.canvas.draw()
    arrow.set_connectionstyle("arc3")
    fig.canvas.draw()


def test_bboxes_are_untouched():
    # A box of the package's still converts as matplotlib's rect caster.
    square = Path([(0, 0), (2, 0), (2, 2), (0, 2), (0, 0)], closed=True)
    clipped = square.clip_to_bbox(Bbox([[0, 0], [1, 1]]))
    assert clipped.get_extents().bounds == (0.0, 0.0, 1.0, 1.0)


# -- steps and path patches ----------------------------------------------------

def test_stairs_take_any_orientation_and_check_as_step_patches():
    fig, ax = plt.subplots()
    patch = ax.stairs([1, 3, 2], orientation='x')
    assert patch.orientation == 'x'
    with pytest.raises(TypeError, match="^object of type 'NoneType' has no len\\(\\)$"):
        ax.stairs(None)
    with pytest.raises(ValueError, match="^Size mismatch between \"values\" and \"edges\""):
        ax.stairs('ab')


def test_step_patches_keep_their_data_as_given():
    fig, ax = plt.subplots()
    patch = ax.stairs([1, 3, 2])
    with pytest.raises(ValueError, match="^Size mismatch"):
        patch.set_data([2])
    values, edges, baseline = patch.get_data()
    assert values.tolist() == [2] and edges.tolist() == [0, 1, 2, 3]
    assert baseline.tolist() == 0
    with pytest.raises(TypeError, match="^ufunc 'isnan' not supported"):
        patch.set_data([2, 1, 2], [0, 1, 2, 3], baseline='sym')
    fig.canvas.draw()


def test_path_patches_keep_paths_that_are_no_paths():
    fig, ax = plt.subplots()
    patch = patches.PathPatch(Path([(0, 0), (1, 0), (1, 1)]))
    ax.add_patch(patch)
    patch.set_path(None)
    assert patch.get_path() is None
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'vertices'$"):
        fig.canvas.draw()
    patch.set_path(Path([(0, 0), (1, 1)]))
    fig.canvas.draw()


def test_hist2d_bins_unusual_arguments_with_numpy():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^`bins\\[0\\]` must be an integer, when a scalar$"):
        ax.hist2d(X, Y, bins=3.5)
    with pytest.raises(ValueError, match="^`bins\\[0\\]` must be positive, when an integer$"):
        ax.hist2d(X, Y, bins=0)
    with pytest.raises(ValueError, match="^The weights and list don't have the same length.$"):
        ax.hist2d(X, Y, bins=3, weights=np.ones(6))
    with pytest.raises(TypeError, match="did not contain a loop with signature"):
        ax.hist2d(X, Y, bins=3, cmin='1')


def test_collections_take_no_sizes_as_matplotlib():
    from pltrs import collections
    for collection in (collections.RegularPolyCollection(4, sizes=None),
                       collections.CircleCollection(None)):
        assert collection.get_sizes().tolist() == []
