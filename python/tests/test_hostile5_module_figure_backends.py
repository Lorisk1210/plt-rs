"""
What the differential tester's hostile programs (fifth round) found pltrs
doing otherwise than matplotlib 3.11 with module-level classes and
functions (patches, quivers, contour sets, lines, hatches, images,
collections, text paths, stacking, Bézier curves, colours, mlab), figures,
layouts, renderers and mathtext. Each expected value is recorded from
matplotlib.
"""

import io
import logging
import re
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import bezier, collections, colorbar, colors, contour, hatch, image, lines, mathtext, offsetbox, patches, quiver
from pltrs import mlab, spines, stackplot, transforms, tri
from pltrs.backends import backend_pdf, backend_ps
from pltrs.backends.backend_agg import RendererAgg
from pltrs.font_manager import FontProperties
from pltrs.path import Path
from pltrs.textpath import TextPath, TextToPath
from pltrs.units import ConversionError


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))


def drawn(fig):
    """Draw *fig*; the exception the draw raised, if any."""
    try:
        fig.canvas.draw()
    except Exception as err:  # noqa: BLE001  (compared with matplotlib's)
        return err
    return None


# -- patches ----------------------------------------------------------------------

@pytest.mark.parametrize("args, kwargs", [
    (((), 6), {}), (('ab', 6), {}), ((None, 6), {}), (((0.2, 0.3), -1), {}),
    (((0.2, 0.3), 10.5), {}), (((0.2, 0.3), 6), dict(radius=None)),
    (((0.2, 0.3), 6), dict(orientation='x')),
])
def test_regular_polygons_keep_what_they_are_given(args, kwargs):
    polygon = patches.RegularPolygon(*args, **kwargs)
    assert polygon.xy is args[0] and polygon.numvertices is args[1]


@pytest.mark.parametrize("vertices, error, message", [
    (0, ZeroDivisionError, "division by zero"),
    (float('nan'), ValueError, "arange: cannot compute length"),
    ('6', TypeError, "'<=' not supported between instances of 'str' and 'int'"),
])
def test_regular_polygons_make_their_path_as_matplotlib(vertices, error, message):
    with pytest.raises(error, match=f"^{message}$"):
        patches.RegularPolygon((0.2, 0.3), vertices)


def test_a_regular_polygon_of_odd_vertices_has_matplotlibs_path():
    assert len(patches.RegularPolygon((0.2, 0.3), 10.5).get_path().vertices) == 12
    assert len(patches.CirclePolygon((0.5, 0.5), 0.2, resolution=-1).get_path().vertices) == 0
    fig, ax = plt.subplots()
    with pytest.raises(RuntimeError, match="^Invalid vertices array.$"):
        ax.add_patch(patches.RegularPolygon((0.2, 0.3), -1))


@pytest.mark.parametrize("xy, kwargs, error, message", [
    ((0.2,), {}, TypeError, r"Affine2D.translate\(\) missing 1 required positional argument: 'ty'"),
    ((0.2, 0.3), dict(radius=None), TypeError,
     "unsupported operand type\\(s\\) for \\*: 'float' and 'NoneType'"),
    ((0.2, 0.3), dict(orientation='x'), TypeError, "must be real number, not str"),
])
def test_regular_polygons_fail_where_drawn(xy, kwargs, error, message):
    fig, ax = plt.subplots()
    polygon = patches.RegularPolygon(xy, 6, **kwargs)
    with pytest.raises(error, match=f"^{message}$"):
        ax.add_patch(polygon)


@pytest.mark.parametrize("args, kwargs", [
    (((), (0.9, 0.9), 'data'), {}), (('ab', (0.9, 0.9), 'data'), {}),
    (((0.1, 0.1), (0.9, 0.9), None), {}), (((0.1, 0.1), (0.9, 0.9), 'data'), dict(axesA=0)),
    (((0.1, 0.1), (0.9, 0.9), 'data'), dict(arrowstyle=None)),
    (((0.1, 0.1), (0.9, 0.9), 'data'), dict(connectionstyle=None)),
    (((0.1, 0.1), (0.9, 0.9), 'data'), dict(shrinkA=None)),
    (((0.1, 0.1), (0.9, 0.9), 'data'), dict(mutation_scale=None)),
])
def test_connection_patches_keep_what_they_are_given(args, kwargs):
    patch = patches.ConnectionPatch(*args, **kwargs)
    assert patch.xy1 is args[0] and patch.coords1 is args[2]


