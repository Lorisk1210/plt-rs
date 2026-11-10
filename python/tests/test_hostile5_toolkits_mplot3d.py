"""
What the fifth round of the differential tester's hostile programs
(``spec/diff/generators/hostile.py``) found axisartist, axes_grid1,
mplot3d, offset boxes, Sankey diagrams and inset indicators of pltrs doing
otherwise than matplotlib 3.11: where Agg rounds the positions of glyphs,
what constructors read first of what they are given, values kept as given
and failing where matplotlib's draws compute with them, and what a failed
call leaves. Each expected value, error and warning is recorded from
matplotlib 3.11.2.
"""

import io
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, offsetbox, sankey
from pltrs.axes import Axes
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import axes_divider, host_axes, inset_locator, parasite_axes
from pltrs.mpl_toolkits.axisartist import axis_artist, axislines, grid_helper_curvelinear
from pltrs.mpl_toolkits.mplot3d import Axes3D, art3d, axes3d, axis3d
from pltrs.transforms import Affine2D

import build_facts


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def caught(call):
    """``(category name, message)`` of each warning *call* issues."""
    with warnings.catch_warnings(record=True) as seen:
        warnings.simplefilter("always")
        call()
    return [(w.category.__name__, str(w.message)) for w in seen]


def failure(call):
    """``(type name, message)`` of what *call* raises, with its warnings."""
    with warnings.catch_warnings(record=True) as seen:
        warnings.simplefilter("always")
        try:
            call()
        except Exception as error:
            raised = (type(error).__name__, str(error))
        else:
            raised = None
    return raised, [str(w.message) for w in seen if w.category is RuntimeWarning]


