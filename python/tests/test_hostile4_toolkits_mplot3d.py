"""
What the fourth round of the differential tester's hostile programs
(``spec/diff/generators/hostile.py``, ``orders.py``) found mplot3d,
axisartist, removed artists and the core artist model of pltrs doing
otherwise than matplotlib 3.11: values kept as given and failing where
matplotlib's draw computes with them, numpy's warnings where matplotlib's
projections and aspect arithmetic warn, and the state a draw or a removal
leaves. Each expected value, error and warning is recorded from matplotlib
3.11.2.
"""

import io
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, patches, units
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axisartist import axis_artist
from pltrs.mpl_toolkits.mplot3d import art3d

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


def runtime_warnings(call):
    """The messages of the runtime warnings *call* issues."""
    return [message for category, message in caught(call) if category == "RuntimeWarning"]


def rounded(bbox):
    """The bounds of *bbox*, rounded."""
    return [round(float(value), 4) for value in bbox.bounds]


# -- units ----------------------------------------------------------------------

def test_a_category_axis_converts_no_number_through_its_converter():
    fig, ax = plt.subplots()
    ax.plot(["a", "b"], [1, 2])
    for value in (None, [None, 1]):
        with pytest.raises(units.ConversionError,
                           match=r"Failed to convert value\(s\) to axis units"):
            ax.xaxis.convert_units(value)
        with pytest.raises(units.ConversionError):
            ax.set_xticks(value)
    assert list(ax.xaxis.convert_units([])) == []
    assert list(ax.xaxis.convert_units([1.5])) == [1.5]


# -- axisartist -----------------------------------------------------------------

def test_an_axis_offset_of_no_numbers_fails_where_it_is_drawn():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    axis = ax.new_fixed_axis("left", offset="ab")
    with pytest.raises(TypeError, match=r"^affine_transform\(\): incompatible"):
        axis.offset_transform.get_matrix()
    ax.axis["new"] = axis
    with pytest.raises(TypeError, match=r"^affine_transform\(\): incompatible"):
        fig.canvas.draw()
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    axis = ax.new_fixed_axis("left", offset=[None, 1])
    assert np.isnan(axis.offset_transform.get_matrix()[0, 2])
    ax.axis["new"] = axis
    fig.canvas.draw()


@pytest.mark.parametrize("name", ["label", "major_ticklabels"])
def test_axis_parts_lay_their_font_features_out_when_drawn(name):
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    part = getattr(ax.axis["bottom"], name)
    part.set(fontfeatures=[0, 1])
    if name == "label":
        # An empty label draws no line, which matplotlib lays out.
        fig.canvas.draw()
        ax.axis["bottom"].label.set_text("x")
    with pytest.raises(TypeError, match=r"^_layout\(\): incompatible function arguments"):
        fig.canvas.draw()
    part.set(fontfeatures=None)
    fig.canvas.draw()


def test_tick_labels_keep_the_offsets_their_draw_left():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.canvas.draw()
    labels = ax.axis["bottom"].major_ticklabels
    assert labels._ref_angle == -90
    assert labels._offset_radius == pytest.approx(19.72222222222222)
    assert labels._axislabel_pad == pytest.approx(19.72222222222222)
    assert labels.get_text() == "1.0"
    extent = axis_artist.LabelBase.get_window_extent(labels)
    assert rounded(extent) == [565.0, 29.7412, 22.0, 13.8889]
    # `AxisLabel.get_window_extent` measures at the pad alone.
    assert rounded(labels.get_window_extent()) == [565.0, 39.7412, 22.0, 13.8889]
    assert labels._offset_radius == pytest.approx(9.722222222222221)


# -- snapping -------------------------------------------------------------------

def snapped_artists():
    """An Axes with a line, a text, a patch and a collection."""
    fig, ax = plt.subplots()
    return fig, {
        "line": ax.plot([0, 1], [0, 1])[0],
        "text": ax.text(0.5, 0.5, "hi"),
        "patch": ax.add_patch(patches.Rectangle((0.1, 0.1), 0.2, 0.2)),
        "collection": ax.add_collection(collections.LineCollection([[(0, 0), (1, 1)]])),
    }


