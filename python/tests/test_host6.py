"""
What the differential tester's hostile programs (sixth round) found pltrs
doing otherwise than matplotlib 3.11 with wrong and unusual arguments:
constructor orders, formatters given what is no number, failed draws of
host Axes, collections' refused colours and empty paths, 3-D units and
stems, arrows of points that are not finite. Each expected value is
recorded from matplotlib.
"""

import io
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.axis as maxis
import pltrs.collections as mcollections
import pltrs.offsetbox as moffsetbox
import pltrs.patches as mpatches
import pltrs.pyplot as plt
import pltrs.ticker as mticker
import pltrs.units as munits
from pltrs import bezier, transforms
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import host_subplot, parasite_axes
from pltrs.mpl_toolkits.axes_grid1 import axes_size as Size
from pltrs.mpl_toolkits.axisartist import angle_helper
from pltrs.mpl_toolkits.mplot3d import art3d, axes3d
from pltrs.path import Path
from pltrs.projections import polar
from pltrs.textpath import TextPath


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)
Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))


def draw_error(fig):
    """The exception drawing *fig* raises, as ``(type name, message)``."""
    with pytest.raises(Exception) as info:
        fig.canvas.draw()
    return type(info.value).__name__, str(info.value)


# --- Constructors in matplotlib's order ----------------------------------

def test_axis_checks_its_pick_radius_before_it_clears():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="pick radius should be a distance"):
        maxis.Axis(ax, pickradius=-1)
    with pytest.raises(ValueError, match="pick radius should be a distance"):
        maxis.Axis(ax, pickradius=None)
    with pytest.raises(AttributeError, match="'Axis' object has no attribute '_init'"):
        maxis.Axis(ax, pickradius=0)


@pytest.mark.parametrize("cls", [maxis.Tick, polar.ThetaTick])
def test_ticks_read_the_figure_of_their_axes_first(cls):
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'get_figure'"):
        cls(None, 0.5)
    with pytest.raises(AttributeError,
                       match="'str' object has no attribute 'get_figure'"):
        cls('x', 0.5)


def test_line_collection_2d_to_3d_reads_the_paths_first():
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'get_paths'"):
        art3d.line_collection_2d_to_3d(None)


def test_poly3d_collection_takes_its_faces_before_checking_them():
    with pytest.raises(TypeError, match="object of type 'NoneType' has no len"):
        art3d.Poly3DCollection(None)
    with pytest.raises(ValueError, match=r"verts must be a list of \(N, 3\)"):
        art3d.Poly3DCollection([[1, 2, 3]])


def test_connection_patch_keeps_the_annotation_clip_given():
    patch = mpatches.ConnectionPatch((0, 0), (1, 1), "data")
    for clip in (1, 'False', np.True_, None):
        patch.set_annotation_clip(clip)
        assert patch.get_annotation_clip() is clip


# --- Renderers given to measuring methods ---------------------------------

def test_packers_convert_points_with_the_renderer_first():
    fig, ax = plt.subplots()
    box = moffsetbox.HPacker(children=[moffsetbox.TextArea('a'),
                                       moffsetbox.TextArea('b')], pad=2, sep=3)
    ax.add_artist(moffsetbox.AnchoredOffsetbox('center', child=box))
    for renderer, name in ((None, 'NoneType'), (3, 'int')):
        with pytest.raises(AttributeError, match=f"'{name}' object has no "
                           "attribute 'points_to_pixels'"):
            box.get_bbox(renderer)


def test_axis_labels_lay_out_with_the_renderer_given():
    fig, ax = plt.subplots()
    label = ax.set_ylabel('label')
    with pytest.raises(TypeError, match="cannot create weak reference to 'int'"):
        label.get_window_extent(3)
    size = Size.MaxWidth([label])
    with pytest.raises(TypeError, match="cannot create weak reference to 'str'"):
        size.get_size('x')


# --- Formatters and locators given what is no number ----------------------