@pytest.mark.parametrize("args, kwargs, error, message", [
    (('ab', (0.9, 0.9), 'data'), {}, ConversionError,
     "Failed to convert value\\(s\\) to axis units: array\\('a', dtype='<U1'\\)"),
    (((0.1, 0.1), (0.9, 0.9), 'nope'), {}, ValueError,
     "nope is not a valid coordinate transformation"),
    (((0.1, 0.1), (0.9, 0.9), 'data'), dict(axesA=0), AttributeError,
     "'int' object has no attribute 'transData'"),
    (((0.1, 0.1), (0.9, 0.9), 'data'), dict(shrinkA=None), TypeError,
     "unsupported operand type\\(s\\) for \\*: 'NoneType' and 'float'"),
])
def test_connection_patches_fail_where_placed(args, kwargs, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{message}$"):
        ax.add_patch(patches.ConnectionPatch(*args, **kwargs))


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(mutation_scale='2222'), TypeError, "can't multiply sequence by non-int of type 'float'"),
    (dict(arrowstyle=None), AttributeError,
     "'FancyArrowPatch' object has no attribute '_arrow_transmuter'"),
    (dict(arrowstyle=3), TypeError, "'int' object is not callable"),
    (dict(arrowstyle=None, mutation_scale='2'), AttributeError,
     "'FancyArrowPatch' object has no attribute '_arrow_transmuter'"),
])
def test_fancy_arrows_fail_where_measured(kwargs, error, message):
    fig, ax = plt.subplots()
    arrow = patches.FancyArrowPatch((0, 0), (1, 1), **kwargs)
    with pytest.raises(error, match=f"^{message}$"):
        ax.add_patch(arrow)
    assert arrow not in ax.patches


def test_fancy_arrow_patches_check_their_properties_first():
    with pytest.raises(AttributeError,
                       match="^FancyArrowPatch.set\\(\\) got an unexpected keyword argument "
                             "'nonexistent_kw'"):
        patches.FancyArrowPatch(nonexistent_kw=1)
    arrow = patches.FancyArrowPatch(path='')
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match="^'str' object has no attribute 'vertices'$"):
        ax.add_patch(arrow)


@pytest.mark.parametrize("args, kwargs, error, message", [
    (('0.5', 0.25, 0.3, 0.4), {}, TypeError,
     'can only concatenate str \\(not "float"\\) to str'),
    ((None, 0.25, 0.3, 0.4), {}, TypeError,
     "unsupported operand type\\(s\\) for \\+: 'NoneType' and 'float'"),
    ((0.5, 0.25, 0.3, 0.4), dict(width=None), TypeError,
     "unsupported operand type\\(s\\) for \\*: 'int' and 'NoneType'"),
    ((0.5, 0.25, 0.3, 0.4), dict(shape=None), ValueError, "Got unknown shape: None"),
    ((0.5, 0.25, 0.3, 0.4), dict(overhang=None), TypeError,
     "unsupported operand type\\(s\\) for -: 'int' and 'NoneType'"),
])
def test_fancy_arrows_make_their_vertices_as_matplotlib(args, kwargs, error, message):
    with pytest.raises(error, match=f"^{message}$"):
        patches.FancyArrow(*args, **kwargs)


def test_fancy_arrows_warn_on_what_is_no_number():
    with pytest.warns(RuntimeWarning, match="invalid value encountered in scalar divide"):
        arrow = patches.FancyArrow(0.5, 0.25, float('inf'), 0.4)
    assert np.isnan(arrow.get_xy()).all()
    with pytest.raises(TypeError, match='can only concatenate str'):
        arrow.set_data(x='a')


@pytest.mark.parametrize("xy, error, message", [
    ((), ValueError, "not enough values to unpack \\(expected 2, got 0\\)"),
    ((0.2, 0.3, 0.3), ValueError, "too many values to unpack \\(expected 2, got 3\\)"),
    (None, TypeError, "cannot unpack non-iterable NoneType object"),
])
def test_fancy_boxes_unpack_their_corner(xy, error, message):
    with pytest.raises(error, match=f"^{message}$"):
        patches.FancyBboxPatch(xy, 0.5, 0.4)


@pytest.mark.parametrize("args, kwargs, error, message", [
    (((0.2, 0.3), None, 0.4), {}, TypeError,
     "unsupported operand type\\(s\\) for \\+: 'NoneType' and 'float'"),
    (('ab', 0.5, 0.4), {}, TypeError,
     "unsupported operand type\\(s\\) for /: 'str' and 'int'"),
    (((0.2, 0.3), 0.5, 0.4), dict(boxstyle=None), AttributeError,
     "'FancyBboxPatch' object has no attribute '_bbox_transmuter'"),
    (((0.2, 0.3), 0.5, 0.4), dict(boxstyle=3), TypeError, "'int' object is not callable"),
])
def test_fancy_boxes_fail_where_drawn(args, kwargs, error, message):
    box = patches.FancyBboxPatch(*args, **kwargs)
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{message}$"):
        ax.add_patch(box)