@pytest.mark.parametrize("name", ["line", "text", "patch", "collection"])
def test_a_snap_no_snap_mode_casts_from_fails_where_the_artist_draws(name):
    fig, artists = snapped_artists()
    artist = artists[name]
    artist.set_snap("False")
    assert artist.get_snap() == "False"
    with pytest.raises(RuntimeError, match=r"^Unable to cast Python instance of type <class 'str'>"):
        fig.canvas.draw()
    artist.set_snap([1])
    with pytest.raises(RuntimeError, match="<class 'list'>"):
        fig.canvas.draw()
    artist.set_snap(2)
    fig.canvas.draw()
    artist.set_snap(None)
    fig.canvas.draw()


def test_snaps_of_parts_that_draw_with_their_own_context_never_fail():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.axis["bottom"].major_ticks.set_snap("False")
    fig.canvas.draw()
    ax.axis["bottom"].major_ticklabels.set_snap("False")
    with pytest.raises(RuntimeError, match="Unable to cast"):
        fig.canvas.draw()
    # A line of no data draws nothing.
    fig, ax = plt.subplots()
    line, = ax.plot([], [])
    line.set_snap("False")
    fig.canvas.draw()


# -- aspect ---------------------------------------------------------------------

IDENTICAL = ("UserWarning", "Attempting to set identical low and high xlims makes "
             "transformation singular; automatically expanding.")


def test_datalim_collapsing_the_view_warns_at_each_draw():
    fig, ax = plt.subplots()
    ax.set(aspect=1.5, adjustable="datalim", box_aspect=float("inf"))
    assert IDENTICAL in caught(fig.canvas.draw)
    assert IDENTICAL in caught(fig.canvas.draw)


def test_unknown_anchors_fail_only_where_the_box_is_shrunk():
    fig, ax = plt.subplots()
    ax.set(aspect=1.0, adjustable="datalim")
    ax.set_anchor("ne")
    assert ax.get_anchor() == "ne"
    fig.canvas.draw()
    ax.set_adjustable("box")
    with pytest.raises(KeyError, match="'ne'"):
        fig.canvas.draw()
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.set_anchor("ne")
    with pytest.raises(KeyError, match="'ne'"):
        fig.canvas.draw()
    ax.set_anchor("NE")
    fig.canvas.draw()


# -- removal --------------------------------------------------------------------

@pytest.mark.parametrize("how", ["remove", "delaxes", "clear"])
def test_a_removed_3d_axes_measures_where_it_was_drawn(how):
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot([0, 1], [0, 1], [0, 1])
    fig.canvas.draw()
    if how == "remove":
        ax.remove()
    elif how == "delaxes":
        fig.delaxes(ax)
    else:
        fig.clear()
    assert rounded(ax.get_window_extent()) == [143.2, 52.8, 369.6, 369.6]


def test_a_removed_shadow_keeps_the_offset_its_draw_gave_it():
    fig, ax = plt.subplots()
    circle = ax.add_patch(patches.Circle((0.5, 0.5), 0.2))
    shadow = ax.add_patch(patches.Shadow(circle, 0.02, -0.02))
    fig.canvas.draw()
    shadow.remove()
    shadow.set_visible(False)
    shadow.set_alpha(0.5)
    assert shadow.get_transform().get_matrix()[:2, 2] == pytest.approx(
        [341.77777777777777, 227.33333333333331])


# -- mplot3d --------------------------------------------------------------------

def axes3d():
    """A 3-D axes with a line, drawn once."""
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot([0, 1, 2], [0, 1, 0], [0, 1, 2])
    fig.canvas.draw()
    return fig, ax


PROJECTING = build_facts.as_numpy_warns(["invalid value encountered in dot",
                                         "invalid value encountered in divide"])