def test_scalar_formatter_compares_odd_locations_with_its_view():
    fmt = mticker.ScalarFormatter(useOffset=False)
    fmt.create_dummy_axis()
    with pytest.raises(TypeError, match="'>=' not supported between instances "
                       "of 'NoneType' and 'int'"):
        fmt.format_ticks([0.5, 1.0, None])


def test_scalar_formatter_of_no_axis_asks_none_for_its_view():
    fmt = mticker.ScalarFormatter()
    with pytest.raises(AttributeError, match="'NoneType' object has no "
                       "attribute 'get_view_interval'"):
        fmt.set_locs([1.0, 2.0])


def test_scalar_formatter_compares_power_limits_as_given():
    fig, ax = plt.subplots()
    ax.plot([1e5, 2e5])
    fig.canvas.draw()
    fmt = ax.yaxis.get_major_formatter()
    fmt.set_powerlimits((float('nan'), 2))
    fig.canvas.draw()
    assert [t.get_text() for t in ax.get_yticklabels()][:3] == ['0.8', '1.0', '1.2']
    assert fmt.get_offset() == '1e5'
    fmt.set_powerlimits('ab')
    assert draw_error(fig) == (
        'TypeError', "'<=' not supported between instances of 'int' and 'str'")


def test_eng_formatter_subtracts_its_offset_from_text():
    fmt = mticker.EngFormatter()
    fmt.create_dummy_axis()
    with pytest.raises(TypeError, match="ufunc 'subtract' did not contain a loop"):
        fmt.set_locs('ab')


def test_logit_formatter_keeps_locations_that_are_not_finite():
    fmt = mticker.LogitFormatter()
    fmt.set_locs([float('nan'), 0.1, 0.5, 0.9])
    assert fmt(0.1) == r'$\mathdefault{10^{-1}}$'
    assert fmt(0.5) == r'$\mathdefault{\frac{1}{2}}$'
    fmt.set_locs('ab')
    assert fmt(1.5) == ''


def test_max_n_locator_of_fewer_than_one_bin_scales_as_matplotlib():
    with pytest.raises(ZeroDivisionError):
        mticker.MaxNLocator(nbins=0).tick_values(0, 8)
    with pytest.raises(ValueError, match="expected a positive input, got -8.0"):
        mticker.MaxNLocator(nbins=-1).tick_values(0, 8)


def test_select_step_sub_compares_in_the_precision_given():
    assert angle_helper.select_step_sub(np.float32(0.3)) == (2, 10.0)
    assert angle_helper.select_step_sub(0.7) == (5, 10.0)


# --- Units ----------------------------------------------------------------

def test_empty_paths_convert_through_a_converter_without_categories():
    fig, ax = plt.subplots()
    cs = ax.contour(Z, levels=3)
    with pytest.raises(munits.ConversionError):
        ax.xaxis.convert_units('2')
    assert draw_error(fig) == (
        'ConversionError',
        "Failed to convert value(s) to axis units: array([], dtype=float64)")
    assert cs.get_paths()


def test_date_axes_convert_what_is_no_date_with_their_converter():
    fig, ax = plt.subplots()
    ax.plot(np.arange('2024-01-01', '2024-01-08', dtype='datetime64[D]'), np.arange(7))
    for start in ('0.2', None):
        with pytest.raises(munits.ConversionError,
                           match=r"Failed to convert value\(s\) to axis units"):
            ax.axvspan(start, 0.4)


def test_category_axes_convert_what_is_no_name_with_their_converter():
    fig, ax = plt.subplots()
    ax.plot(['a', 'b'], [1, 2])
    with pytest.raises(munits.ConversionError,
                       match=r"Failed to convert value\(s\) to axis units"):
        ax.axvspan('0.2', 0.4)


def test_converter_set_to_none_reads_none():
    fig, ax = plt.subplots()
    ax.plot(['a', 'b'], [1, 2])
    with warnings.catch_warnings():
        warnings.simplefilter('ignore')
        ax.xaxis.converter = None
        assert ax.xaxis.converter is None
    assert ax.xaxis.get_converter() is None


