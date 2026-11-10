"""
What the third round of the differential tester's hostile programs
(``spec/diff/generators/hostile.py``) found mplot3d, axisartist,
axes_grid1 and the core artist model of pltrs doing otherwise than
matplotlib 3.11: errors raised where matplotlib's arithmetic raises them,
numpy's warnings where it warns, values kept as given and failing where
matplotlib computes with them, and parasites kept through a clear. Each
expected value, error and warning is recorded from matplotlib 3.11.2.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.artist import Artist
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import host_subplot
from pltrs.mpl_toolkits.mplot3d import art3d

import build_facts


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def axes3d():
    """A 3-D axes with a line, drawn once."""
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot([0, 1, 2], [0, 1, 0], [0, 1, 2])
    fig.canvas.draw()
    return fig, ax


def runtime_warnings(call):
    """The messages of the runtime warnings *call* issues."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        call()
    return [str(w.message) for w in caught if w.category is RuntimeWarning]


# -- voxels ---------------------------------------------------------------------

@pytest.mark.parametrize("kwargs, error, message", [
    (dict(facecolors=[]), ValueError,
     "operands could not be broadcast together with shapes (3,1) (0,4) "),
    (dict(facecolors=[0, 1]), ValueError, "Invalid RGBA argument: np.int64(0)"),
    (dict(edgecolors=""), ValueError, "np.str_('') is not a valid color value."),
    (dict(edgecolors="K"), ValueError, "np.str_('K') is not a valid color value."),
    (dict(edgecolors=3), TypeError, "object of type 'numpy.int64' has no len()"),
    (dict(edgecolors="none"), ValueError,
     "operands could not be broadcast together with shapes (3,1) (0,4) "),
])
def test_voxel_colours_fail_as_each_voxel_is_made(kwargs, error, message):
    _, ax = axes3d()
    count = len(ax.collections)
    with pytest.raises(error) as raised:
        ax.voxels(np.ones((2, 2, 2), bool), **kwargs)
    assert str(raised.value) == message
    assert len(ax.collections) == count
    # The view took in the corners before the first voxel was made.
    assert ax.get_xlim3d() == pytest.approx((-0.1458333, 2.1458333), abs=1e-6)


def test_voxels_take_one_colour_per_face_or_no_shading():
    _, ax = axes3d()
    faces = ax.voxels(np.ones((2, 2, 2), bool), facecolors=["r", "g", "b"])
    assert len(faces) == 8
    assert len(ax.voxels(np.ones((2, 2, 2), bool), edgecolors="none", shade=False)) == 8


def test_no_colour_shades_one_face_but_not_two():
    face = np.array([[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]])
    art3d.Poly3DCollection([face], facecolors="r", edgecolors="none", shade=True)
    with pytest.raises(ValueError, match=r"shapes \(2,1\) \(0,4\) $"):
        art3d.Poly3DCollection([face, face + 1], facecolors="none", shade=True)


# -- errorbar -------------------------------------------------------------------

X, Y, Z = np.linspace(0, 6, 7), np.sin(np.linspace(0, 6, 7)), np.arange(7.0)


@pytest.mark.parametrize("kwargs, error, message, left", [
    (dict(zerr=0.5, fmt=None), ValueError, "third arg must be a format string", 0),
    (dict(zerr=None), IndexError,
     "too many indices for array: array is 1-dimensional, but 4 were indexed", 1),
    (dict(zerr=0.5, yerr=[]), ValueError,
     "operands could not be broadcast together with shapes (7,) (0,) ", 1),
    (dict(zerr=0.5, ecolor=""), ValueError, "'' is not a valid color value.", 1),
    (dict(zerr=0.5, ecolor=3), TypeError, "object of type 'int' has no len()", 1),
    (dict(zerr=0.5, capsize="x"), TypeError,
     "'>' not supported between instances of 'str' and 'int'", 1),
])
def test_errorbar_fails_as_its_arithmetic_fails(kwargs, error, message, left):
    _, ax = axes3d()
    lines = len(ax.lines)
    with pytest.raises(error) as raised:
        ax.errorbar(X, Y, Z, **kwargs)
    assert str(raised.value) == message
    # Past the format, matplotlib has added the data line.
    assert len(ax.lines) == lines + left


