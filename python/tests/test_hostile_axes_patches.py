"""
What the differential tester's hostile programs of Axes, axes' axes,
spines, ticks, transforms, paths, patches and collections
(``spec/diff/generators/hostile.py``) found pltrs doing otherwise than
matplotlib 3.11: values matplotlib's setters keep unchecked and fail on
when they are drawn, the arithmetic it raises and warns in, the order of
its checks. Each expected value is recorded from matplotlib.
"""
import math
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, patches, ticker, transforms, units
from pltrs.path import Path

import build_facts


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _no_renderer(artist):
    """The error *artist*'s ``draw(None)`` raises."""
    with pytest.raises(AttributeError) as error:
        artist.draw(None)
    return str(error.value)


# -- draws, arithmetic --------------------------------------------------------

def test_draws_start_as_allow_rasterization_does():
    fig, ax = plt.subplots()
    artists = [patches.Rectangle((0, 0), 1, 1), patches.Arc((0.5, 0.5), 0.2, 0.1),
               collections.LineCollection([]), ax, ax.xaxis,
               ax.xaxis.get_major_ticks()[0], ax.imshow(np.zeros((2, 2))),
               ax.contour(np.arange(16.).reshape(4, 4))]
    assert {_no_renderer(artist) for artist in artists} == {
        "'NoneType' object has no attribute '_rasterizing'"}


def test_path_arcs_compute_with_numpy():
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for -: 'int' and 'str'$"):
        Path.arc('0', 90)
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        Path.arc(float('nan'), 90)
    with pytest.warns(RuntimeWarning, match="invalid value encountered in scalar subtract"):
        with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
            Path.wedge(float('inf'), 90)
    with pytest.raises(TypeError, match="^'float' object cannot be interpreted as an integer$"):
        Path.arc(0, 90, 3.5)
    assert len(Path.arc(0, 90, np.int64(3)).vertices) == 10


def test_affine_arithmetic_is_matplotlibs():
    with pytest.raises(ValueError, match=f"^{build_facts.raised(lambda: math.cos(math.inf))}$"):
        transforms.Affine2D().rotate_deg(float('inf'))
    with pytest.raises(TypeError, match="^must be real number, not str$"):
        transforms.Affine2D().rotate('30')
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \*: 'float' and 'NoneType'$"):
        transforms.Affine2D().scale(None)
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \+: 'float' and 'NoneType'$"):
        transforms.Affine2D().translate(None, 1)
    with pytest.warns(RuntimeWarning, match="invalid value encountered in scalar multiply"):
        scaled = transforms.Affine2D().scale(float('inf'), 1)
    assert np.isnan(scaled.get_matrix()[0, 1])


def test_extents_of_paths_of_no_numbers():
    straight = Path([(np.nan, 0), (np.nan, 1)]).get_extents()
    assert np.isnan(straight.x0) and (straight.y0, straight.y1) == (0, 1)
    with pytest.raises(ValueError, match="^need at least one array to concatenate$"):
        Path.arc(0, 90).get_extents(transforms.Affine2D().translate(np.nan, 0))
    assert Path(np.empty((0, 2))).get_extents().x0 == np.inf


# -- spines -------------------------------------------------------------------

def test_spine_bounds_are_kept_and_fail_drawing():
    fig, ax = plt.subplots()
    spine = ax.spines['bottom']
    spine.set_bounds('ab')
    assert spine.get_bounds() == ('a', 'b')
    with pytest.raises(ValueError, match="^could not convert string to float: 'a'$"):
        fig.canvas.draw()
    # Measured beside the axis' label, it fails drawn or not.
    spine.set_animated(True)
    with pytest.raises(ValueError, match="^could not convert string to float: 'a'$"):
        fig.canvas.draw()
    spine.set_bounds(None)
    assert spine.get_bounds() == ('a', 'b')
    spine.set_bounds(0.25)
    assert spine.get_bounds() == (0.25, 'b')
    spine.set_bounds(0.25, 0.75)
    fig.canvas.draw()