def test_fancy_box_sides_are_kept():
    box = patches.FancyBboxPatch((0.2, 0.3), 0.5, 0.4)
    box.set_width('1')
    assert box.get_width() == '1'
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^can only concatenate str"):
        ax.add_patch(box)


@pytest.mark.parametrize("path", [None, '', io.StringIO()])
def test_path_patches_of_no_path_fail_as_added(path):
    patch = patches.PathPatch(path)
    assert patch.get_path() is path
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match="has no attribute 'iter_bezier'$"):
        ax.add_patch(patch)
    assert patch not in ax.patches


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(theta1=float('nan')), ValueError, "cannot convert float NaN to integer"),
    (dict(theta1='10'), TypeError, "not all arguments converted during string formatting"),
    (dict(theta2=None), TypeError,
     "unsupported operand type\\(s\\) for %: 'NoneType' and 'int'"),
])
def test_arcs_compute_with_their_angles(kwargs, error, message):
    with pytest.raises(error, match=f"^{message}$"):
        patches.Arc((0.5, 0.5), 0.4, 0.3, **kwargs)


@pytest.mark.parametrize("args, kwargs, error, message", [
    ((None, 0, 1, 1), {}, AttributeError, "'Arrow' object has no attribute '_x'"),
    ((0, 0, '1', 1), {}, TypeError, "ufunc 'hypot' not supported for the input types"),
    ((0, 0, 1, 1), dict(width=None), AttributeError,
     "'Arrow' object has no attribute '_width'"),
])
def test_arrows_build_their_transform_as_matplotlib(args, kwargs, error, message):
    with pytest.raises(error, match=f"^{message}"):
        patches.Arrow(*args, **kwargs)


def test_shadows_keep_their_offset():
    circle = patches.Circle((0.5, 0.5), 0.2)
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute '_transform'$"):
        patches.Shadow(None, 0.02, -0.02)
    with pytest.raises(TypeError, match="^'<=' not supported between instances of 'int' and 'str'$"):
        patches.Shadow(circle, 0.02, -0.02, shade='0.7')
    with pytest.raises(AttributeError, match="^'Shadow' object has no property 'nonexistent_kw'$"):
        patches.Shadow(circle, 0.02, -0.02, nonexistent_kw=1)
    fig, ax = plt.subplots()
    ax.add_patch(circle)
    ax.add_patch(patches.Shadow(circle, None, -0.02))
    assert isinstance(drawn(fig), TypeError)


def test_patches_drawn_with_no_transform_fail_to_compose_it():
    fig, ax = plt.subplots()
    fig.canvas.draw()
    bbox = transforms.Bbox.from_bounds(0.1, 0.2, 0.5, 0.4)
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for \\+: "
                                        "'CompositeGenericTransform' and 'int'$"):
        patches.draw_bbox(bbox, fig.canvas.get_renderer(), trans=3)


# -- quivers ----------------------------------------------------------------------

ARROWS = ([0, 1], [0, 1], [1, 2], [2, 1])


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(scale='asinh'), TypeError,
     "can't multiply sequence by non-int of type 'numpy.float64'"),
    (dict(units='points'), ValueError, "'points' is not a valid value for units"),
    (dict(angles='data'), TypeError, "ufunc 'deg2rad' not supported for the input types"),
])
def test_quivers_keep_their_settings_until_drawn(kwargs, error, message):
    fig, ax = plt.subplots()
    arrows = quiver.Quiver(ax, *ARROWS, **kwargs)
    ax.add_collection(arrows)
    with pytest.raises(error, match=f"^{message}"):
        fig.canvas.draw()


def test_quivers_read_the_data_transform_of_their_axes():
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'transData'$"):
        quiver.Quiver(None, *ARROWS)


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(pivot='tail'), KeyError, "'tail'"),
    (dict(length='4'), TypeError,
     "unsupported operand type\\(s\\) for \\*\\* or pow\\(\\): 'str' and 'int'"),
    (dict(sizes=[10]), AttributeError, "'list' object has no attribute 'get'"),
    (dict(barb_increments='x'), TypeError,
     "pltrs.quiver.Barbs._find_tails\\(\\) argument after \\*\\* must be a mapping, not str"),
])
def test_barbs_compute_with_their_settings_as_made(kwargs, error, message):
    fig, ax = plt.subplots()
    message = message.replace("pltrs", pltrs.__name__)
    with pytest.raises(error, match=f"^{message}$"):
        quiver.Barbs(ax, [0, 1], [0, 1], [10, 20], [5, 5], **kwargs)


