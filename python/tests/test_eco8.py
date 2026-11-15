"""
What the libraries of ``cargo xtask ecosystem suite`` need of matplotlib,
beyond `test_eco7`: a contour set's argument processing read and called
(sunpy reads ``QuadContourSet._process_args``), texts measured while a
vector figure is saved at a resolution of its own (aplpy saves its PDFs
at 2.3 dpi), lines plotted in a transform whose box the program moves
afterwards (mpltern's ``transTernaryAxes``), and the stale callback an
Axes has before and after it joins its figure (ultraplot's containers ask
``super()`` for it). Each test does what a library does, without the
library. Then what ultraplot's selective draws, navigation previews,
colorbars, legends and layouts read: artists as fresh or stale after a
draw as matplotlib's, Axes left out of a draw through their
``_animated``, a canvas region restored after a later draw, 3-D Axes
made directly drawing through their class' own ``draw``, a removed 3-D
collection added back, axis setters marking the axis stale, hatches of
``None`` among others, a colormap registry put in matplotlib's place, a
class' own ``pcolormesh`` drawing ``hist2d``, a scale of the program's
autoscaling an empty view, a child Axes' original position in figure
fractions, a line's marker keeping its cap style, and axis labels
placed through setters a program gave them (ultraplot's spanning labels
follow them). The expected values
were recorded from matplotlib 3.11.2 running the same code.
"""

import io

import numpy as np
import pytest

import pltrs as mpl
import pltrs.cm as mcm
import pltrs.lines as mlines
import pltrs.pyplot as plt
import pltrs.scale as mscale
from pltrs._enums import CapStyle
from pltrs.artist import Artist
from pltrs.axes import Axes
from pltrs.colors import ListedColormap
from pltrs.markers import MarkerStyle
from pltrs.mpl_toolkits.mplot3d import Axes3D
from pltrs.ticker import MaxNLocator
from pltrs.transforms import Bbox, BboxTransformTo

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_processed_arguments_reach_contourpys_generator():
    # The generator is contourpy's where contourpy is installed (matplotlib
    # requires it, pltrs does not); pltrs' own has no `lines`.
    pytest.importorskip("contourpy")
    fig, ax = plt.subplots()
    z = np.outer(np.linspace(0, 1, 4), np.linspace(0, 1, 5))
    contours = ax.contour(z)
    contours._process_args(np.arange(5.0), np.arange(4.0) * 2, z * 2, corner_mask=True)
    lines = contours._contour_generator.lines(0.5)
    assert np.round(np.asarray(lines[0][0]), 3).tolist() == [
        [1.0, 6.0], [1.5, 4.0], [2.0, 3.0], [3.0, 2.0], [3.0, 2.0], [4.0, 1.5]]


def test_a_contour_set_processes_its_arguments_as_matplotlib_does():
    fig, ax = plt.subplots()
    z = np.outer(np.linspace(0, 1, 4), np.linspace(0, 1, 5))
    contours = ax.contour(z)
    kwargs = contours._process_args(np.arange(5.0), np.arange(4.0) * 2, z * 2,
                                    corner_mask=True, foo=1)
    assert kwargs == {"foo": 1}
    other = ax.contour(z)
    assert other._process_args(contours, bar=2) == {"bar": 2}
    assert other._contour_generator is contours._contour_generator
    with pytest.raises(ValueError, match="is not a valid value for contour.algorithm"):
        contours._process_args(z, algorithm="bad")
    with pytest.raises(TypeError, match=r"Length of x \(3\) must match"):
        contours._process_args(np.arange(3.0), np.arange(4.0), z)


class _Measure(Artist):
    """Measures a text where it is drawn, with the renderer drawing it."""

    def __init__(self, text, seen):
        super().__init__()
        self._text, self._seen = text, seen

    def draw(self, renderer):
        self._seen.append(self._text.get_window_extent(renderer).bounds)


@pytest.mark.parametrize("fmt, dpi", [("pdf", 300), ("pdf", 2.3), ("eps", 50),
                                      ("svg", 300), ("png", 50)])
def test_texts_are_measured_in_points_while_a_vector_figure_is_saved(fmt, dpi):
    fig, ax = plt.subplots(figsize=(3, 2))
    text = ax.text(0.2, 0.5, "Hello 123", fontsize=10)
    seen = []
    ax.add_artist(_Measure(text, seen))
    fig.savefig(io.BytesIO(), format=fmt, dpi=dpi)
    expected = {"pdf": (60.48, 68.878, 47.609, 10.0),
                "eps": (60.48, 68.878, 47.609, 10.0),
                "svg": (60.48, 68.878, 47.614, 10.001),
                "png": (42.0, 47.832, 31.0, 7.668)}[fmt]
    assert seen[-1] == pytest.approx(expected, abs=1e-3)