def test_spine_axis_registered_is_used_as_matplotlib_uses_it():
    fig, ax = plt.subplots()
    spine = ax.spines['bottom']
    spine.register_axis('nope')
    with pytest.raises(AttributeError, match="^'str' object has no attribute 'get_visible'$"):
        fig.canvas.draw()
    with pytest.raises(AttributeError, match="^'str' object has no attribute 'reset_ticks'$"):
        spine.set_position(('outward', 10))
    assert spine.get_position() == ('outward', 10)
    spine.register_axis(ax.xaxis)
    fig.canvas.draw()


def test_spine_shapes_are_kept_and_fail_drawing():
    fig, ax = plt.subplots()
    spine = ax.spines['bottom']
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \*: 'NoneType' and 'int'$"):
        spine.set_patch_arc((0.5, 0.5), None, 0, 90)
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for -: 'int' and 'str'$"):
        spine.set_patch_arc((0.5, 0.5), 0.5, '0', 90)
    spine.set_patch_arc((), 0.5, 0, 90)
    with pytest.raises(IndexError, match="^tuple index out of range$"):
        fig.canvas.draw()
    spine.set_patch_arc((float('nan'), 0.5), 0.5, 0, 90)
    with pytest.raises(ValueError, match="^need at least one array to concatenate$"):
        fig.canvas.draw()
    spine.set_patch_arc((0.5, 0.5, 0.5), '0.5', 0, 90)
    fig.canvas.draw()
    spine.set_patch_line()
    with pytest.raises(AssertionError, match="^unexpected vertices shape$"):
        fig.canvas.draw()


def test_spine_styles_take_patches_defaults():
    fig, ax = plt.subplots()
    spine = ax.spines['left']
    spine.set_color(None)
    spine.set_linewidth(None)
    assert spine.get_edgecolor() == (0, 0, 0, 0) and spine.get_linewidth() == 1.0
    with pytest.raises(ValueError, match="^Unrecognized linestyle: 3$"):
        spine.set_linestyle(3)
    with pytest.raises(ValueError, match="^Unrecognized linestyle: 3$"):
        patches.Rectangle((0, 0), 1, 1).set_linestyle(3)


def test_spine_constructors_ask_the_axes_for_its_figure():
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'get_figure'$"):
        pltrs.spines.Spine.linear_spine(3, 'left')


# -- patches ------------------------------------------------------------------

def test_ellipse_geometry_is_kept_and_fails_drawing():
    fig, ax = plt.subplots()
    ellipse = patches.Ellipse((0.5, 0.5), 0.3, 0.2)
    ax.add_patch(ellipse)
    ellipse.set_center('ab')
    assert ellipse.get_center() == 'ab'
    with pytest.raises(units.ConversionError,
                       match="^Failed to convert value\\(s\\) to axis units: 'a'$"):
        fig.canvas.draw()
    ellipse.set_center((0.4, 0.4))
    ellipse.set_angle(float('inf'))
    with pytest.raises(ValueError, match=f"^{build_facts.raised(lambda: math.cos(math.inf))}$"):
        fig.canvas.draw()
    ellipse.set_angle('30')
    assert ellipse.get_angle() == '30'
    with pytest.raises(TypeError, match="^must be real number, not str$"):
        ellipse.get_patch_transform()
    ellipse.set_angle(30)
    fig.canvas.draw()
    circle = patches.Circle((0.5, 0.5), 0.1)
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \*: 'int' and 'NoneType'$"):
        circle.set_radius(None)
    circle.set_radius('0.5')
    assert circle.get_width() == '0.50.5'


def test_category_converters_without_categories_refuse_whenever_asked():
    fig, ax = plt.subplots()
    for value in ['a', 'b', None]:
        with pytest.raises(units.ConversionError):
            ax.xaxis.convert_units(value)
    assert ax.xaxis.units is None