def test_barbs_ignore_false_colours():
    fig, ax = plt.subplots()
    quiver.Barbs(ax, [0, 1], [0, 1], [10, 20], [5, 5], barbcolor=0)


def test_quiver_keys_keep_their_angle_and_label_space():
    fig, ax = plt.subplots()
    arrows = ax.quiver([0], [0], [1], [1])
    ax.quiverkey(arrows, 0.5, 0.5, 1, 'k', angle=None)
    assert isinstance(drawn(fig), TypeError)
    with pytest.raises(ValueError, match="^0 is not a valid value for color"):
        quiver.QuiverKey(arrows, 0.5, 0.5, 1, 'k', labelcolor=0)


def test_draws_warn_only_for_the_artists_they_reach():
    fig, ax = plt.subplots()
    ax.quiverkey(ax.quiver([0, 1], [0, 1], [1, -1], [1, 2]), 0.8, 0.9, float('inf'), 'inf')
    with pytest.warns(RuntimeWarning, match="^invalid value encountered in multiply$"):
        fig.canvas.draw()
    rectangle = ax.add_patch(patches.Rectangle((0.1, 0.2), 0.5, 0.3))
    rectangle.set_angle('30')
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        with pytest.raises(TypeError, match="^must be real number, not str$"):
            fig.canvas.draw()


# -- contour sets -------------------------------------------------------------------

SEGMENTS = [[[(0, 0), (1, 1), (0, 1)]], [[(0.2, 0.2), (0.5, 0.5)]]]


@pytest.mark.parametrize("ax, message", [
    (None, "'NoneType' object has no attribute 'add_collection'"),
    (3, "'int' object has no attribute 'stale' and no __dict__ for setting new attributes"),
])
def test_contour_sets_made_for_no_axes(ax, message):
    with pytest.raises(AttributeError, match=f"^{message}$"):
        contour.ContourSet(ax, [0, 1], SEGMENTS)


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(linestyles='DASHED'), ValueError,
     "Do not know how to convert \\['DASHED', 'DASHED'\\] to dashes"),
    (dict(linestyles=3), ValueError, "Unrecognized type for linestyles kwarg"),
    (dict(colorizer=3), TypeError,
     "'colorizer' must be an instance of pltrs.colorizer.Colorizer, not a int"),
])
def test_contour_sets_check_their_styles(kwargs, error, message):
    fig, ax = plt.subplots()
    message = message.replace("pltrs", pltrs.__name__)
    with pytest.raises(error, match=f"^{message}$"):
        contour.ContourSet(ax, [0, 1], SEGMENTS, **kwargs)
    assert not ax.collections