def test_a_line_follows_the_box_of_its_transform_the_program_moves():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    box = Bbox.unit()
    line, = ax.plot([0, 1], [0, 1], transform=BboxTransformTo(box) + ax.transAxes)
    box.update_from_data_xy(np.array([[0.25, 0.5], [0.75, 1.0]]), ignore=True)
    fig.canvas.draw()
    path = line.get_transform().transform(line.get_xydata())
    assert path.ravel().tolist() == pytest.approx([127.5, 148.5, 282.5, 264.0])
    drawn = line.get_path().transformed(line.get_transform()).vertices
    assert drawn.ravel().tolist() == pytest.approx([127.5, 148.5, 282.5, 264.0])


class _Container(Axes):
    """An Axes with a stale callback of its own (ultraplot's
    ``ExternalAxesContainer``), which asks its parent class for one."""

    def stale_callback(self, *args, **kwargs):
        pass


def test_an_axes_has_a_stale_callback_once_in_its_figure():
    fig = plt.figure()
    ax = Axes(fig, (0.1, 0.1, 0.8, 0.8))
    assert ax.stale_callback is None
    fig.add_axes(ax)
    assert ax.stale_callback.__name__ == "_stale_figure_callback"
    # matplotlib keeps the attribute in the instance: the class has none.
    container = _Container(fig, (0.1, 0.1, 0.4, 0.4))
    assert not hasattr(super(_Container, container), "stale_callback")


def test_a_drawn_figure_leaves_its_parts_as_fresh_as_matplotlib_does():
    fig, ax = plt.subplots()
    ax.plot([0, 1])
    fig.canvas.draw()
    # The background patches drawn with their owners, read again or not.
    assert (fig.patch.stale, ax.patch.stale) == (False, False)
    assert not [type(c).__name__ for c in fig.get_children() if c.stale]
    # Empty titles are not drawn, which leaves them stale.
    assert [type(c).__name__ for c in ax.get_children() if c.stale] == ["Text"] * 3
    ax.patch.set_facecolor("red")
    assert ax.patch.stale
    fig.canvas.draw()
    assert not ax.patch.stale


def test_a_program_drawing_an_artist_itself_leaves_it_fresh():
    fig, ax = plt.subplots()
    fig.canvas.draw()
    ax.xaxis.draw = ax.xaxis.draw  # As a mock wrapping it would.
    assert not ax.xaxis.stale


@pytest.mark.parametrize("call", [
    lambda a: a.set_tick_params(labelsize=20),
    lambda a: a._set_lim(0, 5, auto=False),
    lambda a: a.grid(True),
    lambda a: a.set_major_locator(MaxNLocator(3)),
    lambda a: a.set_label_position("top" if a.axis_name == "x" else "right"),
    lambda a: a.set_ticks_position("top" if a.axis_name == "x" else "right"),
    lambda a: a.update_units(["a", "b"]),
])
def test_axis_setters_mark_the_axis_stale(call):
    fig, ax = plt.subplots()
    fig.canvas.draw()
    for axis in (ax.xaxis, ax.yaxis):
        assert not axis.stale
        call(axis)
        assert axis.stale


def test_axes_animated_through_the_attribute_and_drawn_alone_over_a_saved_region():
    fig, axs = plt.subplots(ncols=2, figsize=(4, 2), dpi=50)
    line, = axs[0].plot([0, 1], [0, 1], lw=5)
    axs[1].plot([0, 1], [1, 0], lw=5)
    fig.canvas.draw()
    complete = np.asarray(fig.canvas.buffer_rgba()).copy()
    for ax in axs:
        ax._animated = True  # Left out of the figure's draw, not stale.
    fig.canvas.draw()
    background = fig.canvas.copy_from_bbox(fig.bbox)
    assert not np.array_equal(np.asarray(fig.canvas.buffer_rgba()), complete)
    fig.canvas.draw()  # A region saved before a draw is still restored.
    fig.canvas.restore_region(background)
    for ax in axs:
        fig.draw_artist(ax)
    assert np.array_equal(np.asarray(fig.canvas.buffer_rgba()), complete)


class _Drawn3D(Axes3D):
    """A 3-D Axes class with a ``draw`` of its own, made directly."""

    def draw(self, renderer):
        self.figure._drawn_by_class = True
        super().draw(renderer)


def test_a_3d_axes_made_directly_draws_through_its_class():
    fig = plt.figure()
    ax = _Drawn3D(fig)
    fig.add_axes(ax)
    fig.canvas.draw()
    assert fig._drawn_by_class


def test_a_removed_3d_collection_is_added_back():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    xx, yy = np.meshgrid(np.linspace(0, 1, 4), np.linspace(0, 1, 4))
    surface = ax.plot_surface(xx, yy, xx * yy)
    surface.remove()
    assert ax.add_collection(surface, autolim=False) is surface
    assert surface in ax.collections
    fig.canvas.draw()