def test_collections_project_infinite_data_with_numpys_warnings():
    fig, ax = axes3d()
    ax.add_collection3d(collections.LineCollection([[(0, 0), (1, 1)]]),
                        zs=float("inf"))
    assert runtime_warnings(fig.canvas.draw) == PROJECTING
    fig, ax = axes3d()
    points = ax.scatter([0, 1], [0, 1], [0, 1])
    points.set_3d_properties([0, float("inf")], "z")
    assert runtime_warnings(fig.canvas.draw) == PROJECTING


def test_lines_project_infinite_data_as_nan():
    fig, ax = axes3d()
    ax.plot([0, 1], [0, 1], [0, float("inf")])
    assert runtime_warnings(fig.canvas.draw) == []


def test_a_polygon_collection_projects_its_sort_position():
    fig, ax = axes3d()
    polys = ax.add_collection3d(art3d.Poly3DCollection([[(0, 0, 0), (1, 0, 0), (1, 1, 0)]]))
    polys.set_sort_zpos(float("inf"))
    assert runtime_warnings(fig.canvas.draw) == PROJECTING
    polys.set_sort_zpos(float("nan"))
    assert runtime_warnings(fig.canvas.draw) == []


def test_an_infinite_label_pad_moves_labels_with_numpys_warnings():
    fig, ax = axes3d()
    ax.set_zlabel("z", labelpad=float("inf"))
    assert runtime_warnings(fig.canvas.draw) == build_facts.as_numpy_warns([
        "invalid value encountered in multiply", "invalid value encountered in dot",
        "invalid value encountered in multiply", "invalid value encountered in dot"])
    ax.set_zlabel("z", labelpad=float("nan"))
    assert runtime_warnings(fig.canvas.draw) == []


def test_any_box_aspect_is_kept():
    fig, ax = axes3d()
    ax.set_box_aspect((float("nan"), 2, 1))
    assert np.isnan(ax.get_box_aspect()).all()
    fig.canvas.draw()
    ax.set_box_aspect((1, 2, 1), zoom=float("nan"))
    assert np.isnan(ax.get_box_aspect()).all()
    with pytest.raises(ValueError, match=r"^Argument zoom = -1.5 must be > 0$"):
        ax.set_box_aspect((1, 2, 1), zoom=-1.5)
    assert runtime_warnings(lambda: ax.set_box_aspect((0, 0, 0))) == [
        "divide by zero encountered in scalar divide",
        "invalid value encountered in multiply"]
    ax.set_box_aspect((1, -2, 1))
    assert ax.get_box_aspect()[1] < 0
    fig.savefig(io.BytesIO())


def test_3d_axes_are_squared_animated_and_by_apply_aspect():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.set_animated(True)
    fig.canvas.draw()
    assert rounded(ax._position) == [0.2238, 0.11, 0.5775, 0.77]
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.set(subplotspec=fig.add_gridspec(2, 2)[0, 0])
    ax.apply_aspect()
    assert rounded(ax._position) == [0.1699, 0.53, 0.2625, 0.35]


def test_3d_tick_spaces_follow_the_squared_box():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.set_anchor("NE")
    ax.set(subplotspec=fig.add_gridspec(2, 2)[0, 0])
    ax.set_animated(True)
    fig.canvas.draw()
    assert ax.yaxis.get_tick_space() == 4


def test_a_tick_labels_tight_box_is_its_extent_at_its_pad():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.canvas.draw()
    labels = ax.axis["bottom"].major_ticklabels
    assert rounded(labels.get_tightbbox()) == [565.0, 39.7412, 22.0, 13.8889]


# -- lines ----------------------------------------------------------------------

def lines_and_ticks():
    """A line and the major ticks of an axis artist."""
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    return ax.plot([0, 1])[0], ax.axis["bottom"].major_ticks


@pytest.mark.parametrize("value", [None, 1, "False", np.True_])
def test_lines_keep_the_antialiasing_given(value):
    for line in lines_and_ticks():
        line.set_antialiased(value)
        assert line.get_antialiased() is value
        assert line.get_aa() is value