@pytest.mark.parametrize("kwargs", [
    dict(hatches=None), dict(colors=[]), dict(nchunk=0.5), dict(locator=0),
    dict(negative_linestyles='x'),
])
def test_contour_sets_keep_what_they_do_not_use(kwargs):
    fig, ax = plt.subplots()
    contour.ContourSet(ax, [0, 1], SEGMENTS, **kwargs)
    fig.canvas.draw()


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(extent='ab'), ValueError, "If given, 'extent' must be None or \\(x0, x1, y0, y1\\)"),
    (dict(nchunk=-1), ValueError, "chunk_size cannot be negative"),
    (dict(locator=0), AttributeError, "'int' object has no attribute 'tick_values'"),
    (dict(transform=3), AttributeError,
     "'int' object has no attribute 'contains_branch_separately'"),
])
def test_quad_contour_sets_check_what_they_contour_with(kwargs, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{message}$"):
        contour.QuadContourSet(ax, Z, **kwargs)


def test_quad_contour_sets_take_contour_sets_constructor():
    with pytest.raises(TypeError, match="^ContourSet.__init__\\(\\) missing 1 required "
                                        "positional argument: 'ax'$"):
        contour.QuadContourSet()


# -- lines, hatches, images, spines, offset boxes -------------------------------------

@pytest.mark.parametrize("kwargs, message", [
    (dict(linestyle=3), "Unrecognized linestyle: 3"),
    (dict(fillstyle=''), "'' is not a valid value for fillstyle"),
    (dict(drawstyle='STEPS-MID'), "'STEPS-MID' is not a valid value for drawstyle"),
    (dict(pickradius=None), "pick radius should be a distance"),
])
def test_lines_check_their_styles_as_set(kwargs, message):
    with pytest.raises(ValueError, match=f"^{message}"):
        lines.Line2D([0, 1, 2], [2, 0, 1], **kwargs)


def test_lines_keep_marker_colours_that_are_no_colours():
    line = lines.Line2D([0, 1, 2], [2, 0, 1], markeredgecolor=3, marker='o')
    assert line.get_markeredgecolor() == 3
    fig, ax = plt.subplots()
    ax.add_line(line)
    with pytest.raises(ValueError, match="^Invalid RGBA argument: 3$"):
        fig.canvas.draw()


def test_hatch_paths_are_made_as_matplotlib():
    assert hatch.NorthEastHatch('//', -1.5).num_vertices == -4
    assert hatch.HorizontalHatch('-', -2).num_vertices == -4
    with pytest.raises(ValueError, match="^negative dimensions are not allowed$"):
        hatch.get_path('//', density=-1.5)
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'count'$"):
        hatch.get_path(3)
    assert hatch.get_path('/') is not hatch.get_path('/')


def test_axes_images_keep_their_extent():
    fig, ax = plt.subplots()
    im = image.AxesImage(ax, extent='ab')
    assert im.get_extent() == 'ab'
    with pytest.raises(AttributeError, match="has no attribute 'stale'"):
        image.NonUniformImage(3)


def test_rgba_images_of_booleans_fail_to_resample():
    fig, ax = plt.subplots()
    ax.imshow(np.ones((3, 4, 4), bool))
    with pytest.raises(ValueError, match="^arrays must be of dtype byte, short, float32 or "
                                         "float64$"):
        fig.canvas.draw()


@pytest.mark.parametrize("fmt", ["pdf", "svg"])
def test_an_empty_image_fails_its_composite(fmt):
    fig, ax = plt.subplots()
    ax.imshow(np.empty((0, 6)))
    ax.imshow(np.ones((1, 6)))
    with pytest.raises(RuntimeError, match="^_make_image must get a non-empty image"):
        fig.savefig(io.BytesIO(), format=fmt)


def test_offset_images_check_their_settings():
    with pytest.raises(ValueError, match="^'' is not a valid value for interpolation"):
        offsetbox.OffsetImage(Z, interpolation='')
    with pytest.raises(ValueError, match="^The filter radius must be a positive number$"):
        offsetbox.OffsetImage(Z, filterrad=0.0)
    with pytest.raises(AttributeError, match="^'str' object has no attribute 'items'$"):
        offsetbox.PaddedBox(offsetbox.TextArea('c'), patch_attrs='x')


def test_annotation_boxes_update_their_frame_with_a_mapping():
    with pytest.raises(TypeError, match="argument after \\*\\* must be a mapping, not str$"):
        offsetbox.AnnotationBbox(offsetbox.TextArea('ab'), (0.5, 0.5), bboxprops='x')


def test_spines_refuse_extra_arguments_in_their_constructor():
    with pytest.raises(TypeError, match="^Spine.__init__\\(\\) takes 4 positional arguments "
                                        "but 5 were given$"):
        spines.Spine(None, None, None, None)


def test_colorbar_axes_of_no_aspect():
    fig, ax = plt.subplots()
    cax, kwargs = colorbar.make_axes_gridspec(ax, aspect=None)
    assert cax.get_box_aspect() is None
    with pytest.raises(TypeError, match="^unsupported operand type"):
        colorbar.make_axes_gridspec(ax, aspect=None, location='top')


def test_colorbars_of_unknown_keywords_are_refused_in_their_axes():
    fig, ax = plt.subplots()
    other, other_ax = plt.subplots()
    mappable = other_ax.imshow(Z)
    with pytest.warns(UserWarning, match="^Adding colorbar to a different Figure"):
        with pytest.raises(TypeError, match="unexpected keyword argument 'nonexistent_kw'"):
            fig.colorbar(mappable, nonexistent_kw=1)
    assert len(other.axes) == 2


# -- collections -------------------------------------------------------------------------

@pytest.mark.parametrize("cls, vertices", [
    (collections.RegularPolyCollection, 6),
    (collections.StarPolygonCollection, 11),
    (collections.AsteriskPolygonCollection, 11),
])
def test_regular_collections_make_their_classes_paths(cls, vertices):
    made = cls(5, sizes=(10, 20))
    assert made.get_numsides() == 5
    assert len(made.get_paths()[0].vertices) == vertices
    assert made.get_sizes().tolist() == [10, 20]


@pytest.mark.parametrize("cls, numsides, error, message", [
    (collections.RegularPolyCollection, 0, ZeroDivisionError, "division by zero"),
    (collections.RegularPolyCollection, float("nan"), ValueError,
     "arange: cannot compute length"),
    (collections.RegularPolyCollection, "4", TypeError,
     "'<=' not supported between instances of 'str' and 'int'"),
    (collections.StarPolygonCollection, -1, ValueError,
     "negative dimensions are not allowed"),
    (collections.StarPolygonCollection, 5.5, TypeError,
     "expected a sequence of integers or a single integer, got '12.0'"),
    (collections.AsteriskPolygonCollection, None, TypeError,
     "'<=' not supported between instances of 'NoneType' and 'int'"),
])
def test_regular_collections_fail_on_numbers_of_sides(cls, numsides, error, message):
    with pytest.raises(error, match=f"^{re.escape(message)}$"):
        cls(numsides)


def test_regular_collections_of_no_sides_have_an_empty_path():
    made = collections.RegularPolyCollection(-1)
    assert made.get_numsides() == -1
    assert made.get_paths()[0].vertices.shape == (0, 2)


@pytest.mark.parametrize("make", [
    lambda: collections.RegularPolyCollection(4, sizes='ab'),
    lambda: collections.CircleCollection('ab'),
    lambda: collections.CircleCollection(5),
])
def test_collections_with_sizes_fail_on_sizes_of_no_length(make):
    with pytest.raises(TypeError, match="^len\\(\\) of unsized object$"):
        make()


def test_ellipse_collections_keep_their_sizes_unbroadcast():
    made = collections.EllipseCollection([1, 2, 2], [0.5, 1], [0, 30])
    assert made.get_widths().tolist() == [1, 2, 2]
    assert made.get_heights().tolist() == [0.5, 1]
    made = collections.EllipseCollection([1, 2], 3, 0)
    assert made.get_heights().tolist() == [3]
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for \\*: "
                                        "'float' and 'NoneType'$"):
        collections.EllipseCollection(None, [0.5, 1], [0, 30])
    with pytest.raises(TypeError, match="^ufunc 'multiply' did not contain a loop"):
        collections.EllipseCollection([1, 2], 'ab', [0, 30])
    with pytest.raises(TypeError, match="^loop of ufunc does not support argument 0"):
        collections.EllipseCollection([1, 2], [0.5, 1], None)