def test_annulus_geometry_is_kept_and_checked_as_given():
    fig, ax = plt.subplots()
    annulus = patches.Annulus((0.5, 0.5), 0.3, 0.1)
    ax.add_patch(annulus)
    with pytest.raises(TypeError, match="^'>' not supported between instances of 'str' and 'float'$"):
        annulus.set_width('0.05')
    annulus.set_semimajor(-1.5)
    annulus.set_semiminor(0.0)
    assert annulus.get_radii() == (-1.5, 0.0)
    annulus.set_radii(0.3)
    annulus.set_center(None)
    assert annulus.get_center() is None
    with pytest.raises(TypeError, match="^'NoneType' object is not subscriptable$"):
        fig.canvas.draw()
    annulus.set_center((0.5, 0.5))
    fig.canvas.draw()


def test_wedge_geometry_is_kept_and_computed_as_matplotlib_does():
    fig, ax = plt.subplots()
    wedge = patches.Wedge((0.5, 0.5), 0.3, 0, 90, width=0.1)
    ax.add_patch(wedge)
    wedge.set_center((0.2, 0.2, 0.2))
    with pytest.raises(ValueError, match="^operands could not be broadcast together"):
        fig.canvas.draw()
    wedge.set_center(0.2)
    fig.canvas.draw()
    wedge.set_radius(0)
    with pytest.warns(RuntimeWarning, match="divide by zero encountered in divide"):
        fig.canvas.draw()
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for -: 'int' and 'str'$"):
        patches.Wedge((0.5, 0.5), 0.3, '0', 90)


def test_rectangle_geometry_is_kept_and_fails_drawing():
    fig, ax = plt.subplots()
    rectangle = patches.Rectangle((0.1, 0.2), 0.5, 0.3)
    ax.add_patch(rectangle)
    with pytest.raises(ValueError, match=r"^not enough values to unpack \(expected 2, got 0\)$"):
        rectangle.set_xy(())
    rectangle.set_angle('30')
    with pytest.raises(TypeError, match="^must be real number, not str$"):
        fig.canvas.draw()
    rectangle.set_angle(30)
    rectangle.set_width(None)
    assert rectangle.get_width() is None
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \+: 'float' and 'NoneType'$"):
        fig.canvas.draw()
    rectangle.set_width(0.5)
    fig.canvas.draw()


def test_fancy_patches_keep_their_mutation_scale():
    fig, ax = plt.subplots()
    box = patches.FancyBboxPatch((0.1, 0.1), 0.3, 0.3, boxstyle='round')
    ax.add_patch(box)
    box.set_mutation_scale(None)
    assert box.get_mutation_scale() is None
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \*: 'NoneType' and 'float'$"):
        fig.canvas.draw()
    box.set_mutation_scale(2)
    fig.canvas.draw()
    with pytest.raises(TypeError, match=r"^FancyBboxPatch\.set_mutation_scale\(\) got an unexpected"):
        box.set_mutation_scale(2, nonexistent_kw=1)
    arrow = patches.FancyArrowPatch((0, 0), (1, 1))
    with pytest.raises(TypeError, match=r"^FancyArrowPatch\.set_mutation_aspect\(\) takes 2"):
        arrow.set_mutation_aspect(1, 2)


# -- collections --------------------------------------------------------------

def test_event_collections_compute_with_their_segments():
    fig, ax = plt.subplots()
    events = collections.EventCollection([1, 2, 4], linelength=0.5)
    ax.add_collection(events)
    events.set_positions([float('nan'), 2])
    assert [len(segment) for segment in events.get_segments()] == [2, 0]
    with pytest.raises(IndexError, match="^too many indices for array"):
        events.get_positions()
    with pytest.raises(IndexError, match="^too many indices for array"):
        events.set_linelength(2)
    with pytest.raises(ValueError, match="^Input must be >= 2-d.$"):
        events.set_orientation('vertical')
    assert events.get_orientation() == 'horizontal' and events.get_linelength() == 0.5
    events.set_positions([1, 2])
    events.set_linelength(2)
    assert events.get_linelength() == 2 and events.get_positions() == [1.0, 2.0]