@pytest.mark.parametrize("style", [None, 3])
def test_a_style_of_no_dash_pattern_is_named_dashed(style):
    for line in lines_and_ticks():
        line.set_linestyle("-.")
        with pytest.raises(ValueError, match=f"^Unrecognized linestyle: {style}$"):
            line.set_linestyle(style)
        assert line.get_linestyle() == "--"
        line.set_linestyle(":")
        assert line.get_linestyle() == ":"


def test_an_alternate_marker_face_colour_of_none_is_auto():
    line, ticks = lines_and_ticks()
    line.set_markerfacecoloralt(None)
    assert line.get_markerfacecoloralt() == pytest.approx(
        (0.12156862745098039, 0.4666666666666667, 0.7058823529411765))
    ticks.set_markerfacecoloralt(None)
    assert ticks.get_markerfacecoloralt() == "auto"


# -- dates ----------------------------------------------------------------------

def test_a_rule_wrapper_keeps_its_rule_when_making_one_fails():
    import datetime
    from pltrs import dates
    rule = dates.rrulewrapper(dates.DAILY, interval=2, count=4,
                              dtstart=datetime.datetime(2024, 3, 1, 12, 30))
    with pytest.raises(TypeError, match=r"^rrule.__init__\(\) got an unexpected "
                                        r"keyword argument 'nonexistent_kw'$"):
        rule.set(interval=3, nonexistent_kw=1)
    with pytest.raises(TypeError, match="nonexistent_kw"):
        rule.set(interval=3)
    with pytest.raises(AttributeError, match="^'rrule' object has no attribute 'nope'$"):
        rule.__getattr__("nope")
    assert rule._rrule._interval == 2


# -- inherited methods, kept values ---------------------------------------------

def test_extents_matplotlib_inherits_are_named_as_its():
    from pltrs import image, offsetbox
    fig, ax = plt.subplots()
    box = ax.add_artist(offsetbox.AnchoredText("hi", loc="upper left"))
    with pytest.raises(TypeError, match=r"^OffsetBox.get_window_extent\(\) got an unexpected"):
        box.get_window_extent(nonexistent_kw=1)
    with pytest.raises(AttributeError, match="'int' object has no attribute 'points_to_pixels'"):
        box.get_window_extent(renderer=3)
    im = image.NonUniformImage(ax)
    with pytest.raises(TypeError, match=r"^AxesImage.get_window_extent\(\) got an unexpected"):
        im.get_window_extent(nonexistent_kw=1)


@pytest.mark.parametrize("point", [(), (0,), (0, 0, 0), "ab", None])
def test_axline_points_of_no_pairs_fail_where_they_are_transformed(point):
    fig, ax = plt.subplots()
    line = ax.axline((0, 0), (1, 2))
    line.set_xy1(point)
    assert line.get_xy1() == point
    with pytest.raises(TypeError, match=r"^affine_transform\(\): incompatible"):
        fig.canvas.draw()
    line.set_visible(False)
    fig.canvas.draw()
    line.set_visible(True)
    line.set_xy1((0, 1))
    fig.canvas.draw()


def test_3d_tick_labels_measure_with_the_renderer_last_given():
    fig, ax = axes3d()
    with pytest.raises(TypeError, match="cannot create weak reference to 'str' object"):
        ax.xaxis.get_tightbbox(renderer="x")
    with pytest.raises(TypeError, match="cannot create weak reference to 'str' object"):
        ax.xaxis.get_tightbbox()
    fig.canvas.draw()
    ax.xaxis.get_tightbbox()


def test_axis_parts_fail_on_missing_font_files_as_matplotlib():
    import pathlib
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    labels = ax.axis["bottom"].major_ticklabels
    labels.set_fontproperties(pathlib.Path("missing-directory") / "file")
    missing = str(pathlib.Path.cwd() / "missing-directory" / "file")
    with pytest.raises(FileNotFoundError) as caught:
        fig.canvas.draw()
    assert str(caught.value) == f"[Errno 2] No such file or directory: {missing!r}"