def test_collections_fail_on_what_is_no_patch_path_or_triangulation():
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'get_transform'$"):
        collections.PatchCollection([None])
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'get_fill'$"):
        collections.PatchCollection([None], match_original=True)
    assert collections.PathCollection([None]).get_paths() == [None]
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'x'$"):
        collections.TriMesh(None)


def test_collections_are_made_with_no_offsets():
    assert collections.Collection(offsets=[]).get_offsets().size == 0


def test_fill_between_collections_check_shapes_before_dimensions():
    x = np.linspace(0, 1, 7)
    with pytest.raises(ValueError, match="^'x' has size 6, but 'y1' has an unequal "
                                         "size of 7$"):
        collections.FillBetweenPolyCollection('x', x[:-1], x, 0)
    with pytest.raises(ValueError, match="^'y1' is not 1-dimensional$"):
        collections.FillBetweenPolyCollection('x', x, x[None, :], 0)
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'ndim'$"):
        collections.FillBetweenPolyCollection('x', x, x, 0)


# -- tri ---------------------------------------------------------------------------------

def test_tri_classes_refuse_arguments_in_their_own_names():
    triangulation = tri.Triangulation([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5])
    with pytest.raises(TypeError, match="^LinearTriInterpolator.__init__\\(\\) got an "
                                        "unexpected keyword argument"):
        tri.LinearTriInterpolator(triangulation, [1, 2, 3, 4, 0], nonexistent_kw=1)
    with pytest.raises(TypeError, match="^UniformTriRefiner.__init__\\(\\) missing"):
        tri.UniformTriRefiner()
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'add_collection'$"):
        tri.tricontour(None, triangulation, [1, 2, 3, 4, 0])
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'grid'$"):
        tri.tripcolor(3, triangulation, [1, 2, 3, 4, 0])


# -- annotations, layouts, figures -----------------------------------------------------

def test_annotations_of_category_names_fail_at_every_draw():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.annotate('a', 'ab')
    for _ in range(2):
        with pytest.raises(ConversionError,
                           match="^Failed to convert value\\(s\\) to axis units: 'a'$"):
            fig.canvas.draw()


def test_a_tight_layout_of_nan_spaces_stays_nan():
    fig = plt.figure(layout='tight')
    ax = fig.add_subplot()
    ax.plot([0, 1], [0, 1])
    engine = fig.get_layout_engine()
    engine.set(pad=float('nan'))
    assert isinstance(drawn(fig), ValueError)
    engine.set(pad=2)
    assert isinstance(drawn(fig), ValueError)
    assert np.isnan(fig.subplotpars.left)