def pixels(fig):
    """The figure drawn by Agg, as an array."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


# -- Agg rounds the positions of glyphs ------------------------------------------

NAN_TO_INTEGER = ("ValueError", "cannot convert float NaN to integer")


@pytest.mark.parametrize("prop, value, warned", [
    ("linespacing", float("nan"), []),
    ("linespacing", float("inf"), []),
    ("rotation", float("nan"), ["invalid value encountered in cast"]),
    ("rotation", float("inf"), ["invalid value encountered in cast"]),
])
def test_texts_placed_at_no_number_fail_where_agg_rounds_them(prop, value, warned):
    fig, ax = plt.subplots()
    ax.text(0.5, 0.5, "hello", **{prop: value})
    assert failure(fig.canvas.draw) == (NAN_TO_INTEGER, warned)
    # The PDF writer refuses them, the SVG and PostScript ones write them.
    with pytest.raises(ValueError, match="^Can only output finite numbers in PDF$"):
        fig.savefig(io.BytesIO(), format="pdf")
    fig.savefig(io.BytesIO(), format="svg")


def test_a_draw_ends_at_the_first_text_agg_cannot_place():
    fig, ax = plt.subplots()
    ax.tick_params(labelrotation=float("nan"))
    assert failure(fig.canvas.draw) == (NAN_TO_INTEGER, ["invalid value encountered in cast"])


@pytest.mark.parametrize("prop, value", [
    ("linespacing", float("nan")), ("linespacing", float("inf")),
    ("rotation", float("nan")),
])
def test_axis_artist_labels_fail_where_agg_rounds_them(prop, value):
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.axis["left"].label.set_text("radius")
    ax.axis["left"].label.set(**{prop: value})
    assert failure(fig.canvas.draw)[0] == NAN_TO_INTEGER


def test_infinite_tick_sizes_warn_as_numpy_scales_and_turns_the_marks():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.axis["bottom"].major_ticks.set_ticksize(float("inf"))
    multiply = "invalid value encountered in scalar multiply"
    dot = "invalid value encountered in dot"
    # Six ticks, each mark turned through two products.
    assert failure(fig.canvas.draw) == (
        None, build_facts.as_numpy_warns([multiply] * 4 + [dot] * 12))


# -- what constructors read first ------------------------------------------------

@pytest.mark.parametrize("make", [
    lambda fig: Axes(fig, [0.1, 0.1, 0.8, 0.8]),
    lambda fig: Axes3D(fig),
    lambda fig: host_axes([0.1, 0.1, 0.8, 0.8], figure=fig),
])
@pytest.mark.parametrize("fig, name", [(3, "int"), ("x", "str")])
def test_axes_made_in_no_figure_fail_reading_its_transform(make, fig, name):
    with pytest.raises(AttributeError,
                       match=f"^'{name}' object has no attribute 'transSubfigure'$"):
        make(fig)
    if fig == 3:
        with pytest.raises(AttributeError, match="^'NoneType' object has no "
                                                 "attribute 'transSubfigure'$"):
            Axes3D(None)


@pytest.mark.parametrize("make", [
    lambda parent: parasite_axes.ParasiteAxes(parent),
    lambda axes: axis3d.XAxis(axes),
    lambda axes: axis3d.Axis(axes),
])
def test_parts_of_no_axes_fail_reading_its_figure(make):
    with pytest.raises(AttributeError,
                       match="^'NoneType' object has no attribute 'get_figure'$"):
        make(None)


def test_3d_axis_constructors_take_matplotlibs_signatures():
    with pytest.raises(TypeError, match=r"^Axis\._new_init\(\) missing 1 required "
                                        r"positional argument: 'axes'$"):
        axis3d.XAxis()
    with pytest.raises(TypeError, match=r"^Axis\._new_init\(\) takes 2 positional "
                                        r"arguments but 3 were given$"):
        axis3d.ZAxis(None, None)


def test_axis_artists_take_no_properties_and_read_the_helper_first():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    with pytest.raises(AttributeError,
                       match="^'NoneType' object has no attribute 'get_line_transform'$"):
        axis_artist.AxisArtist(ax, None)
    helper = axislines.AxisArtistHelperRectlinear.Fixed(ax, "bottom")
    with pytest.raises(TypeError, match=r"^Artist\.__init__\(\) got an unexpected "
                                        r"keyword argument 'nonexistent_kw'$"):
        axis_artist.AxisArtist(ax, helper, nonexistent_kw=1)


@pytest.mark.parametrize("cls", [axes_divider.SubplotDivider, axes_divider.HBoxDivider,
                                 axes_divider.VBoxDivider])
def test_dividers_check_their_anchor_before_their_subplot(cls):
    fig = plt.figure()
    with pytest.raises(ValueError, match="^'ne' is not a valid value for anchor"):
        cls(fig, anchor="ne")
    with pytest.raises(TypeError, match="^anchor must be str or 2-tuple$"):
        cls(fig, anchor=None)


def test_3d_axes_keep_view_angles_of_no_number_until_they_project():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d", elev=None)
    assert (ax.elev, ax.azim, ax.roll, ax.initial_elev) == (None, -60, 0, None)
    with pytest.raises(TypeError, match="^loop of ufunc does not support argument 0 "
                                        "of type NoneType"):
        fig.canvas.draw()
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d", azim="30")
    assert ax.azim == "30"
    with pytest.raises(TypeError, match="^ufunc 'deg2rad' not supported"):
        fig.canvas.draw()
    assert Axes3D(plt.figure(), roll=None).roll is None


def test_3d_axes_made_at_a_rectangle_of_no_numbers_keep_it():
    fig = plt.figure()
    ax = Axes3D(fig, rect=(float("nan"), 0, 1, 0.9))
    assert np.isnan(ax.get_position().x0) and ax.get_position().y1 == 0.9


@pytest.mark.parametrize("delta, error, message", [
    (0.0, ZeroDivisionError, "^division by zero$"),
    (float("nan"), ValueError, "^arange: cannot compute length$"),
    ("0.05", TypeError, r"^arange\(\) not supported for inputs"),
])
def test_test_data_refuses_steps_as_numpy_arange(delta, error, message):
    with pytest.raises(error, match=message):
        axes3d.get_test_data(delta)


def test_test_data_takes_none_for_a_step_of_one():
    X, Y, Z = axes3d.get_test_data(None)
    assert X.shape == (6, 6)
    assert np.isclose(Z.sum(), -66.93398809494846)


# -- 2-D collections lifted to 3-D -----------------------------------------------

def test_lifting_what_is_no_collection_fails_as_matplotlib():
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'_depthshade'"):
        art3d.patch_collection_2d_to_3d(None)
    with pytest.raises(AttributeError,
                       match="^'NoneType' object has no attribute 'get_paths'$"):
        art3d.poly_collection_2d_to_3d(None)


def test_3d_scatter_sorts_its_sizes_as_given():
    x = np.linspace(0, 6, 7)

    def drawn(sizes):
        fig = plt.figure()
        markers = fig.add_subplot(projection="3d").scatter(x, np.sin(x), x)
        markers.set_sizes(sizes)
        return failure(fig.canvas.draw)[0]

    scalar_index = "only integer scalar arrays can be converted to a scalar index"
    assert drawn([10, 40]) == ("TypeError", scalar_index)
    assert drawn((10,) * 7) == ("TypeError", scalar_index)
    assert drawn(None) == ("TypeError", "object of type 'NoneType' has no len()")
    assert drawn(np.array([10, 40])) == ("IndexError",
                                         "index 2 is out of bounds for axis 0 with size 2")
    assert drawn(np.full(7, 10)) is None


def test_sizes_that_cannot_be_counted_are_kept():
    markers = collections.PathCollection([], sizes=[10, 20])
    with pytest.raises(TypeError, match="^len\\(\\) of unsized object$"):
        markers.set_sizes("ab")
    sizes = markers.get_sizes()
    assert sizes.shape == () and sizes.dtype == np.dtype("<U2")


# -- the background of 3-D axes, hidden inset indicators --------------------------

def test_the_background_of_3d_axes_is_their_patch():
    fig = plt.figure(facecolor="red")
    ax = fig.add_subplot(projection="3d")
    ax.set_facecolor("none")
    assert tuple(pixels(fig)[240, 320]) == (240, 119, 119, 255)


def test_hidden_inset_indicators_are_drawn():
    def drawn(change):
        fig, ax = plt.subplots()
        indicator = ax.indicate_inset_zoom(ax.inset_axes([0.6, 0.6, 0.3, 0.3]))
        change(indicator)
        return pixels(fig)

    shown = drawn(lambda indicator: None)
    assert np.array_equal(drawn(lambda indicator: indicator.set_visible(False)), shown)
    assert not np.array_equal(drawn(lambda indicator: indicator.remove()), shown)


# -- what a failed call leaves --------------------------------------------------

def test_a_sankey_diagram_failing_on_its_patch_leaves_its_label():
    fig, ax = plt.subplots()
    diagram = sankey.Sankey(ax=ax, unit="W")
    for properties, error in [({"nonexistent_kw": 1}, AttributeError),
                              ({"alpha": "x"}, TypeError)]:
        with pytest.raises(error):
            diagram.add(**properties)
    assert len(ax.patches) == 0 and [t.get_text() for t in ax.texts] == ["", ""]
    diagram.add(label="ok")
    assert len(ax.patches) == 1
    assert [t.get_text() for t in ax.texts] == ["", "", "", "1W", "1W"]
    # The colours of the cycle the failed diagrams took are gone.
    assert ax.patches[0].get_facecolor()[:3] == pytest.approx(
        (0.17254901960784313, 0.6274509803921569, 0.17254901960784313))


def test_a_grid_finder_update_failing_after_its_transform_keeps_it():
    def drawn(fail):
        fig = plt.figure()
        helper = grid_helper_curvelinear.GridHelperCurveLinear(Affine2D())
        fig.add_subplot(axes_class=axisartist.Axes, grid_helper=helper)
        properties = {"nonexistent_kw": 1} if fail else {}
        try:
            helper.update_grid_finder(aux_trans=Affine2D().scale(2), **properties)
        except ValueError as error:
            assert str(error) == "Unknown update property 'nonexistent_kw'"
        return pixels(fig)

    assert np.array_equal(drawn(True), drawn(False))



# -- 3-D plotting takes the units of its data ------------------------------------

def test_3d_plots_take_the_units_of_their_data():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot(np.array([False, True]), [1, 2], [0, 1])
    ax.plot(np.datetime64("2024-01-01") + np.arange(2), [1, 2], [0, 1])
    # Category names replace the dates, and the line of booleans fails to
    # convert through them.
    with pytest.raises(pltrs.units.ConversionError,
                       match=r"^Failed to convert value\(s\) to axis units: "
                             r"array\(\[False,  True\]\)$"):
        with pytest.warns(UserWarning, match="^This axis already has a converter set"):
            ax.plot(np.array(["a", "b"]), [1, 2], [0, 1])
    assert type(ax.xaxis.get_converter()).__name__ == "StrCategoryConverter"
    assert len(ax.lines) == 2


def test_3d_scatter_converts_its_data_through_the_units_taken():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot(np.datetime64("2024-01-01") + np.arange(2), [1, 2], [0, 1])
    with pytest.raises(pltrs.units.ConversionError,
                       match=r"^Failed to convert value\(s\) to axis units: "
                             r"array\(\['a', 'b'\]"):
        ax.scatter(np.array(["a", "b"]), [1, 2], [0, 1])


def test_3d_texts_turn_towards_their_direction_beyond_the_view():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    label = ax.text(0.5, 0.5, 0.5, "label", zdir="x")
    label.set_3d_properties(1, axlim_clip=True)
    fig.canvas.draw()


# -- what draws of axis artist texts that failed leave ----------------------------

@pytest.mark.filterwarnings("ignore:invalid value encountered in cast:RuntimeWarning")
@pytest.mark.parametrize("prop, bad", [("fontfeatures", [0, 1]),
                                       ("rotation", float("nan"))])
def test_failed_draws_of_tick_labels_leave_them_moved(prop, bad):
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    labels = ax.axis["bottom"].major_ticklabels
    fig.canvas.draw()
    assert [round(b.y0, 3) for b in labels.get_window_extents()][:2] == [29.741, 29.741]
    good = getattr(labels, f"get_{prop}")()
    labels.set(**{prop: bad})
    for _ in range(2):
        with pytest.raises((TypeError, ValueError)):
            fig.canvas.draw()
    labels.set(**{prop: good})
    fig.canvas.draw()
    assert [round(b.y0, 3) for b in labels.get_window_extents()][:2] == [-9.703, -9.703]


# -- parasites, insets and collections --------------------------------------------

def test_parasites_never_call_their_own_locator():
    fig = plt.figure()
    host = parasite_axes.host_subplot(111, figure=fig)
    parasite = host.twin()
    parasite.set_axes_locator("x")
    assert parasite.get_axes_locator() is None
    fig.canvas.draw()


def test_marking_an_inset_makes_its_outline_with_the_properties_first():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match=r"^BboxPatch\.set\(\) got an unexpected "
                                             r"keyword argument 'nonexistent_kw'$"):
        inset_locator.mark_inset(ax, ax.inset_axes([0.6, 0.6, 0.3, 0.3]), 1, 3,
                                 nonexistent_kw=1)
    assert len(ax.patches) == 0


@pytest.mark.parametrize("make, error, message", [
    (lambda: collections.LineCollection([None]), ValueError, r"shape \(\)$"),
    (lambda: collections.PolyCollection([None]), TypeError,
     r"^object of type 'NoneType' has no len\(\)$"),
    (lambda: collections.PolyCollection([[0, 1, 2, 3]]), ValueError, r"shape \(5,\)$"),
    (lambda: collections.PolyCollection([[]], closed=False), ValueError,
     r"shape \(0,\)$"),
    (lambda: collections.LineCollection([[(0, 0), (1, 1)]]).set_paths([None]),
     ValueError, r"shape \(\)$"),
])
def test_collections_refuse_vertices_as_their_paths_do(make, error, message):
    with pytest.raises(error, match=message):
        make()