def test_bars_keep_the_place_of_their_hatches_of_none():
    fig, ax = plt.subplots()
    bars = ax.bar(np.arange(4), [1, 2, 3, 4], hatch=[None, "//"] * 2)
    assert [bar.get_hatch() for bar in bars] == [None, "//", None, "//"]
    groups = ax.grouped_bar([[1, 2], [3, 4]], hatch=[None, "o"])
    assert [bar.get_hatch() for bar in groups.bar_containers[0]] == [None, None]


def test_a_colormap_registry_put_in_matplotlibs_place_maps_by_default():
    registry = mcm.ColormapRegistry(
        {**mcm._colormaps, "Fire": ListedColormap(["red", "yellow"], name="Fire")})
    old = mpl.colormaps, mcm._colormaps
    mpl.colormaps = mcm._colormaps = registry
    try:
        with mpl.rc_context({"image.cmap": "Fire"}):
            fig, ax = plt.subplots()
            scatter = ax.scatter([0, 1], [0, 1], c=[0, 1])
            contours = ax.contour(np.arange(9.0).reshape(3, 3), colors="k")
    finally:
        mpl.colormaps, mcm._colormaps = old
    assert scatter.get_cmap().name == "Fire"
    assert scatter.to_rgba(np.array([0.0, 1.0])).tolist() == [
        [1.0, 0.0, 0.0, 1.0], [1.0, 1.0, 0.0, 1.0]]
    assert contours.get_edgecolor().tolist()[0] == [0.0, 0.0, 0.0, 1.0]


class _OwnMesh(Axes):
    """An Axes class whose ``pcolormesh`` takes keywords of its own."""

    name = "eco8_own_mesh"

    def pcolormesh(self, *args, shade=None, **kwargs):
        self._shaded = shade
        return super().pcolormesh(*args, **kwargs)


def test_hist2d_draws_through_a_class_own_pcolormesh():
    fig = plt.figure()
    ax = _OwnMesh(fig, (0.1, 0.1, 0.8, 0.8))
    fig.add_axes(ax)
    h, xedges, yedges, mesh = ax.hist2d([0, 1, 1], [0, 1, 1], bins=2, cmin=1,
                                        shade="soft")
    assert ax._shaded == "soft"
    assert np.isnan(h).tolist() == [[False, True], [True, False]]
    assert mesh in ax.collections


class _OwnLog(mscale.LogScale):
    """A log scale of the program's (ultraplot's)."""


def test_a_program_scale_autoscales_an_empty_view():
    fig, ax = plt.subplots()
    ax.set_yscale(_OwnLog(ax.yaxis))
    assert ax.get_ylim() == pytest.approx((0.8912509381337455, 11.220184543019636))


def test_a_child_axes_keeps_its_original_position_in_figure_fractions():
    fig = plt.figure(figsize=(6, 3))
    host = fig.add_subplot(121)
    child = Axes(fig, (0.3, 0.2, 0.1, 0.5))
    host.add_child_axes(child)
    assert child.get_position(original=True).bounds == pytest.approx((0.3, 0.2, 0.1, 0.5))
    fig.canvas.draw()
    assert child.get_position(original=True).bounds == pytest.approx((0.3, 0.2, 0.1, 0.5))
    child.set_position((0.1, 0.1, 0.2, 0.2))
    assert child.get_position(original=True).bounds == pytest.approx((0.1, 0.1, 0.2, 0.2))


def test_a_lines_marker_keeps_the_cap_style_given():
    style = MarkerStyle("o", fillstyle="left", capstyle=CapStyle.round, joinstyle="round")
    line = mlines.Line2D([], [], marker=style)
    assert line._marker.get_capstyle() == "round"
    assert line._marker.get_joinstyle() == "round"


def test_an_axis_label_given_its_own_setter_is_placed_through_it():
    # ultraplot moves its spanning labels from the axis labels' ``set_y``
    # (``set_x`` of a y-axis' label), which matplotlib's draw calls.
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_xlabel("time")
    ax.tick_params(axis="x", labelrotation=90)
    label = ax.xaxis.label
    seen = []

    def set_y(self, y):
        seen.append(float(y))
        type(self).set_y(self, y)

    label.set_y = set_y.__get__(label)
    fig.canvas.draw()
    assert seen[-1] == pytest.approx(-4.2778, abs=1e-4)


def test_a_y_axis_label_turns_about_its_anchor():
    fig, ax = plt.subplots()
    label = ax.yaxis.label
    assert (label.get_ha(), label.get_va(), label.get_rotation(),
            label.get_rotation_mode()) == ("center", "bottom", 90.0, "anchor")
    assert ax.xaxis.label.get_rotation_mode() == "default"