def test_tight_layout_ticks_an_axes_before_measuring_its_texts():
    fig, ax = plt.subplots()
    ax.plot([1, 2], [3, 4])
    ax.text('0.5', 0.5, 'hi')
    with pytest.raises(ConversionError):
        fig.tight_layout()
    ax.ticklabel_format(style='sci', scilimits=(0, 0), useOffset=[])
    with pytest.raises(ValueError, match="^operands could not be broadcast together"):
        fig.tight_layout()


def test_layouts_do_not_measure_texts_they_leave_out():
    fig, ax = plt.subplots()
    ax.text('0.5', 0.5, 'hi', in_layout=False)
    fig.tight_layout()
    ax.get_tightbbox()


def test_constrained_layout_does_not_measure_figure_texts():
    fig = plt.figure(layout='constrained')
    fig.subplots(height_ratios=[float('nan')])
    fig.text(None, 0.25, 'text')
    err = drawn(fig)
    assert isinstance(err, ValueError) and str(err) == "cannot convert float NaN to integer"


def test_saving_draws_a_laid_out_figure_before_telling_its_observers():
    fig = plt.figure(layout='constrained')
    fig.subplot_mosaic('AB;CC', width_ratios=[float('nan'), 1])
    fig.add_callback(None)
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        fig.savefig(io.BytesIO(), format='pdf')


def test_vector_formats_print_at_an_integer_resolution():
    fig, ax = plt.subplots()
    ax.set_xlabel('x', labelpad='5')
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for /: 'str' and "
                                        "'int'$"):
        fig.savefig(io.BytesIO(), format='svg')
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type 'float'$"):
        fig.savefig(io.BytesIO(), format='png')


# -- renderers -------------------------------------------------------------------------

def test_pdf_alpha_states_are_kept_as_given():
    for alpha, error in [(('a', 'b'), None), ((float('nan'), 0.5), ValueError),
                         ((object(), 1), TypeError)]:
        pdf = backend_pdf.PdfFile(io.BytesIO())
        assert pdf.alphaState(alpha).name == b'A1'
        pdf.newPage(3, 2)
        if error is None:
            pdf.finalize()
        else:
            with pytest.raises(error):
                pdf.finalize()


def test_pdf_renderers_write_coordinates_of_no_number():
    pdf = backend_pdf.PdfFile(io.BytesIO())
    pdf.newPage(4, 3)
    renderer = backend_pdf.RendererPdf(pdf, 72, 3, 4)
    gc = renderer.new_gc()
    prop = pltrs.font_manager.FontProperties(size=12)
    renderer.draw_text(gc, None, 10, 'ab', prop, 0)
    renderer.draw_mathtext(gc, 10, None, '$x^2$', prop, 0)
    renderer.draw_image(gc, None, 0, np.zeros((3, 4, 4), np.uint8))
    renderer.draw_path(gc, Path([(0, 0), (1, 1)]), transforms.Affine2D(), rgbFace='ab')


def test_ps_renderers_take_what_matplotlibs_writes():
    renderer = backend_ps.RendererPS(4, 3, io.StringIO(), imagedpi=72)
    gc = renderer.new_gc()
    renderer.draw_markers(gc, None, transforms.Affine2D(), Path([(0, 0), (1, 1)]),
                          transforms.Affine2D())
    points = np.array([[[float('nan'), 0.0], [10.0, 0.0], [0.0, 10.0]]])
    colors = np.array([[[1, 0, 0, 1], [0, 1, 0, 1], [0, 0, 1, 1]]], float)
    renderer.draw_gouraud_triangles(None, points[:, :, :] * 0, colors,
                                    transforms.IdentityTransform())
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        renderer.draw_gouraud_triangles(gc, points, colors, transforms.IdentityTransform())
    assert [str(w.message) for w in caught] == ["invalid value encountered in cast"]


def test_ps_text_typeset_by_tex_draws_through_draw_tex(caplog):
    renderer = backend_ps.RendererPS(4, 3, io.StringIO(), imagedpi=72)
    prop = pltrs.font_manager.FontProperties(size=12)
    with caplog.at_level(logging.WARNING):
        renderer.draw_text(renderer.new_gc(), 10, 10, 'ab', prop, 0, ismath='TeX')
    assert "determines usetex status solely" in caplog.text


def test_agg_renderers_have_matplotlibs_box():
    assert RendererAgg(40, 30, 72).bbox.bounds == (0.0, 0.0, 40.0, 30.0)
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for \\+: 'int' and "
                                        "'str'$"):
        RendererAgg('40', 30, 72)