def test_collections_keep_what_matplotlib_keeps():
    fig, ax = plt.subplots()
    stars = collections.StarPolygonCollection(5, sizes=(20,), offsets=[[0, 0], [1, 1]],
                                              offset_transform=ax.transData)
    ax.add_collection(stars)
    stars.set_offset_transform(3)
    assert stars.get_offset_transform() == 3
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'is_affine'$"):
        fig.canvas.draw()
    stars.set_offset_transform(ax.transData)
    stars.set_paths([None])
    assert stars.get_paths() == [None]
    with pytest.raises(TypeError, match=r"^len\(\) of unsized object$"):
        stars.set_sizes('ab')
    stars.set_linestyle('--')
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type 'float'$"):
        stars.set_linewidth('2.0')
    ellipses = collections.EllipseCollection([1], [1], [0])
    with pytest.raises(TypeError, match=r"^Collection\.get_paths\(\) takes 1"):
        ellipses.get_paths(None)
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \*: 'float' and 'NoneType'$"):
        ellipses.set_widths(None)


# -- Axes and their axes ------------------------------------------------------

def test_tick_locations_convert_before_they_are_checked():
    fig, ax = plt.subplots()
    with pytest.raises(units.ConversionError,
                       match="^Failed to convert value\\(s\\) to axis units: 'ab'$"):
        ax.set_xticks('ab')
    with pytest.raises(ValueError, match="^'locs' must be 1D with shape \\(N,\\), but your input has shape \\(\\)$"):
        ax.set_yticks(5)


def test_fill_between_checks_in_matplotlibs_order():
    fig, ax = plt.subplots()
    x = np.linspace(0, 2 * np.pi, 7)
    y = np.sin(x)
    with pytest.raises(ValueError, match=r"^operands could not be broadcast together with shapes \(0,\) \(7,\) $"):
        ax.fill_between(np.empty((0,)), y)
    with pytest.raises(ValueError, match="^'x' has size 6, but 'y1' has an unequal size of 7$"):
        ax.fill_between(x[:6], y)
    with pytest.raises(IndexError, match="^too many indices for array"):
        ax.fill_betweenx(x, y, where=np.ones((1, 7), dtype=bool))


def test_hist2d_fails_as_numpys_histogram():
    fig, ax = plt.subplots()
    x = np.linspace(0, 2 * np.pi, 7)
    with pytest.raises(ValueError, match="^x and y must have the same length.$"):
        ax.hist2d(np.empty((0,)), x)
    with pytest.raises(TypeError, match=r"^object of type 'NoneType' has no len\(\)$"):
        ax.hist2d(None, x)
    with pytest.raises(ValueError, match=r"^autodetected range of \[0.0, inf\] is not finite$"):
        ax.hist2d(np.append(x[:-1], np.inf), x)


def test_linear_locators_keep_their_presets():
    locator = ticker.LinearLocator(presets=[])
    assert len(locator.tick_values(0, 1)) == 11
    with pytest.raises(TypeError, match="^'<' not supported between instances of 'float' and 'str'$"):
        locator.view_limits('0.1', 0.9)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert locator.view_limits(0.9, 0.1) == (0.1, 0.9)


def test_shadows_and_infinite_lines_apply_their_filters():
    fig, ax = plt.subplots()
    rectangle = patches.Rectangle((0, 0), 0.5, 0.5)
    ax.add_patch(rectangle)
    shadow = patches.Shadow(rectangle, 0.1, 0.1)
    ax.add_patch(shadow)
    for artist in (ax.axline((0, 0), (1, 1)), shadow):
        artist.set_agg_filter(3)
        with pytest.raises(TypeError, match="^'int' object is not callable$"):
            fig.canvas.draw()
        artist.set_agg_filter(None)
    fig.canvas.draw()