def test_errorbar_sizes_that_are_strings_fail_as_numpy_multiplies_them():
    _, ax = axes3d()
    with pytest.raises(TypeError, match="The 'out' kwarg is necessary"):
        ax.errorbar(X, Y, Z, zerr="0.5")


def test_errorbar_on_dates_fails_subtracting_the_sizes():
    _, ax = axes3d()
    dates = np.datetime64("2024-01-01") + np.arange(7)
    with pytest.raises(TypeError, match="ufunc 'subtract' cannot use operands"):
        ax.errorbar(dates, Y, Z, zerr=0.5)


def test_errorbar_warns_on_infinite_sizes():
    _, ax = axes3d()
    caught = runtime_warnings(lambda: ax.errorbar(X, Y, Z, zerr=np.inf))
    assert caught == (["invalid value encountered in multiply"] * 2
                      + ["All-NaN slice encountered"] * 4)


def test_bars_of_a_string_width_fail_to_draw():
    fig, ax = axes3d()
    ax.errorbar(X, Y, Z, zerr=0.5, elinewidth="2", capthick="2")
    with pytest.raises(RuntimeError, match=r"Unable to cast Python instance of type "
                                           r"<class 'tuple'> to C\+\+ type"):
        fig.canvas.draw()


# -- the view and the status bar ------------------------------------------------

def test_format_coord_brings_back_no_number_as_none():
    _, ax = axes3d()
    assert ax.format_coord(np.nan, 0.02) == "x pane=nan, y=nan, z=nan"
    caught = runtime_warnings(lambda: ax.format_coord(np.inf, 0.02))
    assert caught == build_facts.as_numpy_warns(["invalid value encountered in dot"])
    with pytest.raises(TypeError, match="unsupported operand type"):
        ax.format_coord(None, 0.02)
    with pytest.raises(TypeError, match="unsupported operand type"):
        ax.format_zdata(None)


def test_infinite_limits_warn_as_the_world_transformation_divides():
    fig, ax = axes3d()
    ax.xaxis.set_view_interval(np.inf, -0.5, ignore=True)
    caught = runtime_warnings(lambda: ax.get_proj())
    assert caught == ["invalid value encountered in scalar divide"]


def test_infinite_points_warn_as_the_plane_check_computes():
    caught = runtime_warnings(lambda: art3d._all_points_on_plane(
        [0, 1, 2, np.inf, 0], [0, 0, 1, 1, 2], [0, 1, 0, 1, 3]))
    assert caught == ["invalid value encountered in divide"]


def test_draw_grid_fails_on_no_renderer_as_its_rasterization_does():
    _, ax = axes3d()
    with pytest.raises(AttributeError, match="has no attribute '_rasterizing'"):
        ax.xaxis.draw_grid(None)


# -- 3-D polygons ---------------------------------------------------------------

def test_polygons_count_their_faces_first():
    fig, ax = axes3d()
    poly = ax.plot_surface(*np.meshgrid(np.arange(3), np.arange(3)), np.ones((3, 3)))
    with pytest.raises(TypeError, match="object of type 'NoneType' has no len"):
        poly.set_verts(None)
    with pytest.raises(TypeError, match="not <class 'str'>"):
        poly.set_alpha("0.5")
    face = [(0, 0, 0), (1, 0, 0), (1, 1, 1)]
    poly.set_verts_and_codes([face], [])
    fig.canvas.draw()
    poly.set_verts_and_codes([face, face], [[1, 2, 2]])
    with pytest.raises(IndexError, match="list index out of range"):
        fig.canvas.draw()


# -- axes_grid1 -----------------------------------------------------------------

def test_parasites_with_no_transform_fail_as_they_compose():
    host = host_subplot(111, figure=plt.figure())
    with pytest.raises(TypeError, match="unsupported operand type"):
        host.get_aux_axes(tr=0)
    with pytest.raises(TypeError, match=r"can only concatenate str"):
        host.twin(aux_trans="x")
    with pytest.raises(AttributeError):
        host.get_aux_axes(nonexistent_kw=1)
    assert host.parasites == []


def test_clearing_a_host_keeps_its_parasites_cleared():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    par = host.twinx()
    par.plot([1, 2], [3, 4])
    host.cla()
    assert host.parasites == [par]
    assert par.lines[:] == []
    fig.canvas.draw()


# -- axisartist -----------------------------------------------------------------

def artist_ticks():
    """The major ticks of the bottom axis of an axisartist axes."""
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.canvas.draw()
    return fig, ax.axis["bottom"].major_ticks