# -- mathtext ----------------------------------------------------------------------------

@pytest.mark.parametrize("dpi, error, message", [
    (-1, RuntimeError, "FT_Set_Char_Size \\(ft2font.cpp line 248\\) failed with error 0x17: "
                       "invalid pixel size"),
    (1e10, RuntimeError, "FT_Set_Char_Size \\(ft2font.cpp line 248\\) failed with error 0x17"),
    ('72', TypeError, "set_size\\(\\): incompatible function arguments"),
    (None, TypeError, "set_size\\(\\): incompatible function arguments"),
])
def test_mathtext_sizes_its_fonts_as_freetype(dpi, error, message):
    with pytest.raises(error, match=f"^{message}"):
        mathtext.MathTextParser('path').parse('$x^2$', dpi=dpi)


def test_mathtext_logs_lists_its_glue_cannot_fit(caplog):
    with caplog.at_level(logging.WARNING):
        mathtext.MathTextParser('path').parse('$x^2$', dpi=float('nan'))
    messages = [record.getMessage() for record in caplog.records]
    assert len(messages) == 4
    assert all(message.startswith("Underful Hlist: Hlist<w=nan ") for message in messages)
    assert all(message.endswith("\n]") and "  knan,\n" in message for message in messages)


@pytest.mark.parametrize("symbol, message", [
    (None, "'NoneType' object has no attribute 'strip'"),
    (3, "'int' object has no attribute 'strip'"),
])
def test_unicode_indices_of_what_is_neither_character_nor_name(symbol, message):
    with pytest.raises(AttributeError, match=f"^{message}$"):
        mathtext.get_unicode_index(symbol)
    assert mathtext.get_unicode_index(b'a') == 97


# -- text paths --------------------------------------------------------------------------

@pytest.mark.parametrize("s, ismath, count, first", [
    ('$x^2$', False, 194, [33.7969, -14.7031]),
    ('$x^2$', True, 53, [60.0156, 55.4344]),
    ('Ab', False, 56, [34.1875, 63.1875]),
    ('Ab', 'nope', 56, [34.1875, 63.2031]),
])
def test_text_paths_typeset_as_their_ismath_says(s, ismath, count, first):
    vertices, codes = TextToPath().get_text_path(FontProperties(), s, ismath=ismath)
    assert len(vertices) == len(codes) == count
    assert np.asarray(vertices)[0].round(4).tolist() == first


def test_text_paths_of_math_text_typeset_it():
    assert TextPath((0, 0), '$Ab$', size=10).vertices[0].round(5).tolist() == [3.68125, 7.29219]


# -- module functions --------------------------------------------------------------------

def test_stacking_on_what_is_no_axes_stacks_first():
    with pytest.raises(ValueError, match="^need at least one array to concatenate$"):
        stackplot.stackplot(None, 0.5)
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute '_get_lines'$"):
        stackplot.stackplot(None, [1, 2], [3, 4])


def test_parallels_of_no_width_keep_the_end_points():
    left, right = bezier.get_parallels([(0, 0), (1, 1), (2, 0)], 0.0)
    assert type(left[0][0]) is int and type(right[2][1]) is int
    assert type(left[1][0]) is float


def test_hsv_of_no_finite_numbers_warns_as_numpy():
    with pytest.warns(RuntimeWarning, match="^invalid value encountered in cast$"):
        colors.hsv_to_rgb(np.array([np.nan, 1.0, 1.0]))
    with pytest.warns(RuntimeWarning, match="^invalid value encountered in multiply$"):
        colors.hsv_to_rgb(np.array([0.5, 1.0, np.inf]))


def test_kernel_density_estimates_of_no_numbers_fail_as_numpy():
    with pytest.raises(np.exceptions.DTypePromotionError):
        mlab.GaussianKDE(np.datetime64('2024-01-01') + np.arange(7))
    with pytest.warns(RuntimeWarning, match="^invalid value encountered in det$"):
        mlab.GaussianKDE(np.array([np.nan, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5]))


def test_spectra_of_signals_of_two_dimensions_as_matplotlib():
    pxy, freqs = mlab.csd(np.sin(np.arange(64.0))[None, :], np.cos(np.arange(64.0)), NFFT=16)
    assert pxy.shape == (9,) and not pxy.any()


def test_linear_trend_of_one_value_warns_of_its_slope():
    with pytest.warns(RuntimeWarning, match="^invalid value encountered in scalar divide$"):
        mlab.detrend_linear(np.array([0.0]))