def test_cleared_3d_axes_no_longer_follow_units():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    ax.plot(X, Y, Y)
    ax.cla()
    ax.plot3D(np.array([False, True, True, False, True, True, False]), Y, Y)
    ax.plot3D(np.datetime64('2024-01-01') + np.arange(7), Y, Y)
    with pytest.raises(ValueError, match="could not convert string to float"):
        ax.plot3D('data', Y, Y)


# --- 3-D ------------------------------------------------------------------

def test_stem_plots_a_marker_format_of_no_string_as_data():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    with pytest.raises(ValueError, match=r"operands could not be broadcast "
                       r"together with remapped shapes \[original->remapped\]: "
                       r"\(7,\)  and requested shape \(1,\)"):
        ax.stem(X, Y, Y, markerfmt=3)


def test_get_test_data_steps_as_numpy_does():
    x, y, z = axes3d.get_test_data(float('inf'))
    assert x.shape == (1, 1)
    x, y, z = axes3d.get_test_data(np.float32(0.05))
    assert x[0, 1] == pytest.approx(-29.500000476837158, abs=1e-12)


def test_animated_surface_maps_its_array_as_it_is_projected():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    surf = ax.plot_surface(*np.meshgrid(np.arange(6), np.arange(5)), Z)
    surf.set_animated(True)
    surf.set_array(np.array([[0.0, 1.0, 2.0]]))
    assert draw_error(fig) == ('ValueError', 'Collections can only map rank 1 arrays')
    surf.set_array(np.arange(3.0))
    fig.canvas.draw()
    with pytest.raises(ValueError):
        surf.set_edgecolor('K')
    for _ in range(2):
        assert draw_error(fig) == ('ValueError', "'K' is not a valid color value.")
    surf.set_edgecolor('k')
    fig.canvas.draw()


# --- Collections and images -----------------------------------------------

def test_collections_of_no_paths_fail_to_count_them():
    fig, ax = plt.subplots()
    coll = mcollections.EllipseCollection([1, 2], [0.5, 1], [0, 30], units='xy',
                                          offsets=[[0, 0], [1, 1]],
                                          offset_transform=ax.transData)
    ax.add_collection(coll)
    coll.set_paths(None)
    assert draw_error(fig) == ('TypeError', "object of type 'NoneType' has no len()")
    # A draw of offsets, none of them, returns before it counts them.
    coll.set_offsets(np.empty((0, 2)))
    fig.canvas.draw()


def test_refused_edge_colours_fail_every_draw_while_mapped():
    fig, ax = plt.subplots()
    coll = ax.scatter(X, Y, c=Y)
    fig.canvas.draw()
    with pytest.raises(ValueError):
        coll.set_edgecolor('R')
    for _ in range(2):
        assert draw_error(fig) == ('ValueError', "'R' is not a valid color value.")
    coll.set_edgecolor('r')
    fig.canvas.draw()


def test_collections_of_no_offsets_draw_nothing_to_fail_on():
    fig, ax = plt.subplots()
    mapped = ax.scatter([], [], c=[])
    fig.canvas.draw()
    with pytest.raises(ValueError):
        mapped.set_edgecolor('R')
    unmapped = ax.scatter([], [])
    with pytest.raises(ValueError):
        unmapped.set_facecolor('R')
    with pytest.raises(munits.ConversionError):
        ax.xaxis.convert_units('2')
    for _ in range(2):
        fig.canvas.draw()
    # Their draws returned before mapping: the face colour still fails the
    # first draw of an offset.
    unmapped.set_offsets([[0.5, 0.5]])
    assert draw_error(fig) == ('ValueError', "'R' is not a valid color value.")
    unmapped.set_facecolor('b')
    mapped.set_offsets([[0.5, 0.5]])
    mapped.set_array(np.array([1.0]))
    assert draw_error(fig) == ('ValueError', "'R' is not a valid color value.")


def test_quiver_components_of_different_sizes_fail_xy_draws():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1, 2], [0, 1, 0], [1, -1, 0.5], [1, 2, -1], angles='xy')
    q.set_UVC([1], [0, 1, 0])
    name, message = draw_error(fig)
    assert name == 'ValueError'
    assert message.startswith("all the input array dimensions except for the "
                              "concatenation axis must match exactly")