def test_ticks_keep_an_edge_colour_their_draw_fails_on():
    fig, ticks = artist_ticks()
    ticks.set_markeredgecolor(3)
    assert ticks.get_markeredgecolor() == 3
    with pytest.raises(ValueError, match="Invalid RGBA argument: 3"):
        fig.canvas.draw()
    ticks.set_markeredgecolor("k")
    fig.canvas.draw()


def test_ticks_draw_whatever_dashes_and_marker_sampling_they_have():
    fig, ticks = artist_ticks()
    ticks.set_dashes((0,))
    ticks.set_marker("o")
    ticks.set_markevery("2")
    fig.canvas.draw()


def test_axis_artists_multiply_the_tick_label_pad_by_the_tick_visibility():
    fig, ticks = artist_ticks()
    ticks.set_visible(None)
    with pytest.raises(TypeError, match="'float' and 'NoneType'"):
        fig.canvas.draw()
    ticks.set_visible(False)
    fig.canvas.draw()


def test_lines_keep_dashes_their_graphics_context_refuses():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1], [0, 1])
    line.set_dashes((0,))
    assert (line.get_linestyle(), line.is_dashed()) == ("--", True)
    with pytest.raises(ValueError, match="At least one value in the dash list"):
        fig.canvas.draw()
    line.set_dashes([np.nan, 1])
    fig.canvas.draw()
    line.set_dashes((None, None))
    assert line.get_linestyle() == "-"
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for \+"):
        line.set_dashes("ab")


# -- the core artist model ------------------------------------------------------

def test_texts_copy_from_no_artist_as_matplotlib_reads_it():
    text = plt.figure().text(0, 0, "a")
    with pytest.raises(AttributeError, match="has no attribute '_transform'"):
        text.update_from(None)
    with pytest.raises(TypeError, match="pop expected at most 1 argument, got 2"):
        text.set_bbox([])


class _FilteredList(Artist):
    """The gallery's ``FilteredArtistList``: its artists drawn through one
    filter."""

    def __init__(self, artists, image_filter):
        super().__init__()
        self._artists = artists
        self._filter = image_filter

    def draw(self, renderer):
        renderer.start_rasterizing()
        renderer.start_filter()
        for artist in self._artists:
            artist.draw(renderer)
        renderer.stop_filter(self._filter)
        renderer.stop_rasterizing()


def test_artists_drawn_by_a_program_go_through_their_filter_once():
    fig, ax = plt.subplots()
    calls = []

    def light(image, dpi):
        calls.append("light")
        return image, 0, 0

    wedges = ax.pie([15, 30, 45, 10]).wedges
    for wedge in wedges:
        wedge.set_agg_filter(light)
    ax.add_artist(_FilteredList(wedges, lambda image, dpi: (image, 0, 0)))
    fig.canvas.draw()
    # Once drawn by the axes, once drawn in the list.
    assert len(calls) == 2 * len(wedges)


def test_axis_artist_labels_look_their_fonts_up_as_matplotlib_draws_them(caplog):
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.canvas.draw()
    ax.axis["bottom"].major_ticklabels.set(fontfamily="")
    counts = []
    for _ in range(2):
        caplog.clear()
        fig.canvas.draw()
        counts.append(sum("Font family '' not found" in r.getMessage()
                          for r in caplog.records))
    # Measured (once), laid out and drawn: six labels.
    assert counts == [18, 12]


# -- sankey ---------------------------------------------------------------------

def test_sankey_keeps_the_angles_matplotlib_turns():
    from pltrs.sankey import Sankey
    sankey = Sankey(ax=plt.figure().add_subplot(), unit="W")
    sankey.add(rotation=45)
    sankey.add()
    sankey.add(rotation=0, connect=())
    angles = [diagram.angles for diagram in sankey.diagrams]
    assert angles == [[0.5, 0.5], [0, 0], [0, 0]]
    assert [type(a) for a in angles[0] + angles[1]] == [float, float, int, int]
    with pytest.raises(ValueError, match="expected a finite input, got inf"):
        sankey.add(rotation=np.inf)
    with pytest.raises(TypeError, match="ufunc 'add' did not contain a loop"):
        sankey.add(labels=[0, 1])
    with pytest.raises(ValueError, match="zero-size array to reduction"):
        sankey.add(flows=[])
    assert len(sankey.diagrams) == 3