def test_bool_rgba_images_fail_to_composite():
    fig, ax = plt.subplots()
    image = np.zeros((3, 4, 4), dtype=bool)
    ax.imshow(image)
    ax.imshow(image)
    for fmt in ('pdf', 'svg'):
        with pytest.raises(ValueError, match="arrays must be of dtype byte, "
                           "short, float32 or float64"):
            fig.savefig(io.BytesIO(), format=fmt)


def test_contour_limits_set_again_map_again():
    fig, ax = plt.subplots()
    plt.contour(Z)
    with pytest.raises(ValueError, match="minvalue must be less than or equal"):
        plt.clim(vmin=float('inf'))
    plt.clim(vmin=0.0)
    fig.canvas.draw()


# --- Patches, arrows, offset boxes ----------------------------------------

def test_path_patch_keeps_a_text_path_of_no_size():
    fig, ax = plt.subplots()
    path = TextPath((0, 0), 'X', size='12')
    patch = mpatches.PathPatch(path)
    assert patch.get_path() is path
    ax.add_artist(patch)
    assert draw_error(fig) == (
        'TypeError', "unsupported operand type(s) for /: 'str' and 'float'")


def test_split_path_inout_of_no_segment_stops():
    path = Path([(np.nan, 0.0), (np.nan, np.nan), (1.0, 1.0)], [1, 3, 3])
    with pytest.raises(StopIteration):
        bezier.split_path_inout(path, lambda xy: False)


def test_arrows_of_points_that_are_not_finite_stop_their_draw():
    fig, ax = plt.subplots()
    arrow = mpatches.FancyArrowPatch((0.1, 0.1), (0.8, 0.6), arrowstyle='-|>',
                                     mutation_scale=20)
    ax.add_patch(arrow)
    arrow.set_positions((float('nan'), 0), (1, 1))
    assert draw_error(fig) == ('StopIteration', '')
    arrow.set_positions((0, 0), (1, 1))
    fig.canvas.draw()
    fig, ax = plt.subplots()
    box = moffsetbox.AnnotationBbox(moffsetbox.TextArea('ab'), (0.5, 0.5),
                                    xybox=(0.8, 0.8), arrowprops=dict(arrowstyle='->'))
    ax.add_artist(box)
    box.xyann = (float('nan'), 0.8)
    assert draw_error(fig) == ('StopIteration', '')


def test_anchored_box_fails_on_its_child_before_its_anchor():
    fig, ax = plt.subplots()
    box = moffsetbox.AnchoredOffsetbox('upper right', child=moffsetbox.TextArea('a'))
    ax.add_artist(box)
    with pytest.raises(AttributeError):
        box.set_child('x')
    box.set_bbox_to_anchor((0.5, 0.5), transform=3)
    assert draw_error(fig) == (
        'AttributeError', "'str' object has no attribute 'get_bbox'")


# --- Host Axes whose draw failed ------------------------------------------

def test_host_axes_whose_draw_failed_lose_their_parasites_axes():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    parasite = host.twinx()
    parasite.plot(X, Y)
    fig.canvas.draw()
    host.cla()
    with pytest.raises(AttributeError, match="'_rasterizing'"):
        parasite_axes.HostAxesBase.draw(host, None)
    fig.canvas.draw()
    for _ in range(2):
        with pytest.raises(AttributeError,
                           match="'NoneType' object has no attribute '_axis_map'"):
            host.cla()
    assert draw_error(fig) == (
        'AttributeError', "'NoneType' object has no attribute 'viewLim'")


def test_axisartist_host_whose_draw_failed_fails_on_parasite_gridlines():
    fig = plt.figure()
    host = host_subplot(111, axes_class=axisartist.Axes, figure=fig)
    host.twiny()
    host.twin(transforms.Affine2D().scale(2, 1))
    with pytest.raises(AttributeError):
        parasite_axes.HostAxesBase.draw(host, 3)
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute '_axis_map'"):
        host.cla()
    assert draw_error(fig) == (
        'AttributeError', "'NoneType' object has no attribute 'get_xlim'")
